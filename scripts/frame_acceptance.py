"""Decision rules for the recorded live-append acceptance policy."""
import math

WORK_BUDGET_NS = 16_666_667
PRESENTATION_BUDGET_NS = 33_333_334


def evaluate_update_growth(screens, controls):
    try:
        if len(screens) != 6 or {row['run'] for row in screens} != set(range(1, 7)):
            raise ValueError('six unique history pairs are required')
        expected = {(rows, reference, current) for rows in [1200, 100000]
                    for reference, current in [(1, 2), (3, 4), (5, 6)]}
        if len(controls) != 6 or {(row['rows'], row['reference_run'], row['run']) for row in controls} != expected:
            raise ValueError('complete same-history control pairs are required')
        if any(row['count'] != 2000 or row['start'] != 0 for row in controls):
            raise ValueError('controls must use all 2000 samples')
        for row in screens + controls:
            for key in ['median_growth', 'p95_growth']:
                if not math.isfinite(row[key]) or row[key] < -1:
                    raise ValueError('invalid growth value')
    except (KeyError, TypeError, ValueError) as error:
        return dict(status='INCONCLUSIVE', reason=str(error))
    exceeded = any(max(row['median_growth'], row['p95_growth']) > .1 for row in screens)
    noisy = any(max(row['median_growth'], row['p95_growth']) > .1 for row in controls)
    if not exceeded:
        return dict(status='PASS', reason='all six complete history pairs satisfy the 10% limit')
    return dict(status='INCONCLUSIVE' if noisy else 'FAIL',
                reason='history screen exceeded; same-history control also exceeded' if noisy
                else 'history screen exceeded without a same-history control exceedance')


def evaluate_frame_cohort(records):
    discarded = 0
    work, latency = [], []
    try:
        if len(records) != 100:
            raise ValueError('exactly 100 measured frames are required')
        for row in records:
            start, duration, presented = row['started_ns'], row['work_ns'], row['presented_ns']
            if not math.isfinite(start) or start <= 0 or not math.isfinite(duration) or duration < 0:
                raise ValueError('invalid frame clock/work value')
            if not isinstance(row['discarded'], bool) or not row.get('resolved', True):
                raise ValueError('invalid or unresolved feedback')
            work.append(duration)
            if row['discarded']:
                if presented is not None:
                    raise ValueError('discarded frame cannot also be presented')
                discarded += 1
                latency.append(math.inf)
            else:
                if presented is None or not math.isfinite(presented) or presented < start:
                    raise ValueError('missing or invalid presentation feedback')
                latency.append(presented - start)
    except (KeyError, TypeError, ValueError) as error:
        return dict(status='INCONCLUSIVE', reason=str(error), work_p95_ns=None,
                    presentation_p95_ns=None, discarded_count=discarded)
    work.sort()
    latency.sort()
    passed = work[95] <= WORK_BUDGET_NS and latency[95] <= PRESENTATION_BUDGET_NS
    return dict(status='PASS' if passed else 'FAIL', reason='budgets satisfied' if passed else 'frame budget exceeded',
                work_p95_ns=work[95], presentation_p95_ns=latency[95] if math.isfinite(latency[95]) else None,
                discarded_count=discarded)


def main():
    import argparse
    import ast
    import hashlib
    import json
    from pathlib import Path
    import re
    from histogram_window_analysis import analyze_windows

    parser = argparse.ArgumentParser(description='Evaluate complete update samples without changing the original strict screen.')
    parser.add_argument('--updates', type=Path, required=True)
    args = parser.parse_args()
    root = args.updates
    try:
        protocol = json.loads((root / 'protocol.json').read_text())
        if not all(protocol[k] == v for k, v in dict(runs=6, samples=2000, warmups=20,
                                                    growth_limit=.1, histogram_tail=True).items()):
            raise ValueError('protocol mismatch')
        if hashlib.sha256((root / 'current-profile').read_bytes()).hexdigest() != protocol['artifacts']['current']:
            raise ValueError('artifact mismatch')
        runs = {}
        for run in range(1, 7):
            runs[run] = {}
            for rows in [1200, 100000]:
                log = (root / f'run-{run}-current-histogram-{rows}-16-append.log').read_text()
                if 'diagnostic_counters=enabled' in log or 'diagnostic_only=true' in log:
                    raise ValueError('instrumented diagnostics cannot establish acceptance')
                matches = re.findall(rf'^raw Histogram rows={rows} sources=16 phase=append/update ms=(.+)$', log, re.M)
                if len(matches) != 1:
                    raise ValueError('missing/duplicate raw update samples')
                values = ast.literal_eval(matches[0])
                if len(values) != 2000:
                    raise ValueError('incomplete update samples')
                runs[run][rows] = values
        complete = analyze_windows(runs, window_size=2000)
        result = evaluate_update_growth(complete['history_windows'], complete['control_windows'])
        result.update(complete=complete, short_windows=analyze_windows(runs))
    except (OSError, ValueError, KeyError, TypeError, AssertionError, SyntaxError) as error:
        result = dict(status='INCONCLUSIVE', reason=str(error))
    print(json.dumps(result, indent=2, allow_nan=False))
    raise SystemExit(0 if result['status'] == 'PASS' else 1)


if __name__ == '__main__':
    main()
