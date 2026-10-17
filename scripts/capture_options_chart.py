#!/usr/bin/env python3
"""Capture the frozen price-chart matrix and compare both pre-existing families."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    output = Path(sys.argv[1] if len(sys.argv) > 1 else 'target/options-chart/final')
    output.mkdir(parents=True, exist_ok=False)
    baseline = Path('target/options-chart/before')
    commands = []
    for backend in ('tiny-skia', 'wgpu'):
        flag = ['--wgpu'] if backend == 'wgpu' else []
        commands.extend([
            ('feature', backend, ['target/debug/examples/options_chart', '--captures', str(output / 'feature' / backend)] + flag),
            ('calendar', backend, ['target/debug/examples/public_chart', '--line', '--conflation', '--up-down-captures', str(output / 'calendar' / backend)] + flag),
            ('yield', backend, ['target/debug/examples/yield_curve', '--captures', str(output / 'yield' / backend)] + flag),
        ])
    fonts = []
    for family in ('sans-serif', 'monospace'):
        font = Path(subprocess.check_output(['fc-match', '--format', '%{file}', family], text=True))
        fonts.append({'family': family, 'file': str(font), 'sha256': digest(font)})
    protocol = {
        'commands': commands, 'baselines': str(baseline),
        'baseline_protocol_sha256': digest(baseline / 'protocol.json'),
        'binaries': {str(p): digest(p) for p in (Path('target/debug/examples') / n for n in ('options_chart', 'public_chart', 'yield_curve'))},
        'sources': {str(p): digest(p) for p in sorted(Path('src').rglob('*.rs'))},
        'fixtures': {str(p): digest(p) for p in [Path('examples/options_chart.rs'), *sorted(Path('examples/options_chart').glob('*.rs')), Path('examples/yield_curve.rs'), *sorted(Path('examples/yield_curve').glob('*.rs')), Path('examples/public_chart.rs')]},
        'vendor_sources': {str(p): digest(p) for p in sorted(Path('vendor').rglob('*.rs'))},
        'fontconfig_identity': fonts,
        'scenes': ['line', 'volume', 'families', 'precision', 'empty'],
        'sizes': [[800, 500], [300, 300], [300, 90]], 'dpr': [1, 1.25, 1.5, 2],
        'frozen_clock': 'Pulse/feed disabled; manual update markers have no deadlines.',
    }
    (output / 'protocol.json').write_text(json.dumps(protocol, indent=2) + '\n')
    results = []
    for family, backend, command in commands:
        with (output / f'{family}-{backend}.log').open('w') as log:
            try:
                status = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=120, check=False).returncode
            except subprocess.TimeoutExpired:
                log.write('Timed out; failure retained.\n')
                status = 124
        row = {'family': family, 'backend': backend, 'exit': status}
        directory = output / family / backend
        if status == 0:
            images = sorted(directory.glob('*.rgba'))
            assert len(images) == (72 if family == 'calendar' else 60), (family, len(images))
            row['images'] = {p.name: digest(p) for p in images}
            if family != 'feature':
                references = sorted((baseline / family / backend).glob('*.rgba'))
                assert len(references) == 12
                assert all((directory / p.name).read_bytes() == p.read_bytes() for p in references), (family, backend, 'baseline pixels changed')
                row['unchanged'] = len(references)
        results.append(row)
        (output / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
        print(f'{family}/{backend}: exit={status}', flush=True)
    assert all(r['exit'] == 0 for r in results), 'Failures retained; inspect logs.'
    print('120 price-chart images, 24 unchanged calendar and 24 unchanged yield images pass.')


if __name__ == '__main__':
    main()
