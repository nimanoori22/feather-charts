"""Summarize diagnostic real-Iced traces without issuing acceptance decisions."""
import argparse
import bisect
from collections import defaultdict
import csv
import json
from pathlib import Path
import statistics


def stats(values):
    values = sorted(values)
    return dict(count=len(values), median_ns=statistics.median(values) if values else None,
                p95_ns=values[int(len(values)*.95)] if values else None,
                max_ns=max(values) if values else None)


def summarize(rows):
    if sum(r['stage'] == 'diagnostic_only' and r['a'] == 1 for r in rows) != 1:
        raise ValueError('diagnostic marker missing or duplicated')
    overflow = [r['a'] for r in rows if r['stage'] == 'overflow']
    if overflow != [0]:
        raise ValueError('trace overflow or missing completeness marker')
    if {r['a'] for r in rows if r['stage'] == 'clock_id'} != {1}:
        raise ValueError('trace must use matched Linux CLOCK_MONOTONIC')
    submitted, feedback, stages, sizes = {}, {}, defaultdict(list), {}
    for row in rows:
        if row['end_ns'] < row['start_ns'] or row['cpu_ns'] < 0:
            raise ValueError('invalid span clocks')
        stage = row['stage']
        if stage == 'submit':
            if row['id'] in submitted:
                raise ValueError('duplicate submission')
            submitted[row['id']] = row
        elif stage in ['presented', 'discarded']:
            if row['id'] in feedback:
                raise ValueError('duplicate feedback')
            feedback[row['id']] = row
        elif stage == 'physical_size':
            sizes[row['id']] = (row['a'], row['b'])
        elif row['end_ns'] > row['start_ns']:
            stages[stage].append(row)
    if not submitted or not set(feedback).issubset(submitted):
        raise ValueError('missing submission or orphan feedback')
    latency, by_size, frames = [], defaultdict(list), []
    discarded = unresolved = 0
    for frame, submit in sorted(submitted.items()):
        result = feedback.get(frame)
        delay = None
        state = 'unresolved'
        if result is None:
            unresolved += 1
        elif result['stage'] == 'discarded':
            discarded += 1
            state = 'discarded'
        else:
            if not submit['start_ns'] <= result['a'] <= result['start_ns']:
                raise ValueError('presentation clock does not match submission/arrival')
            delay = result['a'] - submit['start_ns']
            latency.append(delay)
            state = 'presented'
        frames.append(dict(frame=frame, state=state, submission_to_presentation_ns=delay,
                           physical_size=sizes.get(frame)))
        if delay is not None:
            by_size[str(sizes.get(frame))].append(delay)
    inputs = sorted(r['start_ns'] for r in rows if r['stage'].startswith('input_'))
    submits = sorted((r['start_ns'], frame) for frame, r in submitted.items())
    times = [t for t, _ in submits]
    candidates = []
    for at in inputs:
        next_index = bisect.bisect_left(times, at)
        if next_index < len(times):
            result = feedback.get(submits[next_index][1])
            if result is not None and result['stage'] == 'presented':
                candidates.append(result['a'] - at)
    return dict(diagnostic_only=True, submitted=len(submitted), discarded=discarded,
                unresolved=unresolved,
                submission_to_presentation_p95_ns=None if discarded or unresolved else stats(latency)['p95_ns'],
                resolved_latency=stats(latency), resolved_latency_by_size={k: stats(v) for k,v in by_size.items()},
                stages={stage: dict(wall=stats([r['end_ns']-r['start_ns'] for r in records]),
                                    cpu=stats([r['cpu_ns'] for r in records]))
                        for stage,records in stages.items()},
                input_to_next_surface_candidate=stats(candidates),
                candidate_limit='First subsequent submission; not proof the frame contains that input, not device-to-photon latency.',
                frames=frames)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('trace', type=Path)
    args = parser.parse_args()
    with args.trace.open() as f:
        rows = [dict(stage=r['stage'], **{k:int(v) for k,v in r.items() if k!='stage'})
                for r in csv.DictReader(f)]
    print(json.dumps(summarize(rows), indent=2, allow_nan=False))


if __name__ == '__main__':
    main()
