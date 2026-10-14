# Touch and kinetic navigation: source contract

Date: 2026-10-14. Touch and kinetic implementation is complete; deterministic
and renderer acceptance is recorded below. Physical acceptance remains pending.

Upstream `src/gui/mouse-event-handler.ts` distinguishes native touch from mouse
events. It tracks the first finger by identity, cancels taps once Manhattan
movement reaches five logical pixels, starts long-press tracking after 240 ms,
and suppresses synthetic mouse input for 500 ms after touch. Pinch requires two
fingers before one-finger movement or tracking has prevented it. Its anchor is
the initial midpoint, not the moving midpoint. Scale is distance divided by
initial distance; PaneWidget applies `(scale - previous_scale) * 5` as time zoom.

For initial touch drag arbitration, vertical movement has priority: horizontal
distance is halved before comparison. Disabling a touch direction delegates a
gesture in that direction instead of starting chart navigation. A native host
must leave uncaptured disabled gestures available to its enclosing UI.

`src/gui/pane-widget.ts` delegates recognized scrolling to model time and price
navigation. Tracking uses initial applied crosshair position plus finger delta,
clamped into the owning plot. OnNextTap keeps tracking after release and exits
on a subsequent unmoved touch. OnTouchEnd exits on release. Mouse input exits
tracking. Starting a press or pinch stops time animation.

Source defaults enable horizontal/vertical touch drag and pinch. Kinetic touch
scrolling is enabled and kinetic mouse scrolling disabled. Tracking exit defaults
to OnNextTap. Source inertia changes only time offset; it does not add vertical
price inertia.

## Kinetic algorithm

Read the complete `src/model/kinetic-animation.ts`. Retain up to four ordered
positions. Equal-time input replaces the most recent position; movement below
the sampling threshold is ignored. Start requires two samples and the most
recent sample within 50 ms. Segment speeds are capped, weighted by signed
distance and included only while direction agrees with the newest segment.
Reject a start below minimum speed. The decay coefficient is 0.997 per ms.
Source pane sampling uses minimum speed 0.2, maximum speed 7 and minimum move
15, each divided by current bar spacing because animation positions are logical
right offsets. Duration stops when remaining decay distance reaches one logical
offset unit. Keep the source units rather than silently changing this to pixels.

An idiomatic Rust sampler can own fixed-size sample state and return an immutable
started animation; it need not reproduce nullable JavaScript fields or read a
clock internally. Compare that against a mutable object containing sampling and
running state. Prefer the former because the existing `TimeScaleAnimation`
interface already accepts immutable samples at supplied Instants. No async timer,
channel or new animation owner is required.

## Native delivery and ownership

Pinned Iced `core/src/touch.rs` supplies FingerPressed/Moved/Lifted/Lost with
stable `Finger(u64)` identifiers. `winit/src/conversion.rs::touch_event` converts
physical positions into window logical coordinates using the scale factor and
maps native cancellation to FingerLost. Canvas reexports the core Event and
passes it unchanged to Program::update; subtract Canvas bounds and owning plot
origin, as for native mouse coordinates. A touchscreen is required to verify
the actual device's event delivery. A touchpad wheel stream is a different input.

```text
Canvas state -> synchronous touch recognizer (IDs, plot, deadline)
published gesture -> Chart::update -> existing model navigation
completed sampler -> AnimationRequest -> existing frame controller
prepared frame -> immutable geometry
```

The recognizer does not borrow Chart across callbacks. Tick/redraw scheduling
checks the long-press deadline and existing animation; idle state requests no
timer loop. Gesture recognition should use explicit Idle/Pressed/Pan/Pinch/
Tracking states rather than independent contradictory flags. Existing primitive
mouse capture takes precedence. Finger loss, focus loss, incompatible layout,
owner removal and explicit navigation cancel gestures and animation. Additional
finger and pinch-to-one-finger behavior must be fixed and tested before coding;
native cancellation must not invent a click or fresh pan.

## Public contract and transitions

Keep navigation options separate from the widget's transient recognizer:

