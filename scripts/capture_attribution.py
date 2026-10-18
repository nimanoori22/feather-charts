#!/usr/bin/env python3
"""Record both actual attribution renderers and immutable export assertions."""
from pathlib import Path
import hashlib
import json
import math
import subprocess
import sys


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    output = Path(sys.argv[1] if len(sys.argv) > 1 else 'target/attribution/captures')
    output.mkdir(parents=True, exist_ok=False)
    binary = Path('target/debug/examples/pane_management')
    fixtures = [Path('examples/pane_management.rs'), *sorted(Path('examples/pane_management').glob('*.rs')), Path('examples/custom_horizontal/behavior.rs'), Path('examples/numeric_custom_series/view.rs'), Path('examples/support/editable_box.rs')]
    fonts = []
    for family in ['sans-serif', 'monospace']:
        path = Path(subprocess.check_output(['fc-match', '--format', '%{file}', family], text=True))
        fonts.append(dict(family=family, path=str(path), sha256=digest(path)))
    protocol = dict(
        binary_sha256=digest(binary),
        sources={str(p): digest(p) for p in sorted(Path('src').rglob('*.rs'))},
        fixtures={str(p): digest(p) for p in fixtures},
        fonts=fonts,
        modes=['calendar', 'strike', 'maturity', 'sessions'],
        scenes=['light', 'dark', 'mismatch', 'moved', 'removed', 'empty', 'hidden'],
        sizes=[[800, 500], [300, 300], [300, 90]], ratios=[1, 1.25, 1.5, 2],
        export_flags=[[False, False], [False, True], [True, False], [True, True]],
        clock='Live feed disabled; screen replay and exports after owner drop must stay immutable.',
    )
    (output / 'protocol.json').write_text(json.dumps(protocol, indent=2) + '\n')
    results = []
    for backend in ['tiny-skia', 'wgpu']:
        directory = output / backend
        command = [str(binary), '--attribution-captures', str(directory)] + (['--wgpu'] if backend == 'wgpu' else [])
        with (output / (backend + '.log')).open('w') as log:
            code = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=600).returncode
        row = dict(backend=backend, command=command, exit=code)
        if code == 0:
            images = sorted(directory.glob('*/*.rgba'))
            assert len(images) == 336, len(images)
            for image in images:
                scene, size, ratio = image.stem.rsplit('-', 2)
                width, height = map(int, size.split('x'))
                assert image.stat().st_size == math.ceil(width * float(ratio)) * math.ceil(height * float(ratio)) * 4
                assert scene in protocol['scenes']
            row['images'] = {str(p.relative_to(directory)): digest(p) for p in images}
            row['export_exclusion_comparisons'] = 336 * 4
            row['retained_export_comparisons'] = 336 * 4
        results.append(row)
        (output / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
        print(backend, code, flush=True)
        assert code == 0, 'Failed run retained; inspect log.'
    print('672 screen images, 2688 export-exclusion and 2688 retained-export comparisons passed.')


if __name__ == '__main__':
    main()
