# Histogram short-window cause investigation

Follow-up to [the extended study](histogram-tail-follow-up.md), 2026-10-15.
Engine remains `f69bd11`; no engine, renderer or system-setting changes.

## What was established

The 100-sample screen confounds ordinary temporal/process variation with a
history-size effect. It compares the p95 of two independent executions and calls
any ratio above 1.10 a failed screen. At 100 samples the profiler's p95 is sorted
index 95, with only four observations above it. Short changes in memory-access
latency can move that cutoff across the threshold.

A same-history control demonstrates the attribution problem without changing the
binary, data size, viewport, sample position or threshold. For each history size,
compare runs 1→2, 3→4 and 5→6 in the retained extended study, using every complete,
nonoverlapping 100-sample window:

| Comparison | Flagged windows | Total windows |
| --- | ---: | ---: |
| 1,200→100,000 rows | 7 | 120 |
| 1,200→1,200 rows | 15 | 60 |
| 100,000→100,000 rows | 22 | 60 |

All [240 comparisons](histogram-window-controls.csv) are retained. The 37
same-history flags cannot be caused by increasing history size: that variable
never changed. They establish that a flagged point estimate alone does not
identify a history regression. They do not prove every flagged history comparison
is harmless or that every engine path is optimal.

One previously flagged large-history window has median/p95 14.540/21.176 µs
against 7.520/13.995 µs in its smaller-history comparison. Its following
large-history window returns to approximately 6.8 µs median. This is a temporary
execution-cost change, not evidence of permanent growth with stored history.

## What makes individual updates slower

Four fresh instrumented executions use 2,000 samples each, 20 warmups, CPU 5,
16 Histogram sources, in order 1,200/100k/100k/1,200. The same public update fixture
prepares a snapshot and glyph frame after each update. No agent builds, tests or
captures run concurrently. All 8,000 samples and hardware enabled/running times
are retained; none of the three events was multiplexed.

Compare each execution's slowest 5% with its middle 50%. Ratios below compare the
median of each group's measurements; CPI means cycles per retired instruction.

| History/run | Wall ratio | Instruction ratio | CPI ratio | Generic cache-miss ratio | Slow samples with faults/switches |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1,200 / 1 | 1.938 | 1.000 | 1.647 | 10.27 | 34/100 |
| 100k / 2 | 1.798 | 1.003 | 1.722 | 17.63 | 0/100 |
| 100k / 3 | 1.757 | 0.998 | 1.669 | 17.27 | 0/100 |
| 1,200 / 4 | 1.972 | 0.999 | 1.704 | 11.39 | 35/100 |

Both large-history executions have zero faults and context switches over all
samples. Their slow updates execute essentially unchanged instructions with more
CPU stalls/cache misses. Small-history runs include minor faults, so not every
slow update has one cause. The user-cycle/thread-nanosecond proxy changes by only
1.026–1.038× for the large-history slow groups; it is not a calibrated CPU-frequency
measurement. These observations support memory/cache latency variation rather
than extra engine work in those measured tails.

Hardware counters enclose fixed instrumentation overhead and perturb the process.
They do not identify a specific cache level, allocation or source instruction.
The original seven wall-only windows cannot retrospectively be assigned exact
hardware causes. No claim is made that a particular historical sample was proven
to be a cache miss, or that operating-system interrupts never contributed.

Artifacts: `target/incremental-append/histogram-window-cause/` contains the pre-run
protocol and binary hash, four counter logs, counter analysis, all 80 diagnostic
windows, and the complete 240-window descriptive analysis.

## Fix

The fix addresses the demonstrated attribution problem. The focused runner now
loads every raw update sample, verifies its sample budget, and writes
`window-analysis.json` with every history comparison and same-history control.
The pure analysis lives in `scripts/histogram_window_analysis.py`.

When both history and same-history controls exceed 10%, it reports
`attribution=control_failed`: history attribution is inconclusive. Stable controls
with observed history growth report `history_growth_observed`; a data set with no
history flags reports `no_growth_observed`. These are descriptive labels, not
statistical confidence intervals or proof of causality.

No failure is removed, divided out, accepted using a noise allowance, or replaced
with a passing sample. Existing strict screens, their 10% threshold and exit
statuses stay unchanged. Both a genuine doubled-history timing fixture and noisy
same-history fixtures have regression tests. Incomplete/invalid raw samples are
rejected rather than silently reducing the sample budget.

No engine change is justified by these controls alone. Warming chart state outside
the timer, filtering slow samples or changing global CPU settings would not repair
the causal comparison. The next engine optimization requires a specific measured
operation whose removal improves latency while preserving behavior.

## Validation and limits

- Eight independent analysis tests were RED before implementation, then passed.
- Two runner integration tests were RED before the report existed, then passed.
- All 12 Python tests pass with `python3 -m unittest discover -s scripts -p 'test_*.py'`.
- Replay of the actual saved logs through the updated runner preserves its 6/6
  extended result and reproduces 7/120 history flags plus 37/120 control flags.
  Replay validates parsing/reporting only; it is not another timing measurement.
- All 240 checked-in CSV records match the retained analysis; no raw input changed.
- Diff whitespace checks pass. No Rust code changed; the previously validated
  engine remains the same. GPU presentation and physical-device feel were not tested.

The original full matrix remains 23/24; supplemental extended evidence remains
6/6. Short-window microsecond variation remains observable. Its attribution is now
qualified by a control, rather than presented as an established history-size bug.
