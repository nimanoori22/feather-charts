# Histogram work-count investigation

Date: 2026-10-17. Follow-up to [phase profiling](frame-phase-profiling.md).

All six instrumented executions recorded the same per-series Histogram work on
all 12,000 measured frames: one refresh, one preparation and 83 projected points
per series. The 16 series therefore produced 16 refreshes, 16 preparations and
1,328 projected points per frame. This includes the 600 frames selected from
the same executions' total-work p95 tails. Per-series vectors match, so unchanged
totals are not hiding work moving between series.

This rules out extra **counted Histogram work in these captures**. It does not
establish why some snapshots take longer. Other snapshot work, including axis
layout and font work, is unmeasured by these counters. Equal point counts also
do not imply equal instruction, memory, allocation or scheduling costs.

## Protocol and isolation

The frozen reference uses engine source `ae067e2` with the previous wall/thread
CPU phase recorder. The new build verifies every reference source hash and its
executable hash before copying it. A separate Cargo target directory excludes
old Feather library/example artifacts when reusing dependency artifacts.
Instrumentation only patches this isolated copy; production `src/` is unchanged.

The protocol was written before the first execution. There were 18 serial
executions: reference, counters disabled, and counters enabled, at both 1,200
and 100,000 history rows, repeated three times. The second repetition reverses
history and mode order. Each execution uses CPU 5, 20 warmups, 2,000 measured
samples, 16 Histogram series, append updates with fixed last price, visible
logical range 20..100, size 917 by 870.80005 and pixel ratio 1.25. This is a
headless CPU fixture; it does not render or measure presentation.

Counters use thread-local Cells with fixed slots for fixture IDs 1..=16. The
capture brackets snapshot preparation only. Hooks count refresh attempts,
preparation entries (including early exits) and the successfully projected
`items.len()` once per preparation. There are no per-point hooks, clocks,
atomics or file writes in that path. The fixture preallocates frame records and
writes them after timing; collection and buffering overhead is included in the
raw update/snapshot/glyph total used for calibration.

The parser rejects missing, duplicate, negative and out-of-order source/sample
records. Phase records validate clock, PID/thread identity and sample coverage.
Analysis compares exact per-series vectors, using the existing total-tail sample
selection rather than independently selecting slow Histogram frames.

## Results and calibration

Every enabled execution has one work pattern, 100 total-tail frames and 1,900
normal frames. The snapshot share of total-tail time remains 92.74–93.39%.
Timing values below are milliseconds, diagnostic only.

| Execution | Snapshot median | Snapshot p95 | Total p95 |
| --- | ---: | ---: | ---: |
| 1-1200-on | 1.069649 | 1.123815 | 1.210931 |
| 1-100000-on | 1.065882 | 1.121645 | 1.202301 |
| 2-100000-on | 1.087764 | 1.146524 | 1.224361 |
| 2-1200-on | 1.082838 | 1.136385 | 1.220716 |
| 3-1200-on | 1.091464 | 1.155360 | 1.235081 |
| 3-100000-on | 1.069808 | 1.125507 | 1.204155 |

There are **zero flags in 12 enabled/control comparisons**. The predeclared
flag is a >5% increase in total median or p95 against either the frozen reference
or the same executable with counters disabled. Median changes span −2.27% to
+2.34%; p95 changes span −2.02% to +1.88%. This batch meets that guard; it does
not prove zero observer effect or repair the earlier phase-profiler calibration
flags. No execution was retried to obtain a pass.

The [machine-readable summary](histogram-work-study-summary.json) retains all
comparisons and hashes of raw logs, phase/count CSVs, build manifest, protocol,
environment and analysis. Local artifacts are under
`target/histogram-work-study/`; the plot is
`target/histogram-work-study/analysis/work-and-latency.svg`. These target files
are local and are not committed. The tracked tools and hashes preserve the
procedure and evidence identity; rerunning requires the retained frozen reference.

## Source verification and validation

`HistogramPaneView::refresh` clears the old renderer data and prepares the
series. `prepare` selects visible rows, projects coordinates and clones each
point's color before returning renderer data. The actual consumer is the
Histogram match arm in `src/ui/line_chart.rs::prepare_series_plot`, after axis
snapshot preparation. The graph omitted this generic/dynamic incoming edge;
direct source inspection verified it. Coverage metadata matched for the relied
source and tools; graph coverage remains best effort.

Validation: 399 isolated Rust library tests (including two counter tests), 61
Python tests (including three new analysis tests), one-series and 16-series
smoke captures, Rust formatting and strict Clippy on the isolated example passed. The tests cover reset, inactive and
thread-local capture, per-series work redistribution, exact slow-frame joining
and malformed count captures. The runner checks fixture metadata, phase/sample
coverage and active projected-point counts on every enabled execution.

## Decision and next measurement

No production performance change is justified by these counts. The original
historical p95 regression is not established as reproduced or explained, and
unchanged acceptance has not been rerun. Existing update, actual-presentation
and physical-device acceptance gates remain open.

The next discriminating experiment is aggregate time (and, if calibrated,
user-cycle/instruction counts) inside Histogram preparation versus the remaining
snapshot work, joined to the same slow frames. Use an isolated build, a fixed
finite protocol and reference/off/on overhead controls again. If Histogram time
does not explain the tail, measure the axis/font portion next. Do not add a
cache or change engine scheduling solely from sample enrichment or these counts.
