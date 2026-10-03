# Ordinary price-axis measurement and drawing

## Source contract

The complete `lightweight-charts/src/gui/price-axis-widget.ts` and
`gui/internal-layout-sizes-hints.ts` were inspected. Graph traces establish that
PaneWidget creates independent left/right widgets. ChartWidget applies minimum
width after widget measurement and performs layout separately. The existing Rust
PriceAxisSnapshot already separates visibility, scale marks, styles and gradients.

- `_drawBackground`: solid or full top-to-bottom gradient.
- `_drawBorder`: plot-facing edge, thickness `max(1, floor(borderSize * horizontalRatio))`.
- `_drawTickMarks`: tick strokes require border AND tick visibility; labels do
  not. Tick text spacing is reserved regardless of these flags. Left text is
  right-aligned and right text left-aligned. Text anchors round in logical space;
  tick rectangles snap in bitmap space. Tick labels paint in reverse mark order.
- `optimalWidth`: maximum width of FIRST/LAST ordinary labels, fallback 34,
  plus border + tick length + inner/outer padding + label offset 5; ceiling and
  even rounding. Floating labels and crosshair guard labels contribute additional
  widths in the source, but are explicitly deferred here. Minimum width applies
  after this intrinsic measurement, with the source width-hint operation reapplied.
- `TextWidthCache.yMidCorrection`: `(ascent - descent) / 2`, with missing values
  zero. Measurement and correction do not mutate scale marks.

Required behavior is independent of TypeScript widget inheritance, DOM ownership,
manual destruction, subscriptions, mouse handlers and mutable canvas caches.
Those mechanisms are not copied. Ordinary ticks do not use floating-label APIs.

## Design and ownership

Direct Iced drawing from snapshots would mix measurements with Canvas draw and
make exact placement tests backend-dependent. Chosen: backend-neutral owned
measurement and commands, followed by an Iced adapter. A measurement owns its
input snapshot and metrics so it cannot accidentally be applied to a different
snapshot. Explicit allocated bounds remain independent from required width.

```text
Application -> settled PlotSnapshot.axes
            -> backend text measurements
            -> owned price-axis commands
            -> owned Iced glyph paths / fills
Canvas      -> reads paths only, clips and draws
```

No model references, shared mutable handles, clocks or geometry caches are added.
All engine operations remain synchronous. Preparation never drains invalidation
or changes plot size. The UI provides actual plot bounds through the existing
frame preparation operation. Axes use the same frame's scale marks and height.

## Iced boundary

The neighboring Iced checkout is 0.15-dev, whereas Cargo uses 0.14. Both the local
indexed APIs and the actual downloaded 0.14 APIs were read. In 0.14 Canvas text
is layered separately; TinySkia's cached-text path has an infinite clip. Prepared
`canvas::Text::draw_with` glyph paths filled inside `Frame::with_clip` preserve
clipping and source paint order on both backends. Glyph preparation occurs before
Canvas draw, not during it.

The adapter uses the same paragraph font, size, line height (1.2), shaping and
unbounded single-line width for measurement and glyph preparation. Exact label
strings are measured; there is no digit-normalizing cache or stale font cache.
Iced 0.14 exposes paragraph dimensions, not equivalent canvas ink ascent/descent.
Metrics therefore leave those values absent and glyph paths center the paragraph
line box at the tick coordinate. This explicit fallback is not pixel-identical
browser ink centering, and paragraph height is never presented as glyph ascent.

Iced 0.14 named font families require static strings. The adapter accepts a
UI-owned font resolver with registered static names and supports generic CSS
families/fallback lists. Unsupported names without a registered/generic fallback
produce a typed error, not an unbounded leaked string or silent font mismatch.

Command geometry is logical; border/tick rectangles explicitly snap via the
horizontal and vertical device ratios and convert back once. Text never receives
another device-scale multiplication. Colors use the shared CSS parser; backgrounds
preserve their gradient representation. Display-P3 conversion is not newly added
to this existing sRGB adapter boundary.

## Scope

The demo places fixed-width axis slots around the actual plot and reports both
measured width requests. Fixed allocation is deliberate: it demonstrates drawing
without a multi-pass solver, shrinking/growing feedback or retained-mask replay
after measurement. A left-axis control loads an independently scaled series.
Full width negotiation across panes, time-axis rendering, floating labels,
crosshair safety widths, axis interactions and general layout replay remain deferred.

## Implementation and consumer mapping

`measure_price_axis(snapshot, measurer)` returns a fallible owned
PriceAxisMeasurement. `prepare_price_axis(measurement, bounds, ratio)` returns
PreparedPriceAxis with a required width and ordered background/rectangle/text
commands. Binding the snapshot into its measurement replaces the provisional
separate snapshot argument and makes accidental mismatched replay impossible.
Math.round's negative-half behavior is explicitly preserved.