```rust,ignore
pub struct TouchNavigationOptions {
    pub horizontal_drag: bool,
    pub vertical_drag: bool,
    pub pinch: bool,
    pub tracking_exit: TrackingExitMode,
}

pub enum TrackingExitMode { OnNextTap, OnTouchEnd }
pub struct KineticScrollOptions { pub mouse: bool, pub touch: bool }

pub struct TouchInteraction {
    pub pane: usize,
    pub now: Instant,
    pub action: TouchAction,
}
pub enum TouchAction {
    Press,
    Pan { start: Option<Point>, position: Point },
    Pinch { anchor: Point, zoom: f64 },
    Track { position: Point },
    Tap { position: Point },
    End,
    ExitTracking,
    Cancel,
}

pub struct KineticSampler { /* up to four owned samples */ }
impl KineticSampler {
    pub fn new(min_speed: f64, max_speed: f64, decay: f64, min_move: f64) -> Option<Self>;
    pub fn add_position(&mut self, position: f64, now: Instant);
    pub fn start(&self, position: f64, now: Instant) -> Option<KineticAnimation>;
}

impl Chart {
    pub fn set_touch_navigation_options(&mut self, options: TouchNavigationOptions);
    pub fn set_kinetic_scroll_options(&mut self, options: KineticScrollOptions);
}
```

These names express Rust composition rather than reproducing nested JavaScript
option objects. The Canvas recognizer publishes a typed gesture containing its
owning pane, logical point and supplied Instant. Chart applies recognized pan,
pinch, tracking and tap synchronously through existing model operations. Raw
finger bookkeeping stays with Canvas state. Compare recognition in Chart with
recognition in Canvas: Canvas already owns capture, bounds and redraw deadlines,
so moving it into Chart would duplicate those responsibilities across owners.

| State and event | Required transition |
| --- | --- |
| Idle, first press in plot | Pressed; remember primary ID, position and long-press deadline; stop animation |
| Pressed, movement | Cancel tap at five-pixel Manhattan threshold; choose chart Pan or delegated gesture using initial direction |
| Pressed, deadline with one unmoved finger | Tracking; use the model's applied crosshair as the tracking origin |
| Pressed, second finger before movement | Pinch; freeze midpoint and initial distance; cancel tap |
| Pan, movement | Continue both time and price navigation; retain primary ID |
| Pinch, two-finger movement | Apply change in distance ratio around initial midpoint |
| Pinch, third finger | Stop pinch; returning to exactly two fingers may start a new pinch with a fresh anchor |
| Primary lifted while another remains | End primary gesture; do not adopt the other ID or invent a fresh pan |
| Tracking, primary movement | Apply finger delta to initial applied crosshair and clamp into owner plot |
| Tracking, lift | Keep tracking in OnNextTap; exit in OnTouchEnd |
| Persistent tracking, subsequent unmoved tap | Exit tracking without starting a pan |
| Any active state, loss/focus/layout/owner cancellation | End capture and animation; emit no click |

Pinch eligibility is separate from adopting a primary finger. The source target
touchend listener checks the remaining pair before the root listener ends the
primary gesture: a three-to-two transition can therefore begin a fresh pinch
even when the lifted finger was the primary. Preserve that eligible pair behavior
without allowing the remaining fingers to start a new one-finger pan. Primary
movement while a third finger has suspended pinch prevents a later pinch in
that gesture, as in the source. FingerLost is native cancellation and ends the
gesture rather than following the ordinary lift/pair transition.

Upstream touch tap emits the click delegate without changing the crosshair.
Ordinary touch pan likewise does not act like mouse movement. Tracking explicitly
sets the crosshair. Preserve that distinction instead of routing tap through the
existing mouse-click branch. Native pinch cancels tap explicitly to prevent a
second click after zoom; document this guard where upstream browser delivery
otherwise allows its tap bookkeeping to survive pinch.

Iced has no browser sourceCapabilities provenance for synthesized mouse input.
Use the source's 500 ms suppression interval for native mouse button/movement
events after touch, also suppressing pointer events while a finger remains held,
while retaining independent wheel events. Mouse interaction
exits persistent tracking. Existing mouse primitive/axis capture takes priority;
touch navigation does not become annotation editing in this milestone.

Action publication already requests a redraw. On that redraw, request the next
long-press deadline using the pinned Canvas Action::request_redraw_at. A due
deadline publishes tracking; future animation ticks use the existing controller.
Schedule the earlier pending deadline and request no tick when both are idle.
An explicit viewport interaction epoch invalidates recognizer state on options,
programmatic navigation, owner membership or incompatible layout changes.

