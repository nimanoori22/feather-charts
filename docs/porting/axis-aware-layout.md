# Measured axis-aware layout

## Scope

One pane, independently visible left/right price axes and a shared time axis.
The frame owner reserves measured space, updates the model to the allocated plot,
and publishes owned line, grid, axis, and corner geometry from one settled state.
Drawing neither measures nor mutates the model. Floating series/crosshair labels,
multiple panes, gestures, and geometry/measurement caching remain deferred.

## Source investigation and consumers

The behavioral references are:

- `lightweight-charts/src/gui/chart-widget.ts`: `resize`, `_adjustSizeImpl`,
  `_drawImpl`, `_applyMomentaryAutoScale`, and `_applyTimeScaleInvalidations`.
- `gui/price-axis-widget.ts`: `optimalWidth` and `_onMarksChanged`.
- `gui/time-axis-widget.ts`: `optimalHeight`, `setSizes`, and `_recreateStubs`.
- `gui/price-axis-stub.ts`: corner background and border drawing.
- `gui/internal-layout-sizes-hints.ts`: chart-size and axis-size rounding.
- `model/invalidate-mask.ts`: directional merge and ordered deferred commands.

The graph was used to trace drawing and mark-change consumers; material source
and the existing Rust frame/renderer implementations were also read directly.
Coverage checks reported no recorded issues for the material indexed paths;
this remains best-effort metadata, not a completeness guarantee.

ChartWidget owns layout and drives pane/time-axis dimensions. Full invalidation
adjusts GUI layout before model commands. Light/full drawing applies momentary
autoscaling, then ordered time-scale commands, then updates scale marks.
PriceAxisWidget can request a full update when newly generated marks require a
larger width. Shrinking marks alone do not request a smaller axis: doing so could
cycle between label sets. Full updates and outer resizing can remeasure widths.

The source detects a pending full update during drawing, merges the retained
mask into it, adjusts layout, and replays autoscale/time-scale commands at the
same timestamp. Fit-content and logical-range commands must therefore remain
available after their first application. Data updates are not replayed.

## Required contract and deliberate boundaries

- Chart dimensions are floored and rounded down to even logical sizes.
- Existing measurement outputs already include axis minimum sizes and size
  hints. Allocation must not apply those hints twice: a fractional minimum of
  81.5 can become 83 under the source hint, and must not become 84 here.
- Reserve independent left/right widths and time height. Plot width is clamped
  to zero; the source's last pane rounds height upward at the device ratio and
  retains a minimum height of two logical pixels.
- Undersized nonempty containers retain requested axis sizes and the minimum
  pane; the root clips overflow. Axes are not proportionally shrunk.
- Explicit zero-sized layouts suppress all regions. This is an intentional
  Rust startup/minimization extension to the DOM source's minimum-pane behavior.
- Price axes share the plot height. Time-axis width and origin match the plot.
  Corners occupy the regions directly beneath price axes.
- Original commands are borrowed and replayed in order after a changed layout.
  There is no second invalidation accumulator or independent mark generator.
- In accordance with this task, newly generated work remains pending for the
  next operation. Unlike ChartWidget's immediate consumption of a newly pending
  full mask, this owner replays only the original detached mask while handling
  measured size corrections locally.
- An animation starts once and is sampled once per frame timestamp. Replays
  reuse its sampled offset and do not install another continuation. Manual
  navigation still cancels continuation through existing mask/controller rules.

Ordinary-tick measurements intentionally exclude the source's floating-label
and crosshair width contributions; this continues the previous axis milestones.

## Design, alternatives, and ownership

Two viable designs were considered: application-managed rows/columns of separate
canvases, and a single chart region that allocates all rectangles together. The
single Canvas was chosen: its actual available bounds are unambiguous, all
regions share one layout, and bounds reporting cannot race separate widgets.
Allocation remains a pure backend-neutral function. Backend text metrics enter
through `AxisMeasurer`; the Iced adapter supplies matching normal/bold fonts.

```text
Demo application
  owns DataLayer, ChartModel, animation controller, LinePaneView
  owns ChartFrameOwner
    owns prior allocation and retained original mask
  owns final IcedChartFrame
    owns plot, glyph paths, axis/corner commands, region rectangles

Canvas borrows final frame for drawing only
```

Preparation borrows model/controller/view/measurement backend for one operation.
Returned snapshots have no model references or callbacks. No new Rc cycles,
threads, asynchronous engine work, subscriptions, or ownership back-links were
introduced. TypeScript DOM sizing, inheritance, callback identity, and mutable
renderer sharing are replaced with explicit rectangles and owned frame data.

## API and settlement

`ui::chart_layout` provides `AxisSizeRequests`, `ChartLayout`,
`ChartLayoutError`, and `allocate_chart_layout(bounds, requests, pixel_ratio)`.
Required requests are separate from allocated rectangles.

`ui::chart_frame` provides `AxisMeasurer`, `AxisMeasurements`,
`ChartFrameInput`, `ChartFrameOwner::prepare`, `ChartFrameSnapshot`, and the
read-only drawing adapter `IcedChartFrame`. `with_pass_limit` controls the
nonzero settlement budget; the default is eight passes. Only one pane is
currently accepted, with typed errors for invalid input or unsupported layouts.

