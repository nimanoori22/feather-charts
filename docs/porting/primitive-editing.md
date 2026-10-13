# Interactive primitive editing

Status: design accepted for implementation; validation pending.
Foundation: `380f4bc`. Roadmap: `tasks/primitive-editing-plan.md`.

## Source and consumers

Read the complete upstream `model/ipane-primitive.ts`, `model/iseries-primitive.ts`,
`api/iseries-primitive-api.ts`, `model/pane-hit-test.ts` and the complete
`plugin-examples/src/plugins/rectangle-drawing-tool/rectangle-drawing-tool.ts`.
Inspected pane mouse handlers and the Rust chart/widget/primitive adapters,
coordinate algorithms, preparation/layout and hit arbitration. Graph lookup and
attachment traces support these reads; dynamic/interface callers are not fully
resolved by the graph. Exact-path coverage reported no recorded gaps, a
best-effort indication rather than proof of exhaustive coverage.

Primitive consumers require stable owner identity, layer-aware hit precedence,
copied render geometry, redraw requests and deterministic detach. The upstream
rectangle tool owns its points and preview in the application, subscribes to
chart click/crosshair events and converts y through its own series scale. It
creates rectangles from two clicks; it does not specify drag-resize behavior.
This milestone is a Rust application extension, not a claim of upstream or
TradingView web-app annotation parity.

The pinned Iced Canvas forwards native events to Program::update and publishes
one action/message synchronously with optional capture. Typed horizontal,
vertical and both diagonal resize cursors exist. Event capture prevents sibling
handling; it is not operating-system pointer capture. Leaving the window cancels;
leaving only the plot continues editing when coordinates remain available.

## Contract and design

Choose application-owned geometry with typed widget events. The alternative is
mutable chart-owned primitive editing callbacks: it makes direct mutation easy,
but mixes shape rules with navigation, complicates owner borrowing/reentrancy and
snapshot ownership. Application events cost a small owned payload per update
and fit existing synchronous host updates. Neither needs Send/Sync or async.

Proposed additive public API:

```rust
// Default false; external_id identifies the application-defined edit target.
PrimitiveHitCandidate { editable: bool, /* existing fields */ }
PrimitiveHit { editable: bool, /* existing fields */ }
pub struct PrimitiveEditId(/* private token */);
pub enum PrimitiveEditPhase { Begin, Update, Commit, Cancel(PrimitiveEditCancelReason) }
pub struct PrimitiveEditEvent {
    pub id: PrimitiveEditId,
    pub hit: PrimitiveHit,
    pub pane: usize,
    pub start: iced::Point,
    pub position: iced::Point,
    pub phase: PrimitiveEditPhase,
}
ChartMessage::PrimitiveEdit(PrimitiveEditEvent)
ChartEvent::PrimitiveEdit(PrimitiveEditEvent)
impl Chart {
    pub fn primitive_context(&self, id: PrimitiveId) -> Result<PrimitiveContext<'_>, ChartError>;
    pub fn request_primitive_update(&mut self, id: PrimitiveId) -> Result<(), ChartError>;
    pub fn preview_primitive_update(&mut self, id: PrimitiveEditId) -> Result<(), ChartError>;
}
impl PrimitiveContext<'_> {
    pub fn continuous_logical_to_coordinate(&self, value: f64) -> Option<f64>;
    pub fn coordinate_to_logical(&self, x: f64) -> Option<f64>;
    pub fn coordinate_to_price(&self, y: f64) -> Option<f64>;
}
```

Final signatures will be recorded alongside validation. New enum variants and
public struct fields require downstream exhaustive matches/literals to adapt.

Chart owns attachment Rc -> application primitive adapter -> Rc<RefCell<box>>.
Application Scene owns the other Rc and selection/original bounds. Preparation
borrows the box only long enough to copy geometry; retained renderers capture no
RefCell or Chart. Widget capture owns only a token, hit and coordinates. No
strong back-reference or Rc cycle. No callback runs while geometry is borrowed.

On press the widget claims an editable hit before navigation and emits Begin;
the application selects it. A 5-logical-pixel Manhattan threshold separates a
click from a drag. Updates are start-relative, release commits once, and ordinary
noneditable clicks/panning keep their path. Selected handles and outline draw on
top; their selected-region hit overlay takes precedence. Unselected graphics use
the existing source/layer arbitration. Eight handles use a 7-pixel hit radius;
resizing clamps to an 8-pixel starting-transform extent without swapping edges.

Chart temporarily disables only the owner's effective price autoscale and holds
allocated axis sizes. It keeps prior scale policy, rejects navigation during
capture, and cancels on incompatible viewport/owner/first-value changes. Live
data that preserves the mapping can continue. Commit/cancel restores the prior
policy only if no external scale change superseded it. An explicit preview
notification updates geometry without detach/reattach or canceling its session.
External replacement/update cancels without stale rollback. Removal never
resurrects the object. Escape/focus/window exit and viewport cancellation roll
back the application's original bounds when still valid. Pending cancellations
are delivered once through the normal update/tick path.

Time conversion explicitly interpolates integer coordinates using bar spacing;
the inverse is its exact algebraic counterpart, without ceil or whitespace snap.
Price conversion delegates to the owner's existing scale and visible first value,
including left/right/overlay, inversion and nonlinear modes. Nonfinite inputs or
outputs and empty mappings return None. Existing source-compatible integer
conversion remains unchanged.

## Validation

Pending implementation. Record behavioral tests, native sequences, renderer
captures, profile results, source-reference checks and physical acceptance here.
