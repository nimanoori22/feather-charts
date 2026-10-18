#!/usr/bin/env python3
"""Capture the external custom-domain consumer on both explicit renderers."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    output = Path(sys.argv[1] if len(sys.argv) > 1 else 'target/custom-horizontal/captures')
    output.mkdir(parents=True, exist_ok=False)
    binary = Path('target/debug/examples/custom_horizontal')
    protocol = {
        'binary_sha256': digest(binary),
        'source': {str(p): digest(p) for p in sorted(Path('src').rglob('*.rs'))},
        'fixtures': {str(p): digest(p) for p in [Path('examples/custom_horizontal.rs'), *sorted(Path('examples/custom_horizontal').glob('*.rs')), Path('examples/numeric_custom_series/view.rs'), Path('examples/support/editable_box.rs')]},
        'sizes': [[800, 500], [300, 300], [300, 90]],
        'ratios': [1, 1.25, 1.5, 2],
        'scenes': ['initial', 'updates', 'labels', 'panes', 'empty'],
        'layers': ['base', 'base+top'],
        'clock': 'Live feed disabled; retained snapshots are redrawn after owner mutation/drop.',
    }
    (output / 'protocol.json').write_text(json.dumps(protocol, indent=2) + '\n')
    results = []
    for backend, flag in [('tiny-skia', []), ('wgpu', ['--wgpu'])]:
        directory = output / backend
        # The binary rejects fallback. Tiny-skia must be requested explicitly.
        env = dict(os.environ, ICED_BACKEND=backend)
        command = [str(binary), '--captures', str(directory), *flag]
        with (output / (backend + '.log')).open('w') as log:
            status = subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=240).returncode
        row = {'backend': backend, 'command': command, 'exit': status}
        if status == 0:
            images = sorted(directory.glob('*.rgba'))
            assert len(images) == 120, len(images)
            for image in images:
                metadata = json.loads(image.with_suffix('.json').read_text())
                assert image.stat().st_size == metadata['width'] * metadata['height'] * 4
            for scene in ['labels', 'updates', 'empty', 'panes']:
                assert (directory / f'{scene}-800x500-1-toptrue.rgba').read_bytes() != (directory / 'initial-800x500-1-toptrue.rgba').read_bytes(), scene
            row['images'] = {p.name: digest(p) for p in images}
        results.append(row)
        (output / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
        print(backend, status, flush=True)
        assert status == 0, 'Retained failed capture; inspect log.'
    print('240 custom-domain exports passed.')


if __name__ == '__main__':
    main()
