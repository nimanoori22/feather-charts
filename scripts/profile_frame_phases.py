"""Bounded diagnostic controls or CPU profiles; never an acceptance runner."""
import argparse
import json
import os
from pathlib import Path
import subprocess

from analyze_frame_phases import digest, parse_phases, read_trace, summarize


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--mode', choices=['overhead', 'stacks'], required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--stack-bytes', type=int, default=65528)
    parser.add_argument('--frame-pointers', action='store_true')
    args = parser.parse_args()
    root = Path.cwd()
    build = root / ('target/frame-phase-investigation/build-fp' if args.frame_pointers
                    else 'target/frame-phase-investigation/build-v3')
    args.output.mkdir(parents=True, exist_ok=False)
    cases = []
    for pair in range(1, 4):
        trace_modes = ([False, True] if pair % 2 else [True, False]) if args.mode == 'overhead' else [True]
        for rows in ([1200, 100000] if pair % 2 else [100000, 1200]):
            for version in (['baseline', 'candidate'] if pair % 2 else ['candidate', 'baseline']):
                for enabled in trace_modes:
                    cases.append(dict(pair=pair, rows=rows, version=version, trace=enabled,
                                      name=f'{pair}-{rows}-{version}-{int(enabled)}'))
    binaries = {v: build / v / 'preparation_profile' for v in ['baseline', 'candidate']}
    identities = {v: digest(p) for v, p in binaries.items()}
    assert len(set(identities.values())) == 2, 'baseline/candidate artifact collision'
    for version, value in identities.items():
        assert value == json.loads((build / version / 'manifest.json').read_text())['binary_sha256']
    protocol = dict(diagnostic_only=True, cases=cases, samples=2000, warmups=20,
                    cpu=5, mode=args.mode, event='cycles:u', frequency=199, clock='mono', stack_bytes=args.stack_bytes,
                    callgraph='fp' if args.frame_pointers else f'dwarf,{args.stack_bytes}',
                    binaries=identities,
                    runner_sha256=digest(Path(__file__)),
                    timing_inference='INCONCLUSIVE: both overhead batches had one >5% control flag; profiles are diagnostic evidence only.',
                    overhead_guard='Any paired >5% total median/p95 increase flags perturbation; no acceptance result.')
    (args.output / 'protocol.json').write_text(json.dumps(protocol, indent=2))
    results = []
    for case in cases:
        name = case['name']
        binary = binaries[case['version']]
        trace = (args.output / (name + '.csv')).resolve()
        env = dict(os.environ)
        env.pop('FEATHER_PHASE_TRACE', None)
        if case['trace']:
            env['FEATHER_PHASE_TRACE'] = str(trace)
        command = ['taskset', '-c', '5', str(binary), '--rows', str(case['rows']), '--series', '16',
                   '--kind', 'histogram', '--phase', 'append', '--fixed-last-price', '--samples', '2000']
        data = args.output / (name + '.data')
        if args.mode == 'stacks':
            command = ['perf', 'record', '--clockid', 'mono', '-F', '199', '-e', 'cycles:u',
                       '--call-graph', protocol['callgraph'], '-o', str(data), '--', *command]
        path = args.output / (name + '.log')
        with path.open('x') as log:
            subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        text = path.read_text()
        assert f'fixture Histogram rows={case["rows"]} sources=16 phase=append fixed_last_price=true' in text
        assert 'profile=release warmup=20 samples=2000 ' in text
        values = parse_phases(text, 2000)
        result = dict(**case, stats=summarize(values), binary_sha256=digest(binary))
        if case['trace']:
            records = read_trace(trace, 2000)
            result['cpu'] = {}
            for phase in ['update', 'snapshot', 'glyph']:
                selected = [r for r in records if r['sample'] >= 20 and r['phase'] == phase]
                wall = sum(r['end_ns'] - r['start_ns'] for r in selected)
                cpu = sum(r['cpu_ns'] for r in selected)
                result['cpu'][phase] = dict(wall_ns=wall, cpu_ns=cpu, cpu_wall_ratio=cpu / wall,
                    **{k: sum(r[k] for r in selected) if all(r[k] is not None for r in selected) else None
                       for k in ['minor', 'major', 'voluntary', 'involuntary']})
        if args.mode == 'stacks':
            with (args.output / (name + '.stacks')).open('x') as output, \
                    (args.output / (name + '.script-stderr')).open('x') as errors:
                subprocess.run(['perf', 'script', '--ns', '--show-lost-events', '-i', str(data),
                                '-F', 'comm,pid,tid,time,event,ip,sym,dso'],
                               stdout=output, stderr=errors, check=True)
            with (args.output / (name + '.leaves')).open('x') as output, \
                    (args.output / (name + '.leaves-stderr')).open('x') as errors:
                subprocess.run(['perf', 'script', '--ns', '-G', '--show-lost-events', '-i', str(data),
                                '-F', 'comm,pid,tid,time,event,ip,sym,dso'],
                               stdout=output, stderr=errors, check=True)
        results.append(result)
        (args.output / 'analysis.json').write_text(json.dumps(results, indent=2))
        print(name, result['stats']['total'], result.get('cpu', {}).get('snapshot'), flush=True)
    if args.mode == 'overhead':
        controls = []
        for off in [r for r in results if not r['trace']]:
            on = next(r for r in results if r['trace'] and all(r[k] == off[k] for k in ['pair', 'rows', 'version']))
            growth = {k: on['stats']['total'][k] / off['stats']['total'][k] - 1 for k in ['median', 'p95']}
            controls.append(dict(pair=off['pair'], rows=off['rows'], version=off['version'],
                                 **growth, flagged=max(growth.values()) > .05))
        (args.output / 'overhead.json').write_text(json.dumps(controls, indent=2))
        print('perturbation flags', sum(r['flagged'] for r in controls), '/', len(controls), flush=True)


if __name__ == '__main__':
    main()
