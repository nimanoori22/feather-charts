# Series and price-line hover selection

Source revision: `8a7c94ff8ddffde68955db61ef8b58098f239de0`.
The reference working tree contains pre-existing port-progress comments only;
source comparisons execute these local files. No reference source was modified.
Rust baseline: `13ac49b`. Implementation and automated/native validation complete. Physical acceptance is pending.

## Investigation and consumers

Read source hit-test-common, line-hit-test, range-hit-test, internal-hit-test,
pane-hit-test, hovered-source-order, series pane-view bases, Line/Area/Baseline/
Histogram views, hovered fill composite, horizontal price-line views and renderer,
marker plugin hits, pane-widget drawing and public MouseEventParams conversion.
Imports affecting semantics include binary bounds, control-point generation,
visibleTimedValues, scale projection and primitive wrapper layer metadata.

Two-way graph traces connect Line geometry to its three-family pane-view base;
pane selection is consumed by PaneWidget.hitTest and ChartModel pointer updates.
Pane-view consumers require invalidation before selection, logical coordinates,
visible-range geometry, point/range distinctions and source priority rules.
Renderer consumers require separate hovered drawing order and filled-series passes.
Public consumers require owned semantic identity without changing exact-index data.
Coverage is best-effort: relevant helper paths have no recorded gaps; metadata-
changed model/series.ts was read directly for selection/view grouping. Source e2e
line/overlap, marker-price-line overlap and hovered fill-only scenes were inspected.
Source execution comparisons and exact file/result inventory are recorded below.

## Behavioral contract

Line/Area/Baseline use segment/point selection; filled interiors are not targets.
Curves approximate the actual control points with twelve segments. A single row
has the source horizontal hit interval. Hidden outlines retain the source's hit
geometry/options semantics. Bar/Candlestick select high/low ranges inside logical
time slots; Histogram selects value-to-base ranges. Gaps and adjacency affect
slot edges. Main series tolerances are logical pixels. Horizontal decoration hits
use width plus seven logical pixels independently of main-series tolerance.

Point candidates override non-point candidates. Otherwise lower distance wins;
equal ties keep stable reverse source order and stable pane-view order. Marker
foreground precedence is applied at its source position, not by globally sorting
all candidates. Marker draw z-order remains independent of its reported normal
hit z-order. Price-line labels alone do not create plot hits. Automatic/base/custom
lines have distinct typed identity; external strings may be absent or duplicated.

Hovered drawing must not change stable selection order. Ordinary series can move
temporarily to the front. Area/Baseline keep fills and normal attachments in their
normal pass and raise their line/point content and price lines. No double blending.
Targets change on pointer movement and stationary mutation/navigation, even if
the snapped crosshair is unchanged; reconciliation must settle without redraw loops.

## Design alternatives and ownership

Compared model-driven selection through mutable view caches against selection
from prepared frames. The model path avoids metadata copies but risks selecting
geometry from a layout different from the displayed frame and complicates immutable
host access. Choose prepared-frame selection: retain visible logical points/ranges,
neighbor context needed by curve controls and stable source/view IDs. Reuse existing
prepared pane data rather than adding a second projector. Binary x-bounds restrict
pointer candidate work; drawing and selection borrow owned immutable metadata.

Chart owns the model/series/attachments/price lines. Preparation creates owned frame
geometry and IDs. Events own typed hit values; no borrowed model reference escapes.
The frame has no owner back-reference and creates no new Rc cycle. Explicit removal
invalidates subsequent prepared targets; retained older frames remain snapshots.
No new async tasks, subscriptions, locks or manual destruction are introduced.

## Public API

`ChartHit { series: SeriesId, target: HitTarget }` identifies the winning target.
HitTarget distinguishes series point/line/range, typed price lines and existing
MarkerHit. Price-line variants distinguish automatic last value, relative base and
Custom(PriceLineId); optional external IDs are separate data.
`Chart::hit_test(pane, point)` resolves from the settled frame.
PointerEvent gains `hit: Option<ChartHit>`; its marker field reports the winning
marker, while Chart::marker_hit remains a documented marker-only lookup.
`Chart::set_hovered_series_on_top(bool)` controls drawing; the getter is
`hovered_series_on_top()`. The default is `true`, verified against source
chart-options-defaults.ts. The option belongs to the public frame owner, avoiding
a new rendering concern in engine-only ChartModelOptions constructors.

