# Slow-frame phase attribution and CPU profiling

Date: 2026-10-17. Result: snapshot preparation located and profiled; the cause
of the historical timing regression remains **INCONCLUSIVE**. Acceptance remains
**OPEN**. No production engine behavior changed.

Plan: [roadmap](../../tasks/frame-phase-profiling-plan.md) and
[checklist](../../tasks/frame-phase-profiling-todo.md). Machine-readable evidence:
[summary](frame-phase-profiling-summary.json).

## What was established

All 24 retained uninstrumented executions were reconstructed from their original
ordered arrays, with executable and raw-log hashes checked. The total for each
sample is update + snapshot + glyph. Its slowest 5% selects the same sample IDs
for phase attribution; independent phase p95s are never added.

Snapshot preparation accounts for **89.6–93.4%** of the selected total-tail time
across these executions. In the original pair 4, 100,000-row comparison, total
p95 remains **1.259479 -> 1.663795 ms**, or +32.10%. The separately selected
total-tail mean snapshot durations are 1.226228 and 1.621749 ms. This locates a
broad phase; it does not recover the old stacks or prove an engine defect.

Raw attribution, sample timelines and per-execution distributions are in
`target/frame-phase-investigation/retained/`. The CSV preserves all 48,000
measured sample identities, including the warmup offset.

## Reproduction and tooling

The user installed `perf`; version 7.2.3-1 and user-space hardware sampling were
verified on kernel 7.2.5-3-omarchy. No global permission, power, priority or
desktop settings were changed. Scheduler tracepoints were denied both inside
and outside the sandbox. Privileged scheduling evidence was not collected.

The headless workload remains 16 Histogram sources, 1,200/100,000 starting rows,
fixed viewport/price formatting, 20 warmups and 2,000 measured iterations on
CPU 5. Logical size is 917 x 870.80005 at ratio 1.25; this fixture has no GPU
renderer, monitor or actual presentation measurement. The environment manifest
records hardware, compiler, kernel, affinity, observed power/frequency/load and
dependency-lock identity. Sources are baseline `e148fde` and candidate `ae067e2`.

Diagnostic sources and executables are isolated under the artifact directory.
They have release optimization and package debug information. The final profile
build additionally forces frame pointers for Rust dependencies. Those codegen
and instrumentation differences preclude treating its durations as acceptance.
Original frozen executables and earlier failed results remain intact.

The first shared-cache build copied a stale executable; its functional smoke
check failed on an unrelated old sampling requirement. It was excluded. The
corrected builder uses separate target directories and excludes chart/library/
example artifacts when reusing dependencies. Both source revisions then rebuilt
and produced distinct executable hashes and complete trace records. The initial
injection-anchor assertion failure is also retained.

## Instrumentation controls

The first recorder observed wall time, thread CPU time and resource usage at
phase boundaries. Its predefined three-pair control batch at both histories and
both source revisions contains 24 executions. One of 12 off/on comparisons
failed the 5% perturbation guard: candidate, 1,200 rows, pair 2, total p95
**+12.01%** (median +4.03%).

The controlled intervention removed all per-boundary resource-usage queries.
The reduced recorder buffers only wall/CPU stamps and flushes after measurement.
Unavailable resource fields are blank and parsed as null, never as measured
zeroes. Its second fixed 24-execution control batch still has one flag: baseline,
1,200 rows, pair 2, p95 **+7.29%** (median +1.70%). This does not prove either
flag was caused by the recorder, scheduling or noise. **Overhead qualification
remains INCONCLUSIVE.** No further overhead retries were made.

Records carry sample, phase, PID/TID, warmup and clock identity. CSV validation
requires complete ordered phases, contiguous boundaries within an iteration,
one thread and nonnegative observations. The recorder rejects overflow or an
incomplete count. Timestamp observations use CLOCK_MONOTONIC, as does perf;
thread CPU duration uses CLOCK_THREAD_CPUTIME_ID. Boundary overhead and clock
read ordering mean wall-minus-CPU is not exact off-CPU accounting.

## Stack capture and findings

Two fixed profile batches, each 12 executions in AB/BA/AB order at both histories,
use 199 Hz user-space cycle sampling. All startup and warmup samples are excluded
from phase findings by timestamp and process/thread identity. Raw recordings,
stack exports and separate instruction-pointer exports are preserved.
The excluded-sample count also includes iteration gaps and other threads; it
must not be interpreted as startup cost alone.

