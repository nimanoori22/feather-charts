# Ordinary axis snapshots

## Scope and source analysis

This step prepares owned, backend-neutral ordinary tick labels and display policy.
It does **not** measure text, compute final axis dimensions, clamp coordinates,
resolve label overlaps, draw labels, or reserve axis space in the demo.

The complete source widgets were inspected, including construction, update,
paint, measurement, subscriptions, interaction and destruction. Relevant source
responsibilities and consumers are:

- [`PriceAxisWidget`](../../../lightweight-charts/src/gui/price-axis-widget.ts):
  `rendererOptions`, `_drawBackground`, `_drawBorder`, `_drawTickMarks`, and
  `optimalWidth`. Ordinary ticks read `PriceScale.marks()` directly. Text color
  is the scale override or layout text color. `_alignLabels` and `_drawBackLabels`
  concern separate floating views and do not apply to ordinary ticks.
- [`TimeAxisWidget`](../../../lightweight-charts/src/gui/time-axis-widget.ts):
  `_drawTickMarks` preserves weight and edge-alignment requests;
  `_getRendererOptions` derives font-relative padding. Width measurement and
  edge clamping happen in the widget, not mark generation.
- [`PriceAxisRendererOptionsProvider`](../../../lightweight-charts/src/renderers/price-axis-renderer-options-provider.ts)
  derives border/tick size and typography spacing. Its existing Rust equivalent
  is reused, not duplicated.
- [`HorzScaleBehaviorTime.maxTickMarkWeight`](../../../lightweight-charts/src/model/horz-scale-behavior-time/horz-scale-behavior-time.ts)
  may reduce an intraday maximum to Hour1. Other horizontal behaviors own their
  own policy; taking a raw numeric maximum is incorrect.
- [`PaneWidget._recreatePriceAxisWidgets`](../../../lightweight-charts/src/gui/pane-widget.ts)
  constructs left/right widgets independently from scale visibility. DOM
  widget lifetime and subscriptions are incidental to owned snapshots.
- [`ChartWidget._drawImpl`](../../../lightweight-charts/src/gui/chart-widget.ts)
  applies layout, momentary autoscaling and deferred commands before updating
  axes. It can replay a retained mask after a layout change. Layout elsewhere
  applies scale minimum width/height after text measurement.

Structural relationships were traced with the knowledge graph and checked
against current source. Coverage is best-effort; metadata changes on the
renderer-options provider and time behavior were resolved by direct source reads.

## Behavioral contract and tests

| Required behavior | Source responsibility | Behavioral coverage |
| --- | --- | --- |
| Coordinates are logical plot-relative pixels, with original labels and mark order | Both widgets' `_drawTickMarks`; scale `marks()` | Copy/coordinate/grid tests and fractional device-scale frame tests |
| Left and right scales are independent | Pane widget construction, each price widget's attached scale | Independent left/right data and formatting tests |
| Normal, logarithmic, percentage and indexed formatting is scale-owned | Price scale mark builder/formatters | All-mode tests compare snapshots to actual marks |
| Axis, border and tick flags are independent; hidden strokes do not hide text | Both widgets' border/tick draws | All eight flag combinations for both axes |
| Tick-related text spacing remains even with hidden ticks | Price text x expression; time text y expression | Renderer-option spacing assertions with invisible ticks |
| Price text color uses scale override then layout; borders/ticks use scale border color | Price `_drawTickMarks` | Default, override, clear-override and atomic invalid patch tests |
| Weights and alignment flags remain unchanged | Time `_drawTickMarks` | Mark equality and fixed-edge alignment tests |
| Behavior chooses priority threshold; priority paints last, even if bold is disabled | Time `_drawTickMarks` and behavior `maxTickMarkWeight` | Policy threshold and disabled-bold tests |
| Empty scales return fresh empty collections, without stale output | Both widgets' empty guards | Empty, removal, zero-width and zero-height tests |
| Price axes retain the full background gradient; time fills with its bottom color | Widgets' `_drawBackground` | Gradient, bottom-fill and color-space tests |
| Typography determines mark density as well as rendering metrics | Scale tick selection and widget font options | Small/large-font cache refresh tests, including direct TimeScale calls |
| Prepared values survive later updates without mutation | Frame ownership boundary | Retained snapshot, resize, append, history and animation tests |
| Invalid pane requests change no state or pending work | Rust owner boundary | Typed invalid-pane and no-partial-mutation tests |