```rust
pub struct ChartHit { pub series: SeriesId, pub target: HitTarget }
impl Chart {
    pub fn hit_test(&self, pane: usize, point: iced::Point) -> Option<ChartHit>;
    pub fn set_hovered_series_on_top(&mut self, enabled: bool);
    pub fn hovered_series_on_top(&self) -> bool;
}
```

These are signatures; the complete typed variants are in `src/model/hit_test.rs`.

Series-view consumers supply prepared logical geometry. Frame consumers preserve
stable source ordering alongside temporary render order. Input uses raw positions;
magnet and exact-index event rows keep existing semantics. Native drawing remains
immutable. JS object identity/unknown payloads become typed enums and handles;
view inheritance and draw-time state mutation are not reproduced.

## Implementation and consumer re-check

Primary hit geometry is built from the same settled pane-view data as drawing.
Line metadata keeps the visible slice and one neighbor on each side for curve
controls. OHLC/Histogram retain visible intervals. No second full-history selector
or new projection cache was added. Selection still visits all supported sources;
binary x bounds limit candidate items. Price metadata is sorted by SeriesId,
with stable order inside each source, so lookup does not scan every pane's price
lines separately for each series. Drawing keeps its original arrays and IDs.

Pane selection uses stable reverse source/view order. Marker foreground arbitration
occurs at its source position, including a source with no main geometry. The event
adapter computes the winner once for both hit and marker fields. Existing exact-
index rows, raw origin and crosshair magnet are preserved. Tick reconciliation
compares typed winners, including price-line removal, and stops requesting updates
once settled. Kept frames/events own their data and survive later mutation.

Drawing uses an allocation-free iterator of Normal/Background/Foreground passes.
Ordinary sources move as a unit; filled sources split their fill/base/normal marker
pass from their outline/point/automatic/custom price-line pass. Crosshair and
AboveSeries/Top annotations keep existing order. This maps back to source
hovered-source-order, filled view composites, Series.mainPaneViews and PaneWidget
without reproducing TS view inheritance or mutable renderer callbacks.

Source relative base lines were also prepared: non-overlay Percentage/IndexedTo100,
baseLineVisible and visible series with a first value. Existing test/demo fixtures
used an empty base-line color while that view was absent. They now use the actual
source default `#B2B5BE`; assertions and rendering checks were preserved.

The frame-cache design was chosen over a model-cache selector for consistency
between displayed layout and selected targets. Strong ownership is:

```text
Chart -> model -> series / price lines / marker attachments
Chart -> prepared frame -> owned geometry / typed IDs
Event -> owned ChartHit
```

There are no new weak references, owner back-references, Rc cycles, asynchronous
subscriptions, locks or cleanup callbacks. Drop releases owned metadata normally.
JS object identity is replaced by SeriesId/PriceLineId/MarkerCollectionId. Optional
external strings remain payloads, not unique ownership keys. Model/view/public
consumers can select, reorder for drawing and remove objects without borrowing
mutable engine state during immutable drawing.

`prepare_snapshot` supplies geometry before font conversion. A custom host must
settle marker text widths in its own conversion before final text-hit/hover lookup.
The public `prepare` does this through the Iced frame converter and then resolves
hover again, so both shape and text hits follow the displayed frame.

## Files changed

Engine/public identities and adapters:

- `src/model/hit_test.rs`, `src/model/mod.rs`
- `src/renderers/line_hit_test.rs`, `src/renderers/range_hit_test.rs`, `src/renderers/mod.rs`
- `src/model/chart_model/price_lines.rs`
- `src/ui/hit_testing.rs`, `src/ui/mod.rs`
- `src/ui/line_chart.rs`, `src/ui/line_chart/series_drawing.rs`, `src/ui/multi_pane_frame.rs`
- `src/api/chart.rs`, `src/api/interaction.rs`, `src/api/mod.rs`

Tests, demonstrations and verification:

- `tests/hover_selection.rs`, `src/model/chart_model/tests.rs`
- `examples/hover_selection_reference.rs`, `examples/hover_selection/pane_cases.rs`
- `scripts/check_hover_selection_reference.cjs`
- `examples/public_chart.rs`, `examples/public_chart/hover.rs`, `examples/public_chart/tests.rs`
- `examples/public_api_profile.rs`, `examples/support/chart_demo/series.rs`
- `README.md`, this report, `tasks/hover-selection-{plan,todo}.md`

## Behavioral tests and source execution

Thirteen new public integration tests cover all six families at ratios 1/1.25/1.5/2,
owned click identity before drawing, native press/release input, custom levels and
label-only visibility, duplicate external IDs, built-in base/last lines, offscreen
horizontal lines inside grab tolerance, marker conflicts, stationary removal,
independent panes, scale modes/inversion, hide/clear, retained frames, failed preparation without stale-target retry, raw-versus-
magnet semantics, navigation/resize reconciliation and stable non-sticky order.
A 20,000-row fixture verifies visible-only hit retention. Two geometry unit tests
cover segment/point/single-row boundaries and range gaps/tolerances.

The actual-source oracle runs 69,264 cases: 67,680 line/point/curve/time-slot/range
cases, 1,536 pane arbitration combinations, 36 horizontal boundaries and 12 ordinary/
filled draw-order option states. It transpiles and executes source helpers,
horizontal renderer, pane-hit-test, hovered-source-order and fill composite
methods. Pane fixtures shim candidates and sources at the tested boundary;
they do not execute a browser event/rendering lifecycle. No copied TS algorithm
stands in for the source. All fourteen earlier oracle scripts also pass, including
marker placement/shape hits and price-line/label fixtures.

Regression sensitivity was checked by inverting the new hovered-series option
condition. The ordinary/split-order test failed as intended. Restoring the condition
returned the full suite to green. No tests, assertions or thresholds were weakened.

## Validation and renderer evidence

Passed: `cargo fmt`, `cargo test --all-targets --quiet` (448 tests),
`cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`,
`cargo build --examples`, all fifteen source-comparison scripts and `git diff --check`.
The existing narrow-window widget layout regression also checks the new target
status and passes.

Both TinySkia and WGPU passed 56 captures each: fourteen scenes at all four ratios.
Scenes cover the six families, disabled Line/Area/Baseline raising, fill-only on/off,
crossover, removal and a narrow pane. Pixel checks verify one blend per filled
source and whether the raised outline covers the upper fill. Frames repeat
identically over different clear colors. A contact sheet was visually inspected;
layering, axes, labels and pane boundaries agree across the two backends.
A bounded native startup ran the public hover scene for eight seconds without
errors and ended with timeout code 124 (intentional termination).

Reproduction:

```sh
cargo run --example public_chart -- --hover-selection
cargo run --example public_chart -- --hover-selection --hover-captures-only --capture-dir /tmp/feather-hover-tiny
cargo run --example public_chart -- --hover-selection --hover-captures-only --capture-dir /tmp/feather-hover-wgpu --wgpu
cargo run --example hover_selection_reference
node scripts/check_hover_selection_reference.cjs
cargo run --example public_api_profile -- --selection-profile
```

Capture evidence: `/tmp/feather-hover-{tiny,wgpu}`, contact sheet
`/tmp/feather-hover-contact.png`, tests `/tmp/feather-hover-tests.log`, mutation
`/tmp/feather-hover-mutation.log`. These are local validation artifacts, not checked
in binary assets.

## Performance

Normal debug workload uses 1,200 rows, 120 operations (first 20 warmup), identical
public API arguments, baseline `13ac49b` executable saved before rebuilding and
three alternating baseline/current runs. Values below are median-of-run medians
and median-of-run p95s in milliseconds for model/frame/glyph preparation, excluding
GPU presentation. The first concurrent-load trial had outliers; controlled repeats
below resolved them.

| Workload | Baseline median / p95 | Current median / p95 |
| --- | --- | --- |
| Line hover | 6.483 / 6.724 | 6.475 / 6.743 |
| Candle hover | 10.081 / 10.321 | 10.057 / 10.370 |
| Histogram hover | 7.767 / 7.999 | 7.728 / 7.916 |
| Line zoom | 6.527 / 6.798 | 6.583 / 6.990 |
| Candle zoom | 10.120 / 10.529 | 10.073 / 10.495 |
| Histogram zoom | 7.807 / 8.091 | 7.778 / 8.093 |
| Line append (zoom run) | 10.501 / 11.179 | 10.561 / 11.284 |
| Candle append (zoom run) | 14.239 / 15.096 | 14.207 / 14.876 |
| Histogram append (zoom run) | 11.789 / 12.453 | 11.825 / 12.500 |