A 16 KiB DWARF smoke capture had substantial missing call chains. A 64 KiB
capture improved the smoke result, but the full DWARF batch still lacks call
chains for 4,748 of 5,661 measured snapshot samples. Instruction pointers were
retained separately; empty chains were not silently discarded. An alternate
unwinder did not restore the missing smoke chains.

The frame-pointer fallback passed its smoke guard (26/26 preparation samples
with multiple frames). Its fixed batch recovers multi-frame call chains for all
**6,001 measured snapshot samples**, including **476 in total-tail frames**.
There are no reported lost-event records in the parsed exports. Some leaf
symbols remain unresolved: **82/476** snapshot-tail samples, mainly in libc.

Snapshot CPU/wall ratios in the frame-pointer batch's total-tail subsets range
from **97.28% to 97.94%**. Active computation therefore dominates these captured
intervals. This is diagnostic evidence under profiling; it does not exclude
scheduling effects in other frames or establish the historical regression's cause.

Descriptive caller tags from the recovered paths are:

| Observed path | Tail samples (of 476) | Normal samples (of 5,525) |
|---|---:|---:|
| Font/text work | 142 (29.8%) | 1,999 (36.2%) |
| Histogram projection | 77 (16.2%) | 418 (7.6%) |
| Numeric formatting | 39 (8.2%) | 719 (13.0%) |
| Dashed-path construction | 40 (8.4%) | 410 (7.4%) |

Tags overlap, represent sampled paths rather than exact runtime shares, and are
not statistical confidence intervals. Histogram projection is enriched in the
captured tail subset; font/text work is substantial in both subsets. Neither
observation proves redundant work or justifies adding a cache.

Targeted source investigation confirms that HistogramPaneView.refresh clears
its prepared data and calls the model's Histogram preparation. That preparation
selects visible rows, allocates points, converts coordinates and clones colors.
IcedAxisTextMeasurer creates an advanced-shaped paragraph to measure text, but
AxisFontResolver and TextWidthCache already provide caching paths. These are
investigation sites, not newly established defects. Source graph searches,
traces, coverage checks and direct snippets support these limited statements;
generic dynamic call edges were not treated as exhaustive consumer coverage.

## Decision and next discriminating experiment

Phase location and selected CPU profiling are complete. The original +32.10%
regression is still **INCONCLUSIVE**, and the instrumentation controls did not
qualify an unperturbed timing result. No production optimization, acceptance
retry, native/compositor capture or conflation implementation followed.

The next useful experiment is to count Histogram refreshes, projected points
and existing font/width-cache misses by frame and series, together with relevant
viewport/layout/source-revision changes. This distinguishes extra work in a slow
frame from the same work executing more slowly. Use cheap per-call counters,
not per-bar timestamps. Investigate the actual cache/invalidation lifetime before
considering another cache. If an appreciable wall/CPU gap reproduces without
the sampling profiler, obtain a bounded privileged scheduler capture with verified
thread/clock identity. The separate native post-submit delay remains unresolved.

## Validation and artifacts

All **58 Python tests** pass, including same-sample percentile attribution,
missing/duplicate/nonfinite data, phase/thread/clock validation, missing resource
counters, nanosecond matching, lost-event rejection and instruction-pointer
recovery. Isolated release builds, strict diagnostic-example Clippy and Rust
format checks pass. The unchanged engine test suite was not rerun for this
diagnostic-only change.

Replay entry points are `scripts/analyze_frame_phases.py`,
`scripts/build_phase_profile.py`, `scripts/profile_frame_phases.py` and
`scripts/analyze_phase_stacks.py`. The builder rejects existing output directories;
preserve previous artifacts and use a fresh workspace/artifact tree for a rebuild.
Build dependency reuse is optional. Raw data and Perfetto-compatible per-run JSON
timelines live under `target/frame-phase-investigation/`; the artifact index
records their hashes. Target artifacts are local and are not committed. The
checked-in summary retains decisions, counts and primary artifact hashes.

The [acceptance policy](frame-acceptance-policy.md), earlier failures and separate
physical-device requirements remain unchanged.
