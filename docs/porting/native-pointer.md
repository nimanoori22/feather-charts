# Native pointer metadata (milestone G)

Baseline: `89602eb`. Implementation complete; final acceptance in progress.

## Source and real consumers

Read the complete `TouchMouseEventData` interface and mouse-event handler, its
pointer/touch translation, suppression and timer lifecycle. Traced handler calls
into PaneWidget click/double-click/crosshair, ChartWidget's lazy event suppliers
and ChartApi's public conversion. Source translation copies coordinates and
Ctrl/Alt/Shift/Meta states, uses the changed finger for touch, and keeps the original
press event for long-tap. ChartApi forwards the optional source payload. Synthetic
crosshair operations have no physical source. Public readouts/plugins need a
stable event-time snapshot, not access to mutable native event objects.

Graph traces and direct source reads were combined. Relevant named files have no
recorded coverage issue (best effort); flagged HTML/config lines were read. Rust
interaction/widget/touch/coordinate consumers were read directly. Iced Canvas
forwards core Event, including keyboard modifier notifications and focus events;
mouse coordinates are logical window coordinates. Touch has a typed finger ID.
The reference Flowsurface search supplied no modifier-state abstraction to reuse.

## Native contract and API

`PointerEvent<T, M>` gains `source_event: Option<PointerInput>`. An owned Copy
payload contains device (Mouse or Touch(Finger)), kind (enter/move/press/release/leave/
wheel/cancel), changed mouse button when supplied, optional chart-local and window
positions, and optional `iced::keyboard::Modifiers`. Coordinates are logical
pixels before device scaling; chart position includes axes and pane offsets.
Existing PointerEvent.point remains pane-plot-local and may differ from an actual
finger position while tracking. Window coordinates are content-relative, never
claimed to be desktop-screen coordinates. DOM page/screen/target/view and
preventDefault are omitted; Iced capture remains the existing policy.
Mouse is the Iced event family (also used for touchpad wheel/cursor input), not a
hardware identifier. Finger IDs identify contacts and may be reused later.

Modifiers are the latest keyboard state observed by this Canvas (including key
press/release snapshots). Unknown before observation and after either focus
transition, rather than invented empty flags. Modifier updates must be observed
before any gesture early return. Focus loss clears retained touch source as well.
No global input hook or thread is introduced. Source is sampled before dispatch,
so delayed message consumption cannot replace its values with newer state.

A flat `ChartMessage::PointerInput { action: PointerAction, input: PointerInput }`
wraps recognized pointer actions. `with_pointer_input` attaches values to existing
pointer messages; `into_pointer_parts` recovers the existing message and optional
payload. Legacy programmatic messages keep None, and programmatic callers may
explicitly supply metadata. Redraw/bounds/animation messages never acquire stale
mouse metadata. Touch tap/double-tap use the lifted finger; timer-driven tracking
retains its originating press snapshot. Existing synthetic-mouse suppression,
click thresholds, notifications and gesture/edit priority remain unchanged.
Non-finite supplied source coordinates are rejected before mutation.

## Alternatives and ownership

A Box<ChartMessage> envelope is smaller syntactically but allocates on each
recognized mouse movement. The chosen flat action enum reuses existing typed
interaction values, requires one additional ChartMessage variant in exhaustive
consumers and avoids recursive envelopes and hot-path allocation. Adding fields
to every low-level interaction would spread optional metadata into unrelated
engine/UI APIs. No asynchronous machinery, shared mutable source object or new
callback lifetime is needed.

ChartState owns observed modifiers and the originating touch-press snapshot.
A recognized message copies a payload, then Chart::update copies it to emitted
pointer events. Events/queues own their snapshots and survive later input and
chart drop. No back-reference or Rc cycle is possible. Source data is scoped to
one update; model-generated ranges/data events and programmatic updates do not
inherit it. Existing typed custom/built-in metadata remains independent.

G0 freezes this contract. G1 implements and tests native sequences, focus,
queues, touch and programmatic behavior. G2 adds an external four-domain consumer,
source observations, renderer checks and software/GPU mouse acceptance. Existing
timing, formatting-GPU, drawing-order and untested-device exceptions remain open.

## Implementation and consumer checkpoints

