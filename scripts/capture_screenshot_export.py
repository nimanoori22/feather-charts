#!/usr/bin/env python3
"""Public screenshot matrix; retain all logs/images and old compatibility gates."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    output = Path(sys.argv[1] if len(sys.argv) > 1 else 'target/screenshot-export/final')
    output.mkdir(parents=True, exist_ok=False)
    protocol = {
        'modes': ['calendar', 'price', 'yield'],
        'sizes': [[800, 500], [300, 300], [300, 90]],
        'ratios': [1, 1.25, 1.5, 2],
        'scenes': ['base', 'crosshair', 'transferred', 'hidden-axis'],
        'flags': [[False, False], [False, True], [True, False], [True, True]],
        'backends': ['tiny-skia', 'wgpu'],
        'expected_images': 1152, 'extra_alpha_images': 6,
        'source': {str(p): digest(p) for p in sorted(Path('src').rglob('*.rs'))},
        'fixtures': {str(p): digest(p) for p in [Path('examples/pane_management.rs'), Path('examples/support/editable_box.rs'), Path('examples/numeric_custom_series/view.rs'), *sorted(Path('examples/pane_management').glob('*.rs'))]},
        'vendor_sources': {str(p): digest(p) for p in sorted(Path('vendor').rglob('*.rs'))},
        'fonts': {family: {'path': (font := Path(subprocess.check_output(['fc-match','--format','%{file}',family],text=True))).as_posix(), 'sha256': digest(font)} for family in ('sans-serif','monospace')},
        'binary_sha256': digest(Path('target/debug/examples/pane_management')),
        'source_oracle_sha256': digest(Path('docs/porting/screenshot-states.json')),
        'baseline_images_sha256': digest(Path('target/public-events/before/images.json')),
    }
    (output / 'protocol.json').write_text(json.dumps(protocol, indent=2) + '\n')
    results = []
    for backend in protocol['backends']:
        command = ['target/debug/examples/pane_management', '--screenshot-captures', str(output / backend)]
        if backend == 'wgpu':
            command += ['--wgpu']
        with (output / (backend + '.log')).open('w') as log:
            try:
                status = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=600).returncode
            except subprocess.TimeoutExpired:
                log.write('Timeout retained for investigation.\n')
                status = 124
        images = sorted((output / backend).glob('*/*.rgba'))
        row = {'backend': backend, 'exit': status, 'images': {str(p.relative_to(output)): digest(p) for p in images}}
        results.append(row)
        (output / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
        assert status == 0, (backend, status)
        assert len(images) == 579, (backend, len(images))
        print(f'{backend}: 576 layer images + 3 alpha-fit images passed', flush=True)
    command = ['python', 'scripts/capture_public_events.py', str(output / 'compatibility')]
    with (output / 'compatibility.log').open('w') as log:
        status = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT).returncode
    assert status == 0, 'existing compatibility gate failed; retained logs'
    print('1158 exports, previous 288 event images and 96 exact compatibility comparisons passed.')


if __name__ == '__main__':
    main()
