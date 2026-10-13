# Implementation roadmap: interactive primitive editing

Date: 2026-10-13. Status: planned; implementation not started.
Foundation: `380f4bc` (custom drawing primitives).
Execution checklist: [primitive-editing-todo.md](primitive-editing-todo.md).
Use separate milestone files following the existing project convention. Keep
earlier plans and pending physical acceptance checks intact.

## Outcome

In `cargo run --example public_chart -- --primitives`, click the shaded box to
select it, drag inside it to move it, and drag its edges or corners to resize it.
Dragging elsewhere continues to pan the chart. Escape restores the bounds from
the start of the gesture. Selection and resize cursors stay consistent across
window resizing, scaling, owner removal and focus changes.

Deliver reusable gesture routing through the public chart widget, with the box
owning its geometry and editing rules. This is an application drawing feature
built on Lightweight Charts primitive behavior; do not claim that annotation
editing is part of the upstream Lightweight Charts primitive contract without
source evidence. TradingView's full web application is a separate product.

## Scope and defaults

- Mouse editing of one selected series-owned box at a time.
- Eight handles: four corners and four edge midpoints. Corners change both
  dimensions; edges change one. A selected box interior starts a move.
- Click empty plot space to deselect. A click without a drag selects but does
  not alter bounds or emit a drag-induced chart click.
- Store horizontal bounds as continuous logical positions and vertical bounds
  as prices. Preserve existing programmatic replacement and attachment identity.
- Start a drag only in the plot. Continue a captured drag outside the plot when
  the runtime delivers movement, and finish on release. Do not clamp the drawing
  to visible bounds; clip rendering at existing pane boundaries.
- Reject inverted or collapsed bounds by clamping the dragged edge to a small
  positive separation from its opposite edge; do not swap handle identity.
  Final minimum extent and hit radius must be explicit, tested logical-pixel
  constants, converted using the gesture's starting transform.
- Escape and focus loss cancel and restore original bounds when the same owner
  and attachment still exist. Removal drops the edit safely without resurrecting
  an attachment. External replacement cancels without overwriting newer state.
- Wheel/axis navigation is suspended during a captured edit; normal navigation
  resumes after commit/cancel. Define keyboard navigation similarly at task 0.

Touch editing, snapping, undo history, multi-selection, drawing creation tools,
rotation, persistence and additional drawing types are later milestones. No new
async runtime, locks, general scene graph or dependency is justified here.

## Investigation and design gate

Before production edits, complete the AGENTS source/consumer/design workflow.
Use codebase-memory with coverage verification and source fallback to trace
primitive hits, native event routing, chart navigation and attachment lifecycle.
Inspect relevant upstream primitive APIs, pane hit arbitration and pane mouse
handlers, plus actual plugin consumers/tests. Inspect the exact local Iced event,
Canvas, cursor and focus behavior; consult Flowsurface only for relevant examples.
Record the evidence and final signatures in `docs/porting/primitive-editing.md`.

Resolve these questions before committing to an API:

1. How can a prepared, immutable primitive declare an editable hit target while
   keeping ordinary noneditable primitives and built-in hits unchanged?
2. How does the widget claim the press synchronously, before chart panning starts,
   while the application receives owned begin/update/commit/cancel events?
3. How do inverse conversions use the owner's actual price scale, first value,
   pane and time scale? `PrimitiveContext::logical_to_coordinate` currently maps
   fractional indices to zero like the source; editing needs explicit continuous
   interpolation and inverse conversion, not accidental use of that method.
4. How is the viewport held stable during editing, including autoscale, axis
   label measurement and incoming data? Freeze navigation and the effective
   layout/scale mapping for the gesture; cancel on incompatible changes. Restore
   prior scale policy afterward without undoing legitimate external updates.
5. How are external replacement and self-originated preview updates distinguished?
   Repeated `replace_primitive` detaches/reattaches today. Avoid a per-move lifecycle
   churn path if a small explicit update contract can preserve attachment lifetime.
6. What does Iced actually deliver when the pointer leaves the window or focus
   changes? Do not promise operating-system pointer capture without evidence.

