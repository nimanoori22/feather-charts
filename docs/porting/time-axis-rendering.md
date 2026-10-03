# Ordinary time-axis rendering

## Source and consumers

Behavioral reference: `lightweight-charts/src/gui/time-axis-widget.ts`, read
in full. This port covers ordinary tick drawing and sizing, not the entire
interactive widget.

Graph-assisted consumer analysis traced `_drawTickMarks` through `paint` and
its font, edge-alignment, color and renderer-option helpers. `ChartWidget`
owns layout: `_adjustSizeImpl` (476–565) takes the maximum of `optimalHeight`
and `minimumHeight`, applies `suggestTimeScaleHeight`, reserves the resulting
height, then gives the time widget the same width as the plot. The GUI's
`update` refreshes lazy scale marks; normal light/full painting reads them.
Cursor painting uses a separate layer and does not repaint ordinary ticks.

`TimeScale.marks` (623–700) supplies formatted labels, weights, logical
coordinates and edge-alignment requests. `TickMarks._buildMarksImpl` chooses
marks by descending weight with minimum index separation; uniform distribution
and fulfilled-index filtering also live there. **There is no extra measured
overlap-suppression pass in the ordinary time widget.** Rust retains those
existing model algorithms; drawing all supplied ticks is intentional, even for
custom labels too wide for the model's estimated character budget.

Coverage checks were performed for the source widget, chart widget, sizing
hints, mark-selection dependencies, existing Rust boundaries, Iced text/font
APIs and the Flowsurface example. The original time-scale/tick-mark graph
metadata was stale; the relevant implementations were read directly rather
than relying on stale call edges. Graph coverage is best-effort, not exhaustive
proof of call discovery.

## Behavioral contract and tests

| Rule | Source | Verification |
| --- | --- | --- |
| Coordinates are local logical pixels; tick coordinates do not move when text is aligned | `_drawTickMarks`, 380–442 | fractional snapping and connected grid tests |
| Ticks draw only with both border and ticks enabled; text and text spacing survive either being disabled | `_drawTickMarks`, 395–420 | all visibility combinations |
| Border is top-facing: max(1, floor(borderSize × vertical ratio)) bitmap pixels | `_drawBorder`, 370–378 | fractional/nonuniform ratios and oracle |
| Tick width is max(1, floor(horizontal ratio)); offset is floor(horizontal ratio / 2); length and x use JS rounding | `_drawTickMarks`, 400–409 | ratios 1, 1.25, 1.5, 2; negative-half test |
| Tick rectangles paint in reverse mark order | `_drawTickMarks`, 407–410 | recording oracle |
| Ordinary text is horizontally centered and vertically middle-aligned at border + tick + top padding + fontSize / 2 | `_drawTickMarks`, 415–426 | exact positions and real-Iced checks |
| Nonpriority labels paint first, priority labels last; priority remains independent of bold permission | `_drawTickMarks`, 427–440 | priority-order and normal/bold tests |
| Emphasis comes from horizontal behavior's threshold, not the numeric maximum | `_drawTickMarks`, 389; existing snapshot builder | existing behavior-threshold snapshot test |
| Align only requested labels: left = floor(coord − width/2) + 0.5; correct left overflow, otherwise right overflow | `_alignTickMarkLabelCoordinate`, 445–457 | exact left/right/narrow positions and oracle |
| Oversized labels use the source's single-sided correction, not iterative clamping | same | narrow-axis test |
| Background uses the configured bottom color, including gradients | `_drawBackground`, 366–368 | bottom-background test and oracle |
| Intrinsic height is ceil(border + tick + font + top/bottom padding + bottom offset) | `optimalHeight`, 268–279 | height/typography tests |
| Minimum height applies after intrinsic sizing; final hint is height + height % 2 | `ChartWidget._adjustSizeImpl`, 511–512; `internal-layout-sizes-hints.ts`, 14–16 | fractional minimum test |
| Hidden axes require zero height; empty axes retain background/border but no ticks/text | widget painting and chart visibility/layout boundaries | empty/hidden/zero tests |

