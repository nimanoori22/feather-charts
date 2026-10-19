#!/usr/bin/env python3
"""Capture actual time-locale renderers and verify retained snapshot exports."""
from pathlib import Path
import hashlib
import json
import math
import subprocess
import sys


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    output = Path(sys.argv[1] if len(sys.argv) > 1 else 'target/public-parity/time-locale/captures')
    output.mkdir(parents=True, exist_ok=False)
    binary = Path('target/debug/examples/pane_management')
    fixtures = [Path('examples/pane_management.rs'), *sorted(Path('examples/pane_management').glob('*.rs')), Path('examples/custom_horizontal/behavior.rs'), Path('examples/numeric_custom_series/view.rs'), Path('examples/support/editable_box.rs')]
    protocol = dict(binary_sha256=digest(binary), sources={str(p): digest(p) for p in sorted(Path('src').rglob('*.rs'))}, fixtures={str(p): digest(p) for p in fixtures}, modes=['calendar','strike','maturity','sessions'], scenes=['english','french','german','arabic','latin','extended','german-arabic','fallback','custom','recovered'], sizes=[[800,500],[300,300],[300,90]], ratios=[1,1.25,1.5,2], clock='Supported Gregorian locales, numbering overrides, fallback, custom formatter and pane/short-height recovery; feed disabled; retained exports after locale change, Reset and owner drop.')
    protocol['fonts'] = []
    for family in ['sans-serif','monospace']:
        path = Path(subprocess.check_output(['fc-match','--format','%{file}',family],text=True))
        protocol['fonts'].append(dict(family=family,path=str(path),sha256=digest(path)))
    (output/'protocol.json').write_text(json.dumps(protocol,indent=2)+'\n')
    results = []
    for backend in ['tiny-skia','wgpu']:
        directory=output/backend
        command=[str(binary),'--locale-captures',str(directory)]+(['--wgpu'] if backend=='wgpu' else [])
        with (output/(backend+'.log')).open('w') as log:
            code=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT,timeout=600).returncode
        row=dict(backend=backend,command=command,exit=code)
        if code==0:
            images=sorted(directory.glob('*/*x*-*.rgba'))
            assert len(images)==480,len(images)
            for image in images:
                scene,size,ratio=image.stem.rsplit('-',2)
                width,height=map(int,size.split('x'))
                assert scene in protocol['scenes']
                assert image.stat().st_size==math.ceil(width*float(ratio))*math.ceil(height*float(ratio))*4
            row['images']={str(p.relative_to(directory)):digest(p) for p in images}
            row['retained_exports']=480
        results.append(row)
        (output/'results.json').write_text(json.dumps(results,indent=2)+'\n')
        print(backend,code,flush=True)
        assert code==0,'Failed capture retained; inspect log.'
    print('960 locale screen replays and 960 retained exports passed.')


if __name__=='__main__':
    main()
