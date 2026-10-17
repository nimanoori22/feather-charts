"""Join per-series Histogram work to the same measured slow frames."""
import argparse
import csv
import json
from pathlib import Path

from analyze_frame_phases import digest, parse_phases, summarize, read_trace


def read_counts(path, samples, warmups=20):
    with path.open() as f:
        rows = list(csv.DictReader(f))
    if len(rows) != (samples + warmups) * 16:
        raise ValueError('Incomplete per-series work capture')
    frames = [[] for _ in range(samples + warmups)]
    for index, row in enumerate(rows):
        fields = {k: int(row[k]) for k in ['sample', 'series', 'refreshes', 'preparations', 'points']}
        if fields['sample'] != index // 16 or fields['series'] != index % 16 + 1 or min(fields.values()) < 0:
            raise ValueError('Invalid work count/order/identity')
        frames[index // 16].append(tuple(fields[k] for k in ['refreshes', 'preparations', 'points']))
    return frames


def compare_work(values, frames, warmups=20):
    stats = summarize(values)
    count = len(values['snapshot'])
    if len(frames) != count + warmups:
        raise ValueError('Count/timing sample mismatch')
    tail = set(stats['tail_samples'])
    groups, frame_patterns = {}, []
    for i, frame in enumerate(frames[warmups:]):
        key = tuple(frame)
        if key not in groups:
            groups[key] = dict(pattern=len(groups), per_series=frame, totals=[sum(v) for v in zip(*frame)],
                               tail_count=0, normal_count=0, indices=[])
        group = groups[key]
        group['tail_count' if i in tail else 'normal_count'] += 1
        group['indices'].append(i)
        frame_patterns.append(group['pattern'])
    patterns = []
    for group in groups.values():
        durations = sorted(values['snapshot'][i] for i in group['indices'])
        patterns.append({k: v for k, v in group.items() if k != 'indices'} |
                        dict(snapshot_median=durations[len(durations) // 2],
                             snapshot_p95=durations[len(durations) * 95 // 100]))
    return dict(work_constant=len(patterns) == 1, patterns=patterns, frame_patterns=frame_patterns,
                p95_sample=stats['p95_sample'], stats=stats)


def split_snapshot(values, histogram_ns):
    if (len(histogram_ns) != len(values['snapshot']) or
            any(type(ns) is not int or ns < 0 or ns / 1e6 > duration
                for ns, duration in zip(histogram_ns, values['snapshot']))):
        raise ValueError('Invalid Histogram timing or snapshot residual')
    histogram = [ns / 1e6 for ns in histogram_ns]
    parts = dict(histogram=histogram,
                 other_snapshot=[s - h for s, h in zip(values['snapshot'], histogram)])
    tail = set(summarize(values)['tail_samples'])
    groups = {label: {k: sum(v[i] for i in indices) / len(indices) for k, v in parts.items()}
              for label, indices in [('tail', sorted(tail)),
                                     ('normal', [i for i in range(len(histogram)) if i not in tail])]}
    return dict(**groups, tail_minus_normal={k: groups['tail'][k] - groups['normal'][k] for k in parts})


def read_histogram_times(path, samples, warmups=20):
    read_counts(path, samples, warmups)
    with path.open() as f:
        rows = list(csv.DictReader(f))
    times = [int(r['histogram_ns']) for r in rows]
    if any(ns <= 0 for ns in times):
        raise ValueError('Inactive or invalid Histogram timer')
    return [sum(times[i:i + 16]) for i in range(warmups * 16, len(times), 16)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input', type=Path, default=Path('target/histogram-work-study/runs'))
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    protocol = json.loads((args.input / 'protocol.json').read_text())
    args.output.mkdir(parents=True, exist_ok=False)
    results, traces, hashes = [], {}, {}
    for case in protocol['cases']:
        log = args.input / (case['name'] + '.log')
        hashes[str(log)] = digest(log)
        values = parse_phases(log.read_text(), protocol['samples'])
        if case['mode'] != 'on':
            results.append(dict(**case, stats=summarize(values)))
            continue
        path = args.input / (case['name'] + '.counts.csv')
        hashes[str(path)] = digest(path)
        counts = read_counts(path, protocol['samples'])
        result = compare_work(values, counts)
        if protocol.get('timing'):
            result['snapshot_split'] = split_snapshot(values, read_histogram_times(path, protocol['samples']))
        results.append(dict(**case, **result))
        traces[case['name']] = (values, counts, result)
    controls = []
    for on in [r for r in results if r['mode'] == 'on']:
        for mode in ['reference', 'off']:
            control = next(r for r in results if r['mode'] == mode and r['rows'] == on['rows'] and r['pair'] == on['pair']
                           and r.get('condition') == on.get('condition'))
            growth = {k: on['stats']['total'][k] / control['stats']['total'][k] - 1 for k in ['median', 'p95']}
            controls.append(dict(pair=on['pair'], rows=on['rows'], condition=on.get('condition'),
                                 control=mode, **growth, flagged=max(growth.values()) > .05))
    load_effects = []
    if protocol.get('timing'):
        for r in results:
            path = args.input / (r['name'] + '.phase.csv')
            hashes[str(path)] = digest(path)
            rows = [v for v in read_trace(path, protocol['samples']) if v['phase'] == 'snapshot' and v['sample'] >= 20]
            tail = set(r['stats']['tail_samples'])
            r['snapshot_clock_means'] = {
                label: dict(wall_ms=sum((rows[i]['end_ns'] - rows[i]['start_ns']) / 1e6 for i in indices) / len(indices),
                            thread_cpu_ms=sum(rows[i]['cpu_ns'] / 1e6 for i in indices) / len(indices))
                for label, indices in [('tail', sorted(tail)), ('normal', [i for i in range(len(rows)) if i not in tail])]}
            if r['condition'] == 'cpu-contention':
                ambient = next(v for v in results if v['pair'] == r['pair'] and v['rows'] == r['rows']
                               and v['mode'] == r['mode'] and v['condition'] == 'ambient')
                load_effects.append(dict(pair=r['pair'], rows=r['rows'], mode=r['mode'],
                    total_growth={k: r['stats']['total'][k] / ambient['stats']['total'][k] - 1 for k in ['median', 'p95']}))
    (args.output / 'analysis.json').write_text(json.dumps(dict(diagnostic_only=True, runs=results,
        controls=controls, load_effects=load_effects, hashes=hashes), indent=2))
    with (args.output / 'frame-work.csv').open('x') as f:
        writer = csv.writer(f)
        writer.writerow(['execution', 'measured_index', 'fixture_sample', 'total_ms', 'snapshot_ms',
                         'refreshes', 'preparations', 'points', 'pattern', 'total_tail'] +
                        (['histogram_ms', 'other_snapshot_ms'] if protocol.get('timing') else []))
        for name, (values, counts, result) in traces.items():
            tail = set(result['stats']['tail_samples'])
            times = (read_histogram_times(args.input / (name + '.counts.csv'), protocol['samples'])
                     if protocol.get('timing') else None)
            for i in range(protocol['samples']):
                writer.writerow([name, i, i + 20, sum(values[k][i] for k in values), values['snapshot'][i],
                                 *[sum(v) for v in zip(*counts[i + 20])], result['frame_patterns'][i], i in tail] +
                                ([times[i] / 1e6, values['snapshot'][i] - times[i] / 1e6] if times else []))
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    fig, axes = plt.subplots(2, 2, figsize=(13, 6), sharex=True, constrained_layout=True)
    for col, rows in enumerate([1200, 100000]):
        values, counts, result = traces[f'1-{rows}-on' + ('-ambient' if protocol.get('timing') else '')]
        axes[0][col].plot(values['snapshot'], linewidth=.6)
        tail = result['stats']['tail_samples']
        axes[0][col].scatter(tail, [values['snapshot'][i] for i in tail], color='red', s=3, label='Total tail')
        axes[0][col].set(title=f'{rows} rows', ylabel='Snapshot wall time (ms)')
        axes[0][col].legend()
        for k, label in enumerate(['refreshes', 'preparations', 'points']):
            axes[1][col].plot([sum(c[k] for c in frame) for frame in counts[20:]], label=label)
        axes[1][col].set(xlabel='Measured sample index', ylabel='Work count')
        axes[1][col].legend()
    fig.savefig(args.output / 'work-and-latency.svg')
    plt.close(fig)
    if protocol.get('timing'):
        enabled = [r for r in results if r['mode'] == 'on']
        fig, ax = plt.subplots(figsize=(12, 5), constrained_layout=True)
        positions = list(range(len(enabled)))
        histogram = [r['snapshot_split']['tail']['histogram'] for r in enabled]
        ax.bar(positions, histogram, label='Histogram preparation')
        ax.bar(positions, [r['snapshot_split']['tail']['other_snapshot'] for r in enabled],
               bottom=histogram, label='Other snapshot work')
        ax.set(xticks=positions, xticklabels=[r['name'].replace('-on-', '\n') for r in enabled],
               ylabel='Mean snapshot wall time in total-tail frames (ms)')
        ax.tick_params(axis='x', labelsize=7)
        ax.legend()
        fig.savefig(args.output / 'snapshot-split.svg')
        plt.close(fig)
    for r in results:
        if r['mode'] == 'on':
            print(r['name'], 'constant', r['work_constant'], 'patterns', len(r['patterns']),
                  'totals', [p['totals'] for p in r['patterns']])
    print('Overhead flags', sum(c['flagged'] for c in controls), '/', len(controls))


if __name__ == '__main__':
    main()