Preparation proceeds as follows:

1. Validate bounds, ratio, pane count, and selected series; clear view output.
2. Obtain provisional axis measurements at the previous dimensions. This
   preflight occurs before detaching pending work; no plot dimensions change.
3. Convert new animation descriptors once and take pending invalidation once.
4. On a full update or changed outer bounds/ratio, allow fresh axis sizes;
   otherwise retain prior allocation and grow it if needed. Hidden axes release
   their slots.
5. Allocate rectangles and set the actual model plot width/height.
6. Apply the retained mask and sample animation on the first pass. Later
   corrections replay the same mask and reuse the sampled offset.
7. Refresh axis snapshots and measurements. Grow allocations if necessary;
   never shrink within the settlement loop.
8. Once stable, prepare final line/grid/axes/corners and publish them together.

Growth-only correction prevents oscillation between smaller/larger label sets.
Convergence means no measured request exceeds its current allocation. A smaller
request can be reported separately without shrinking the allocated region.
If the pass budget is exhausted, `LayoutDidNotConverge` is returned and no
intermediate frame is published. This is not a transactional model rollback:
the model remains at the last attempted dimensions, the original mask remains
inspectable, and newly generated work remains pending. Callers should handle the
error and request another full update or correct the measurement backend.

The existing synchronous `prepare_frame` API remains available. Its new
`prepare_settled_plot` helper validates that supplied dimensions equal the model
and prepares geometry without taking work, resizing, or advancing animation.

## Coordinate and corner contracts

Allocation and measurements use logical pixels. Plot dimensions are explicitly
canonicalized through f32 once before updating the model, matching Iced geometry
exactly; all neighboring region boundaries are adjusted consistently. Existing
grid commands retain their bitmap contract and are converted once by the plot
adapter. Border/tick snapping remains in the existing axis renderers.

Iced 0.14's WGPU and TinySkia clipped drafts do not inherit frame transforms.
Each region is clipped in chart coordinates and explicitly translated to its
origin inside the draft. Plot drawing avoids another nested clip that would
discard that translation. Glyph paths were prepared outside drawing.

The source PriceAxisStub fills each corner with the background's **bottom
color**, rather than repeating a vertical gradient. Its border is a small
plot-facing top-corner rectangle, snapped with floor at each pixel ratio, using
the time-axis border color. The source passes the same left-price-scale border
visibility getter to both stubs; the Rust implementation intentionally preserves
that asymmetry instead of substituting the right scale's visibility.

## Changed files

- `src/ui/chart_layout.rs`: pure allocation, growth policy, and geometry tests.
- `src/ui/chart_frame.rs`: measured settlement, replay, corners, final frame.
- `src/ui/mod.rs`: exports.
- `src/ui/line_chart.rs`: settled plot preparation and translated region drawing.
- `src/ui/price_axis.rs`: reusable clipped region drawing.
- `src/ui/time_axis.rs`: matching font access and region drawing.
- `src/views/time_scale_animation.rs`: separate detach/apply/sample operations.
- `examples/line_chart.rs`: actual chart bounds and measured Canvas regions;
  removes fixed 120-pixel price and 40-pixel time slots.
- `src/model/chart_model/tests.rs` and `tests/chart_layout.rs`: connected tests.
- `examples/chart_layout_reference.rs` and
  `scripts/check_chart_layout_reference.cjs`: allocator/source differential oracle.
- `docs/porting/axis-aware-layout.md`: this report.

## Tests and validation

Thirteen new tests cover visibility combinations, shared boundaries, fractional
ratios/minima, zero/undersized bounds, growth/shrink policy, fit/range command
replay, normalized command order, newly pending work, one animation sample across
multiple passes, navigation cancellation, oscillating measurements, pass-limit failure, historical
appends, removal/reload, immutable frames, and corner policies. Connected tests
use real DataLayer, ChartModel, scales, and series with deterministic metrics.

The differential oracle executes the original ChartWidget `_adjustSizeImpl`
and size hints from the neighboring TypeScript checkout. All 192 cases match
for axis combinations, fractional minima, undersized containers, and ratios
1, 1.25, 1.5, and 2.

Validation completed:

- `cargo fmt --check`: passed.
- `cargo test --all-targets`: 272 library tests and one connected demo test passed.
- `cargo check --all-targets`: passed.
- `cargo clippy --all-targets`: passed with five pre-existing test warnings.
- `cargo clippy --lib --examples -- -D warnings`: passed.
- `node scripts/check_chart_layout_reference.cjs`: 192 cases passed.
- Demo smoke runs: default and gradient/left-axis/ticks/fixed-edge scenes passed.
- GUI launch and window capture: inspected the default chart's measured regions,
  aligned series/grid/ticks, and nonoverlapping plot/right/time placement.

Not all requested visual scenes were manually inspected. Fractional scaling,
animation/replay, minima, and cancellation have deterministic test coverage;
broader real-device visual comparison remains step 5. Multi-pane shared widths,
floating-label sizing, and full axis interaction remain deferred.
