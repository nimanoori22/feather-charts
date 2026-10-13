# Typed custom-series rendering

Date: 2026-10-14. Automated custom-series checks passed; physical acceptance pending.
Design and complete consumer/ownership analysis:
[custom-series-design.md](custom-series-design.md).

## Source and behavior

Investigated upstream custom view/renderer/data contracts, Series/CustomPaneView,
DataLayer, public Chart/Pane/Series APIs, hit arbitration, decorations, markers,
primitives and mouse event data. The box-whisker plugin and toolkit candle width,
position and grid width helpers provide the rendering oracle.

Chart registers the existing typed CustomSeries owner. A private borrowed
projection shares that owner's current/extreme prices and extension state with
ordinary decorations and primitives. The public handle keeps typed data alive
but holds only a weak view reference. Removal rejects later chart mutations and
cleans up the view once; retained immutable renderer snapshots remain drawable.
Frames own bounded visible payloads and a copied price transform, with no live
Chart/view borrow during drawing. Multiple different custom types can share time
indexes and occupy different panes/scales.

Common chart options and renderer-specific options compose. JavaScript option
inheritance, mutable nullable renderer references, prototype identity and manual
public destruction are discarded. A generic Chart/application series enum and
a second drawing-command vocabulary were considered; the existing typed owner
and Canvas boundary avoid duplicating those abstractions. Renderer clones must
share only immutable geometry. The example uses immutable Rc data for inexpensive
snapshot cloning; application renderers may instead copy bounded owned geometry.

The price builder's last value is the current price, while its extrema drive
autoscale. Original public time and typed extra data survive exact-index lookup.
Custom hits retain optional typed payload, object identity, priority and cursor
separately; arbitration runs once in preparation, and drawing never reconstructs
the winning hit through another callback. Missing custom hits use the source
price-envelope fallback. Invalid times, ordering, colors or builder values fail
before membership mutation. Genuine shared-time insertion still reindexes.

## Files

The public adapter is `src/api/custom_series.rs`; frozen renderer composition is
`src/ui/custom_series.rs`. `src/model/series/projection.rs` bridges existing owners
for ChartModel decoration/marker/primitive consumers. Related integration changes
are in Chart, interaction/widget, series defaults/custom contracts, model hit
types, and the existing line/multi-pane drawing and hit arbitration modules.

The host is `examples/custom_series.rs`, with plugin-local renderer/capture
helpers in `examples/custom_series/`. `examples/custom_series_reference.rs` and
`scripts/check_custom_series_reference.cjs` compare against upstream source.
`examples/custom_series_profile.rs` measures fixed-visible preparation.
Behavioral tests are in `tests/public_custom_series.rs`; lower transactional and
incremental custom tests are in the existing DataLayer/coordinator/series modules.

## Validation

- All six public integration tests pass: retained frames/cleanup, typed events
  and hit payloads, transactional errors/whitespace/history/clear/reload,
  actual-owner decorations and primitive conversion, mixed custom types with
  shared reindexing, and the 100,000-row copy budget.
- All 349 library tests and the full all-target Rust suite pass. Cargo check,
  strict all-target Clippy, formatting and diff checks pass.
- 320 geometry and high/low/median price-builder fixtures match the actual
  TypeScript box-whisker plugin. Ratios 1/1.25/1.5/2, candle width boundaries,
  negative/fractional positions and inverted quartiles are included.
- 92 captures per backend pass on TinySkia and WGPU: ordinary/hover, append,
  replace, historical update, whitespace, style, clear/reload/remove/re-add,
  narrow/zoom, inversion, logarithmic/percentage/indexed/normal and hidden data,
  left-scale ownership, a second pane, manual price pan, history pan and markers.
  Captures check repeatable drawing and present/absent custom fill pixels.
  A representative contact sheet was visually inspected. Artifacts are under
  ignored `target/custom-series-captures/`.
- A deliberate membership-guard mutation caused the original four public
  tests to fail; restoring the guard returned them to green.

Price converters may be unavailable during initial layout; primitives must
honor their Option contract. The primitive test verifies a settled owner-price
round trip. The native example skips unavailable/invalid converted geometry,
while the browser example defaults a null coordinate to zero; finite supported
cases match source exactly.

## Limits and remaining work

Initial public horizontal behavior is Time, with application metadata retained
inside the typed datum and unit common metadata. Full conflation rendering and
arbitrary public horizontal-scale behavior remain separate milestones. Physical
mouse/touchpad acceptance for the new custom demo is pending, including narrow
windows and high DPI. Touch/kinetic navigation is the next roadmap stage.

## Custom preparation performance

Release, one pane at 900×600 logical pixels, ratio 1.25, fixed logical slots
20..100, 20 warmups and 100 measured samples. All three acceptance runs were
isolated from Cargo, captures and the built-in benchmark. A diagnostic run that
overlapped Cargo check is excluded. The latest quartiles are fixed at
95/98/100/103/106 so global labels and price lines do not change the scene as
history increases. Timings are CPU snapshot preparation, not GPU frame time.

| Run | Sources | 1,200 median/p95 ms | 20,000 median/p95 ms | 100,000 median/p95 ms |
| --- | --- | --- | --- | --- |
| 1 | 1 | .323345/.333597 | .326985/.363043 | .323804/.331386 |
| 2 | 1 | .334029/.340558 | .340310/.347362 | .336258/.342258 |
| 3 | 1 | .317091/.325617 | .322439/.328847 | .315632/.352801 |
| 1 | 16 | 4.535126/4.672840 | 4.732255/4.931967 | 4.683562/4.912261 |
| 2 | 16 | 4.565684/4.787106 | 4.778579/5.301142 | 4.745601/4.937070 |
| 3 | 16 | 4.632353/4.805383 | 4.659530/4.839419 | 4.671810/4.883344 |

Every paired 1,200→100,000 median and p95 increase is below the roadmap's 10%
screen; the largest is 8.35% on one-source p95. Source count has a real cost,
reported separately. Against the post-P built-in preparation diagnostic,
custom rendering preserves the same history-independent shape; these are
different visual scenes, so their absolute timings are not a parity ratio.
Raw logs are `target/custom-series-profile-{1,2,3}.log`.

The operation-count test retains 100,000 rows while preparation clones at most
170 payloads (81 visible slots plus bounded neighbors, and a test renderer that
copies its visible Vec once when frozen). The example's immutable Rc snapshot
avoids that second payload copy. An existing-time update clones at most four
payloads. These assertions guard bounded work independently of timing noise.
