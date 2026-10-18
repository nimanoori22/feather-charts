# Horizontal-axis input and independent settings (milestone C)

Baseline: `2c42782`. Status: C0–C2 implemented; C3 demo and automated gates pass; available physical acceptance pending.

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

## C2 checkpoint

The 16-flag Canvas truth table first compiled and failed when drag was disabled but
still moved. It now matches all 16 frozen actual DOM gesture cases. Ten focused
public tests and the library/pane/event suites pass; strict all-target Clippy/check
pass. Atomic mixed-invalid patches preserve settings and captures. Disabling an axis
drag ends its owner; unrelated axis and plot gestures survive. Native price-axis
wheel remains independent. Both axes reuse the owned pointer helper and release-time
reset pairing. This intentionally corrects the former native price reset on second
press to source reset on second successful release. Touch direction/options, lost
finger cancellation, double taps without plot events and synthetic mouse suppression
are tested. Existing ClickSequence was reused without changing plot timing policy.

## Final consumer, capture protocol and migration

The `--axis-interaction` pane-management demo exposes all four flags, touch X/Y,
All off/All on and configured Defaults 9 / +3, with current spacing/offset readback.
It works in calendar, strike, maturity and sessions, retaining typed custom series,
pane controls, live feed, editable box and screenshot preview/export. Reset preserves
this demo mode. Axis flags alone leave wheel/pinch unchanged; All off explicitly
disables plot wheel/pressed movement and touch navigation as well.

The final capture protocol uses four domains × nine scenes × three sizes
(800×500, 300×300, 300×90) × four DPRs (1, 1.25, 1.5, 2) × two asserted renderers:
**864 screens and retained exports**. Scenes are baseline, all-off, stretched,
compressed, reset, drag-off/reset-on, drag-on/reset-off, pane move and touch reset.
Gestures are actual Canvas events returned to the public Chart owner, not direct
engine scale calls. Retained exports are rerendered after settings/default/size
changes and owner drop. The initial eight-scene output is retained as superseded;
final evidence uses the separate completion-captures directory. No old baseline or
threshold changes. Six unchanged compatibility protocols additionally cover events,
custom domains, screenshot/export, attribution, formatting and scale assignment.

C3 review also found the existing TimeScale.interactions_enabled cache policy had
no public input-settings connection. A source browser assertion and compiling native
red regression proved all-disabled edge labels failed to align. Public setters and
atomic patches now reuse that engine flag, including cache invalidation on reenabling
an axis, wheel or touch setting. The old demo reset regression now asserts Start on
second press and Reset on second release; its geometry/time/cancellation assertions
remain intact. Earlier failed logs and superseded captures are preserved.

Public migration: exhaustive ChartMessage matches add TimeAxis; the shared legacy
example adapter forwards it. Explicit ChartOptionsPatch/RuntimeChartOptions literals
gain axis_interaction; default-update patch literals remain compatible. Existing
PriceAxisInput.update callers retain their signature, with default-enabled gesture
options. Reset pairing intentionally moves from second press to successful second
release. Release/cancel clears owners; a reset-only press does not advertise resizing.

Rust files: src/ui/{axis_pointer,time_axis_input,price_axis_input,mod}.rs;
src/helpers/{click_sequence,mod}.rs (click helper moved from api);
src/api/{chart,configuration,interaction,widget,touch,touch_input,yield_curve,mod}.rs;
src/model/{chart_model,time_scale}.rs. Tests: tests/axis_interaction.rs and the
configuration unit regression; migrated examples/support/chart_demo.rs and its
existing tests. Demo: examples/pane_management.rs plus
pane_management/{axis_interaction,axis_interaction_captures}.rs. Source fixtures,
the source checker, capture runner, inventory, README and checklist accompany them.

Ownership remains synchronous and unchanged: host → Chart → model; Iced Canvas →
ChartState → axis pointer histories; borrowing ChartProgram → immutable prepared
frame; owned action → host update → Chart. No new dependency, threads, shared model
mutation or retained callbacks.

## Physical mouse procedure

Use the final normal live-feed example separately on tiny-skia and wgpu in all four
modes. At regular size:

