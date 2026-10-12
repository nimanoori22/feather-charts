# Public chart API and Iced component

## Contract and source investigation

This milestone makes the existing line, candlestick, histogram and price/volume
paths usable outside the demos. Investigated complete `api/chart-api.ts` and
`api/series-api.ts`, their data validators, and ChartWidget mouse-event assembly.
ChartApi owns data normalization and coordinates model updates; SeriesApi validates
before mutation. Removal updates shared time indexes and other series. Model
invalidation must precede time reindexing/crosshair recalculation. Events use exact
logical-index rows across all series; whitespace contributes time but no price row.
Real consumers include PaneApi/addSeries, YieldChartApi, Vue examples, and widget
click/crosshair delegates. The Rust demos exercise the same update/removal path.
Graph traces are best-effort, supplemented by direct source reads.

Required: synchronous ordered replacement and incremental/historical updates,
non-destructive validation errors, membership checks, independent pane price
scales, shared time navigation, multiple series per pane, current mouse behavior,
typed click/crosshair payloads, and read-only drawing. Unsupported rendered series
must fail explicitly. Public scope is the built-in Time behavior and these three
rendered series kinds, not full TradingView API parity.

## Designs and ownership

1. A mutable custom widget owning data/model: fewer application messages but new
   widget lifecycle and borrow machinery, different from the proven Canvas path.
2. Application-owned Chart plus borrowing Canvas Program: explicit synchronous
   updates, immutable prepared frames, existing event capture/animation semantics.

Choose 2. Chart owns DataLayer, ChartModel, series views, frame owner/controller,
fonts and gesture sessions. Canvas state owns native gesture recognition only;
Canvas borrows the prepared frame and publishes owned messages. No new Rc cycles,
callbacks capturing chart owners, channels, async engine or renderer registry.
Existing model Rc/RefCell handles remain internal. Drop releases owned resources.
IDs are monotonically allocated process-wide and never recycled, so foreign or
removed IDs fail membership validation. No JS object-identity map or DOM destroy API.

## Proposed API and slices

`Chart::new(ChartModelOptions)`, `add_series(SeriesOptionsMap, pane,
PriceScalePosition) -> Result<SeriesId, ChartError>`, `set_data`, `update_data`,
`remove_series`, `add_pane`, pane stretch/preservation and price-axis options,
time/crosshair configuration and navigation, `prepare(size, ratio, now)`,
`update(ChartMessage) -> Result<Vec<ChartEvent>, ChartError>`, and
`element() -> Element<ChartMessage>`. Existing typed data/options are reused.
`model()` is read-only inspection, not independent mutation of DataLayer/model.

Slices: data owner/validation and tests; composed frame preparation; reusable
Canvas and typed events; migrate four demos and add minimal public example;
regressions, source comparisons, rendering checks, documentation and review.
Important consumers map directly: data calls coordinate both owners; widgets send
typed input; API events return owned exact-index series rows; model and renderer
keep existing synchronous invalidation/settlement. Geometry is prepared once per
settled pane and drawing never consumes updates. Device scale remains supplied
by the host application's window events.

## Acceptance

All four demos use library chart ownership and Canvas integration, retaining
controls/capture paths. A small application imports only public APIs. Tests cover
invalid replacement without losing old data, removed/foreign IDs, update before
first set_data, historical updates, pane lifecycle, multiple series composition,
exact-index events/whitespace, click versus drag, and existing gestures. Run fmt,
all-target tests/check/Clippy, source oracles and TinySkia/WGPU captures.

## Implemented result

`src/api/{chart,defaults,interaction,widget,mod}.rs` provide the synchronous owner,
validated data/options/navigation operations, typed owned events and borrowing
Canvas integration. `src/model/time_scale.rs` exposes original time for exact-index
events. `src/ui/{multi_pane_frame,line_chart}.rs` compose multiple series in creation
order while retaining shared grids, axes and crosshair. The legacy one-series frame
entry point still validates its complete input length.

All four demos now use this owner/widget through `examples/support/chart_demo.rs`;
its `series.rs`, `capture.rs` and `tests.rs` retain controls, captures and input
regressions. `examples/public_chart.rs` is an independent public-API application.
`README.md` documents embedding, host DPI forwarding and supported scope.

