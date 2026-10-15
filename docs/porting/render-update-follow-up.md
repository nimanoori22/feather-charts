# Software rendering and update acceptance follow-up

2026-10-15. The milestone remains **OPEN**. This follows the
[initial native study](native-frame-acceptance.md) under the unchanged
[acceptance policy](frame-acceptance-policy.md). Historical failures are retained.

## Result

| Check | Latest result |
| --- | --- |
| Software render CPU p95, diagnostic regular-size pilot | About 24.6 ms before; 5.0–5.4 ms after |
| Software native phase cohorts | 32/32 pass across two sets |
| All native phase cohorts | 63/64 pass; set three stopped by policy |
| Uninstrumented update comparisons | 5/6 pass; decision FAIL |
| Existing renderer images | 984/984 byte-identical |
| Targeted optimized/reference RGBA comparisons | 16/16 byte-identical |
| Physical mouse | GPU regular/half-size passed; software drag lag reproduced |

Passing two sets does not complete the three-set native gate. The measurements
cover the defined 16-source fixture, not every possible chart. No budget was
relaxed, samples removed, or failing study replaced by passing repeats.

## Software bottleneck and design

Sampling the actual tiny-skia renderer found rectangle fill/scan/blit work.
Omitting rectangles in a diagnostic draw mode reduced pooled render CPU p95
from 24.643 ms to 4.618 ms. Omitting strokes or axes instead yielded 20.216 ms
and 20.578 ms. Those modes change output and cannot count as acceptance.
Cropping rectangle geometry before drawing was also measured, gave no useful
improvement, and was removed as a drawing change. Clip-mask setup was not
established as the dominant cost.

The selected fix skips a rectangle only when its visible plot intersection is
fully covered by an opaque rectangle in the next eligible foreground source.
It follows actual source/hover draw order. The same ordinal rectangle is a bounded
candidate search; full geometric containment is still required. Empty candidate
sources are not skipped in search of a later source. Translucent or partially
covering rectangles retain their original drawing. Custom drawing and plot
primitives conservatively disable this optimization because their callbacks may
change drawing state.

Original geometry, source identity, hit targets, ownership and public APIs stay
intact. Surviving rectangles use the same fill operation. There is no cache,
allocation, shared mutable state, async engine work or new dependency. The cost is
one bounded candidate check per rectangle in eligible passes.

Upstream `src/renderers/histogram-renderer.ts` fills bars in order;
`src/gui/pane-widget.ts` establishes pane/source ordering. Existing Rust
`PlotSnapshot` drawing already owns this ordering and immutable prepared geometry.
The required contract is identical visible pixels, blending, clipping, hover
raising and hit behavior. TypeScript object identity/class mechanics are not
needed for this internal drawing optimization. This is a Rust renderer efficiency
fix, rather than a new source-component port.

Alternatives were retaining all fills (the measured bottleneck), cropping
(measured neutral), merging paths (could change anti-aliasing/blending), or a
raster cache (requires additional invalidation and lifetime rules). Conservative
occlusion is smaller and preserves existing drawing operations.

The matched software pilot after the fix reports milliseconds:

| Phase | Render CPU p95 | Work p95 | Presentation p95 |
| --- | ---: | ---: | ---: |
| Append | 5.005 | 6.779 | 32.993 |
| Zoom | 5.424 | 8.658 | 32.974 |
| Pan | 5.371 | 9.174 | 32.977 |
| Price axis | 5.317 | 8.282 | 32.977 |

These are diagnostic pilots, separate from the fixed acceptance matrix.

## Diagnostics in the actual update fixture

`preparation_profile --diagnostic-counters` brackets its existing timed update
section with current-thread allocation, thread CPU, Linux rusage and hardware
counter records. Input generation and snapshot/glyph preparation remain outside
that section. Records join to the raw measured samples by index. Allocation
tracking forwards to System; requested bytes sum allocation sizes and full
reallocation destinations, not resident memory. Hardware descriptors close by
RAII. Unsupported/unavailable counters fail explicitly. Counter intervals include
measurement overhead, so these values are diagnostic, not substitute acceptance
latencies. The acceptance reader rejects enabled diagnostic logs.

