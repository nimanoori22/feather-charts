# Candlestick rendering

## Source and consumers

Investigated complete `model/series/candlesticks-pane-view.ts`,
`bars-pane-view-base.ts`, `series-pane-view-base.ts`, `candlestick-series.ts`,
`model/series-bar-colorer.ts`, `renderers/candlesticks-renderer.ts`,
`optimal-bar-width.ts`, the inner-border helper and `model/magnet.ts`.
Source graphic cases cover small bodies, huge ranges, whitespace, per-row
colors, transparency, option changes and MagnetOHLC.

Graph traces connect the series definition to the candle view and its renderer.
Direct source reads establish the additional lifecycle contract: Series constructs
the view, updates it on data/options/scale changes, and exposes it to pane drawing.
The view selects strict visible rows, uses the selected price scale and first
visible close baseline, then hands owned-equivalent coordinates to its renderer.
Pane drawing consumes bitmap geometry; crosshair labels use current scales.
Magnet consumes exact-index OHLC candidates from visible non-overlay series in
source order. Candle series do not supply line-style crosshair point markers.

Indexed paths have no recorded coverage gaps. The broader source scope reports
a partial GUI tsconfig and excluded coverage-test subtree; source reads and
direct graphics-test inspection are used rather than claiming exhaustive graph
coverage. Full public API, hit-testing and conflation remain outside this slice.

## Required behavior

- Open <= close selects up colors, including doji. Row body/border/wick colors
  override their respective directional defaults independently.
- Strict visible range, with no line-neighbor extension; whitespace keeps time
  spacing but does not create candle rows. Hidden/empty/zero-size output clears.
- OHLC coordinates share the selected time/price scales; percentage/indexed
  baselines use the first value rule already implemented by Series.
- Width follows the original atan-based rule, including spacing 2.5–4 and parity
  matching the floored horizontal device ratio. Rounding follows Math.round,
  including negative coordinates.
- Wicks precede borders, then bodies. Neighbor edge correction applies to wicks
  and borders. Preserve individual rectangle ordering and signed rectangle
  extents for inner borders, including transparent overlap semantics.
- Border visibility, narrow widths and collapsed bodies follow source conditions;
  do not invent a minimum logical body height.
- Normal/Magnet/MagnetOhlc modes use existing raw/applied crosshair state;
  candidates are close or ordered open/high/low/close coordinates respectively.

## Rust design and alternatives

Choose typed preparation entry points plus a private borrowed line/candle enum
inside shared frame settlement. The existing public line APIs continue to work;
`ChartFrameOwner::prepare_candlesticks` uses the same layout/animation loop.
A public owned pane-view/output enum was viable but would require migrating line
callers and expose dispatch concerns unnecessarily. A generic renderer registry
would add complexity for two current paths.

The application owns DataLayer, ChartModel, controller, frame owner and pane
view. Model preparation temporarily borrows rows and scales. The view owns
optional renderer data; refresh clears old data first. Frames own converted
rectangles, axes and crosshair output. Canvas borrows them read-only. No new
Rc relationships, owner back-references, cycles, async operations or manual
destruction are required; dropping owners releases their values.

Important proposed signatures:

```rust
ChartModel::prepare_candlesticks(id: SeriesId)
    -> Result<Option<CandlestickRendererData>, CandlestickPreparationError>;
CandlesticksPaneView::refresh(&mut self, model: &mut ChartModel<B, M>, id: SeriesId)
    -> Result<(), CandlestickPreparationError>;
CandlestickRendererData::draw_commands(&self, ratio: PixelRatio)
    -> Result<Vec<CandleRectangle>, CandlestickRenderError>;
ChartFrameOwner::prepare_candlesticks(/* same inputs as prepare, candle view */)
    -> Result<ChartFrameSnapshot, ChartFrameError>;
```

Model consumers retain registry ownership and scale selection. Renderer consumers
receive owned geometry without model references. UI consumers reuse shared frame
settlement and input routing; prepared drawing does not mutate state or measure
text. Lifecycle consumers clear output on missing selection/removal. Conflation
uses an explicit unsupported error, matching the current line scope.

Discard TS inheritance, mutable renderer `setData` sharing, NaN initialization,
manual invalidation flags and runtime casts. Preserve their observable ordering,
visibility and cleanup semantics through enums, typed values and owned snapshots.

## Implementation and validation

Backend-neutral renderer and model preparation implemented. The actual-source
oracle matches 7,776 combinations of spacing, independent device ratios,
visibility, shape and visible ranges, including signed extents and color order.
Connected tests cover all four scale modes, strict selection, independent color
overrides, whitespace, replacement, hidden/zero-size/removal cleanup and actual
candle Magnet/MagnetOhlc labels. No magnet algorithm changes were needed.

Baseline line zoom (debug, 120 rows, 100 samples after 20 warmup): alternating
engine/measurement median 8.954 ms / p95 9.240 ms; glyph paths 0.568 / 0.617 ms.
Sustained engine 8.052 / 8.727 ms; glyph paths 0.514 / 0.610 ms.

UI, demo, final regression/performance and raster validation remain pending.
