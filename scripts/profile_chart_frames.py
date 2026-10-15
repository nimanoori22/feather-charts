#!/usr/bin/env python3
"""Run the fixed native acceptance protocol; retain failures and stop after a failed set."""
import argparse
import csv
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess
import time

from frame_acceptance import evaluate_frame_cohort

PHASES = ['append', 'zoom', 'pan', 'price']


def read_execution(directory, backend, rows, narrow):
    try:
        metadata = dict(line.split('=', 1) for line in
                        (directory / 'metadata.txt').read_text().splitlines())
        required = dict(requested_backend=backend, rows=str(rows), sources='16',
                        frames_per_phase='120', warmups_per_phase='20', size_changed='false')
        if any(metadata.get(k) != v for k, v in required.items()):
            raise ValueError('fixture metadata mismatch')
        if metadata.get('diagnostic_draw', 'full') != 'full':
            raise ValueError('diagnostic drawing cannot establish acceptance')
        scale = float(metadata['scale'])
        width, height = (420, 320) if narrow else (917, 870.8)
        if not math.isfinite(scale) or scale <= 0 or any(abs(int(metadata[k]) - expected * scale) > 1
                             for k, expected in [('physical_width', width), ('physical_height', height)]):
            raise ValueError('unexpected window dimensions/scale')
        if not metadata['monitor_refresh_millihertz'].startswith('Some('):
            raise ValueError('unknown monitor refresh')
        with (directory / 'frames.csv').open() as file:
            records = list(csv.DictReader(file))
        if len(records) != 480:
            raise ValueError('480 raw frames including warmups are required')
        cohorts = {phase: [] for phase in PHASES}
        for index, record in enumerate(records):
            phase = PHASES[index // 120]
            if (int(record['frame']) != index or record['phase'] != phase
                    or record['warmup'] != str(index % 120 < 20).lower()):
                raise ValueError('frame identity/phase/warmup mismatch')
            if any(record[k] not in ['true', 'false'] for k in ['discarded', 'resolved']):
                raise ValueError('invalid feedback flags')
            frame = dict(started_ns=int(record['started_ns']), work_ns=int(record['work_ns']),
                         presented_ns=int(record['presented_ns']) if record['presented_ns'] else None,
                         discarded=record['discarded'] == 'true', resolved=record['resolved'] == 'true')
            if not frame['resolved']:
                raise ValueError('unresolved feedback, including warmups')
            if index % 120 >= 20:
                cohorts[phase].append(frame)
        results = {phase: evaluate_frame_cohort(frames) for phase, frames in cohorts.items()}
        status = ('INCONCLUSIVE' if any(r['status'] == 'INCONCLUSIVE' for r in results.values())
                  else 'FAIL' if any(r['status'] == 'FAIL' for r in results.values()) else 'PASS')
        return dict(status=status, metadata=metadata, phases=results)
    except (OSError, ValueError, KeyError, TypeError) as error:
        return dict(status='INCONCLUSIVE', reason=str(error))


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=Path('target/release/examples/frame_latency_profile'))
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    manifest = dict(started_unix=time.time(), binary=str(args.binary.resolve()),
                    binary_sha256=digest(args.binary), protocol_sha256=digest(Path('docs/porting/frame-acceptance-policy.md')),
                    environment={k: v for k, v in os.environ.items()
                                 if k.startswith(('ICED_', 'WGPU_', 'WINIT_'))}, runs=[])
    (args.output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    for number in range(1, 4):
        backends = ['tiny-skia', 'wgpu'] if number % 2 else ['wgpu', 'tiny-skia']
        histories = [1200, 100000] if number % 2 else [100000, 1200]
        current = []
        for backend in backends:
            for rows in histories:
                for narrow in [False, True]:
                    name = f'set-{number}-{backend}-{rows}-' + ('narrow' if narrow else 'regular')
                    command = [str(args.binary.resolve()), '--backend', backend, '--rows', str(rows),
                               '--output', str(args.output / name)] + (['--narrow'] if narrow else [])
                    print(name, flush=True)
                    with (args.output / f'{name}.log').open('x') as log:
                        try:
                            completed = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=75)
                            exit_code = completed.returncode
                        except subprocess.TimeoutExpired:
                            exit_code = 'timeout'
                    result = read_execution(args.output / name, backend, rows, narrow)
                    if exit_code != 0:
                        result.update(status='INCONCLUSIVE', reason=f'process exit: {exit_code}')
                    result.update(name=name, set=number, backend=backend, rows=rows, narrow=narrow)
                    current.append(result)
                    manifest['runs'].append(result)
                    manifest['status'] = 'INCOMPLETE'
                    (args.output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
        if any(run['status'] != 'PASS' for run in current):
            manifest['status'] = 'FAIL' if any(run['status'] == 'FAIL' for run in current) else 'INCONCLUSIVE'
            break
    else:
        manifest['status'] = 'PASS'
    manifest['finished_unix'] = time.time()
    manifest['artifacts'] = {str(p.relative_to(args.output)): digest(p)
                             for p in sorted(args.output.rglob('*')) if p.is_file() and p.name != 'manifest.json'}
    (args.output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(manifest['status'], flush=True)
    raise SystemExit(0 if manifest['status'] == 'PASS' else 1)


if __name__ == '__main__':
    main()
