"""Run one fixed reference/off/on batch; retain failures without acceptance retries."""
import argparse
from contextlib import contextmanager
import ctypes
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import sys

from analyze_frame_phases import digest, parse_phases, read_trace
from histogram_work_analysis import read_counts, read_histogram_times


def worker_ticks(pid):
    fields = Path(f'/proc/{pid}/stat').read_text().rsplit(') ', 1)[1].split()
    return sum(int(fields[i]) for i in [11, 12])


@contextmanager
def cpu_contender(enabled):
    worker = None
    try:
        if enabled:
            worker = subprocess.Popen(['taskset', '-c', '5', sys.executable, '-u', __file__,
                                       '--load-parent', str(os.getpid())], stdout=subprocess.PIPE, text=True)
            if not select.select([worker.stdout], [], [], 10)[0] or worker.stdout.readline().strip() != 'READY':
                raise RuntimeError('CPU contender did not become ready')
            assert os.sched_getaffinity(worker.pid) == {5}
        yield worker
    finally:
        if worker is not None:
            if worker.poll() is None:
                worker.terminate()
            try:
                worker.wait(timeout=3)
            except subprocess.TimeoutExpired:
                worker.kill()
                worker.wait()
            worker.stdout.close()


def environment():
    return dict(load_average=os.getloadavg(),
                pressure={name: Path(f'/proc/pressure/{name}').read_text() for name in ['cpu', 'memory', 'io']})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--timing-load', action='store_true')
    parser.add_argument('--load-parent', type=int, help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.load_parent is not None:
        assert ctypes.CDLL(None).prctl(1, signal.SIGTERM, 0, 0, 0) == 0
        assert os.getppid() == args.load_parent
        signal.alarm(120)
        print('READY', flush=True)
        value = 1
        while True:
            value = (value * 1664525 + 1013904223) & 0xffffffff
    if args.output is None:
        args.output = Path('target/histogram-timing-load/runs' if args.timing_load else 'target/histogram-work-study/runs')
    args.output.mkdir(parents=True, exist_ok=False)
    reference = Path('target/frame-phase-investigation/build-v3/candidate')
    build = Path('target/histogram-timing-load/build' if args.timing_load else 'target/histogram-work-study/build')
    binaries = dict(reference=reference / 'preparation_profile', off=build / 'preparation_profile', on=build / 'preparation_profile')
    for mode in ['reference', 'on']:
        manifest = json.loads((binaries[mode].parent / 'manifest.json').read_text())
        assert digest(binaries[mode]) == manifest['binary_sha256']
    cases = [dict(pair=pair, rows=rows, mode=mode, condition=condition,
                  name=f'{pair}-{rows}-{mode}' + (f'-{condition}' if args.timing_load else ''))
             for pair in range(1, 4)
             for condition in ((['ambient', 'cpu-contention'] if pair % 2 else ['cpu-contention', 'ambient'])
                               if args.timing_load else ['ambient'])
             for rows in ([1200, 100000] if pair % 2 else [100000, 1200])
             for mode in (['reference', 'off', 'on'] if pair % 2 else ['on', 'off', 'reference'])]
    protocol = dict(diagnostic_only=True, timing=args.timing_load, cases=cases, samples=2000, warmups=20, sources=16,
                    cpu=5, renderer='none', phase='append', fixed_last_price=True,
                    histogram_scope='snapshot only; per-series refresh/preparation/point counts',
                    timing_guard='Any on/control total median or p95 increase >5% is flagged; failed calibration cannot establish unperturbed timing. No retries.',
                    binaries={m: digest(b) for m, b in binaries.items()}, runner_sha256=digest(Path(__file__)))
    if args.timing_load:
        protocol.update(load='One owned same-CPU arithmetic worker; ambient leaves existing applications running',
                        environment=environment(), affinity=sorted(os.sched_getaffinity(0)))
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
        with cpu_contender(args.timing_load and case['condition'] == 'cpu-contention') as worker:
            before = worker_ticks(worker.pid) if worker else None
            observation = dict(before=environment()) if args.timing_load else None
            with log.open('x') as f:
                subprocess.run(command, env=env, stdout=f, stderr=subprocess.STDOUT, check=True, timeout=120)
            if worker:
                assert worker.poll() is None, 'CPU contender exited during capture'
                observation['worker_pid'] = worker.pid
                observation['worker_cpu_ticks'] = worker_ticks(worker.pid) - before
                assert observation['worker_cpu_ticks'] > 0, 'CPU contender did no work'
            if observation is not None:
                observation['after'] = environment()
                (args.output / (name + '.environment.json')).write_text(json.dumps(observation, indent=2))
        text = log.read_text()
        assert f'fixture Histogram rows={case["rows"]} sources=16 phase=append fixed_last_price=true' in text
        parse_phases(text, 2000)
        read_trace(trace, 2000)
        if case['mode'] == 'on':
            frames = read_counts(counts, 2000)
            assert any(c[2] > 0 for frame in frames[20:] for c in frame), 'inactive counters'
            if args.timing_load:
                read_histogram_times(counts, 2000)
        print(name, 'complete', flush=True)


if __name__ == '__main__':
    main()