One additional 20,000-history/200-visible run gives baseline/current hover medians
10.230/10.158 ms (Line), 8.611/8.618 ms (Candle), 7.480/7.662 ms (Histogram).
Append medians were 35.470/35.115, 33.834/33.434 and 32.281/32.017 ms.
These measurements do not show a material sustained preparation regression;
they are local measurements, not a hardware-independent latency guarantee.

Direct lookup across varying raw positions adds median/p95 costs of
0.000169/0.000221 ms empty, 0.001176/0.001562 ms one fully visible series,
0.000703/0.000725 ms one 20,000-row history with 202 retained points,
0.008091/0.008550 ms sixteen overlapping series and 0.032822/0.033925 ms sixty-four.
The latter cases retain 202 points per series (3,232/12,928 total), rather than
full history. Source x-bound helpers restrict candidates; source count and eligible
nearby items still affect work. Text-width/glyph-cache tests preserve one shape
per repeated label/font/style; pointer movement adds no new shaping API.
Existing engine preparation costs, including prior projection/rendering work,
remain outside the direct selection measurement.

Saved timing records: `/tmp/feather-hover-profile-results.json`,
`/tmp/feather-hover-profile-{baseline,current}-offscreen.log`,
`/tmp/feather-hover-selection-profile.log`.
A second paired workload builds the baseline archive and current checkout in
separate target directories with identical dev flags (`debug=0`, `incremental=false`,
optimization unchanged) to fit the sandbox quota. It uses eight overlapping series,
1,200 rows each and a 200-row viewport, with the same 120/20 operation/warmup schedule.
One paired pass produced the following median/p95 values in milliseconds:

| Workload | Baseline | Current |
| --- | --- | --- |
| Line hover | 13.540 / 13.963 | 13.643 / 14.004 |
| Candle hover | 20.313 / 20.544 | 20.510 / 20.999 |
| Histogram hover | 13.380 / 13.726 | 13.573 / 13.913 |
| Line zoom | 13.954 / 14.369 | 13.851 / 14.222 |
| Candle zoom | 20.767 / 47.320 | 20.719 / 21.028 |
| Histogram zoom | 13.674 / 14.119 | 13.722 / 14.043 |
| Line append (hover run) | 22.476 / 24.232 | 22.372 / 23.250 |
| Candle append (hover run) | 26.897 / 28.933 | 27.586 / 29.424 |
| Histogram append (hover run) | 21.693 / 22.708 | 22.450 / 23.426 |
| Empty hover | 0.097 / 0.139 | 0.101 / 0.135 |

This stress pass adds roughly 0–3.5% to medians where slower; it is a single pass,
with a baseline Candle zoom p95 outlier, so do not interpret it as an improvement
or a precise cross-machine regression threshold. Repeatable lookup/preparation
work remains bounded by source count and visible metadata rather than offscreen
history. Records are `/tmp/feather-hover-paired-{baseline,current}-{many-hover,many-zoom,empty}.log`.
The first isolated baseline build with full debug symbols exhausted the temporary
quota; its directory was removed and both reduced-debug builds then succeeded.
Both archives use the same profiler workload; only the new-feature-only direct-hit
branch is omitted in the baseline, whose API lacks that feature.

## Remaining dependencies and acceptance

Fresh physical mouse/touchpad acceptance is still pending. Test moving between
outlines/points/ranges, equal overlaps, custom levels and markers; toggle raising;
check clicks versus drags, zoom/pan/axis scaling, half-size windows and live/hide/
remove changes. Record observed device, backend and scale, and confirm no stale
highlight, sticky selection, layout jump or continuing redraw. Automated captures
and native synthetic input do not close this gate or any earlier hardware gate.

Arbitrary custom series and
primitives, editing, touch/kinetic input, conflation and full API parity are outside
this milestone.
