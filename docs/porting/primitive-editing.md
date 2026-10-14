# Interactive primitive editing

Status: implemented; automated validation complete; physical acceptance pending.
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

Flowsurface's complete `src/chart.rs::canvas_interaction` provided an example of
start-relative movement, release cleanup and suppressing wheel navigation while
panning. Its generic architecture and bounded-pointer rules were not copied.

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

Public API (important signatures):

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
    pub fn primitive_context(&mut self, id: PrimitiveId) -> Result<PrimitiveContext<'_>, ChartError>;
    pub fn request_primitive_update(&mut self, id: PrimitiveId) -> Result<(), ChartError>;
    pub fn preview_primitive_update(&mut self, id: PrimitiveEditId) -> Result<(), ChartError>;
}
impl PrimitiveContext<'_> {
    pub fn continuous_logical_to_coordinate(&self, value: f64) -> Option<f64>;
    pub fn coordinate_to_logical(&self, x: f64) -> Option<f64>;
    pub fn coordinate_to_price(&self, y: f64) -> Option<f64>;
    pub fn coordinate_transform(&self) -> Option<PrimitiveCoordinateTransform>;
}
```

`primitive_context` takes a mutable borrow to settle the source time scale's lazy
visible-range cache, then exposes read-only context. `PrimitiveCoordinateTransform`
is Copy and owns coefficients; its logical/price forward/inverse methods return
Option for unavailable or nonfinite mappings. Its price coefficients come from
`PriceScale::coordinate_transform(base)`, sharing the existing conversion math.
New enum variants and public struct fields require downstream exhaustive
matches/literals to adapt. The prepared drawing traits remain unchanged.

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

## TypeScript-specific patterns discarded

Application-owned typed events replace callback identity and mutable chart-owned
editing hooks. Rust attachment ownership and deterministic detach replace
manual JavaScript destruction conventions. Prepared drawings copy geometry;
they do not share mutable live objects with the renderer. The owned transform
holds coefficients rather than references into the model. No async machinery,
thread-safe locks, unchecked downcasts or new dependencies were introduced.

## Files changed

- Public API/routing: `src/api/mod.rs`, `src/api/chart.rs`,
  `src/api/interaction.rs`, `src/api/primitives.rs`,
  `src/api/primitive_editing.rs`, `src/api/primitive_edit_input.rs`,
  `src/api/widget.rs`.
- Model/layout/hits: `src/model/price_scale.rs`, `src/model/pane.rs`,
  `src/model/chart_model/primitives.rs`, `src/ui/multi_pane_frame.rs`,
  `src/ui/primitive_hit_testing.rs`.
- Demo/captures/profile: `examples/public_chart.rs`,
  `examples/public_chart/primitives.rs`,
  `examples/public_chart/primitive_editing.rs`,
  `examples/public_chart/primitive_editing_capture.rs`,
  `examples/support/chart_demo.rs`.
- Tests: `tests/primitive_editing.rs`, `tests/series_markers.rs`.
- Documentation: `README.md`, this report,
  `tasks/primitive-editing-plan.md`, `tasks/primitive-editing-todo.md`.

## Validation

`cargo fmt --check`, `cargo test --all-targets`, `cargo check --all-targets`,
`cargo clippy --all-targets -- -D warnings`, and `cargo build --examples` pass.
The full suite reports 478 passing tests and zero failures.
All 16 existing source-reference checks pass. Tests preserve integer source
conversion behavior and cover continuous fractional/offscreen round trips,
left/right/overlay owners, all four price modes and inversion, retained
transforms, native input capture before navigation, duplicate moves, outside-plot
release, stale tokens, lifecycle notifications, cancellation and restoration of
prior autoscale policy. A growing primitive contributes wider price labels and
larger autoscale bounds without changing the editing viewport until commit.
Crosshair labels clear when editing starts and remain cleared after focus loss.
The demo tests cover selection/movement/deselection, all eight resize constraints,
minimum extents/crossings, nonfinite input and immutable handle hit geometry.

Renderer validation covers 12 primitive states at ratios 1, 1.25, 1.5 and 2 on
TinySkia and WGPU: 96 captures with pixel/hit assertions. Five editing states
(selected, moving, resized, cancelled and selected in a narrow window) were also
rerun on the final build with explicit geometry assertions. Contact sheets were
visually inspected on both backends. These checks do not replace hardware tests.

Reproduce editing captures with:

```sh
cargo run --example public_chart -- --primitives --primitive-edit-captures-only --capture-dir /tmp/feather-editing
# Add --wgpu for the other renderer.
cargo run --example public_chart -- --primitive-edit-profile
```

## Performance evidence and limits

Compared debug builds against foundation `380f4bc`, using three alternating
baseline/current runs per existing workload (20 warmups, 100 measured samples).
The table gives the median of the three per-run medians in milliseconds:

| Existing workload | Baseline | Editing build |
| --- | ---: | ---: |
| Empty hover | 0.124 | 0.123 |
| 20,000-row line zoom | 11.939 | 12.021 |
| 20,000-row candlestick zoom | 13.574 | 13.530 |
| 20,000-row histogram zoom | 10.303 | 10.222 |
| 16-source line hover | 34.352 | 34.361 |
| 16-source candlestick hover | 79.935 | 80.540 |
| 16-source histogram hover | 45.318 | 45.316 |

Existing append medians improved by 0.36–1.68%; the largest measured median
increase was 0.76%. Tail timings varied: candlestick offscreen append p95 reached
43.543 ms versus baseline maximum 40.655 ms; crowded line append reached
59.978 ms versus 51.313 ms. This evidence supports similar median behavior,
not a guarantee that every tail improved.

The new native preview profile includes widget routing, host update and prepared
frame/glyph work. Results are medians of three runs:

| Rows / sources / concurrent data update | Median ms | p95 ms |
| --- | ---: | ---: |
| 1,200 / 1 / no | 13.014 | 13.192 |
| 20,000 / 1 / no | 17.123 | 17.412 |
| 20,000 / 1 / yes | 40.039 | 41.362 |
| 1,200 / 16 / yes | 42.561 | 42.881 |

Drag geometry is start-relative and bounded in cost; preview notification uses a
light update and never detaches/reattaches. Primitive-free presses skip primitive
hit scanning. The chart's overall preparation/data-update paths still depend on
history and source count: the original roadmap's strict history-independent
per-event performance target was not fully met at this checkpoint. The later
[preparation roadmap](preparation-performance.md) makes static previews and
existing-time replacement bounded; genuine append retains full reindexing and
the broad per-event target remains open for that route. These debug measurements do not establish
60 fps for large live workloads.

## Remaining scope and acceptance

Physical mouse/touchpad acceptance is pending. Run the dedicated checklist in
`tasks/primitive-editing-todo.md` using `cargo run --example public_chart --
--primitives`, including all handles, window exit/focus loss, half-size/high-DPI
layout and navigation after editing. The implementation and automated validation
are complete; the interaction milestone remains awaiting hardware acceptance
and the strict history-independent performance target remains open.

Touch gestures, snapping, undo, multi-selection, drawing creation, persistence,
rotation and more drawing types are outside this milestone. Existing primitive
API consumers remain valid with default noneditable targets; downstream struct
literals and exhaustive event matches must account for the additive API fields
and variants. The upstream rectangle creation tool is a separate future port.
