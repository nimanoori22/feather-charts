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

The primary indexed paths have no recorded coverage gaps. `model/series.ts`
has changed index metadata; its relevant constructor, data/options invalidation,
pane-view and marker consumers were inspected directly. The broader source scope reports
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

The shared frame path and runnable demo are implemented. The line example now
uses `examples/support/chart_demo.rs`; the candle example uses the same
application with typed candle input/preparation. Captures and tests live in
separate support modules. Wick/border/transparency controls and all four
crosshair modes are available for candles. The optional left-scale line remains
an eligible source for magnet and line crosshair markers; candles alone have
no point markers.

## Files changed

Created:

- `src/renderers/candlesticks_renderer.rs`
- `src/model/series/candlesticks_pane_view.rs`
- `src/model/chart_model/tests/candlesticks.rs`
- `examples/candlestick_reference.rs`
- `scripts/check_candlestick_reference.cjs`
- `examples/candlestick_chart.rs`
- `examples/support/chart_demo.rs`
- `examples/support/chart_demo/capture.rs`
- `examples/support/chart_demo/tests.rs`
- `tasks/candlestick-plan.md`, `tasks/candlestick-todo.md`, this report

Modified:

- `src/model/series.rs`, `src/model/chart_model.rs`, its `tests.rs`
- `src/renderers/mod.rs`
- `src/ui/chart_frame.rs`, `src/ui/line_chart.rs`
- `examples/line_chart.rs`, `examples/interaction_reference.rs`
- `scripts/check_interaction_reference.cjs`

No new dependencies or edits to the reference repositories.

## Behavioral verification

Renderer tests cover layer ordering, narrow bodies, doji and invalid ratios,
coordinates and ranges. Connected model tests cover strict selection, all four
scale modes, row/style overrides, whitespace, same-time update, prepend/history,
hidden/zero-size/removal cleanup, retained geometry, conflation rejection,
close/OHLC snapping, unequal scales, hidden and overlay candidate exclusion,
labels and marker eligibility. UI tests cover signed extents, alpha, invalid
colors and f32 overflow. Shared demo tests exercise line and candle native event
sequences, resize, repeated stationary pointers/redraw settlement, animation
cancellation, labels, append/replace/styles/remove/reload and seven scenarios.

A mutation experiment inverted the up/down condition; the candle color test
failed on the doji default colors. The original code was restored before final
validation. Review also corrected two capture assumptions: a left-scale line may
have a marker, and border-only narrow candles need not show a blue body.

Source comparisons: 7,776 new candle rectangle cases; 131 navigation/crosshair/
label/magnet cases (including 42 close/OHLC candidate cases); 180 line, 128 price
axis, 192 time axis, 192 layout and 64 corner cases. Total: 8,663 comparisons.
The oracle executes original TypeScript methods, not a rewritten JS renderer.

112 candle captures (56 TinySkia + 56 WGPU) cover seven display scenarios and
all wick/border/transparency combinations at ratios 1, 1.25, 1.5 and 2.
Assertions cover repeatable read-only drawing, retained state, visible series,
crosshair eligibility and absence of uncovered seams. Representative default,
narrow and transparent output is inspected visually. Another 56 line captures
are byte-identical to the pre-candle baseline across both backends.
Captures are in `/tmp/feather-candles-{tinyskia,wgpu}` and
`/tmp/feather-candles-line-{tinyskia,wgpu}`; they are not committed. Source
rectangle parity does not promise identical Canvas2D/Iced antialiasing pixels.

## Performance and native checks

Debug profile, 120 time points, viewport 917 × 870.80005, 120 zoom frames per
workload with 20 warmup frames excluded. Values below are median / p95 ms;
these are preparation stages, not complete wheel-to-screen latency.

| Workload | Model | Engine/measurement | Glyph paths |
| --- | --- | --- | --- |
| Line alternating, before | 0.024 / 0.030 | 8.954 / 9.240 | 0.568 / 0.617 |
| Line alternating, after | 0.024 / 0.029 | 8.919 / 9.228 | 0.566 / 0.611 |
| Line sustained, after | 0.022 / 0.038 | 8.035 / 8.670 | 0.513 / 0.588 |
| Candle alternating | 0.027 / 0.035 | 7.987 / 8.149 | 0.481 / 0.507 |
| Candle sustained | 0.023 / 0.029 | 6.996 / 7.709 | 0.452 / 0.511 |

No material line preparation regression. Candle warm preparation is about
8.5 ms alternating and 7.5 ms sustained in this workload. The existing glyph
cache and input coalescing remain active. Large-data conflation/performance
and physical input latency are not established by this small-data benchmark.

The native candle application ran for a bounded 15-second startup check without
reported runtime errors; timeout exit 124 was intentional. Actual physical
mouse/touchpad gestures, cancellation and subjective smoothness remain manual
acceptance checks. Earlier user confirmation concerned line charts only.

## Running and final checks

```sh
cargo run --example candlestick_chart
cargo run --example candlestick_chart -- --smoke
cargo build --examples
node scripts/check_candlestick_reference.cjs
node scripts/check_interaction_reference.cjs
cargo test --example line_chart profile_zoom_frame_stages -- --nocapture
cargo test --example candlestick_chart profile_candle_zoom_frame_stages -- --nocapture
```

Capture flags: `--capture-dir <directory> --capture-interactions --capture-styles`;
add `--wgpu` to select WGPU. References require the sibling source checkout and
its installed TypeScript package. Standard Cargo tests need no Node or desktop.

Final checks pass: `cargo fmt --check`, `git diff --check`,
`cargo test --all-targets` (305 library tests and six shared demo tests in each
of the two example targets), `cargo check --all-targets`,
`cargo clippy --lib --examples -- -D warnings` and `cargo build --examples`.
Both example smoke paths pass. No source/reference comparison failed.
Source consumers were rechecked after implementation: selected-scale preparation,
strict visible selection, view refresh on data/options changes and candle marker
ineligibility map cleanly to the implemented types and frame entry points.

## Remaining dependencies

Histogram/volume rendering, multiple-pane UI and the reusable public chart/widget
facade remain subsequent milestones. Conflation, general candle hit testing,
price lines, touch/kinetic/axis gestures, primitives and public API coverage remain
outside this roadmap. Unsupported conflation returns a typed error rather than
silently producing unconflated candles.