The kinetic sampler consumes actual clamped model right offsets after navigation,
not unbounded pointer deltas. A completed sampler returns the existing immutable
TimeScaleAnimation request. One animation controller owns sampling and stopping;
press, pinch, wheel, axes, editing, explicit navigation and cancellation all stop
it. Sampling past the estimated duration still evaluates the source decay at
actual elapsed time; duration determines completion rather than clamping the
sample. Reject nonfinite values, invalid coefficients and backward times before
they enter model state.

Raw press publication is provisional and uncaptured, allowing enclosing UI
scrolling when its direction is disabled. A dedicated touch interaction epoch
invalidates gestures for explicit navigation/options/layout changes; ordinary
data revisions do not invalidate an ongoing native gesture. Stable captured
plot origins avoid jumping when price-label width changes during navigation.
Touch pan follows the source's delayed scroll start: its first accepted movement
records a position; the next changed position starts time/price scrolling at that
position. Tap and tracking remain independent from ordinary mouse crosshair motion.
Mouse pan retains the existing native threshold behavior.

## Required deterministic and physical evidence

Tests cover stable IDs, taps versus drags, disabled direction arbitration,
owning-pane coordinates, moves outside the plot, pinch anchor and finger
transitions, third fingers, loss, nonfinite/zero-distance input, tracking deadlines
and exit modes, synthetic mouse suppression, and all interruption paths. Compare
kinetic sampling/decay directly against upstream source, including reversals and
same-time samples. Renderer regressions must retain ordinary wheel, axis scaling,
primitive editing and custom pointer behavior.

Physical acceptance remains pending for touchscreen pan/tap, pinch, long press,
tracking, flick decay, interruptions and mouse/touchpad use afterward, at narrow
window sizes and high DPI on each available backend. This document is not evidence
that a native gesture has run successfully.

## Implementation and validation

`src/api/touch.rs` defines options and typed messages; `src/api/touch_input.rs`
owns native finger recognition. `src/api/widget.rs` adds recognition, coordinate
conversion, capture, interruption and deadline scheduling. `src/api/interaction.rs`
applies gestures, sampling and cancellation through the existing model. Chart
fields/setters are in `src/api/chart.rs`; exports are in `src/api/mod.rs`.
`src/model/kinetic_animation.rs` owns the sampler/immutable curve and is exported
from `src/model/mod.rs`. `src/ui/chart_input.rs` publishes a stop-animation action
on mouse press. `examples/support/chart_demo.rs` forwards the new touch message
so the older Line/Candlestick/Histogram/PriceVolume demos use the same path.

The engine still has one synchronous animation controller, no async timer,
subscription, channel, shared lock or new runtime dependency. Browser listener
identity, nullable mutable animation state and manual destruction are discarded;
owned Canvas state and immutable animation requests provide the needed lifecycle.
Native cancellation emits no click; zero-distance pinch and invalid numbers do
not enter model navigation. Native axis touches and annotation editing are outside
this pane-navigation milestone. Touch scroll flags control initial delegation;
once accepted, a gesture can pan both axes, matching the source.

Behavioral coverage lives in `tests/touch_navigation.rs`,
`tests/kinetic_navigation.rs`, recognizer unit tests and
`src/api/interaction/tests.rs`. It includes delayed pan start, pane ownership and
outside moves, tap/crosshair separation, stable IDs, finger loss/unknown IDs,
pinch anchor/third-finger/return-to-one transitions, zero distance/NaN, tracking
deadlines/applied snapped origin/clamping/exit modes, mouse suppression, options,
live updates, mouse inertia opt-in, release decay/completion and ten interruption
paths. Supplied Instants make velocity tests independent of cold font preparation
or wall-clock scheduling. Idle redraw has no request after settling.

`examples/{kinetic,touch}_reference.rs` and matching check scripts execute actual
upstream methods: 175 kinetic start/decay/completion cases and 52 native direction/
pinch cases pass. All 19 source-reference scripts pass. Coverage is best-effort;
the graph reports changed metadata for upstream TimeScale/TickMarks, so current
source was read directly. Dynamic callers absent from graph results were checked
in axis-widget and Rust model/grid consumers rather than treated as unused.

