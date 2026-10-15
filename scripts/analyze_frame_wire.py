"""Join diagnostic Wayland requests, surface commits and actual presentation feedback."""
import argparse
import csv
import json
from pathlib import Path
import re


def parse_wire(lines):
    requests, active, pending, buffers = [], {}, {}, {}
    day = previous = 0
    for line in lines:
        stamp = re.match(r'\[(\d\d):(\d\d):(\d\d)\.(\d{6})\]', line)
        if not stamp:
            continue
        h, m, s, us = map(int, stamp.groups())
        now = ((h * 60 + m) * 60 + s) * 1_000_000_000 + us * 1000
        if previous - now > 12 * 3600 * 1_000_000_000:
            day += 24 * 3600 * 1_000_000_000
        previous = now
        now += day
        request = re.search(r'wp_presentation#(\d+)\.feedback\(wl_surface#(\d+), new id wp_presentation_feedback#(\d+)\)', line)
        if request:
            presentation, surface, feedback = map(int, request.groups())
            if feedback in active:
                raise ValueError('feedback identity reused before completion')
            record = dict(surface=surface, presentation=presentation, feedback=feedback, request_wire_ns=now)
            requests.append(record)
            active[feedback] = record
            pending.setdefault(surface, []).append(record)
        attach = re.search(r'wl_surface#(\d+)\.attach\(wl_buffer#(\d+)', line)
        if attach:
            surface, buffer = map(int, attach.groups())
            buffers[surface] = buffer
        commit = re.search(r'wl_surface#(\d+)\.commit\(\)', line)
        if commit:
            surface = int(commit[1])
            committed = pending.pop(surface, [])
            for record in committed:
                record.update(commit_wire_ns=now,
                              shared_commit_requests=sum(r['presentation']==record['presentation'] for r in committed),
                              buffer=buffers.get(surface))
        feedback = re.search(r'wp_presentation_feedback#(\d+)\.(presented|discarded)\(([^)]*)\)', line)
        if feedback:
            record = active.pop(int(feedback[1]), None)
            if record is None:
                raise ValueError('orphan or duplicate feedback')
            if feedback[2] == 'discarded':
                record['presented_ns'] = None
            else:
                values = [int(v.strip()) for v in feedback[3].split(',')]
                if len(values) != 7:
                    raise ValueError('unexpected presentation payload')
                record['presented_ns'] = ((values[0] << 32) | values[1]) * 1_000_000_000 + values[2]
    return requests


def overlap(intervals, start, end, preempt_only=False):
    return sum(max(0, min(end, r['end_ns']) - max(start, r['start_ns']))
               for r in intervals if not preempt_only or r['preempt'])


def correlate(wire, frames, requests, schedule):
    if not wire or len(wire) != len(frames) or len(requests) != len(frames):
        raise ValueError('missing frame/request identities')
    lower, upper = [], []
    for i, (record, frame, request) in enumerate(zip(wire, frames, requests)):
        if frame['frame'] != i or request['frame'] != i:
            raise ValueError('frame identity mismatch')
        if 'presentation' in request and request['presentation'] != record['presentation']:
            raise ValueError('presentation channel identity mismatch')
        if not frame['started_ns'] <= request['begin_ns'] <= request['end_ns'] <= frame['started_ns'] + frame['work_ns']:
            raise ValueError('request lies outside native frame work')
        lower.append(request['begin_ns'] - record['request_wire_ns'] - 1000)
        upper.append(request['end_ns'] - record['request_wire_ns'])
    offset_low, offset_high = max(lower), min(upper)
    if offset_low > offset_high:
        raise ValueError('wire and monotonic clocks have no common offset within request brackets')
    joined = []
    for record, frame in zip(wire, frames):
        if record.get('shared_commit_requests') != 1 or 'presented_ns' not in record:
            raise ValueError('missing commit/feedback or multiple frame requests in one surface commit')
        if record['presented_ns'] != frame['presented_ns']:
            raise ValueError('protocol and frame presentation disagree')
        if record['commit_wire_ns'] < record['request_wire_ns']:
            raise ValueError('surface committed before its request')
        commit_low = record['commit_wire_ns'] + offset_low
        commit_high = record['commit_wire_ns'] + offset_high + 1000
        presented = frame['presented_ns']
        if presented is not None and presented < commit_low:
            raise ValueError('presentation predates surface commit')
        end = frame['started_ns'] + frame['work_ns']
        joined.append(dict(frame=frame['frame'], phase=frame['phase'], warmup=frame['warmup'],
                           work_ns=frame['work_ns'], work_cpu_ns=frame['work_cpu_ns'],
                           presented_ns=presented, discarded=frame['discarded'], resolved=frame['resolved'],
                           presentation_ns=presented-frame['started_ns'] if presented is not None else None,
                           request_to_commit_ns=record['commit_wire_ns']-record['request_wire_ns'],
                           commit_monotonic_bounds_ns=[commit_low, commit_high],
                           commit_to_presentation_bounds_ns=[presented-commit_high, presented-commit_low] if presented is not None else None,
                           client_off_cpu_work_ns=overlap(schedule['intervals'], frame['started_ns'], end),
                           compositor_preempt_commit_to_present_ns=overlap(schedule.get('compositor', {}).get('intervals', []), commit_high, presented, True) if presented is not None else None,
                           surface=record['surface'], buffer=record['buffer']))
    return dict(diagnostic_only=True, wire_clock_offset_bounds_ns=[offset_low, offset_high],
                frames=joined,
                limitation='Commit log is a client-side request, not compositor receipt or GPU completion; compositor sleep alone does not establish delay causality.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    def rows(name):
        with (args.directory / 'frames' / name).open() as file:
            return list(csv.DictReader(file))
    frames = []
    for raw in rows('frames.csv'):
        frame = {k: int(raw[k]) for k in ['frame', 'started_ns', 'work_ns', 'work_cpu_ns']}
        frame.update(phase=raw['phase'], warmup=raw['warmup']=='true', discarded=raw['discarded']=='true',
                     resolved=raw['resolved']=='true', presented_ns=int(raw['presented_ns']) if raw['presented_ns'] else None)
        frames.append(frame)
    requests = [{k:int(v) for k,v in r.items()} for r in rows('requests.csv')]
    wire = parse_wire((args.directory / 'process.log').read_text().splitlines())
    schedule = json.loads((args.directory / 'manifest.json').read_text())
    expected = {r['presentation'] for r in requests if 'presentation' in r}
    candidates = expected or {r['presentation'] for r in wire}
    results = []
    for candidate in sorted(candidates):
        try:
            result = correlate([r for r in wire if r['presentation']==candidate], frames, requests, schedule)
        except ValueError:
            continue
        result['presentation_object'] = candidate
        results.append(result)
    if len(results) != 1:
        raise ValueError(f'expected one request channel matching native brackets and feedback, found {len(results)}')
    result = results[0]
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
