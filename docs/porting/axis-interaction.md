# Horizontal-axis input and independent settings (milestone C)

Baseline: `2c42782`. Status: C0 investigation and red regressions.

## Source and consumers

The complete TimeAxisWidget and MouseEventHandler were inspected, including canvas
ownership, mouse/touch callbacks, double-click release timing and destruction.
PriceAxisWidget's construction and input/reset path share those handlers.
ChartModel delegates start/move/end to TimeScale; movement recalculates panes.
TimeScale.scaleTo uses initial spacing × clamped distance from right / initial
distance from right. Zero distances do nothing. Reset restores configured spacing
and rightOffsetPixels/spacing (or rightOffset), subject to normal range constraints.

ChartApi normalizes JS boolean shorthand to four independent flags, all true by
default. The handle-scale coverage consumer reenables only axis dragging after
disabling scaling. The reset-after-set-visible-range graphics consumer requires
configured defaults rather than fit-content. Axis double clicks reset the scale
without emitting plot double-click events. MouseEventHandler permits the first
touch only on axes, uses a 5px Manhattan drag threshold and a 500ms release pair
(5px mouse / 30px touch). Horizontal-axis touch drag requires horizontal touch
navigation and horizontal intent; price-axis touch requires vertical navigation.

Graph discovery/traces were supplemented by actual source usages because callback
and model edges are sparse. TimeScale metadata differs from its index; current
source was read. The handle-scale coverage subtree is excluded and was read directly.
Exact Iced Canvas forwards events with bounds/cursor to Program and publishes owned
actions synchronously. Flowsurface confirms state-owned capture/release patterns;
its zoom algorithm is not the behavior specification.

## Required behavior and native boundaries

- Drag only the horizontal axis proper, excluding side stubs/plot/separators; use
  axis-local logical coordinates, independent of device pixel ratio or chart domain.
- Keep one gesture owner, emit start/move/end/reset, retain the initial anchor and
  ignore identical movement/redraw. Release outside or focus loss terminates capture.
- Hidden/empty axes do not start a drag. Resize/DPR, pane changes and viewport
  mutations end stale captures. Successful runtime disabling cancels only the
  affected axis capture; unrelated axis/plot/edit captures remain intact.
- Time/price drag/reset flags are independent. Existing plot wheel, pinch, price-axis
  wheel and native manual vertical plot panning remain unchanged.
- Touch axis taps reset under their respective flags, never becoming duplicate mouse
  actions or plot click events. Physical touch acceptance remains separate.
- Invalid mixed patches fail before settings/capture changes; equal patches are inert.

DOM nodes, browser listeners/timers, callback identity and JS boolean-or-object
shorthand are incidental. Rust owns typed flags, synchronous input state and value
messages. Cancellation on native focus/layout change is deliberate stale-state
protection; no browser plumbing or asynchronous engine path is introduced.

## Design alternatives and ownership

1. Add a dedicated horizontal-axis controller beside PriceAxisInput, using the
   existing model scaling algorithm and Chart mutation owner. Chosen: small change,
   preserves plot/edit/price ownership and existing public controller callers.
2. Fold every plot/axis/touch controller into one generic gesture machine. Rejected:
   unnecessarily rewrites accepted input paths and increases regression scope.

Chart owns model gesture state and settings. Canvas ChartState owns event recognition
and click/touch history. A borrowing ChartProgram samples settings/current owner;
owned messages return to Chart::update. No new Rc, callbacks, cycles or cross-thread
state. Retained frames/exports are unaffected by later input/option changes.

## Proposed public API and consumer mapping

```rust
pub struct AxisInteractionOptions {
    pub time_drag: bool,
    pub time_reset: bool,
    pub price_drag: bool,
    pub price_reset: bool,
}
// All true by default. Included in ChartOptionsPatch and RuntimeChartOptions.
impl<B: ChartBehavior> Chart<B> {
    pub fn axis_interaction_options(&self) -> AxisInteractionOptions;
    pub fn set_axis_interaction_options(&mut self, options: AxisInteractionOptions);
}
```

Applications use a typed complete settings value or the existing atomic chart patch;
no ambiguous JS union shorthand. YieldCurve forwards mutation like existing options.
Canvas sends horizontal-axis actions through ChartMessage alongside PriceAxis.
All four horizontal domains share the same logical-coordinate path. Demo buttons
exercise the public settings, actual gestures and export. Existing programmatic
viewport helpers keep their configured behavior and cancel live captures.

## Verification planned before implementation

Freeze actual JS DOM mouse gestures and the 16 flag combinations; require a compiling
real-Canvas horizontal drag regression to fail before production edits. Add reset,
stub/empty/hidden, outside release, focus/resize/DPR/membership, runtime cancellation,
touch-direction/double-tap/synthesis and edit/divider priority checks. Run existing
source, renderer and public consumer compatibility gates without weakening them.

## C1 checkpoint

Two compiling Canvas regressions reproduced unchanged drag spacing and failed reset.
Both now pass, with five focused tests covering anchors, release outside,
hidden/empty/stub targets, seven cancellation causes, runtime configured defaults
and pixel-offset policy. Existing configuration, pane input and public event suites
pass. Strict focused Clippy and all-target check pass. Reset also exposed two proven
engine errors: restore_default used global defaults and discarded rightOffsetPixels;
apply_options did not retain configured bar_spacing. Both now follow current source.
Release pairing was moved unchanged into helpers for reuse by plot/touch/axis inputs.
The shared axis pointer recognizer will also serve price-axis settings in C2.