Compare at least two designs: application-owned geometry plus typed widget edit
events, versus mutable edit hooks on chart-owned primitives. Prefer the first
if synchronous capture and preview updates remain straightforward. It keeps
retained renderers immutable and avoids callbacks mutating their chart owner.
Do not add unchecked downcasts, strong Chart back-references or a generic tool
framework merely to support this rectangle.

Proposed responsibilities, with exact types to be settled by investigation:

```text
Prepared primitive -> optional owned edit target / handle identifier
Widget -> capture state: attachment, owner, target, start point, transform
Widget -> typed Begin / Update / Commit / Cancel events
Application -> original bounds, preview bounds, selected attachment
Application -> validated primitive update -> new immutable prepared frame
```

No live model references in queued events. The application owns the box state;
Chart owns its attachment; frames own copied geometry. Widget capture is cleared
on release/cancellation. Old frames remain drawable without retaining an active
edit or reactivating removed attachments.

## Ordered delivery

### 0. Lock the interaction contract

Complete the investigation above and document signatures, ownership, alternatives,
hit precedence, cancellation, minimum size, inverse conversions and viewport policy.
Add regression cases for ordinary navigation to the planned test matrix.
Acceptance: important native/public consumers map to the design without repeated
workarounds; existing drawing/hit/lifecycle contracts remain explicit.
Verification: source/consumer evidence review before production modifications.
Likely files: `docs/porting/primitive-editing.md`, this plan/checklist.
Dependencies: none. Scope: small.

### 1. Select the box and display handles

Use existing owned primitive identity to select/deselect. Draw handles only for
the selected region and prepare their hit geometry with the same frame revision.
Document handle precedence over the selected boundary/interior and how that
interacts with other sources/layers. Unselected hits keep existing arbitration.
Acceptance: all eight handles appear at the correct positions; selection does
not move the chart or resize its layout; remove/re-add clears stale selection.
Verification: focused selection/hit tests and normal/half-size renderer captures.
Likely files: `examples/public_chart/primitives.rs`, `examples/public_chart.rs`,
`examples/public_chart/tests.rs`, `tests/public_primitives.rs`.
Dependencies: 0. Scope: medium.

### 2. Move the selected box through reusable gesture routing

Add the minimal optional editable-target contract and widget capture/event path.
Implement continuous inverse conversions and a stable gesture mapping. Deliver
begin/preview/commit for interior dragging; compute every preview from the
original bounds and start point, avoiding accumulated rounding drift.
Acceptance: the box follows the pointer in both directions; an edit leaves the
viewport unchanged; empty-space drag still pans; ordinary clicks retain their
documented behavior; one release produces one commit.
Verification: native event sequence tests, round-trip conversion tests and a
public-widget moving-box scenario. Include inverted/log/percentage/indexed
price modes and the owner's left/right/overlay scale.
Likely files: `src/api/primitives.rs`, `src/api/interaction.rs`,
`src/api/widget.rs`, `examples/public_chart/primitives.rs`,
`tests/public_primitives.rs`. Split coordinate helpers into a separate small
commit if their existing owner requires additional files.
Dependencies: 1. Scope: medium, split as needed.

Checkpoint: selection and movement work end to end; baseline chart gestures and
primitive lifecycle tests pass before adding resizing.

### 3. Resize from edges and corners

Map handle identifiers to constraints in the application's box editor. Use the
same capture path as movement. Keep opposite edges fixed, enforce minimum
extent, and provide directional resize cursors including before the first draw.
Acceptance: every edge/corner changes only its intended bounds; crossing an
opposite edge clamps predictably; sub-bar horizontal positions remain smooth.
Verification: behavioral tests for all handles, crossings, small rectangles,
offscreen bounds, inverted axes and repeated drags without coordinate drift.
Likely files: `examples/public_chart/primitives.rs`,
`examples/public_chart/tests.rs`, `tests/public_primitives.rs`.
Dependencies: 2. Scope: medium.

### 4. Complete cancellation and lifecycle handling

