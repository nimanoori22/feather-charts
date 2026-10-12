# Price-axis interaction

## Source, consumers and behavioral contract

Read the complete Lightweight Charts `gui/price-axis-widget.ts`, its input
callbacks and `model/chart-model.ts`/`pane.ts` consumers, and the existing
`PriceScale.startScale/scaleTo/endScale` algorithm. PriceAxisWidget owns the
pressed gesture and routes pane-local Y coordinates to one price scale.
ChartModel refreshes the crosshair and invalidates rendering after range changes.
Manual scaling turns autoscale off and scales the original range around its
center; empty, percentage and indexed scales reject scaling. Axis double-click
restores autoscale. Left/right axes and panes must remain independent; time
coordinates must not change. Graph coverage is best-effort and direct source
reads establish these semantics.

The local Lightweight Charts wheel handler zooms time globally; it does not
provide wheel price scaling. The user explicitly requests both wheel and drag
on the price axis, so wheel scaling is a deliberate extension. Normalize native
line/pixel deltas using the existing plot convention, bound each event, and use
an exponential factor (positive delta narrows the range) around the price-range
center. Reuse the source scale algorithm rather than duplicating range mutation.

## Alternatives and chosen Rust design

1. Add axis hit testing and price actions to every plot/divider input state.
2. Add one reusable `PriceAxisInput` in front of existing plot routing, with
   exclusive gesture ownership and typed pane/side/action messages.

Choose 2: single- and multi-pane demos share one implementation; existing plot
coalescing and divider logic remain intact. Plot/divider drags keep ownership
when crossing an axis, and an axis drag keeps ownership outside its hit region.
Focus loss, leaving the window, disabling a scale, pane membership changes and
height/window changes end the gesture. Axis width changes caused by new labels
must not cancel the scale's own drag. Stationary movement is suppressed.

Application owns model and Canvas state; Canvas state owns PriceAxisInput,
which owns only coordinates, pane index and axis side. No borrowed model state,
callbacks, Rc cycles, thread-safe state or asynchronous machinery is introduced.
The application ends active model gestures before data/pane lifecycle changes.
TS DOM handlers, CSS cursor state, mutable object identity and manual destruction
are replaced by typed input, Iced mouse interaction and ordinary owned state.

Proposed API:

```rust
pub enum PriceAxisAction {
    Start { y: f64 }, Move { y: f64 }, End, Wheel { delta: f64 }, Reset,
}
pub struct PriceAxisInteraction { pub pane: usize, pub side: PriceAxisSide,
                                  pub action: PriceAxisAction }
// PriceAxisInput::update(event, cursor, layouts, enabled_axes) -> Option<...>
// ChartModel::{start_scale_price, scale_price_to, end_scale_price,
//              zoom_price_scale, reset_price_scale}(pane, side, ...)
```

Model methods validate pane/finite input, borrow the selected existing scale,
consume pane effects and refresh crosshair through the established synchronous
model lifecycle. All four demos use the same Canvas path. Synthetic native-event
regressions cover the reported gap and gesture/cancellation/lifecycle behavior.

## Implementation and validation

Implemented in `src/ui/price_axis_input.rs` (exported from `src/ui/mod.rs`),
`src/ui/chart_input.rs` (gesture ownership queries), `src/model/chart_model.rs`
(typed model operations), `src/model/chart_model/tests/price_interaction.rs`
(registered in `tests.rs`), and `examples/support/chart_demo.rs`/`tests.rs`.
No dependencies or rendering primitives were added. The exact Iced Canvas
Program and advanced mouse Click implementation were inspected; Iced provides
native cursor selection and click recognition. Flowsurface's typed price-axis
scale was inspected as context; its separate camera architecture was not copied.

The first native-event regression failed with “wheel over the price axis must
publish a scale action” before routing was added, then passed. Four input tests
cover axis ownership outside the hit region, stationary replay suppression,
label-width changes, layout/focus/disabled cancellation, native line/pixel wheel
normalization, hidden/outside/invalid inputs and double-click reset. Three model
tests cover source snapshot coefficients, range center and time preservation,
end/reset semantics, reversible exponential wheel scaling, existing gesture
protection, unsupported scale modes and finite/pane validation. Four shared demo
tests exercise all four series demos, pane/side isolation, drag/reset/focus,
plot drag ownership, series removal/reload and redraw while axis labels grow.

Final commands: `cargo fmt --check`, `cargo test --all-targets --quiet`,
`cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings` and
`cargo build --examples`. All pass: 325 library tests and 13 tests in each of
four shared demo targets (377 test executions). Existing actual-source scripts
`check_interaction_reference.cjs` (131 cases) and
`check_multi_pane_interaction_reference.cjs` (112 cases) pass. The new drag model
assertions use the original source coefficient; wheel is intentionally an
extension and has no original-source wheel-price oracle. No new raster captures
or latency benchmarks were needed: existing price-coordinate rendering is reused.

Run `cargo run --example price_volume_chart` (or line/candlestick/histogram).
Hover a visible, populated left/right price scale and wheel to stretch/compress;
click and drag up/down for source-compatible scaling. Positive native wheel
input and an upward drag narrow the range. Double-click the axis to restore
autoscale. Manual scaling persists through time navigation and data updates
until reset. Percentage/indexed scales remain non-interactive like the source.

Physical mouse/touchpad feel and sensitivity remain unverified on real devices;
confirm both gestures and reset in the updated demos. Free dragging inside the plot is now implemented; see
[plot panning](plot-panning.md). Touch/kinetic input and the general public
chart/widget facade remain subsequent capabilities. Earlier raster/performance reports are unchanged.
