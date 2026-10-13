# Custom drawing primitives

Status: implemented; automated acceptance complete. Physical acceptance pending.
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
  coordinates bypass collision adjustment. Primitive background padding does not
  change the source collision height, even though it enlarges the painted box.
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
Replacement validates the ID, detaches old state, then attaches new state; this
ordering also supports shared application lifecycle state without an old detach
overwriting the new attach. Primitive updates do not cancel an active navigation
gesture. Stationary selection changes schedule one reconciliation tick.
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

Context provides current pane/series ownership, plot size, pixel ratio, logical,
time and price conversion and borrowed read-only model inspection. Series-only labels,
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

Implemented the public lifecycle, pane identity, all plot layers and separate
background passes, series autoscale, both axis labels, separate axis graphics,
unified hit arbitration, owned click/crosshair identity and typed cursor hints.
Preparation happens in the existing bounded layout settlement. Render/hit work
uses retained prepared output; drawing has no mutable Chart/model access.

The context matches the checked-out source scale's integer-index conversion:
fractional indices map to zero. For continuous logical positions, interpolate
from an integer coordinate with bar spacing. Nonfinite input or conversion
overflow returns None. Missing times do not snap to the nearest point. Invalid
autoscale ranges/margins are ignored; invalid distances cannot win selection.
Generic primitive views are not automatically hidden with their series, matching
the source extraction/drawing pipeline. A primitive controls its own output.

### Exact files changed

Public API and ownership:
`src/api/chart.rs`, `src/api/interaction.rs`, `src/api/markers.rs`, `src/api/mod.rs`, `src/api/primitives.rs`, `src/api/widget.rs`.

Model integration:
`src/model/axis_snapshots.rs`, `src/model/chart_model.rs`, `src/model/chart_model/price_lines.rs`, `src/model/chart_model/primitives.rs`, `src/model/ipane_primitive.rs`, `src/model/pane.rs`, `src/model/series.rs`.

Rendering:
`src/renderers/price_axis_labels.rs`, `src/renderers/price_axis_renderer.rs`, `src/renderers/price_axis_renderer/labels.rs`, `src/renderers/price_axis_renderer/tests.rs`, `src/renderers/time_axis_renderer.rs`, `src/renderers/time_axis_renderer/tests.rs`.

UI adapters:
`src/ui/chart_frame.rs`, `src/ui/hit_testing.rs`, `src/ui/line_chart.rs`, `src/ui/line_chart/series_drawing.rs`, `src/ui/mod.rs`, `src/ui/multi_pane_frame.rs`, `src/ui/price_axis.rs`, `src/ui/primitive_hit_testing.rs`, `src/ui/time_axis.rs`.

Demonstration and evidence:
`examples/interaction_reference.rs`, `examples/price_axis_reference.rs`, `examples/price_lines_reference.rs`, `examples/primitive_reference.rs`, `examples/public_api_profile.rs`, `examples/public_chart.rs`, `examples/public_chart/primitives.rs`, `examples/public_chart/tests.rs`, `examples/time_axis_reference.rs`, `scripts/check_primitive_reference.cjs`, `tests/public_primitives.rs`.

Documentation:
`README.md`, `docs/porting/primitive-support.md`, `tasks/primitive-support-plan.md`, `tasks/primitive-support-todo.md`.
Existing reference/test initializers were adapted to new snapshot fields; their
behavioral assertions remain intact. Cargo dependencies are unchanged.

### Behavioral evidence

Twelve public integration tests cover foreign/stale IDs; retained geometry after
replacement/removal; callback order and cleanup once; stable pane ownership after
index shifts; empty/preserved panes; layer/background ordering; autoscale merge,
last margins, removal restoration and manual-range preservation; invalid output; axis measurement, clipping
and bounded settlement; fixed price versus ignored fixed time coordinates; pane
identity, clicks, stationary reconciliation and cursors before first draw;
foreground/background arbitration; click versus drag, replacement during drag,
focus loss with an available cursor and focus recovery; normal/log/percentage/
indexed scales, inversion, resize, left/overlay ownership, empty/whitespace reload,
time lookup and coordinate overflow. A host test keeps chart bounds stable across
primitive controls/status changes at window widths 360/480/640.

`primitive_reference.rs` plus `check_primitive_reference.cjs` execute the actual
upstream algorithms: 5,832 two-primitive/built-in overlap combinations across three
owners/layers and priority/background/distance choices; 32 autoscale cases; 18
fixed/edge/crowded price-label cases; 12 marker/primitive attachment chronology cases; four pane drawing-pass cases including raised
ordinary and split filled sources. These 5,898 cases pass. The label fixture found
and fixed the source collision-height rule: additional primitive background
padding affects painting, not collision spacing. The new fixtures stub data,
views and DOM infrastructure, not the compared algorithm.

Reference: checked-out Lightweight Charts HEAD
`8a7c94ff8ddffde68955db61ef8b58098f239de0`, including its existing working-tree
changes. Reference files were read only. Graph coverage is best-effort; changed
metadata for Series, PriceRangeImpl and AutoscaleInfoImpl was supplemented with
direct source reads. The exact Iced Canvas forwards window events to Program and
recomputes cursor interaction, supporting the focus-state implementation.

Validation on 2026-10-13:

- `cargo fmt --check`, `cargo check --all-targets`, strict all-target Clippy,
  `cargo build --examples` and `git diff --check` pass.
- `cargo test --all-targets`: 461 tests pass, including 339 library tests. Focused
  primitive/hover tests also pass after the final review fixes.
- All 16 `scripts/check_*_reference.cjs` comparisons pass, including every existing
  series/layout/axis/interaction/marker/hover comparison.