No formatter or second tick selector was introduced. Scale labels, weights,
priority and alignment metadata remain unchanged in the owned measurement.
No arbitrary overlap removal was added. With scrolling enabled, the source
allows unaligned edge labels to be clipped. The demo's optional `--fixed-edges`
scene exercises scale-generated alignment requests; it does not force every
label to be clamped. Wider-than-plot labels remain clipped by allocation.

## Design, alternatives and ownership

Two viable designs were considered: direct Iced measurement/drawing from the
snapshot, or measured backend-neutral commands plus an adapter. The latter
matches the existing line/price-axis boundary, allows deterministic testing,
and keeps drawing read-only. Copying small label collections and generating
glyph paths each relevant frame costs allocations, accepted before caching.

```text
Application owns ChartModel and animation timing
  → settled PlotSnapshot (grid, line, all axis snapshots)
  → TimeAxisMeasurement (owned snapshot, per-tick widths)
  → PreparedTimeAxis (owned logical drawing commands, required height)
  → IcedTimeAxis (owned fills/glyph paths, allocated bounds)
  → Canvas reads and draws within a clip
```

No strong model references, weak references, clocks, callbacks or shared
mutable handles are stored. Dropping a frame drops its data; there are no
cycles, manual destruction or subscription identity requirements. The
TypeScript DOM cells, canvas bindings, inheritance, mutable renderer-option
cache and manual cleanup are incidental to its GUI, not replicated.

API:

```rust
measure_time_axis(snapshot, normal_measurer, emphasized_measurer)
    -> Result<TimeAxisMeasurement, TimeAxisRenderError>
prepare_time_axis(measurement, allocated_bounds, pixel_ratio)
    -> Result<PreparedTimeAxis, TimeAxisRenderError>
prepare_iced_time_axis(snapshot, allocated_bounds, pixel_ratio, fonts)
    -> Result<IcedTimeAxis, TimeAxisUiError>
```

Two `TextMeasurer` contexts explicitly select each tick's font without changing
the existing trait. Widths are measured exactly, without digit substitution or
a width cache. The measurement owns the snapshot, preventing stale-font or
mark/metric pairings during replay. `required_height` is a request, not a
mutation of allocated bounds or model dimensions. Invalid snapshot metrics,
bounds and pixel ratios return typed errors.

The model consumer supplies marks once in frame preparation. The future
layout owner can read requested height and prepare the same measurement at
new bounds. The Canvas consumer only draws owned geometry. Floating axis
views remain on their distinct rendering boundary.

## Iced 0.14 and Flowsurface

Checked the neighboring Iced 0.14.0 source, notably
`graphics/src/geometry/text.rs` and `core/src/font.rs`. Measurement and glyph
preparation share the resolved family, explicit normal/bold weight, font size,
advanced shaping, unbounded horizontal extent and relative line height 1.2.
Fonts use the existing static-name registration and CSS generic-family resolver.

Iced's paragraph exposes line-box metrics, not browser-equivalent glyph ink
ascent/descent. The explicit policy is paragraph-center alignment at the
source's yText coordinate; no fake glyph ascent is synthesized. This is not a
claim of browser-identical font rasterization or baseline pixels.

Glyph paths are prepared with `Text::draw_with`, then drawn through the same
clipped primitive helper as price axes. This avoids `Frame::fill_text` escaping
Canvas clipping on the TinySkia backend. Text positions and paths stay logical
at every device ratio. Only tick/border rectangles are bitmap-snapped and
converted back once; curves are not truncated to simulate clipping.

Flowsurface's `src/widget/chart/heatmap/ui/axisx.rs` was inspected as an example
of a separate axis Canvas. Its camera, approximate spacing, cache and input
architecture are not copied; Lightweight Charts defines this port's rules.

## Frame integration and zero-width correction

The demo prepares axes from the existing settled `PlotSnapshot.axes`, after
dimensions, deferred commands, animation and pane updates. It does not drain
another mask, independently generate marks, or mutate model dimensions during
drawing. A 40-pixel time slot sits under the plot, with blank corner slots below
the existing fixed 120-pixel price slots. The UI reports the required height
(28 pixels at the default typography) separately. Actual plot bounds continue
to drive the model; no full layout solver or geometry caching was added.

