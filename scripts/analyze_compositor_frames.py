"""Join verified chart commits to the next monitor render and output submission."""
import argparse
import bisect
import json
from pathlib import Path


def correlate(observer, frames, client_pid, monitor_id):
    if not observer['complete'] or observer['metadata'].get('client_pid') != client_pid:
        raise ValueError('complete observer with acknowledged chart identity required')
    events = sorted(observer['events'], key=lambda x: x['time_ns'])
    if any(x['raw']['kind'] != 9 for x in events):
        raise ValueError('lost or unknown compositor records')
    surfaces = {x['surface'] for x in frames}
    if len(surfaces) != 1:
        raise ValueError('expected one native chart surface')
    chart = [x for x in events if x.get('owner_pid') == client_pid and x.get('wire_id') in surfaces]
    first = frames[0]['commit_monotonic_bounds_ns'][0]
    schedules = [x['time_ns'] for x in chart if x['probe'] == 'schedule' and x['time_ns'] >= first]
    applies = [x['time_ns'] for x in chart if x['probe'] == 'commit' and x['time_ns'] >= first]
    if len(schedules) != len(frames) or len(applies) != len(frames):
        raise ValueError('chart state count does not match native frame count')
    stacks = {'render': [], 'output': []}
    paired = {'render': [], 'output': []}
    for event in events:
        name = event['probe']
        if name in {'render_begin', 'output_commit'}:
            stacks['render' if name == 'render_begin' else 'output'].append(event)
        elif name in {'render_end', 'output_done'}:
            kind = 'render' if name == 'render_end' else 'output'
            if not stacks[kind]:
                raise ValueError('unpaired compositor return')
            begin = stacks[kind].pop()
            if 'monitor_id' not in begin:
                raise ValueError('monitor identity missing')
            record = dict(start_ns=begin['time_ns'], end_ns=event['time_ns'], monitor_id=begin['monitor_id'])
            if kind == 'output':
                result = event['register_value'] & 255  # SysV bool return is AL, not all of RAX.
                if result not in [0, 1]:
                    raise ValueError('invalid output commit return ABI')
                record['ok'] = bool(result)
            if begin['monitor_id'] == monitor_id:
                paired[kind].append(record)
    if any(stacks.values()):
        raise ValueError('missing compositor return')
    for records in paired.values():
        records.sort(key=lambda x: x['start_ns'])
    render_times = [x['start_ns'] for x in paired['render']]
    output_times = [x['start_ns'] for x in paired['output']]
    joined = []
    for frame, schedule, apply in zip(frames, schedules, applies):
        lo, hi = frame['commit_monotonic_bounds_ns']
        if schedule < lo or apply < schedule:
            raise ValueError('invalid chart commit ordering')
        try:
            render = paired['render'][bisect.bisect_left(render_times, apply)]
            output = paired['output'][bisect.bisect_left(output_times, render['start_ns'])]
        except IndexError as error:
            raise ValueError('missing subsequent monitor render or output') from error
        if output['end_ns'] > render['end_ns']:
            raise ValueError('next render did not contain the selected output submission')
        presented = frame['presented_ns']
        if presented is not None and presented < output['end_ns']:
            raise ValueError('presentation predates selected output submission')
        joined.append(dict(frame=frame['frame'], phase=frame['phase'], warmup=frame['warmup'],
                           work_ns=frame['work_ns'], presentation_ns=frame['presentation_ns'],
                           receipt_ns=schedule, apply_ns=apply, render=render, output=output,
                           client_to_receipt_bounds_ns=[schedule-hi, schedule-lo],
                           receipt_to_apply_ns=apply-schedule,
                           apply_to_render_ns=render['start_ns']-apply,
                           render_to_submit_ns=output['start_ns']-render['start_ns'],
                           submit_call_ns=output['end_ns']-output['start_ns'],
                           submit_to_present_ns=presented-output['end_ns'] if presented is not None else None))
    return dict(diagnostic_only=True, client_pid=client_pid, surface=next(iter(surfaces)),
                monitor_id=monitor_id, frames=joined,
                limitation='Joins the first monitor render after state application; output return is submission, not GPU completion. Actual presentation remains native protocol feedback.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('run', type=Path)
    parser.add_argument('observer', type=Path)
    parser.add_argument('--monitor-id', type=int, required=True)
    args = parser.parse_args()
    frames = json.loads((args.run / 'analysis.json').read_text())['frames']
    manifest = json.loads((args.run / 'manifest.json').read_text())
    observer = json.loads((args.observer / 'events.json').read_text())
    print(json.dumps(correlate(observer, frames, manifest['pid'], args.monitor_id), indent=2))


if __name__ == '__main__':
    main()
