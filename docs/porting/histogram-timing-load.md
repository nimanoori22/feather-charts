# Bounded Histogram timing and CPU-contention check

Date: 2026-10-17. Follow-up to the [count study](histogram-work-study.md).

**Decision: defer further timing investigation and resume missing-feature work.**
The user requested a bounded phase/load check and raised the priority of finishing
the port. This batch establishes no new production engine defect. The timing
exception stays **OPEN**; deferral is a development-priority decision, not an
acceptance pass, waiver or finding that all performance is correct.

## What the experiment establishes

The frozen reference's total CPU-work p95 is 1.199857–1.250529 ms without added
load and 2.823644–2.870699 ms with one competing CPU worker. The reference has no
new Histogram timer/count hooks. Its loaded p95 increases 129.56–135.87% against
the matched ambient execution. Synthetic contention therefore produces a large
repeatable delay in this fixture. It does **not** prove that open applications
caused any historical failure; normal applications need not compete on this CPU.

| Repetition | History rows | Ambient reference total p95 (ms) | CPU-contention reference total p95 (ms) |
| --- | ---: | ---: | ---: |
| 1 | 1,200 | 1.250529 | 2.870699 |
| 1 | 100,000 | 1.202728 | 2.834177 |
| 2 | 1,200 | 1.221469 | 2.842269 |
| 2 | 100,000 | 1.199857 | 2.830139 |
| 3 | 1,200 | 1.223697 | 2.823644 |
| 3 | 100,000 | 1.208312 | 2.841043 |

In the reference's total-tail frames, snapshot trace wall time averages
1.144843–1.198191 ms ambient versus 2.910241–3.341428 ms loaded. Thread CPU time
averages 1.139367–1.192356 ms ambient versus 1.131399–1.171062 ms loaded. Extra
elapsed time under this deliberately controlled load is consistent with waiting
for CPU, rather than proportionately more chart execution. This is not a full
scheduler trace and does not assign exact kernel wait sites.

The trace's CLOCK_MONOTONIC/thread-CPU boundaries and the fixture's Instant
boundaries are separate clock reads. Preemption can occur between them,
particularly under contention. Their intervals include different boundary probe
work and should not be equated sample-for-sample or used to place a wait exactly
inside chart code. The raw total above remains the original Instant-based
update + snapshot + glyph total, used unchanged for all overhead comparisons.

## Where the measured Histogram time goes

All 24,000 enabled measured frames, including their 1,200 total-tail frames,
retain the same per-series vector: one refresh, one preparation, 83 points each
(16/16/1,328 per frame). No extra counted Histogram work appears under load.

In ambient total-tail frames, the Histogram preparation bodies together average
0.067612–0.072892 ms, only **5.90–6.31%** of snapshot time. Their tail-minus-normal
increase is 0.010223–0.013835 ms; the remaining snapshot interval increases
0.059983–0.074319 ms. Most snapshot cost and most observed ambient tail growth
therefore sit outside these measured bodies. Prior Histogram sample enrichment
is not evidence that it dominates snapshot cost.

The scoped timer brackets `histogram_pane_view::prepare`, including early returns.
It excludes the outer refresh clearing old data, the model wrapper before that
body, axis/font work outside it and other snapshot construction. Calls made
inside the body remain included. The residual is simply the same
frame's snapshot duration minus its 16 nonoverlapping body timers, not an exact
attribution of the rest to fonts or any other operation. Under load, a wall timer
can also contain preemption; it is not a measure of CPU instructions.

## Fixed protocol and calibration

One predeclared 36-execution batch: three repetitions of both 1,200 and 100,000
starting rows, reference/off/on instrumentation, ambient/CPU-contention conditions.
The second repetition reverses condition, history and mode order. Each uses 20
warmups, 2,000 measured frames, Histogram16, append/fixed-last-price, logical
20..100, size 917 by 870.80005, pixel ratio 1.25, pinned to CPU 5. Tests, builds,
profilers and analysis did not overlap these serial measurements. No timing run
was retried and no acceptance set was run.

Ambient means **no added synthetic load**, not an idle-certified desktop. Existing
applications remain running. The added worker performs bounded integer arithmetic
on CPU 5, signals READY, is affinity-checked and remains alive with increasing CPU
ticks during each capture. All 18 workers were terminated and reaped afterward;
exception cleanup was exercised. Parent-death SIGTERM and a 120-second alarm
bound abandoned workers. Load averages and CPU/memory/I/O pressure are recorded
before/after each execution. No desktop setting or application was changed.

An isolated source copy extends the existing thread-local counter helper with a
stack-owned Instant guard. It accumulates per-series nanoseconds on drop. No
per-point instrumentation, shared-state synchronization or file I/O enters the
preparation body. Frame records are preallocated and written after the run.
The frozen reference's source/executable hashes are verified; separate Cargo
artifacts prevent reuse of an old chart binary. Root production `src/` is unchanged.

**0/24 overhead comparisons exceed the predeclared >5% guard** for total median
or p95 against either reference or collection-disabled controls. Median changes
span −2.33% to +0.39%; p95 changes span −3.36% to +1.19%. This qualifies this
bounded diagnostic against that guard, not zero observer effect or earlier
profilers. Raw phases, complete per-series count/timing rows, worker/environment
records and control comparisons are retained, not just selected frames.

## Validation, artifacts and next work

400 isolated Rust library tests, 63 Python tests, formatting and strict Clippy
pass. Tests cover timer drop/reset/inactive behavior, source separation,
thread-local capture, same-total-tail joins, per-series redistribution, malformed
count/timing records and negative snapshot residual rejection. One/16-series
smokes verify active timers and unused slots; the worker cleanup path is checked.
Graph/source verification followed the preceding study; the generic refresh edge
missing from the graph was checked directly in `line_chart.rs`. Metadata coverage
matched relied sources and tools, with the graph treated as best effort.

The [summary](histogram-timing-load-summary.json) retains result bounds, raw
artifact/tool hashes and validation. Complete files remain local under
`target/histogram-timing-load/`; they are not committed. The standalone figure is
`analysis/snapshot-split.svg`, and `analysis/frame-work.csv` joins every enabled
frame's counted work, body time, residual and total-tail classification. Reproduce
with `python scripts/build_histogram_work_profile.py --timing`,
`python scripts/profile_histogram_work.py --timing-load`, then
`python scripts/histogram_work_analysis.py --input target/histogram-timing-load/runs --output target/histogram-timing-load/analysis`.
Fresh output directories and the retained frozen reference are required.

Further timing investigation is deferred. The next development sequence is
[conflation rendering](../../tasks/conflation-next.md). Historical update and actual
presentation failures, remaining touchpad/touchscreen/high-DPI checks and the
original acceptance thresholds remain open and unchanged. This headless CPU
experiment neither measures visible dragging nor proves a 60 Hz/native frame pass.
Reopen debugging for reproducible interaction lag, incorrect work or a repeatable
same-protocol regression; do not continue an open-ended search merely to remove
a historical exception. Feature development and acceptance are tracked separately.