`prepare_iced_price_axes` uses only the current PlotSnapshot axes. Measurement
and glyph paths are prepared in application update; Canvas reads prepared fills
and glyph paths. The plot adapter now retains the same configured gradient so
plot and price-axis backgrounds agree. The demo's Left axis and Ticks controls
exercise independent left/right marks. Scene flags `--gradient --left-axis
--ticks --large --negative` permit visual checks without modifying desktop settings.

The two-series demo exposed a pre-existing removal issue: DataLayer kept removed
IDs in its type/custom registries, so a later reindex/removal response could refer
to an already-unregistered model series. Explicit removal now preserves its
current response and deletes the metadata before subsequent responses. This
matches the original data-layer's empty-row cleanup. Regression coverage checks
sequential model removals and subsequent updates after custom removal. Clearing
data on a still-registered series continues to preserve its usable type metadata.

The original endpoint-only sizing rule can underestimate a custom formatter's
wide interior label; this known source limitation is preserved, not silently
replaced with another sizing algorithm. Likewise the exact source hint is
`width + width % 2`: a fractional minimum 81.5 produces 83 after the second hint.
This is covered explicitly rather than assuming an integer minimum.

## Changed files

- `Cargo.toml`: enable Iced advanced paragraph APIs; no new dependency package.
- `src/renderers/price_axis_renderer.rs` and
  `src/renderers/price_axis_renderer/tests.rs`: measurement, logical commands,
  typed validation and deterministic geometry tests.
- `src/renderers/mod.rs`: export.
- `src/ui/price_axis.rs`: font resolution, exact backend measurement, glyph
  preparation, gradient fills, immutable clipped drawing and adapter tests.
- `src/ui/mod.rs`: export.
- `src/ui/line_chart.rs`: shared CSS color parsing and settled-frame gradient.
- `examples/line_chart.rs`: side-by-side axes, fixed allocations, independent
  left data, tick controls, measured-width reporting and representative scenes.
- `src/model/data_layer.rs`: removal cleanup and custom regression assertion.
- `src/model/chart_model/tests.rs` and
  `src/model/chart_model/tests/price_axis.rs`: connected rendering/removal tests.
- `examples/price_axis_reference.rs` and
  `scripts/check_price_axis_reference.cjs`: read-only original-widget differential
  oracle for 128 cases.
- This document: source contract, alternatives, ownership, scope and validation.

## Tests and validation

16 added tests cover endpoint sizing/fallback/minimum policy, visibility
combinations, left/right anchors, measured y corrections, reverse text order,
gradients, exact labels, source rounding, fractional/nonuniform ratios, owned
replay, zero bounds, typed errors, real font measurement, CSS font resolution,
logical glyph positions, clipping bounds, formatting modes, grid/model alignment,
resizes, appends, historical changes, stale-output removal and sequential removal.
The expanded demo smoke test exercises both axes and tick visibility.

The oracle executes the actual original PriceAxisWidget methods with recording
canvas/text targets. All 128 combinations of axis side, scale ratio, border/tick
flags, empty marks and gradient background match required widths and ordered
draw commands. It uses deterministic ink metrics and deliberately excludes
floating/crosshair labels, so it does not claim browser/Iced glyph pixel equality.

- `cargo fmt --check`, `git diff --check`, `cargo check --all-targets`: pass.
- `cargo test --all-targets --quiet`: all 246 library tests and the expanded
  demo test pass.
- `cargo clippy --all-targets`: succeeds with the same five pre-existing test
  warnings; no warnings in the new code.
- `cargo clippy --lib --examples -- -D warnings`: passes.
- `cargo run --example line_chart -- --smoke`: passes.
- The smoke sequence also passes with `--gradient --left-axis --ticks --large
  --negative`.
- `node scripts/check_price_axis_reference.cjs`: all 128 original-widget
  differential cases match.
- Both normal and representative GUI scenes were launched and inspected through
  scoped desktop captures. Right labels align with grid levels. The two-axis
  gradient scene shows readable negative/large labels, plot-facing ticks, correct
  independent anchors and matching gradient backgrounds. The existing window
  also resized from a tall view to a shorter view without stale axis height.
  This inspection used the Omarchy capture guidance without changing desktop
  configuration. Fractional ratios are checked by geometry and glyph tests,
  not claimed as a physical fractional-monitor visual test.

The source methods were rechecked against the completed commands and oracle.
Browser glyph pixel equality is not claimed because the documented Iced
line-box centering fallback differs from browser ink metrics. Font loading,
full CSS font-list parsing, wide interior custom labels, Display-P3 conversion,
crosshair/floating widths and dynamic axis layout remain explicit dependencies.
Demo slots stay fixed at 120 pixels: unusually wide custom labels are clipped
to their allocation, not allowed to paint over the plot. The measured request
is exposed so the later layout solver can resolve that allocation.
