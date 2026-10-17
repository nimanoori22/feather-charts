"""Attribute retained total-latency tails to the phases of the same samples."""
import argparse
import csv
import hashlib
import json
import math
from pathlib import Path
import re

PHASES = ('update', 'snapshot', 'glyph')


def parse_phases(text, expected):
    result = {}
    for name, raw in re.findall(r'^raw Histogram .* phase=append/(\w+) ms=(.+)$', text, re.M):
        if name not in PHASES or name in result:
            raise ValueError(f'Unexpected or repeated phase: {name}')
        values = json.loads(raw)
        if len(values) != expected or any(type(v) not in (int, float) or not math.isfinite(v) or v < 0
                                          for v in values):
            raise ValueError(f'Invalid samples: {name}')
        result[name] = values
    if set(result) != set(PHASES):
        raise ValueError('Incomplete phases')
    return result


def summarize(values):
    total = [sum(v) for v in zip(*(values[k] for k in PHASES))]
    order = sorted(range(len(total)), key=lambda i: (total[i], i))
    tail = order[len(order) * 95 // 100:]
    means = {k: sum(values[k][i] for i in tail) / len(tail) for k in PHASES}
    denominator = sum(means.values())
    result = {k: dict(median=sorted(v)[len(v) // 2], p95=sorted(v)[len(v) * 95 // 100])
              for k, v in {**values, 'total': total}.items()}
    result.update(p95_sample=tail[0], tail_samples=tail, tail_mean=means,
                  tail_share={k: v / denominator if denominator else 0. for k, v in means.items()})
    return result


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_trace(path, samples, warmups=20):
    with path.open() as f:
        rows = list(csv.DictReader(f))
    if len(rows) != (samples + warmups) * 3:
        raise ValueError('Incomplete phase trace')
    identity, previous = None, None
    for index, row in enumerate(rows):
        for key in ['sample', 'pid', 'tid', 'clock_id', 'start_ns', 'end_ns', 'cpu_ns',
                    'minor', 'major', 'voluntary', 'involuntary']:
            if key in ['minor', 'major', 'voluntary', 'involuntary'] and row[key] == '':
                row[key] = None
                continue
            row[key] = int(row[key])
            if row[key] < 0:
                raise ValueError('Negative trace field')
        current = (row['pid'], row['tid'], row['clock_id'])
        if identity is None:
            identity = current
        if current != identity or row['clock_id'] != 1 or row['pid'] <= 0:
            raise ValueError('Trace identity/clock mismatch')
        if (row['sample'] != index // 3 or row['phase'] != PHASES[index % 3]
                or row['warmup'] != str(index // 3 < warmups).lower()
                or row['end_ns'] < row['start_ns']):
            raise ValueError('Trace ordering mismatch')
        if previous is not None and (row['start_ns'] < previous or
                                    (index % 3 and row['start_ns'] != previous)):
            raise ValueError('Noncontiguous trace phases')
        previous = row['end_ns']
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input', type=Path, default=Path('target/lazy-autoscale/comparison'))
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    protocol = json.loads((args.input / 'protocol.json').read_text())
    recorded = json.loads(Path('docs/porting/lazy-autoscale-summary.json').read_text())['artifacts']
    for version, expected in protocol['binaries'].items():
        if digest(args.input.parent / version / 'preparation_profile') != expected:
            raise ValueError(f'{version} executable hash mismatch')
    out = args.output
    out.mkdir(parents=True, exist_ok=False)
    results, all_values, hashes = [], {}, {}
    with (out / 'phase-attribution.csv').open('x') as f:
        writer = csv.writer(f)
        writer.writerow(['execution', 'measured_index', 'fixture_sample', *PHASES, 'total', 'total_tail'])
        for case in protocol['cases']:
            path = args.input / (case['name'] + '.log')
            hashes[str(path)] = digest(path)
            if recorded.get(str(path)) != hashes[str(path)]:
                raise ValueError(f'Retained sample hash mismatch: {path}')
            values = parse_phases(path.read_text(), protocol['samples'])
            summary = summarize(values)
            all_values[case['name']] = values
            results.append(dict(**case, **summary))
            selected = set(summary['tail_samples'])
            for i in range(protocol['samples']):
                phases = [values[k][i] for k in PHASES]
                writer.writerow([case['name'], i, i + protocol['warmups'], *phases, sum(phases), i in selected])
    (out / 'analysis.json').write_text(json.dumps(dict(diagnostic_only=True, runs=results, hashes=hashes), indent=2))
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    fig, axes = plt.subplots(2, 1, figsize=(12, 6), sharex=True, constrained_layout=True)
    for ax, version in zip(axes, ['baseline', 'candidate']):
        name = f'4-100000-{version}'
        values = all_values[name]
        ax.stackplot(range(protocol['samples']), *(values[k] for k in PHASES), labels=PHASES)
        ax.set(title=name, ylabel='CPU pipeline wall time (ms)')
        ax.legend(loc='upper right')
    axes[-1].set_xlabel('Measured sample index (20 warmups excluded)')
    fig.savefig(out / 'pair-4-timeline.svg')
    plt.close(fig)
    fig, axes = plt.subplots(2, 4, figsize=(16, 7), constrained_layout=True)
    for row, history in enumerate([1200, 100000]):
        for col, phase in enumerate([*PHASES, 'total']):
            ax = axes[row][col]
            for r in results:
                if r['rows'] != history:
                    continue
                values = all_values[r['name']]
                v = values.get(phase) if phase != 'total' else [sum(v) for v in zip(*(values[k] for k in PHASES))]
                ax.plot(sorted(v), [(i + 1) / len(v) for i in range(len(v))],
                        color=f'C{r["pair"] - 1}', linestyle='-' if r['version'] == 'candidate' else '--',
                        label=f'{r["pair"]} {r["version"]}')
            ax.set(title=f'{history} rows: {phase}', xlabel='ms (log scale)', ylabel='ECDF', xscale='log')
    axes[0][0].legend(fontsize=6)
    fig.savefig(out / 'distributions.svg')
    plt.close(fig)
    for r in results:
        print(r['name'], 'total_p95_ms=', r['total']['p95'], 'tail_share=', r['tail_share'])


if __name__ == '__main__':
    main()