`examples/custom_series/capture.rs` adds seven native touch scenarios to the
existing matrix. Both TinySkia and WGPU pass 120 captures at ratios 1/1.25/1.5/2,
including pan, pinch, tracking, lost fingers, inertia, narrow layouts and a second
pane. They dispatch native Canvas events and verify model state, repeatable pixels
and the mixed custom/built-in scene. Offscreen event replay cannot establish native
hardware smoothness. A deliberate change of pinch multiplier from 5 to 4 caused
the anchor/zoom test to fail; restoring 5 returned it to green.

Final interruption review also guards wheel input outside the canvas: ignored
wheel events must leave native finger capture intact. Recognizer cancellation
now accompanies an accepted mouse/axis/edit action, not every raw mouse event.
The regression test continues the same finger pan after an ignored wheel event
and verifies clean loss cancellation.
Native resize is processed before a due long-press deadline, so an old plot
cannot briefly enter tracking while new bounds are awaiting model preparation.

Final full validation passes formatting, all-target tests (355 library tests and
163 integration/example tests), cargo check, strict all-target Clippy and example
builds. The final backend matrix passes 228 built-in captures, 56 hover captures,
20 editing captures and 120 custom/touch captures per backend (848 total).
Artifacts/logs are in ignored `target/final-roadmap-captures/`; the touch contact
sheet was inspected and both backends retain the same chart geometry and labels.

## Animated time-label preparation

The first isolated inertia profile exposed a history-sized copy in
`TimeScale::marks`: cached global tick selection was cloned before filtering.
Upstream selects ticks globally by weight, then formats only the inclusive visible
interval. Axis/grid consumers require that selection, label order, coordinates
and edge alignment, not an offscreen payload copy. The current source's sorted
selection permits two partition searches and cloning only the visible slice.

Compare this with selecting ticks anew for each viewport: the latter changes
neighbor/weight decisions and cache ownership. Retaining the existing global
selection is the smaller behavior-preserving design. A more extensive split of
formatter and cache borrows could avoid every visible clone, but adds unnecessary
API/lifetime changes for a small bounded list. `src/model/time_scale.rs` keeps its
public signatures and ownership unchanged.

`tests/time_mark_preparation.rs` drives a real TimeScale with counted custom time
payloads. Moving the viewport initially failed with 220 copies at 1,200 rows;
after the change, 1,200 and 100,000 rows each copy at most 40 visible time payloads.
Cold tick selection or changed spacing/membership can still rebuild its global
cache; this optimization addresses repeated movement at unchanged spacing.

## CPU navigation measurements

`cargo run --release --example touch_navigation_profile` dispatches native Canvas
press/move/release and redraw events through ChartState/Chart::update. Each case
warms 20 ticks and measures 100 inertia ticks, including model, geometry, layout
and glyph preparation, at 800×500 and ratio 1.25. Cold tick-selection construction,
gesture setup and restart are outside the timed tick. Data and latest value are
fixed across history sizes. Runs are isolated from Cargo and captures.

| Run | Sources | 1,200 median/p95 ms | 20,000 median/p95 ms | 100,000 median/p95 ms |
| --- | --- | --- | --- | --- |
| 4 | 1 | .328971/.589134 | .328134/.589698 | .328878/.588736 |
| 5 | 1 | .341639/.604247 | .343329/.603005 | .339417/.587420 |
| 6 | 1 | .316546/.583825 | .316718/.586261 | .314018/.574436 |
| 4 | 16 | 7.919660/9.574993 | — | 7.690282/8.737191 |
| 5 | 16 | 7.841012/8.921194 | — | 7.872611/8.835589 |
| 6 | 16 | 7.581723/8.809117 | — | 7.764621/8.635860 |

The largest paired history growth is 2.42% median for 16 sources; paired p95
does not grow. Before visible tick cropping, the controlled one-source median
grew .321600→.417075 ms and .319519→.413772 ms (runs 2/3). Run 1 had varying
latest-price labels and is diagnostic only. Raw logs are ignored
`target/touch-navigation-profile-{1..6}.log`. These CPU costs establish bounded
history behavior for this navigation workload, not GPU frame time or 60 fps.

Remaining acceptance requires a real touchscreen. Record device/backend/scale
and results for pan/tap, pinch, third-finger and pinch-to-one transitions, tracking
exit modes, flick decay/bounds, new input/focus/layout interruption, and mouse/
touchpad/axis/box editing afterward. Repeat at half-size and high DPI. Touch axis
scaling/editing, browser double-tap gestures, arbitrary public horizontal behavior
and other drawing tools remain separate work; touchpads retain pixel-wheel input.
