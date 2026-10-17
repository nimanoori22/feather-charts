#!/usr/bin/env python3
"""Fixed native capture matrix; keep failures and compare retained pre-feature pixels."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    output = Path(sys.argv[1] if len(sys.argv) > 1 else 'target/image-watermark/after')
    baseline = Path('target/image-watermark/before')
    output.mkdir(parents=True, exist_ok=False)
    binary = Path('target/debug/examples/public_chart')
    commands = [[str(binary), '--line', '--conflation', '--image-watermark-captures', str(output / backend)]
                + (['--wgpu'] if backend == 'wgpu' else []) for backend in ('tiny-skia', 'wgpu')]
    fonts = []
    for family in ('sans-serif', 'monospace'):
        path = Path(subprocess.check_output(['fc-match', '--format', '%{file}', family], text=True))
        fonts.append({'family': family, 'fontconfig_file': str(path), 'sha256': digest(path)})
    protocol = {'commands': commands, 'binary_sha256': digest(binary),
                'fixture_sha256': digest(Path('examples/public_chart/image_watermark.rs')),
                'sources': {str(p): digest(p) for p in sorted(Path('src').rglob('*.rs'))},
                'vendor_sources': {str(p): digest(p) for p in sorted(Path('vendor').rglob('*.rs'))},
                'fontconfig_identity': fonts, 'font_note': 'Actual fallback is handled by Iced/cosmic-text.',
                'scenes': ['default', 'logo', 'limits', 'mixed', 'empty', 'multiple'],
                'sizes': [[800, 500], [300, 300], [300, 90]], 'dpr': [1, 1.25, 1.5, 2]}
    (output / 'protocol.json').write_text(json.dumps(protocol, indent=2) + '\n')
    results = []
    for backend, command in zip(('tiny-skia', 'wgpu'), commands):
        with (output / f'{backend}.log').open('w') as log:
            try:
                status = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=120, check=False).returncode
            except subprocess.TimeoutExpired:
                log.write('Capture timed out; retained for investigation.\n')
                status = 124
        row = {'backend': backend, 'exit': status}
        if status == 0:
            images = sorted((output / backend).glob('*.rgba'))
            assert len(images) == 72
            references = sorted((baseline / backend).glob('default-*.rgba'))
            assert len(references) == 12
            assert all((output / backend / p.name).read_bytes() == p.read_bytes() for p in references)
            row.update({'watermark_captures': 60, 'unchanged_defaults': 12,
                        'images': {p.name: digest(p) for p in images}})
        results.append(row)
        (output / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
        print(f'{backend}: exit={status}', flush=True)
    assert all(r['exit'] == 0 for r in results), 'Failures retained; inspect logs/results.'
    print('120 watermark captures and 24 unchanged default-off images pass.')


if __name__ == '__main__':
    main()