A fixed six-pair diagnostic study retained 24,000 measured samples, with all three
hardware events joined to each sample. Two history comparisons and three full
same-history controls flagged. The particularly useful reproduction used the
same 100,000-row history:

| Run | Wall median µs | CPU median µs | Instructions median | Cycles median | Cache misses median | Allocations / requested bytes median |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 3 | 6.9145 | 8.0465 | 21,570 | 28,278.5 | 156 | 6 / 3,922 |
| 4 | 14.4195 | 15.9205 | 21,372.5 | 57,922.5 | 1,712 | 6 / 3,922 |
| 5 | 6.614 | 7.762 | 21,471 | 27,335 | 45 | 6 / 3,922 |

Run four had no faults, context switches or counter multiplexing in these records.
It reproduces slower execution at identical history with nearly unchanged work
and allocation counts but more cycles/cache misses. This does not identify a
specific memory address or prove the cause of a historical uninstrumented miss.
Small-history capacity growth also produces occasional faults; not every tail
sample has identical allocation bytes. The original timing exception stays open.

Reproduce diagnostics separately from acceptance, after building once:

```sh
cargo build --offline --release --example preparation_profile --example frame_latency_profile
taskset -c 5 target/release/examples/preparation_profile --rows 100000 --series 16 --kind histogram --phase append --fixed-last-price --samples 2000 --diagnostic-counters
```