The demo enables time labels using the model's coordinated options update, so
DataLayer and TimeScale behavior configuration stays coherent. Its existing
tick toggle affects all axes. Existing load, fit, append, replace, older/latest,
remove/reload and animation controls rebuild time geometry in the same frame.

Fixed-edge startup testing exposed a pre-existing Rust zero-width issue:
`correct_bar_spacing` could set spacing to zero when both edges were fixed,
then offset correction divided zero by zero and panicked in `f64::clamp`.
The port explicitly accepts zero-sized layouts, unlike the source widget's
nonpositive-width rejection. The correction functions now defer width-based
correction until width is positive, retaining finite spacing/offset state.
A connected regression covers zero width, empty output and recovery on resize.

## Changed files

- `src/renderers/time_axis_renderer.rs`: owned measurement, height and commands.
- `src/renderers/time_axis_renderer/tests.rs`: deterministic geometry contract.
- `src/renderers/mod.rs`: export.
- `src/ui/time_axis.rs`: matching-font measurement and immutable glyph adapter.
- `src/ui/price_axis.rs`: reusable font-context construction/clipped primitives.
- `src/ui/mod.rs`: export.
- `src/ui/line_chart.rs`: correct settled-axis snapshot documentation.
- `src/model/time_scale.rs`: zero-width correction guard.
- `src/model/chart_model/tests.rs`: connected test module.
- `src/model/chart_model/tests/time_axis.rs`: settled-frame and startup tests.
- `examples/line_chart.rs`: time-axis region, height request and fixed-edge scene.
- `examples/time_axis_reference.rs`: deterministic source-comparison cases.
- `scripts/check_time_axis_reference.cjs`: actual original-widget oracle.
- `docs/porting/time-axis-rendering.md`: this contract and report.

## Validation and omissions

The differential oracle executes the actual original TypeScript widget methods using fake
Canvas/text contexts, rather than duplicating their algorithm in JavaScript.
It covers 192 combinations of ratio, width, visibility, emphasis and empty data.
Real-Iced font tests avoid asserting platform-specific numeric widths.

Completed validation:

- `cargo fmt --check` and `git diff --check`: pass.
- Focused `cargo test time_axis --lib`: 14 matching tests pass, including one
  existing whitespace/time-axis test; 13 tests were added in this milestone.
- `cargo test --all-targets`: 259 library tests and the connected demo test pass.
- `cargo check --all-targets`: pass.
- `cargo clippy --all-targets`: pass with five pre-existing test warnings
  (test-module placement, needless option dereference and field reassignment).
- `cargo clippy --lib --examples -- -D warnings`: pass.
- `cargo build --example time_axis_reference` followed by
  `node scripts/check_time_axis_reference.cjs`: all 192 cases match the original.
- `cargo run --example line_chart -- --smoke`: pass; also passes with
  `--gradient --left-axis --ticks --fixed-edges` after the zero-width correction.
- `cargo run --example line_chart`: GUI launched. Inspected default and
  smaller fixed-edge/gradient/two-price-axis scenes: ordinary time text,
  emphasized day labels, tick/grid alignment and bottom-color fill are correct.
  Captures were limited to the demo windows using Omarchy capture guidance;
  no desktop configuration was changed. Physical fractional monitor scaling
  was not manually changed; deterministic snapping/glyph tests cover it.

Source re-check: the original widget methods are exercised directly by the
oracle; the final API retains the sizing information needed by ChartWidget's
future layout port without coupling preparation to it. Exact measurements
intentionally do not reproduce the source's digit-normalizing width cache.

Deferred: axis-aware multi-pass allocation and retained-command replay after
axis-driven resizing (step 4); multi-pane layout; corner-stub decorations;
floating time/crosshair labels; axis interactions and primitives; measurement
and geometry caches; Display-P3 rendering; exact browser font ink/baseline
equivalence. Ordinary label overlap remains governed by existing scale marks,
not a new GUI selector.