Source consumers constrain the design: ChartApi/DataLayer coordinate shared time
indexes and removal; SeriesApi validators require rejected replacement to preserve
old data; pane/model consumers require independent price scales and synchronized
time; ChartWidget/PaneWidget events require exact-index rows across all series,
raw click coordinates independent of magnet snapping, and settled crosshair data.
Source series/layout defaults were inspected alongside their public definitions.
Represented defaults use a hidden left scale, source series styles and separator
colors. Rust uses a supported generic sans-serif family; attribution remains off
because its rendering is not implemented. Represented options do not promise
unimplemented visual features such as price lines.

JavaScript subscription identity, DOM ownership/manual destroy, class wrappers and
nullable shared callback references are discarded. Hosts consume owned event
vectors synchronously. Canvas owns recognition state, Chart owns engine/frame
state, and immutable drawing cannot mutate either data owner. Native state checks
series membership so replacing a Chart cancels an active gesture even when pane
counts match. Idle data changes do not swallow the next wheel event. Nonempty live
data replacement preserves active axis gestures; removal/empty data cancels them.

## Behavioral tests and validation

Seven public integration tests cover data lifecycle, foreign/removed IDs,
transactional replacement validation, duplicate/invalid times and values,
historical updates, composed rendering, pane preservation/removal, exact-index
click/crosshair events and whitespace, native click versus drag, and malformed
messages without partial mutation. Existing sixteen shared demo regressions run
for all four demos. Temporarily allowing equal replacement timestamps made the
validation regression fail, confirming it protects the ordering contract.

Final validation: 399 test executions pass (328 library, seven API, and sixteen
for each of four demos); formatting, all-target checking, strict Clippy and example
builds pass. The independent example also passes its headless `--smoke` check.
Source comparison scripts pass 19,233 cases across line/candlestick/histogram,
price/time axes, chart/corner layout, multi-pane layout/input and general input.

Twenty-eight price/volume captures pass on each of TinySkia and WGPU (56
combined), covering seven scenarios at scales 1, 1.25, 1.5 and 2 with input
replay. The fractional-scale WGPU frame was visually inspected. A bounded
ten-second native startup of the standalone application completed without an
error and was terminated by the timeout as intended.

Capture acceptance uses deterministic output and complete offscreen coverage at
fractional scales and during interactions. The former purple-color heuristic
falsely flagged a legitimate line/candle blend; comparing the entire image rendered
over two distinct clear colors proves coverage directly. Native double-click
regressions recognize the timed input sequence before expensive font/frame work,
then apply the same typed actions and assert autoscale restoration; this avoids
scheduler-induced timing failures without extending the recognition threshold.

## Remaining dependencies

Area/bar/baseline/custom rendering, arbitrary pane reordering/movement, custom
horizontal behavior, price lines, touch/kinetic input and full upstream API parity
remain future milestones. No new dependency or async engine machinery was added.
Physical mouse/touchpad feel after this migration requires a fresh check; the
user's successful mouse check applies to the preceding interaction milestone.

## Native wheel zoom calibration (2026-10-12)

A user reported needing more wheel steps to zoom than on the TradingView web app.
The engine already preserves `TimeScale.zoom`: bar spacing changes by scale/10
and the point beneath the cursor remains anchored. `ChartWidget._onMousewheel`
normalizes browser deltas and caps scale at one; its DOM line coefficient is 32.
Iced/winit forwards native line deltas directly, so a one-unit native wheel event
previously became scale 0.32 (a 3.2% spacing change). Browser and native event units
are not interchangeable; this is an input calibration change, not an engine
algorithm change or a claim of measured equality with every browser/device.

Two small alternatives were considered: multiply all wheel zoom deltas, or adjust
only native line-unit zoom. The latter preserves pixel-based touchpad behavior
and horizontal panning while giving a full line-unit step scale one (10% zoom in,
9/10 spacing on zoom out). Fractional line deltas remain proportional; large
batches keep the upstream one-step cap. Price-axis scaling retains its existing
mapping. Ownership and public method signatures are unchanged; both single- and
multi-pane widgets use the same ChartInput path.

Changed `src/ui/chart_input.rs` and `tests/chart_api.rs`. The input regression checks
both directions, fractional lines, clamping, pixel deltas, axis exclusion and
unchanged horizontal scroll. A native Canvas-to-Chart test checks actual bar
spacing after wheel input. Both new regressions failed with the previous mapping.
Physical mouse/touchpad comparison still requires the user's hardware check.
Validation after calibration: all 401 test executions pass (329 library, eight
API and sixteen per demo), formatting and strict all-target Clippy pass, all
examples build, and the public example's headless smoke check passes. No renderer
algorithm or data mutation path changed; raster/source-oracle checks were not
repeated for this input-only calibration.