- TinySkia and WGPU: seven primitive states at ratios 1/1.25/1.5/2, 28 captures per
  backend. Normal/top/bottom, hovered, updated, removed and narrow scenes pass.
  Pixel assertions check single background blending, time-axis band visibility,
  removal, complete coverage and repeatability. Visual inspection compares both
  backends; screenshots are temporary PNG artifacts under
  `/tmp/feather-primitives-{tiny,wgpu}-final`.
- Native `public_chart --primitives` starts without reported errors during an
  eight-second bounded launch; timeout exit 124 is expected. This is startup
  evidence, not physical interaction acceptance.

### Performance evidence

Matched debug builds with rustc 1.98.1, comparing `8025ca1` and this implementation.
Three alternating baseline/current passes; 20 warm-up samples then 100 timed
samples per workload. Values below are medians of the three run medians/p95s in
milliseconds. Data, dimensions, ratio, visible ranges and retained hit geometry
are identical for baseline comparisons. Profile code accepts an optional primitive
count; zero follows the original workload. The comparison screen uses the median
of three current medians against the largest measured baseline p95 per workload;
all comparable workloads meet that screen. This is a provisional regression
screen based on observed baseline variability, not a frame-time specification.

| Workload | Baseline median / p95 | Current median / p95 |
| --- | --- | --- |
| Empty hover preparation | 0.106 / 0.121 | 0.123 / 0.130 |
| Empty hit lookup | 0.000178 / 0.000185 | 0.000189 / 0.000194 |
| 1,200-point hit lookup | 0.001203 / 0.001265 | 0.001200 / 0.001577 |
| 20,000 rows, 202 retained hit points | 0.000729 / 0.000772 | 0.000737 / 0.000794 |
| 16 series, 3,232 retained hit points | 0.008298 / 0.008561 | 0.008305 / 0.008557 |
| 64 series, 12,928 retained hit points | 0.032958 / 0.033968 | 0.033172 / 0.034095 |
| Offscreen Line zoom | 10.712 / 11.036 | 10.731 / 11.194 |
| Offscreen Line append + prepare | 35.445 / 36.633 | 35.360 / 36.062 |
| Offscreen Candlestick zoom | 9.065 / 9.464 | 9.005 / 9.375 |
| Offscreen Candlestick append + prepare | 33.642 / 34.372 | 33.475 / 34.121 |
| Offscreen Histogram zoom | 7.897 / 8.217 | 7.979 / 8.344 |
| Offscreen Histogram append + prepare | 32.104 / 32.677 | 31.948 / 32.664 |
| 16-series Line hover preparation | 30.207 / 30.727 | 30.197 / 30.688 |
| 16-series Line append + prepare | 45.770 / 46.863 | 46.674 / 48.071 |
| 16-series Candlestick hover preparation | 45.228 / 45.811 | 44.922 / 45.438 |
| 16-series Candlestick append + prepare | 56.681 / 61.446 | 56.998 / 61.368 |
| 16-series Histogram hover preparation | 29.596 / 30.046 | 29.476 / 29.976 |
| 16-series Histogram append + prepare | 45.500 / 46.778 | 45.953 / 47.447 |

One crowded Line hover pass measured 31.199 ms versus roughly 30.2 ms in the
baseline; the other two current passes measured 30.120 and 30.197 ms. Single-series
lookup tails also varied. These tails are retained in the evidence; the data does
not establish an improvement or absence of every possible regression.

With 64 lightweight primitives (one current run), the offscreen Line workload
measured zoom 10.892/11.355 ms and append plus primitive replacement/prepare
36.039/36.854 ms. Candlestick measured 9.062/9.443 and 34.246/34.907 ms; Histogram
8.041/8.346 and 32.824/33.494 ms. With 16 series plus 64 primitives, hover preparation
measured Line 30.411/30.893, Candlestick 45.262/45.888 and Histogram 29.911/30.363 ms.
Pure lookup with 64 primitives measured about 0.0025 ms empty, 0.0047 ms single,
0.0043 ms offscreen, 0.0282 ms at 16 series and 0.0811 ms at 64 series. Those stress
figures include misses and hits and have no feature-equivalent baseline. Primitive
arbitration scans attachments per source, so its cost grows with sources times
attachments; avoid claiming unlimited extension counts or smoothness from this
fixture. User renderer/preparation cost is outside the engine's control.

Reproduce the workloads with `public_api_profile --empty-profile`,
`--selection-profile`, `--rows 20000 --visible-rows 200`,
`--overlap-series 16 --visible-rows 200 --hover`; add `--primitive-count 64` for
extension workloads. Temporary logs: `/tmp/feather-primitives-profiles`.

### Review, limits and remaining dependencies

Final review covered ownership/drop order, source arbitration, clipping, label
settlement, input cancellation and empty/no-extension hot paths. No unsafe code,
async engine machinery, dependency changes or check suppressions were introduced.
Prepared renderer immutability is an application contract: Rust's `&self` does
not forbid interior mutation. Renderers must copy geometry and avoid retaining
mutable live state; custom Canvas text/work remains the implementer's responsibility.

This completes the planned automated custom-graphics scope, not every upstream
plugin. Custom data-series rendering, editable annotations, plugin discovery,
watermark plugin ports, custom horizontal behavior, touch/kinetic navigation,
pane reordering, conflation and last-price animation remain separate milestones.

Physical mouse/touchpad acceptance remains pending. Run
`cargo run --example public_chart -- --primitives` and check boundary/region
overlaps and clicks versus drag; cursor transitions and focus/leave; plot pan,
wheel and price-axis scale gestures; half-size resizing; live updates, region
move/layer/remove/re-add and series removal. Record device, backend, display scale
and observed results. The earlier marker/hover hardware gates remain unchanged.