Choose an available CPU. This requires usable Linux x86_64 user hardware counters;
no kernel settings are changed. Use the uninstrumented/native reproduction in the
[initial report](native-frame-acceptance.md#reproduction) for acceptance. Diagnostic
draw modes, including captures, are rejected by the native acceptance reader.

## Fresh acceptance and unresolved evidence

The valid update and native studies ran serially, without competing builds,
tests, captures or diagnostics. Later physical mouse diagnostics ran only after
both acceptance studies finished. Source, binary, policy and raw-file
hashes were frozen. No physical demo ran during those studies.

The update study passes five of six complete 2,000-sample pairs. Run one median
is 6.545 → 6.618 µs (+1.115%), but p95 is 11.337 → 14.106 µs (+24.424%), exceeding
the unchanged 10% limit. Complete same-history controls have no positive
exceedance, so policy returns **FAIL**, not INCONCLUSIVE. See the
[six comparison rows](occlusion-update-measurements.csv).

Native set one passes 32/32 phase cohorts; set two passes 31/32. All 32 software
cohorts pass, with maximum work p95 12.511264 ms and presentation p95 33.103597 ms.
The single failure is set-two wgpu, 100,000 rows, narrow window, pan: work p95
12.303360 ms passes, presentation p95 **43.006836 ms** exceeds **33.333334 ms**.
All 7,680 raw frame records, including warmups, remain; zero frames were discarded.
The runner stops before set three as specified. See the
[64 cohort rows](occlusion-frame-measurements.csv).

In the failing cohort, work wall-time p95 is 12.303 ms versus thread CPU p95
6.057 ms. The worst presentation frame has work wall time 13.502 ms, CPU 6.057 ms,
and presentation latency 45.992 ms. This gap indicates waiting/descheduling;
without a correlated scheduling trace it does not establish the exact cause or
a renderer computation defect. It remains a failed presentation gate.

## Validation, files and physical checks

Passed: formatting, all-target check, strict Clippy, 571 Rust tests across 51
targets, 27 Python tests and 20 TypeScript source-reference checks. Regression
coverage includes fractional/offscreen clipping, full versus partial/transparent
coverage, hover/source order, empty next sources, retained geometry/identity,
allocation recording/reset/thread exclusion, and diagnostic acceptance rejection.
RED/GREEN logs are retained.

All 984 existing renderer captures match pre-change images byte for byte. Sixteen
additional reference comparisons cover both renderers, regular/narrow sizes and
four interaction phases with differing bar heights/colors/opacity. The reference
uses an empty primitive to disable occlusion without drawing pixels; both modes
run the same native ChartProgram and compositor screenshot path.

Production drawing changes are in `src/ui/line_chart.rs` and
`src/ui/line_chart/series_drawing.rs`; behavioral tests are their adjacent
`rectangle_clipping_tests.rs` and `series_drawing/tests.rs`. Example diagnostics
are `examples/support/{preparation_diagnostics,update_allocations,update_counters}.rs`,
`examples/preparation_profile.rs`, `examples/update_latency_probe.rs` and
`examples/frame_latency_profile/native.rs`. Allocation tests live in
`tests/update_allocations.rs`; acceptance readers and regression tests are in
`scripts/frame_acceptance.py`, `scripts/profile_chart_frames.py` and their test
files. Earlier shared-pane preparation changes are documented in the initial
report. README/checklist links point here without deleting historical evidence.

Raw artifacts and validation logs are under `target/render-update-follow-up/`.
The [artifact manifest](occlusion-acceptance-artifacts.json) retains hashes;
`frozen-artifacts.json`, `update-decision.json`, `diagnostic-updates/analysis.json`,
`diagnostic-updates/same-history-slow-run.json`, `capture-comparison.json` and
`occlusion-pixel-comparison.json` provide local details. Target artifacts are
local; the repository report/CSVs/manifests preserve the summarized record.

Mouse is the only available physical device. The user passed GPU live-feed zoom,
diagonal pan, price-axis wheel/drag and box editing at regular/half window sizes.
The user reported slight software lag while dragging the plot or vertical axis,
with live feed running at roughly one-third window size. The symptom reproduced
during a 60-stack debugger sample. See the
[device record](../../tasks/live-append-mouse-acceptance.md). Touchpad/touchscreen,
high-DPI physical coverage and external input-to-photon measurement are untested.
No further component port is required by the occlusion fix. Remaining work is the
failed update/presentation gates and outstanding physical coverage; conflation
rendering remains a separate follow-on after append acceptance.

## Software mouse follow-up

The confirmed physical software case remains failed. Of 60 sampled main-thread
stacks during the user's reproduction, 19 are in software primitive drawing,
35 in event waiting, three in text and three elsewhere. Rectangle mask/blending
work dominates the active samples. Sampling pauses perturb execution, so this
is not a latency measurement or proof that one particular fill causes the lag.

A paired headless replay now exercises the demo's actual two-pane Histogram/box
snapshot, appends periodically, alternates draw order, and asserts identical RGBA
for every original/clipped pair. It excludes demo controls, physical event
cadence and presentation. Run it with:

```sh
cargo run --offline --release --example public_chart -- --software-drag-profile --primitives
```

At 800×500, original/clipped raster medians are 7.453/7.407 ms and p95 values
7.848/7.957 ms. At 300×500, medians are 3.784/3.784 ms and p95 values 3.915/3.898 ms.
All 240 image pairs match, including warmups. Clipping is neutral here as in the
earlier pilot, so no production clipping change was adopted. This static draw
comparison does not reproduce physical input timing and cannot pass the failed
mouse check. Logs and summaries are retained as `software-drag-clipping-4.*`
and `software-drag-clipping-summary.json` under the artifact root.

Earlier replay attempts at 300×100 and 300×180 fail preparation with
`Histogram(NonFiniteCoordinates)`; their logs are retained, not counted as valid
performance cohorts. That separate tiny-height boundary is not established as
the cause of the user's slight lag. The successful 300×500 replay tests narrow
width, not a claimed reconstruction of the user's exact window dimensions.

The next diagnostic for the physical case must correlate actual input cadence,
full demo frame/presentation times and exact window/plot bounds. The headless
replay and sparse stack sample do not establish a fix for it. The GPU mouse pass,
32 software automated cohort passes and software physical failure are separate
results; none overrides another.

The later replay adds only a demo diagnostic module/flag; frozen acceptance
production sources and binaries remain unchanged. The
[software mouse diagnostic summary](software-mouse-diagnostics.json) preserves
the later results and hashes. Targeted example tests and strict Clippy pass after
that addition, with formatting and diff checks clean.
