#!/usr/bin/env python3
"""Frozen pane-management matrix and exact pre-change compatibility comparisons."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def digest(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()


def main():
    output = Path(sys.argv[1] if len(sys.argv) > 1 else 'target/pane-management/final')
    output.mkdir(parents=True, exist_ok=False)
    baseline = Path('target/pane-management/before')
    commands = []
    for backend in ('tiny-skia', 'wgpu'):
        flag = ['--wgpu'] if backend == 'wgpu' else []
        commands.extend([
            ('feature', backend, ['target/debug/examples/pane_management', '--captures', str(output / 'feature' / backend)] + flag),
            ('calendar', backend, ['target/debug/examples/public_chart', '--line', '--conflation', '--up-down-captures', str(output / 'calendar' / backend)] + flag),
            ('custom', backend, ['target/debug/examples/custom_series', '--capture-dir', str(output / 'custom' / backend)] + flag),
            ('price', backend, ['target/debug/examples/options_chart', '--captures', str(output / 'price' / backend)] + flag),
            ('yield', backend, ['target/debug/examples/yield_curve', '--captures', str(output / 'yield' / backend)] + flag),
        ])
    names = ['pane_management', 'public_chart', 'custom_series', 'options_chart', 'yield_curve']
    fixtures = []
    for name in names:
        fixtures += [Path('examples') / (name + '.rs'), *sorted((Path('examples') / name).glob('*.rs'))]
    fixtures += [Path('examples/support/editable_box.rs'), Path('examples/numeric_custom_series/view.rs')]
    fonts = []
    for family in ('sans-serif', 'monospace'):
        p = Path(subprocess.check_output(['fc-match', '--format', '%{file}', family], text=True))
        fonts.append({'family': family, 'file': str(p), 'sha256': digest(p)})
    protocol = {
        'commands': commands, 'baseline_protocol_sha256': digest(baseline / 'protocol.json'),
        'baseline_images_sha256': digest(baseline / 'images.json'),
        'binaries': {str(p): digest(p) for p in (Path('target/debug/examples') / name for name in names)},
        'sources': {str(p): digest(p) for p in sorted(Path('src').rglob('*.rs'))},
        'fixtures': {str(p): digest(p) for p in fixtures},
        'vendor_sources': {str(p): digest(p) for p in sorted(Path('vendor').rglob('*.rs'))},
        'fontconfig_identity': fonts,
        'modes': ['calendar', 'price', 'yield'], 'scenes': ['initial', 'move', 'swap', 'builtin-transfer', 'custom-transfer', 'remove'],
        'sizes': [[800, 500], [300, 300], [300, 90]], 'dpr': [1, 1.25, 1.5, 2],
        'clock': 'Feed and peer pulse disabled; frames are immutable snapshots.',
    }
    (output / 'protocol.json').write_text(json.dumps(protocol, indent=2) + '\n')
    results = []
    for family, backend, command in commands:
        with (output / f'{family}-{backend}.log').open('w') as log:
            try:
                status = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=240 if family == 'feature' else 120, check=False).returncode
            except subprocess.TimeoutExpired:
                log.write('Timeout; retained for investigation.\n')
                status = 124
        row = {'family': family, 'backend': backend, 'exit': status}
        directory = output / family / backend
        if status == 0:
            images = sorted(directory.glob('*/*.rgba') if family == 'feature' else directory.glob('*.ppm' if family == 'custom' else '*.rgba'))
            assert len(images) == {'feature': 216, 'calendar': 72, 'custom': 120, 'price': 60, 'yield': 60}[family], (family, len(images))
            row['images'] = {str(p.relative_to(directory)): digest(p) for p in images}
            if family != 'feature':
                references = sorted((baseline / family / backend).iterdir())
                assert len(references) == 12
                assert all((directory / p.name).read_bytes() == p.read_bytes() for p in references), (family, backend, 'baseline mismatch')
                row['unchanged'] = len(references)
        results.append(row)
        (output / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
        print(f'{family}/{backend}: {status}', flush=True)
    assert all(r['exit'] == 0 for r in results), 'Failures retained; inspect logs.'
    print('432 feature captures and 96 exact baseline comparisons pass.')


if __name__ == '__main__':
    main()
