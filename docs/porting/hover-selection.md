# Series and price-line hover selection

Source revision: `8a7c94ff8ddffde68955db61ef8b58098f239de0`.
Rust baseline: `13ac49b`. Implementation in progress.

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
Further source comparisons and file/result inventory follow implementation slices.

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
A Chart option controls hovered-series-on-top with the source-compatible default.

Series-view consumers supply prepared logical geometry. Frame consumers preserve
stable source ordering alongside temporary render order. Input uses raw positions;
magnet and exact-index event rows keep existing semantics. Native drawing remains
immutable. JS object identity/unknown payloads become typed enums and handles;
view inheritance and draw-time state mutation are not reproduced.

## Verification and remaining work

Pending implementation slices and recorded source/native/performance evidence.
Physical mouse/touchpad acceptance remains separate. Arbitrary custom series and
primitives, editing, touch/kinetic input, conflation and full API parity are outside
this milestone.