The envelope and payload live in `src/api/pointer_input.rs`; the existing widget
samples input before early returns and the existing interaction owner normalizes
once, validates source coordinates before mutation, and copies source only into
new pointer notifications. It never patches queued events or stores source in the
model. Ordinary animation ticks and bounds messages cannot acquire a source.
The legacy standalone demo adapter unwraps and intentionally discards source;
its existing application-specific message type has no pointer event consumer.
Two touch tests unwrap the new transport while also asserting finger identity;
all previous timing, gesture and cancellation assertions remain.
The full suite additionally identified the chart click-recognition test's direct
inspection of the old variant. It now unwraps and asserts source presence, keeping
all click/drag checks. The native touch reference unwraps for inspection; the touch
profiler and custom-series capture clock adapters handle the inline touch action
while retaining source metadata. This preserves their existing deterministic clocks.

The external pane-management consumer supports `--pointer-input`, composes with
typed application records, and preserves existing event interests. Ctrl-click
pins an owned PointerEvent and Shift-click clears it. Review added a failing
regression for rapid modifier changes: the second release may be DoubleClick.
The consumer handles modifiers on both click families; gesture recognition is
unchanged. Programmatic clicks carry None and cannot replace/clear the pin.
Readouts scroll horizontally without changing chart height as their text changes.
All four domains pass this consumer after swapping/transferring the blue series.

Focused tests cover optional/supplied ownership, actual event coordinates despite
a different cursor snapshot, observed Ctrl/Shift/Alt/Logo states, key snapshots,
focus invalidation, delayed dispatch, mouse double-click, touch tap/long-press,
synthetic-mouse suppression, redraw/programmatic None and invalid-source atomicity.
The new source oracle executes the actual current 5.2.1 chart and GUI in Chromium
and freezes mouse/touch modifiers, local coordinates and synthetic behavior.

The capture matrix dispatches actual Canvas mouse/keyboard input before rendering:
six scenes (hover, pinned, cleared, programmatic, moved, recovered), four domains,
three sizes and DPR 1/1.25/1.5/2 on both strict backends. It requires 576 screen
replays, 576 exports retained after Reset/owner drop, and 192 modifier-only
pixel comparisons. Existing compatibility protocols are unchanged.

Rust files: `src/api/{mod,interaction,widget,pointer_input}.rs`,
`examples/support/chart_demo.rs`, `examples/pane_management.rs`,
`examples/pane_management/{pointer,pointer_captures,metadata_captures}.rs`,
`tests/native_pointer.rs`, `tests/touch_navigation.rs`. Source/capture scripts,
source fixtures, README, inventory and evidence accompany those files. No new
runtime, dependency, DOM emulation, event subscription or model algorithm is added.
Additional consumers: `tests/chart_api.rs`, `examples/touch_reference.rs`,
`examples/touch_navigation_profile.rs`, `examples/custom_series/capture.rs`.

The first capture attempt retained a fixture failure: f32 addition/subtraction of
a window offset did not reproduce the original pre-offset point bit-for-bit. The
corrected assertion tests the exact documented subtraction from the actual native
event position. No engine change or existing threshold was needed. Both corrected
backends pass all 576 screen/export cases and 192 modifier-only pixel comparisons.
Representative pinned and transferred captures were inspected visually.

Affected public-event and axis-interaction capture protocols are rerun unchanged
against their accepted baselines. Other combined capture families remain for Z0;
their protocols and assertions are preserved. The change is input transport and
event payload only, with no formatter, rasterizer or screenshot selection change.

## Final automated validation

Common gates pass: 866 tests in 88 suites, two compile-fail doctests, all-target
build/check, strict all-target Clippy, formatting and API inventory. All 44 source
comparisons pass; the previous 43 scripts are unchanged. Both new renderer
matrices pass, and the unchanged affected event/axis protocols match all 1,776
accepted images, including the original 96-image baseline. No assertions or
existing thresholds were weakened. API checkpoint: `f356c76`.

[Native pointer evidence](native-pointer-evidence.json) pins source/fixture/binary
hashes and result logs. It retains the missing-API/widget red tests, message-shape
migration failures, rapid modifier/double-click regression, coordinate-fixture
failure and corrected passes. The first common-build attempt ended with SIGTERM
without compiler output establishing a cause; a persistent-terminal retry exposed
the remaining consumer migrations, which were fixed before the final complete run.

Software/GPU physical mouse acceptance is pending. No combined Z0 capture run,
physical touchpad/touchscreen/high-DPI, earlier timing, separate formatting-GPU or
dedicated drawing-order acceptance is inferred from these results. Locale and
configured color parsing remain H and I.
