"""Run immutable baseline/current binaries serially; retain every timing sample."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--directory', type=Path, default=Path('target/incremental-append'))
parser.add_argument('--cpu', type=int, help='CPU used by this process and its children')
parser.add_argument('--baseline-tail-rows', type=int, choices=[1, 4], default=4,
                    help='Response sentinel: 4 for the original engine, 1 for an incremental baseline')
args = parser.parse_args()
root = args.directory.resolve()
root.mkdir(parents=True, exist_ok=True)
# Pin this process and its children so the two artifacts use the same CPU.
if hasattr(os, 'sched_getaffinity'):
    available = os.sched_getaffinity(0)
    cpu = args.cpu if args.cpu is not None else max(available)
    assert cpu in available
    os.sched_setaffinity(0, {cpu})
    (root / 'cpu.json').write_text(json.dumps({'cpu':cpu,'available':sorted(available)},indent=2)+'\n')
artifacts = {f'{version}{suffix}': root / f'{version}{suffix}-profile'
             for version in ['baseline', 'current'] for suffix in ['', '-custom']}
identities = {key: hashlib.sha256(path.read_bytes()).hexdigest() for key, path in artifacts.items()}
for version, expected in [('baseline', args.baseline_tail_rows), ('current', 1)]:
    identity = subprocess.check_output([str(artifacts[version]), '--rows', '10', '--series', '1', '--kind', 'line', '--phase', 'append', '--samples', '1'], text=True)
    assert f'tail_response_rows={expected}\n' in identity, (version, identity)
(root / 'identities.json').write_text(json.dumps(identities, indent=2) + '\n')
pattern = re.compile(r'^(Line|Candlestick|Histogram|Custom) rows=(\d+) sources=(\d+) phase=(\w+)/(\w+) median_ms=([\d.]+) p95_ms=([\d.]+)$', re.M)
results = []
cases = [(kind, rows, sources, 'append', [])
         for kind in ['line', 'candlestick', 'histogram', 'custom']
         for sources in [1, 16] for rows in [1200, 20_000, 100_000]]
cases += [('line', 100_000, 16, 'mixed', ['--mixed']),
          ('line', 100_000, 1, 'whitespace', ['--whitespace-tail', '90000', '--append-whitespace'])]
cases += [('line', rows, 1, 'burst', ['--burst','8']) for rows in [1200,100_000]]
cases += [('line',20_000,2,phase,[]) for phase in ['middle','reload']]
cases += [('line',rows,16,'navigation',[]) for rows in [1200,100_000]]
cases += [('candlestick', 20_000, 16, phase, []) for phase in ['static', 'latest']]
for run in range(1, 4):
    versions = ['baseline', 'current'] if run % 2 else ['current', 'baseline']
    for kind, rows, sources, case, extra in cases:
        phase = case if case in ['static', 'latest', 'middle', 'reload', 'navigation'] else 'append'
        for version in versions:
            custom = kind == 'custom'
            command = [str(artifacts[version + ('-custom' if custom else '')]), '--rows', str(rows), '--series', str(sources), '--phase', phase, '--samples', '100']
            if not custom:
                command += ['--kind', kind, '--fixed-last-price']
            command += extra
            name = f'run-{run}-{version}-{kind}-{rows}-{sources}-{case}'
            with (root / f'{name}.log').open('w') as log:
                subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
            found = list(pattern.finditer((root / f'{name}.log').read_text()))
            assert len(found) == 3, name
            for match in found:
                _, n, count, _, stage, median, p95 = match.groups()
                results.append(dict(run=run, version=version, kind=kind, rows=int(n), sources=int(count), case=case, stage=stage, median=float(median), p95=float(p95)))
            (root / 'measurements.json').write_text(json.dumps(results, indent=2) + '\n')
            update = next(row for row in results[-3:] if row['stage'] == 'update')
            print(f'{name}: update {update["median"]:.6f}/{update["p95"]:.6f} ms', flush=True)
# Keep failed screens visible; the report explains diagnosis and any remaining gate.
screens = []
for kind in ['line', 'candlestick', 'histogram', 'custom']:
    for sources in [1, 16]:
        for run in range(1, 4):
            pairs = [next(row for row in results if row['run'] == run and row['version'] == 'current' and row['kind'] == kind and row['sources'] == sources and row['rows'] == rows and row['case'] == 'append' and row['stage'] == 'update') for rows in [1200,100_000]]
            screens.append(dict(run=run, kind=kind, sources=sources, median_growth=pairs[1]['median']/pairs[0]['median']-1, p95_growth=pairs[1]['p95']/pairs[0]['p95']-1))
(root / 'growth-screens.json').write_text(json.dumps(screens, indent=2) + '\n')
failed = [screen for screen in screens if max(screen['median_growth'],screen['p95_growth']) > .1]
print(f'{len(screens)-len(failed)}/{len(screens)} paired update median/p95 screens pass 10%.', flush=True)
for failure in failed:
    print(json.dumps(failure), flush=True)
