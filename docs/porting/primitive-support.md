# Custom drawing primitives

Status: design established; implementation and validation in progress.
Foundation: `8025ca1`. Roadmap: `tasks/primitive-support-plan.md`.

## Source components and behavior

Investigated complete `model/ipane-primitive.ts`, `iseries-primitive.ts`,
`pane-primitive-wrapper.ts`, `series-primitive-wrapper.ts`,
`primitive-drawing-utils.ts`, both primitive API contracts and plugin wrappers.
Read attachment/detach, view grouping, autoscale, drawing and hit consumers in
Series, Pane, their API owners and pane/price/time widgets. Read the rectangle
plugin and pane-hit interaction fixture. Source remains read-only.

Primitive wrappers adapt user views, cache by reference, expose independent
background/foreground draw calls, optional labels, autoscale and hit results.
Series extensions follow their series scale; pane extensions have no series.

## Consumers and behavioral contract

- Public API and plugin adapters attach/detach and request synchronous redraw.
  Explicit Rust replacement plus prepare supplies that operation without owner
  captures. Attach/detach notifications receive typed owner identity; cleanup
  happens once. Old frame resources can outlive an attachment.
- Pane drawing iterates pane attachments then ordered sources, all backgrounds
  before foregrounds for each layer. Bottom is before grid, top after crosshair.
  Filled hovered series keep primitive normal views with their fills, while
  strokes/price lines rise; ordinary series carry normal primitives when raised.
- Series autoscale merges extension price ranges into built-in ranges; the last
  contributed margins win. The provider receives the combined calculation.
  Empty series bypass extensions. Manual price scales retain their own range.
- Price/time-axis consumers use separate labels and drawing views. Fixed price
  coordinates bypass collision adjustment and paint after unfixed labels.
  Time labels are painted with the top labels before top axis graphics.
- Pane arbitration reverses stable source order, compares primitive layer first,
  then point priority/distance. Top wins immediately; normal foreground blocks
  its source's own built-in hits and lower sources; normal background checks its
  source first; bottom is a fallback. Marker results participate in that path.
- Input emits owned target identity using raw plot coordinates, independently of
  magnet position, and restores the default cursor after the winning target ends.

Graph searches found existing Rust contracts and source APIs/wrappers. Qualified
traces returned empty relationships, so source call reads and repository-wide
literal searches supplemented them; no exhaustive graph claim is made. Coverage
checks for source contracts/wrappers/APIs/widgets and Rust model/API/UI scopes
reported no recorded gaps. `series.ts` metadata differed, so its relevant source
was read directly. The graph remains best-effort. Iced Canvas Program supplies
read-only drawing and cursor interaction; clipping is a synchronous scoped draft.
Flowsurface overlay drawing confirms the adapter pattern, not chart semantics.

## Alternatives and ownership

Chosen: application state implements `DrawingPrimitive`; `prepare(&Context)`
returns owned `PreparedPrimitive` containing immutable `Rc<dyn PrimitiveRenderer>`
views. Renderer `draw(&self, &mut Frame)` and `hit_test(&self, Point)` read that
snapshot. The target is Iced Canvas at this public adapter boundary, supporting
both configured backends and arbitrary paths without a second command language.
No rendering trait requires Send/Sync. Implementers must not capture mutable live
state in their prepared renderer: prepare copies the geometry needed by the frame.

Alternative: a closed drawing-command enum. Easier validation/serialization but
limits user drawing or grows a second scene graph. Rejected for this milestone.
Direct drawing from live model/plugin state was rejected because it violates
retained-frame behavior and allows hit/display revision mismatch.

Chart owns attachments with `Rc<dyn DrawingPrimitive>`; series autoscale stores
read-only callbacks capturing the primitive, never Chart. Typed pane keys survive
index shifts. Frame owns only prepared renderers and labels. Replacement detaches
old state and attaches the replacement using the same attachment ID. Removal,
owner removal and Chart drop detach once; retained frames do not attach/detach.
No back references or Rc cycles are introduced. User code runs synchronously with
read-only model access and cannot reenter a mutable Chart through the API.

## Proposed Rust API and consumer mapping

```rust
pub trait DrawingPrimitive: std::fmt::Debug {
    fn prepare(&self, context: &PrimitiveContext<'_>) -> PreparedPrimitive;
    fn autoscale_info(&self, start: TimePointIndex, end: TimePointIndex)
        -> Option<AutoscaleInfo> { None }
    fn attached(&self, owner: PrimitiveOwner) {}
    fn detached(&self) {}
}
pub trait PrimitiveRenderer: std::fmt::Debug {
    fn draw(&self, target: &mut iced::widget::canvas::Frame);
    fn draw_background(&self, target: &mut iced::widget::canvas::Frame) {}
    fn hit_test(&self, point: iced::Point) -> Option<PrimitiveHitCandidate> { None }
}
// Chart::add_series_primitive / add_pane_primitive -> Result<PrimitiveId, ChartError>
// Chart::replace_primitive(id, impl DrawingPrimitive) -> Result<(), ChartError>
// Chart::remove_primitive(id) -> Result<(), ChartError>
// PointerEvent gains primitive: Option<PrimitiveHit>; hit/marker describe built-ins.
```

Context provides current pane/series ownership, plot size, pixel ratio, logical
and price conversion and borrowed read-only model inspection. Series-only labels,
autoscale and axis views are ignored for pane attachments by contract. Prepared
views own their layer and renderer; empty output replaces old output. Hit results
carry typed attachment/owner IDs and application object strings, never fake series
identity. Normal series hit queries report a built-in only when that built-in wins;
primitive-specific queries report the same unified winner. Marker-only query stays
separate. Exact-index event series rows keep their existing semantics.

JS reference-array caches, callback identity, nullable shared mutable renderer
references, CSS strings and manual wrapper destruction are replaced by owned
snapshots, explicit update operations, typed cursors and Rust lifetime management.
Background status and hit layer remain explicit because real consumers use them.

## Implementation, tests and validation

To be updated after each slice. Physical mouse/touchpad acceptance remains pending
until an actual device check is reported. No full extension parity is claimed.