Make capture an explicit state machine and cover Escape, focus loss, owner or
attachment removal, external replacement and incompatible viewport changes.
Handle pointer exit and release according to actual Iced delivery. Serialize
self-originated previews with external updates; never restore stale state over
an external edit. Restore prior navigation/autoscale policy on every exit path.
Acceptance: cancellation restores original bounds when valid; stale owners never
receive updates; no stuck cursor/capture or unintended chart pan remains.
Verification: synthetic native sequences for every terminal path, retained frame
safety, pane index shifts, resize/data updates mid-drag and subsequent fresh drag.
Likely files: `src/api/widget.rs`, `src/api/interaction.rs`,
`src/api/primitives.rs`, `examples/public_chart/primitives.rs`,
`tests/public_primitives.rs`.
Dependencies: 2, 3. Scope: medium.

Checkpoint: full move/resize/cancel matrix passes; attachment cleanup stays once
per lifecycle; navigation resumes after every completion/cancellation path.

### 5. Verify rendering and drag performance

Add repeatable captures for selected/moving/resized/cancelled states at ratios
1, 1.25, 1.5 and 2 on TinySkia and WGPU. Preserve one-line status layout and
stable chart bounds in narrow windows. Profile repeated drag previews against
the current primitive baseline, including many sources and incoming updates.
Acceptance: handles/hit targets agree in logical pixels; clipping and labels stay
correct; no redraw loop, lifecycle churn or per-event work proportional to full
offscreen history appears. Record timing distributions and measured limits.
Verification: capture assertions, layout regression tests, relevant profile runs,
all existing source-reference scripts and full Rust validation below.
Likely files: `examples/public_chart.rs`, `examples/public_chart/primitives.rs`,
`examples/public_chart/tests.rs`, `examples/public_api_profile.rs`,
`docs/porting/primitive-editing.md`.
Dependencies: 4. Scope: medium.

### 6. Document and complete physical acceptance

Document the reusable public event/update contract, geometry ownership, demo
controls and cancellation rules. Run the physical checklist below and record
observed results. Update only the milestones whose checks were actually run.
Acceptance: automated validation passes and physical editing checks are recorded;
otherwise explicitly label hardware acceptance pending.
Verification: final consumer/API review and user mouse/touchpad results.
Likely files: `README.md`, `docs/porting/primitive-editing.md`,
`tasks/primitive-editing-plan.md`, `tasks/primitive-editing-todo.md`.
Dependencies: 5. Scope: medium.

## Validation and completion

Run focused tests for each slice. At checkpoints and completion run `cargo fmt
--check`, `cargo test --all-targets`, `cargo check --all-targets` and `cargo clippy
--all-targets -- -D warnings`; build examples and run relevant source-reference
scripts. Keep source parity tests separate from application-specific editing
tests. Recheck source consumers against the final API and publish exact changed
files, behavior covered, results and remaining dependencies in the port report.

Physical acceptance: select/deselect; drag interior horizontally/vertically;
resize all eight handles; cross opposite edges; release outside the plot;
Escape/focus loss; zoom and price-axis gestures after editing; half-size window;
high-DPI display; live updates and removal; repeat edits using mouse and touchpad.
Confirm no jitter, viewport movement, stuck cursor or accidental pan. Hardware
acceptance cannot be inferred from screenshots or synthetic input alone.

## Main risks

| Risk | Mitigation |
| --- | --- |
| Autoscale/label layout changes move the box under the pointer | Stable gesture mapping and effective viewport policy; test expanding price bounds |
| Chart panning begins before application handles press | Synchronous widget arbitration from prepared editable targets |
| Pointer updates round fractional positions | Explicit continuous conversion and start-relative previews |
| Replacement invalidates capture or triggers lifecycle churn | Define preview update semantics before routing implementation |
| Cancellation overwrites external updates | Distinguish preview provenance; cancel stale sessions without rollback |
| Full-window pointer capture is unavailable | Verify Iced delivery and document actual outside-window behavior |

No user decision blocks planning. Task 0 resolves API/runtime details from source;
if evidence requires a material change to interaction behavior, record it before
implementation.
