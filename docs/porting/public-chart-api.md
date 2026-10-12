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
