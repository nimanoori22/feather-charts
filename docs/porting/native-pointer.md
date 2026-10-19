# Native pointer metadata (milestone G)

Baseline: `89602eb`. Contract frozen; implementation and acceptance pending.

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
payload contains device (Mouse or Touch(Finger)), kind (move/press/release/leave/
wheel/cancel), changed mouse button when supplied, optional chart-local and window
positions, and optional `iced::keyboard::Modifiers`. Coordinates are logical
pixels before device scaling; chart position includes axes and pane offsets.
Existing PointerEvent.point remains pane-plot-local and may differ from an actual
finger position while tracking. Window coordinates are content-relative, never
claimed to be desktop-screen coordinates. DOM page/screen/target/view and
preventDefault are omitted; Iced capture remains the existing policy.

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
