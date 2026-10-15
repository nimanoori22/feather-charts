# Histogram latency follow-up

Protocol fixed before execution, 2026-10-15. Starting engine `f69bd11`;
the original [shared-cache matrix](live-append-acceptance.md) and its one failed
Histogram screen remain preserved. No production change before this experiment.

Six pairs use the same frozen release artifact, CPU 5, 16 Histogram sources,
1,200/100,000 starting rows, fixed last price, 20 warmups and 2,000 measured
updates per execution. Odd pairs run small/large; even pairs large/small.
The 10% median/p95 limit stays unchanged. All samples and failures are retained.
Any failing pair counts as reproduction; there are no retries until green.

The profiler prepares a snapshot and glyph frame after every update. Per-sample
sums give CPU preparation time; they do not measure GPU rendering, presentation
or physical input latency. Appending 2,020 points means the smaller history grows
appreciably during the longer run; starting sizes and complete series are reported.

Artifacts and pre-run protocol: `target/incremental-append/histogram-tail-extended/`.

The subsequent [cause investigation](histogram-window-cause.md) adds same-history
controls and fresh hardware counters. The runner now reports all short windows
and their controls without changing strict acceptance or discarding any failure.

## Results

**All six extended pairs pass the unchanged 10% update median/p95 limit.**
The complete [phase table](histogram-tail-measurements.csv) retains all 48
summary rows, including per-sample CPU totals and maxima. There are 24,000
measured updates and 72,000 raw phase observations; no sample was discarded.

| Pair | Small median/p95 (µs) | Large median/p95 (µs) | Median growth | p95 growth |
| --- | --- | --- | --- | --- |
| 1 | 8.621/16.780 | 6.627/12.523 | -23.13% | -25.37% |
| 2 | 7.604/13.283 | 6.760/14.451 | -11.10% | +8.79% |
| 3 | 7.921/14.817 | 6.701/12.175 | -15.40% | -17.83% |
| 4 | 9.854/16.741 | 6.920/13.168 | -29.77% | -21.34% |
| 5 | 10.366/18.567 | 6.521/11.926 | -37.09% | -35.77% |
| 6 | 7.874/14.057 | 6.805/12.127 | -13.58% | -13.73% |

The greatest positive growth is +8.79% p95 (pair 2); every median improves.
This experiment does not reproduce a sustained history-size penalty in the
current engine. It does not identify the exact machine state behind the original
100-sample failure, and it does not overwrite that failed result.

As a separate, descriptive analysis, splitting each retained sequence into twenty
non-overlapping 100-sample windows yields seven failed paired windows out of 120.
All windows are included, matched by append sequence position. This was not the
predefined acceptance rule and is not used to select samples or claim a pass.
It demonstrates that the original short-window screen can fail within executions
whose complete distributions pass. Together with the earlier cache/CPI diagnostics,
this supports short-window tail variation; it does not prove a specific cache
miss caused the original sample or exclude every transient engine cost.

## CPU preparation

At 100k, the sum of each update, snapshot and glyph preparation has median
4.814–4.861 ms and p95 5.044–5.148 ms across the six executions. At 1,200 rows,
corresponding totals are 7.681–7.796 ms and 8.004–8.235 ms. These are percentiles
of per-sample sums, not sums of separate percentiles. Both fixtures report the
same plot bounds, 1,328 hit items, 16 price lines and 16 right-axis labels.

Snapshot preparation dominates these CPU timings; update medians at 100k are
6.521–6.920 µs. There is no larger-history CPU preparation slowdown in this
experiment. The cause of the smaller-history preparation difference was not
isolated; these numbers do not establish GPU frame time or input-to-display
latency. No native smoothness claim follows from CPU preparation alone.

## Handling and acceptance

No production optimization or threading change is justified by these results.
The engineering response is a repeatable, larger-sample guard with every failure
retained. The existing profiler runner now exposes `--histogram-tail`: six
alternating pairs, 2,000 samples and 20 warmups, using a current artifact only.
It writes its budget and artifact identities before the matrix, refuses to
overwrite existing run logs, completes every pair, and exits nonzero on any
update median/p95 growth above 10%. The normal full matrix keeps its original
three runs, 100 samples and 24 screens. No library API, ownership or dependency
changes are needed.

The original full matrix remains **23/24**, and the supplemental study is **6/6**.
The original performance gate is not silently relabeled 24/24 or closed by a
changed sample budget. The Histogram reproduction investigation is complete for
this predefined study; the milestone still carries its original strict timing
exception and physical-device acceptance. A repeated failure of the extended
guard should trigger new profiling, with that run preserved.

## Validation and reproduction

Two CLI tests failed before the new mode, then passed. Three final tests cover
fixed sample/run budgets and alternating order, nonzero failure after all logs
are retained, refusal to overwrite, and preservation of the full matrix's
204 executions/612 summaries. Run them with:

```sh
python3 scripts/test_profile_incremental_append.py
```

For a future controlled measurement, create a fresh directory, place the frozen
release profiler there as `current-profile`, and run:

```sh
python3 scripts/profile_incremental_append.py --histogram-tail --directory target/histogram-tail-new --cpu 5
```

Use a CPU in the available affinity set and run without competing builds, tests,
captures or profilers. The measured six-pair study above used these same profiler
arguments before the runner mode was added; CLI behavior is separately validated
with deterministic executable fixtures. There was no second measurement attempt
until green. The original frozen binary SHA-256 is
`cb7d75974773e14d72c8921653a35a9b151968a2d8b5a5f2b65d9b8b66746b56`.
The model/cache/profiler source hashes still match its saved manifest.

No engine or renderer code changed, so the previously recorded 558 Rust tests
and 984 captures remain the validation for that exact engine. This change's
Python CLI regression suite passes; diff whitespace checks pass. Actual GPU
presentation and physical-device acceptance were not measured in this follow-up.
