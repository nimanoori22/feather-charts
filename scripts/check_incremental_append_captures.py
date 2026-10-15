"""Run the existing capture matrices, including live tail and burst scenarios."""
from pathlib import Path
import argparse
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--backend', choices=['tiny-skia','wgpu'])
parser.add_argument('--append-only', action='store_true')
args = parser.parse_args()
root = Path('target/incremental-append') / ('captures-final' if args.append_only else 'captures')
root.mkdir(parents=True, exist_ok=True)
for backend in ([args.backend] if args.backend else ['tiny-skia', 'wgpu']):
    jobs = [(kind, ['--' + kind]) for kind in ['line','area','baseline','bar','candlestick','histogram']]
    jobs += [('hover', ['--hover-selection','--hover-captures-only']),
             ('markers', ['--series-markers','--marker-captures-only']),
             ('editing', ['--primitives','--primitive-edit-captures-only']),
             ('custom', [])]
    if args.append_only:
        jobs = [(name, options + ['--append-captures-only']) for name, options in jobs[:6]]
    for name, options in jobs:
        directory = root / backend / name
        command = ['target/debug/examples/' + ('custom_series' if name == 'custom' else 'public_chart'), '--capture-dir', str(directory)] + options
        if backend == 'wgpu':
            command.append('--wgpu')
        result = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (root / f'{backend}-{name}.log').write_text(result.stdout)
        if result.returncode:
            raise SystemExit(result.stdout)
        print(result.stdout.strip(), flush=True)