1. Drag bottom labels left/right, including past the plot boundary, then release
   outside the chart. Spacing must change smoothly and stop on release. Double-click
   restores configured spacing 9 and offset +3 (subject to ordinary scale limits).
2. Toggle Time drag off, leaving Time reset on: dragging must stop, reset must work.
   Reverse the flags: dragging must work and reset must stop. Repeat for Price
   drag/reset on a populated axis. Price reset restores autoscaling. Use All on.
3. Use All off: plot wheel/pinch and plot/axis dragging/reset are disabled; hover
   and the existing native price-axis wheel extension still work.
   All on restores inputs. Individual axis toggles do not disable plot wheel zoom
   or the native price-axis wheel extension. Touch X/Y toggles do not disable mouse.
4. With live feed, test wheel zoom, diagonal plot drag, both populated price axes,
   horizontal-axis drag/reset and box move/resize/Escape at regular, narrow and the
   earlier problem size. Try pane move/swap and series transfer; then Reset. Shrink
   to short height, enlarge and check recovery. Check preview/export and close all
   four windows. Record the renderer and any concrete failing gesture/size.

Synthetic touch/DPR coverage does not close physical touchpad, touchscreen or
high-DPI acceptance. Older formatting GPU, timing and drawing-order exceptions
retain their own evidence. Broader helpers (D), pane sizing (E), metadata (F/G),
locale (H), and color policy (I) remain future milestones. The inventory also keeps
construction-only default-visible-scale configuration marked partial.

## Capture harness investigation

The first nine-scene capture failed the narrow 300×300, DPR 1.25 touch-reset
assertion while the full profiling suites were running. Its output is preserved in
`target/axis-interaction/accepted-captures`. An isolated real-Canvas reproduction
emitted Start/End/Start/Reset with roughly 10ms per event and unchanged axis geometry.
No engine defect was established from that capture failure. The fixture performed
an extra prepare after Chart::update (which already prepares); that duplicate work
was removed to match the normal demo. Final captures run after the full profiling
suite, preserving the source 500ms pairing rule and every scene/assertion. The
narrow fractional-DPR sequence also has a dedicated consumer test.

## Final lifecycle regressions

Two additional compiling red cases reproduced empty-axis reset staying at the old
spacing and a reset-only press surviving a DPR change. The visible empty horizontal
axis now recognizes reset without starting scaling, matching the actual source DOM
probe. Canvas samples DPR alongside bounds so reset-only captures and tap pairs are
cancelled when it changes; disabling their remaining capability also clears them.
All 13 focused public axis tests pass, including both controllers' reset-only
cancellation. Strict focused Clippy passes. These are lifecycle corrections inside
C; they do not constitute physical high-DPI or touchscreen acceptance.

The source DOM probe also asserts the numeric right-edge scaling formula using
the actual dispatched client coordinates, accounting for browser coordinate
quantization; the native Canvas regression independently asserts the same formula.

The unchanged legacy DPR/wheel regression caught overly broad idle cancellation.
DPR now invalidates pointer ownership and edit state without discarding the first
idle wheel event. That regression, all 13 axis tests and strict all-target Clippy
pass. Its original zoom assertion was retained.

## Final automated validation

On the final source: format check, all-target tests (**832 in 83 suites**), doctests
(**2**), all-target check, strict all-target Clippy, example builds, inventory drift
and diff whitespace checks pass. All **40** source comparisons pass, including the
unchanged 39 earlier scripts. Both strict renderers pass **864** exact screen replays
and retained exports. Representative captures were visually inspected across all
four domains. All **4326** accepted image hashes are unchanged across six prior
protocols (events 912, custom horizontal 240, screenshot 1158, attribution 672,
formatting 576, scale assignment 768). No old scene or threshold was weakened.

The [pinned evidence record](axis-interaction-evidence.json) records binary,
source/protocol/result and log hashes. The capture protocol's production/demo hashes
match the current files and tested normal binary. Core checkpoints: `c5b8ce5`,
`846497c`, `f757d68`, `d38d521`. Mouse acceptance remains pending on each renderer;
earlier timing and device exceptions retain their own status.
