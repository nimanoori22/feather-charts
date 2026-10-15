"""Descriptive same-history controls; strict timing acceptance stays separate."""
import math


def analyze_windows(runs, window_size=100, limit=.1):
    """Compare all windows and consecutive run pairs without filtering samples.

    A failed same-history control confounds attribution to history size. It does
    not cancel a raw failure or prove there is no engine regression.
    """
    if not isinstance(window_size, int) or window_size <= 0:
        raise ValueError('window_size must be a positive integer')
    if not math.isfinite(limit) or limit < 0:
        raise ValueError('limit must be finite and nonnegative')
    if not runs or len(runs) % 2:
        raise ValueError('complete even run pairs are required')
    count = None
    for histories in runs.values():
        if set(histories) != {1200, 100000}:
            raise ValueError('each run must contain both history sizes')
        for values in histories.values():
            if count is None:
                count = len(values)
            if not values or len(values) != count or len(values) % window_size:
                raise ValueError('equal, complete nonempty windows are required')
            if any(not math.isfinite(value) or value <= 0 for value in values):
                raise ValueError('timings must be finite and positive')

    def compare(reference, current, start):
        left = sorted(reference[start:start + window_size])
        right = sorted(current[start:start + window_size])
        median_growth = right[window_size // 2] / left[window_size // 2] - 1
        p95_growth = right[window_size * 95 // 100] / left[window_size * 95 // 100] - 1
        return dict(start=start, count=window_size, median_growth=median_growth,
                    p95_growth=p95_growth, exceeds_limit=max(median_growth, p95_growth) > limit)

    numbers = sorted(runs)
    history = [dict(run=number, **compare(runs[number][1200], runs[number][100000], start))
               for number in numbers for start in range(0, count, window_size)]
    controls = [dict(rows=rows, reference_run=reference, run=current,
                     **compare(runs[reference][rows], runs[current][rows], start))
                for reference, current in zip(numbers[::2], numbers[1::2])
                for rows in [1200, 100000] for start in range(0, count, window_size)]
    history_exceeded = sum(record['exceeds_limit'] for record in history)
    control_exceeded = sum(record['exceeds_limit'] for record in controls)
    attribution = 'no_growth_observed'
    if history_exceeded:
        attribution = 'control_failed' if control_exceeded else 'history_growth_observed'
    return dict(history_windows=history, control_windows=controls,
                history_exceeded=history_exceeded, control_exceeded=control_exceeded,
                attribution=attribution)
