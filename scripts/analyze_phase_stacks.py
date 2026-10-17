"""Join perf samples to measured phase windows, excluding setup and warmups."""
import argparse
from bisect import bisect_right
from collections import Counter, defaultdict
import json
from pathlib import Path
import re

from analyze_frame_phases import read_trace


def parse_samples(text):
    if 'PERF_RECORD_LOST' in text or 'LOST_SAMPLES' in text:
        raise ValueError('Lost perf events; inspect the retained recording')
    header_pattern = r'^\s*.*?\s+(\d+)/(\d+)\s+(\d+)\.(\d{1,9}):\s+cycles:u:'
    blocks = []
    for line in text.splitlines():
        if not line.strip():
            continue
        if re.match(header_pattern, line):
            blocks.append([line])
        elif blocks and re.match(r'^\s+[0-9a-f]+\s+.+$', line):
            blocks[-1].append(line)
        else:
            raise ValueError(f'Unrecognized perf record: {line[:200]}')
    samples = []
    for lines in blocks:
        header = re.match(header_pattern, lines[0])
        pid, tid, seconds, fraction = header.groups()
        symbols = []
        leaf = re.match(r'^\s*[0-9a-f]+\s+(.+)$', lines[0][header.end():])
        if leaf:
            symbols.append(leaf.group(1))
        for line in lines[1:]:
            frame = re.match(r'^\s+[0-9a-f]+\s+(.+)$', line)
            if not frame:
                raise ValueError(f'Unrecognized stack frame: {line[:200]}')
            symbols.append(frame.group(1))
        samples.append(dict(pid=int(pid), tid=int(tid),
                            time_ns=int(seconds) * 10**9 + int(fraction.ljust(9, '0')), symbols=symbols))
    return samples


def matching_phase(rows, starts, sample):
    i = bisect_right(starts, sample['time_ns']) - 1
    if i < 0:
        return None
    row = rows[i]
    if (sample['time_ns'] >= row['end_ns'] or row['pid'] != sample['pid']
            or row['tid'] != sample['tid']):
        return None
    return row


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    runs = json.loads((args.input / 'analysis.json').read_text())
    output = []
    for run in runs:
        name = run['name']
        rows = read_trace(args.input / (name + '.csv'), 2000)
        starts = [r['start_ns'] for r in rows]
        samples = parse_samples((args.input / (name + '.stacks')).read_text())
        leaves = parse_samples((args.input / (name + '.leaves')).read_text())
        if [(r['pid'], r['tid'], r['time_ns']) for r in samples] != \
                [(r['pid'], r['tid'], r['time_ns']) for r in leaves]:
            raise ValueError('Instruction-pointer/callchain identity mismatch')
        tail = {i + 20 for i in run['stats']['tail_samples']}
        groups = defaultdict(lambda: dict(count=0, empty=0, unknown_leaf=0,
                                         self=Counter(), inclusive=Counter(), caller_tags=Counter(), multiple_frames=0))
        events = []
        for r in rows:
            events.append(dict(ph='X', name=r['phase'], pid=r['pid'], tid=r['tid'],
                ts=r['start_ns'] / 1000, dur=(r['end_ns'] - r['start_ns']) / 1000,
                args={k: r[k] for k in ['sample', 'warmup', 'cpu_ns', 'minor', 'major', 'involuntary']}))
        # Excluded samples include setup, warmups, iteration gaps and other threads.
        outside = 0
        for sample, leaf in zip(samples, leaves):
            row = matching_phase(rows, starts, sample)
            if row is None or row['sample'] < 20:
                outside += 1
                continue
            group = groups[row['phase'] + ('/tail' if row['sample'] in tail else '/normal')]
            group['count'] += 1
            symbols = sample['symbols']
            if not symbols:
                group['empty'] += 1
            group['multiple_frames'] += len(symbols) > 1
            leaf_symbol = leaf['symbols'][0] if leaf['symbols'] else '[unknown]'
            group['self'][leaf_symbol.rsplit(' (', 1)[0]] += 1
            group['unknown_leaf'] += '[unknown]' in leaf_symbol
            group['inclusive'].update(set(symbols))
            for tag, needles in {
                    'font_or_text': ['cosmic_text', 'harfrust', 'swash', 'skrifa', 'fontdb', 'read_fonts', 'Paragraph', 'measure_text'],
                    'histogram_projection': ['histogram_pane_view', 'HistogramPaneView'],
                    'formatting': ['format_exact', 'PriceFormatter', 'flt2dec'],
                    'dash_path': ['dash_path']}.items():
                if any(needle in symbol for needle in needles for symbol in [leaf_symbol, *symbols]):
                    group['caller_tags'][tag] += 1
            events.append(dict(ph='i', s='t', name='CPU sample', pid=row['pid'], tid=row['tid'],
                               ts=sample['time_ns'] / 1000, args=dict(sample=row['sample'], stack='\n'.join(symbols),
                                   instruction_pointer=leaf_symbol, callchain_missing=not bool(symbols))))
        cpu_tails = {}
        for phase in ['update', 'snapshot', 'glyph']:
            selected = [r for r in rows if r['sample'] in tail and r['phase'] == phase]
            wall = sum(r['end_ns'] - r['start_ns'] for r in selected)
            cpu_tails[phase] = dict(wall_ns=wall, cpu_ns=sum(r['cpu_ns'] for r in selected),
                                   **{k: sum(r[k] for r in selected) if all(r[k] is not None for r in selected) else None
                                      for k in ['minor', 'major', 'voluntary', 'involuntary']})
        output.append(dict(name=name, rows=run['rows'], version=run['version'], pair=run['pair'],
            perf_samples=len(samples), excluded_setup_warmup=outside, cpu_tails=cpu_tails,
            groups={k: dict(count=v['count'], empty=v['empty'], unknown_leaf=v['unknown_leaf'],
                            multiple_frames=v['multiple_frames'], caller_tags=dict(v['caller_tags']),
                            self=v['self'].most_common(), inclusive=v['inclusive'].most_common(30))
                    for k, v in groups.items()}))
        (args.output / (name + '.trace.json')).write_text(json.dumps(dict(traceEvents=events)))
    (args.output / 'analysis.json').write_text(json.dumps(output, indent=2))
    for r in output:
        g = r['groups'].get('snapshot/tail', {})
        print(r['name'], 'snapshot tail samples', g.get('count'), 'top inclusive', g.get('inclusive', [])[:3])


if __name__ == '__main__':
    main()
