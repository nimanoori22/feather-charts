# Histogram rendering port

## Source and consumers

Investigated complete `src/renderers/histogram-renderer.ts`,
`model/series/histogram-pane-view.ts`, `line-pane-view-base.ts`,
`series-pane-view-base.ts` and `histogram-series.ts` in Lightweight Charts.
The definition constructs the pane view; Series owns it and exposes pane views
to pane widgets. Data/options invalidation rebuilds rows/colors; renderer access
projects coordinates lazily. Source Series first-value and histogram colorer
sections were read directly (Series graph freshness reported changed metadata).
Graph traces identify the definition -> view -> renderer relationship; coverage
checks are best-effort, supplemented by direct source inspection.

Histogram/whitespace and alternating-gap graphics fixtures establish that
missing observations remain gaps, including when another series fills time slots.
Rendering is synchronous. Pane widgets borrow rendering output, while model
updates determine scale/visible state before drawing.

## Required contract

- Histogram is single-value data with a configurable base; autoscale includes it.
- First value is the first real row at/after the visible left bound. Projection
  uses the selected price scale and source extended visible range (one neighbor
  on either side), unlike strict candlestick selection.
- Row color overrides the series color. Whitespace has no rectangle.
- Bitmap width uses rounded bar spacing, parity centering, adjacency correction
  only for consecutive logical indexes, then narrow-column width alignment.
- Base thickness is at least one bitmap pixel; equal-base bars remain visible.
  Positive/negative bars include the source-rounded baseline thickness.
- Empty/hidden/unavailable scales produce no output. Option/data changes replace
  geometry; failures clear previously prepared pane-view data.
- Conflation remains explicitly unsupported, matching existing line/candle port
  entry points; it is not silently approximated.

## Alternatives and chosen ownership/API

1. Extend private typed dispatch and add a typed histogram entry point, keeping
   shared layout, animation, axes and Canvas conversion.
2. A renderer trait/registry with erased preparation and generic frame assembly.

Choose 1: the three known series types need only bounded dispatch. It avoids
dynamic allocation/downcasting and preserves existing public callers. A registry
would add lifecycle/API machinery without a current consumer requirement.

`HistogramPaneView` owns `Option<HistogramRendererData>` and exposes `data`,
`clear`, `refresh`. `ChartModel::prepare_histogram(SeriesId)` returns owned data;
`ChartFrameOwner::prepare_histogram` shares settlement with existing entry points.
Renderer data exposes `draw_commands(PixelRatio) -> Result<Vec<CandleRectangle>,
HistogramRenderError>`; the existing rectangle is a bitmap primitive, independent
of candle semantics, and its Canvas adapter is reused.

Application owns model/controller/frame owner/view. Preparation borrows model
scales/rows; output owns coordinates/colors. Drawing borrows the immutable frame.
There are no strong back-references or cycles; ordinary Drop releases output.
Source cache identity, inheritance, mutable sharing and manual destroy are
discarded. Draw commands calculate width once per preparation/conversion rather
than keeping the TypeScript renderer's mutable lazy cache.

## Implementation evidence

Renderer tests pass; 9,900 bitmap rectangle cases match the actual TypeScript
renderer. Model tests cover extended visible range and projection in normal,
logarithmic, percentage and indexed modes, row colors/whitespace/options,
updates, hidden/zero-width/removal clearing and wrong series types.
`cargo test --all-targets` passes (310 library tests, existing example tests).
`cargo run --example histogram_chart -- --smoke` verifies the shared frame/demo
lifecycle. Final captures, frame-specific failure/conflation coverage and full
validation remain Task 10 work; multi-pane coordination is reported separately.

Changed: `src/renderers/histogram_renderer.rs`, renderer exports,
`src/model/series/histogram_pane_view.rs`, series exports, `chart_model.rs`, its
histogram tests/test registration, `src/ui/line_chart.rs`, `chart_frame.rs`,
`examples/histogram_chart.rs`, shared demo/capture/test support,
`examples/histogram_reference.rs`, `scripts/check_histogram_reference.cjs`.
Related dependencies: multi-pane frame/layout/input and public widget API.