The existing Rust visibility defaults are preserved (both price scale options
default visible). Newly added price styling follows the source: border visible,
border color `#2B2B43`, no text-color override. A text-color patch uses
`Option<Option<String>>`: no patch, explicit override, or reset to layout color.

## Design, alternatives and ownership

Borrowed cache views avoid a few allocations but cannot survive model mutation.
Owned snapshots match the existing line/frame boundary and make replay safe.
Scale-owned styling is added to existing PriceScaleOptions rather than supplied
by a competing application configuration. Typography and background remain
layout-owned. Ordinary ticks reuse renderer **options**, not floating-label
renderer traits, data, positioning or overlap rules.

```text
ChartModel owns scales, panes and layout
  -> short-lived sequential scale borrows
  -> owned AxisSnapshots
       -> future UI measurement/layout/drawing
```

No reference back to ChartModel, callbacks, clocks, Iced types, shared mutable
handles or ownership cycles exist in the snapshots. An ephemeral existing
price renderer-options provider derives both price snapshots; there is no new
snapshot/geometry cache. TypeScript inheritance, DOM lifetimes, cached mutable
renderer objects and manual subscription cleanup are intentionally not copied.

## API and frame integration

`ChartModel::prepare_axis_snapshots(pane)` returns `AxisSnapshots { left, right,
time }` or `AxisSnapshotError::InvalidPane`. Price snapshots contain their side,
marks, independent visibility flags, border styling, effective text color,
background/color space, minimum width and existing typography metrics. Time
ticks wrap TimeMark with independent priority and font-emphasis metadata; time
options carry minimum height and unmeasured typography/padding metrics.

`TimeAxisSnapshot::background_fill_color()` exposes the source bottom-color fill
policy without discarding the configured background representation.
`set_layout_typography` updates both existing and future pane scales and requests
a full update. Price marks invalidate on font-size changes. Time marks key their
cache by font size, including callers outside ChartModel. Font family is carried
to the renderer but does not change the existing scale's approximate density
calculation. `apply_price_axis_options` safely patches a selected built-in axis,
recalculates and requests a full update; it exposes no mutable registry.

The preparation operation validates the pane **before** touching lazy caches.
It copies left, right and time marks, then refreshes the selected pane grid from
those same time marks. This is cache materialization, not a viewport/layout
mutation or invalidation consumption. UI `prepare_frame` calls it after actual
dimensions, deferred commands and animation sampling, and before copying line
and grid geometry. `PlotSnapshot.axes` is populated for model-prepared frames;
the raw command adapter leaves it absent. Canvas drawing does not read or draw
axis labels yet. New invalidation remains pending and retained masks are intact.

## Files and remaining dependencies

- `src/model/axis_snapshots.rs`: owned tick snapshots, metadata and typed errors.
- `src/model/mod.rs`: public module export.
- `src/model/chart_model.rs`: preparation and controlled option/typography updates.
- `src/model/pane.rs`: crate-private scale access and typography propagation.
- `src/model/price_scale.rs`: styling options/patches and font mark invalidation.
- `src/model/time_scale.rs`: font-aware lazy marks.
- `src/model/chart_model/tests.rs` and
  `src/model/chart_model/tests/axis_snapshots.rs`: connected tests.
- `src/ui/line_chart.rs`: shared settled-frame handoff, no drawing changes.

Next: backend text measurement and price-axis drawing/width measurement, then
time labels/overlap handling and axis-aware layout with retained-mask replay.
Floating series/price-line/crosshair labels, baselines, text metric corrections,
axis widgets, crosshair interaction and final dimensions remain deferred.

## Validation

- Focused axis coverage: 14 new tests pass (13 connected tests plus the explicit
  behavior-threshold test).
- `cargo test`: 230 library tests pass, no failures.
- `cargo test --all-targets --quiet`: 230 library tests and one example test pass.
- `cargo fmt --check`, `git diff --check`, `cargo check --all-targets`: pass.
- `cargo clippy --all-targets`: succeeds with five pre-existing test-code warnings
  (`format_date`, `get_series_plot_row_creator`, time behavior and two existing
  price-scale tests); none are introduced by this milestone.
- `cargo clippy --lib --examples -- -D warnings`: passes.

The original tick consumers were re-read against the final API: scale labels,
spacing, visibility, priority passes, gradient policy and retained replay data
are supported without widget identity or measurement. No visual axis comparison
is claimed: this milestone deliberately does not draw or measure axes. Existing
uncommitted line-rendering work is preserved; no changes were published.
