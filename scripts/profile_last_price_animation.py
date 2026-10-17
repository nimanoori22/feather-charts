#!/usr/bin/env python3
"""One fixed, serial diagnostic batch. No pass thresholds or retries."""
import csv
import hashlib
import itertools
import json
from pathlib import Path
import subprocess
import sys


def cases(binary, directory):
    result = []
    for pair, (kind, rows, size, backend) in enumerate(itertools.product(
        ('line', 'area', 'baseline'), (8192, 100000), ((800, 500), (300, 300)), ('tiny-skia', 'wgpu')
    )):
        modes = ('disabled', 'continuous') if pair % 2 == 0 else ('continuous', 'disabled')
        for mode in modes:
            name = f'{pair:02d}-{kind}-{rows}-{size[0]}x{size[1]}-{backend}-{mode}'
            result.append([str(binary), kind, str(rows), *map(str, size), backend, mode, str(directory / f'{name}.csv')])
    assert len(result) == 48
    return result


def main():
    directory = Path(sys.argv[1] if len(sys.argv) > 1 else 'target/last-price-animation/batch')
    directory.mkdir(parents=True, exist_ok=False)
    binary = Path('target/release/examples/last_price_animation_profile')
    commands = cases(binary, directory)
    files = sorted([*Path('src').rglob('*.rs'), Path('Cargo.toml'), Path('Cargo.lock'),
                    Path(__file__), Path('examples/last_price_animation_profile.rs'), binary])
    manifest = {'commands': commands, 'hashes': {str(f): hashlib.sha256(f.read_bytes()).hexdigest() for f in files},
                'warmup_frames': 20, 'measured_frames': 80, 'clock_step_us': 32500, 'dpr': 1,
                'definitions': {'prepare_ns': 'Chart::update(Tick) wall time', 'geometry_ns': 'Canvas Program::draw plus submit geometry CPU wall time', 'raster_ns': 'Headless screenshot wall time, including GPU readback when applicable',
                                'formatter_calls': 'custom price formatter calls during Tick', 'base_retained': 'base stroke and fill vector allocation retained across Tick'},
                'note': 'Diagnostic only; no historical timing gates or acceptance thresholds changed.'}
    (directory / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    results = []
    for command in commands:
        with Path(command[-1] + '.log').open('w') as log:
            run = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=120, check=False)
        results.append({'command': command, 'exit': run.returncode})
        (directory / 'runs.json').write_text(json.dumps(results, indent=2) + '\n')
        print(f'{len(results)}/48 {command[1:7]} exit={run.returncode}', flush=True)
    assert all(r['exit'] == 0 for r in results), 'Failures retained; inspect runs.json and logs.'
    for command in commands:
        with Path(command[-1]).open() as stream:
            rows = list(csv.DictReader(stream))
        assert len(rows) == 80
        assert all(r['formatter_calls'] == '0' and r['base_retained'] == 'true' for r in rows)
    print('48 runs retained; 3840 measured frames passed bounded-work assertions.')


if __name__ == '__main__':
    main()
