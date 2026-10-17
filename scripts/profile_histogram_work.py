"""Run one fixed reference/off/on batch; retain failures without acceptance retries."""
import argparse
import json
import os
from pathlib import Path
import subprocess

from analyze_frame_phases import digest, parse_phases, read_trace
from histogram_work_analysis import read_counts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('target/histogram-work-study/runs'))
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    reference = Path('target/frame-phase-investigation/build-v3/candidate')
    build = Path('target/histogram-work-study/build')
    binaries = dict(reference=reference / 'preparation_profile', off=build / 'preparation_profile', on=build / 'preparation_profile')
    for mode in ['reference', 'on']:
        manifest = json.loads((binaries[mode].parent / 'manifest.json').read_text())
        assert digest(binaries[mode]) == manifest['binary_sha256']
    cases = [dict(pair=pair, rows=rows, mode=mode, name=f'{pair}-{rows}-{mode}')
             for pair in range(1, 4)
             for rows in ([1200, 100000] if pair % 2 else [100000, 1200])
             for mode in (['reference', 'off', 'on'] if pair % 2 else ['on', 'off', 'reference'])]
    protocol = dict(diagnostic_only=True, cases=cases, samples=2000, warmups=20, sources=16,
                    cpu=5, renderer='none', phase='append', fixed_last_price=True,
                    histogram_scope='snapshot only; per-series refresh/preparation/point counts',
                    timing_guard='Any on/control total median or p95 increase >5% is flagged; failed calibration cannot establish unperturbed timing. No retries.',
                    binaries={m: digest(b) for m, b in binaries.items()}, runner_sha256=digest(Path(__file__)))
    (args.output / 'protocol.json').write_text(json.dumps(protocol, indent=2))
    for case in cases:
        name = case['name']
        trace = args.output / (name + '.phase.csv')
        counts = args.output / (name + '.counts.csv')
        env = dict(os.environ, FEATHER_PHASE_TRACE=str(trace.resolve()))
        env.pop('FEATHER_WORK_COUNTS', None)
        if case['mode'] == 'on':
            env['FEATHER_WORK_COUNTS'] = str(counts.resolve())
        command = ['taskset', '-c', '5', str(binaries[case['mode']]), '--rows', str(case['rows']),
                   '--series', '16', '--kind', 'histogram', '--phase', 'append', '--fixed-last-price', '--samples', '2000']
        log = args.output / (name + '.log')
        with log.open('x') as f:
            subprocess.run(command, env=env, stdout=f, stderr=subprocess.STDOUT, check=True)
        text = log.read_text()
        assert f'fixture Histogram rows={case["rows"]} sources=16 phase=append fixed_last_price=true' in text
        parse_phases(text, 2000)
        read_trace(trace, 2000)
        if case['mode'] == 'on':
            frames = read_counts(counts, 2000)
            assert any(c[2] > 0 for frame in frames[20:] for c in frame), 'inactive counters'
        print(name, 'complete', flush=True)


if __name__ == '__main__':
    main()
