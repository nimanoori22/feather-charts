# Implementation roadmap: custom drawing primitives

Date: 2026-10-13. Status: planned; implementation has not started.
Foundation: `8025ca1` (built-in hover selection and rendering).
Execution checklist: [primitive-support-todo.md](primitive-support-todo.md).
Use separate milestone files, following the existing project convention; retain
earlier plans and their pending physical acceptance checks.

## Outcome and scope

Allow an application to attach custom graphics to a built-in series or a pane.
Deliver safe attachment/removal, immutable prepared rendering, source-compatible
layers and background passes, optional series autoscale contributions and axis
labels, and owned hover/click identity. Demonstrate a shaded time/price region
with a selectable boundary through the public Chart widget.

Support default Time behavior and all six built-in series. Pane primitives draw
inside their pane; series primitives use their owner's current pane and scale.
Series axis labels and arbitrary price/time-axis pane graphics are separate
capabilities in upstream: include both, in separate implementation slices.
Pane primitives must not acquire series-only autoscale or axis-label behavior
unless the source consumer investigation demonstrates it.

Interactive editing/drag handles, custom data-series rendering, plugin discovery,
watermark plugin ports, touch/kinetic gestures, pane reordering, conflation and
last-price animation remain later milestones. Do not turn this work into a
general-purpose scene graph or require Tokio, channels, locks or new dependencies.

## Preliminary findings

These findings support planning; they do not replace the complete source and
consumer investigation required before production edits.

- Upstream `ipane-primitive.ts` defines bottom/normal/top views, nullable renderers,
  separate foreground/background drawing and optional hit testing. Top drawing
  is above the crosshair. Hit layer and background metadata are explicit;
  drawing order alone cannot determine selection.
- `iseries-primitive.ts` adds price/time labels, price/time-axis pane graphics and
  optional autoscale information. Its label contract distinguishes collision-
  adjusted coordinates from fixed coordinates.
- Primitive wrappers cache views/renderers by JavaScript reference identity.
  Preserve efficient invalidation, not array identity or wrapper classes.
- Public attachment hooks receive owner context and a requestUpdate callback.
  The Rust design must support application-driven updates without storing a
  mutable Chart reference in a component owned by that same Chart.
- Rust already has `PrimitivePaneViewZOrder`, `PrimitiveHoveredItem` and
  `PrimitiveHitTestSource` in `src/model/ipane_primitive.rs`. Reuse or evolve these
  after checking consumers; their existence does not mean public rendering works.
- Current `ChartHit` requires a `SeriesId`. Pane hits need genuine pane/primitive
  identity; assigning a fabricated series would break the ownership contract.
  Keep existing series/marker event behavior coherent when extending it.
- Current hits use owned settled geometry. Custom hit testing and drawing must
  refer to the same preparation revision, including before the first draw.

Planning graph searches located source attachment APIs/wrappers and existing Rust
primitive contracts. Coverage checks reported no recorded gaps in the inspected
paths. One qualified attachment trace returned no relationships; direct source
reads established its calls. Coverage is best-effort and this is not an exhaustive
consumer audit. Task 0 must complete tracing and inspect upstream tests/plugins.

## Source investigation and design gate

Before production edits follow all AGENTS porting phases: complete source/import
reads, whole-project consumer tracing with coverage and fallback, behavioral
contract, Rust architecture audit, exact Iced/Flowsurface investigation where
relevant, design alternatives, ownership, signatures and consumer mappings.
Record evidence in `docs/porting/primitive-support.md`.

| Starting sources under lightweight-charts/src | Contract to resolve |
| --- | --- |
| `model/{ipane-primitive,iseries-primitive,pane-primitive-wrapper,series-primitive-wrapper,primitive-drawing-utils}.ts` | Views, renderer absence, layer/background passes, label placement, cache invalidation |
| `api/{ipane-primitive-api,iseries-primitive-api,pane-api,series-api}.ts` | Attachment context, update requests, detach behavior and public lifetime |
| `model/{pane,series,chart-model,pane-hit-test,hovered-source-order}.ts` | Removal, scale/pane changes, autoscale merge, hit arbitration and hovered drawing |
| `gui/{pane-widget,price-axis-widget,time-axis-widget,chart-widget}.ts` | Exact pass ordering, clipping, label layout, cursor and event conversion |
| `plugins/{primitive-wrapper-base,primitive-wrapper,pane-primitive-wrapper,series-primitive-adapter}.ts` and marker/watermark consumers | Real consumer expectations, redraw requests and ownership |

Read relevant upstream unit/e2e/graphics tests, especially primitives mixed with
markers and filled series. Inspect local Iced Canvas rendering, clipping, cursor
mapping and redraw dispatch; use Flowsurface only as an implementation example.

## Architecture direction and alternatives

Compare two viable approaches in task 0 before choosing exact signatures:

1. **Prepare owned immutable render objects.** Chart owns mutable primitive state;
   preparation produces backend-adapted immutable draw/hit objects and labels.
   This allows arbitrary custom graphics and fits retained frames, but requires
   careful type erasure and a small, documented drawing target contract.
2. **Prepare owned drawing commands.** Plugins emit geometry/text commands and
   hit geometry consumed by existing adapters. This makes caching and snapshots
   straightforward, but a closed command set can unnecessarily restrict graphics
   or grow into a second rendering engine.

Prefer the first if a small safe adapter can support both existing backends and
preserve frame ownership. Do not expose mutable plugin state to draw, force a
shared mutable owner, or silently limit arbitrary graphics to the demonstration's
rectangle. Reuse renderer utilities; keep Iced-specific types at the adapter
boundary where practical. Separate mutable prepare/update from immutable draw.

Provisional API responsibilities, finalized by task 0:

```text
PrimitiveId                         chart-scoped, typed, never reused
add_series_primitive(series, value)  validated attachment, returns PrimitiveId
add_pane_primitive(pane, value)      validated attachment, returns PrimitiveId
replace/update_primitive(id, value)  explicit update/invalidation path
remove_primitive(id)                deterministic detach and stale-ID handling
prepare(...)                        produce settled render/hit/label snapshots
primitive hit payload               PrimitiveId, owner, external object ID
```

Choose an ergonomic typed update or replacement contract; do not expose unchecked
downcasts as the normal API. Decide whether hit identity extends ChartHit or is
an additive event payload. Explain compatibility and preserve existing marker
queries and exact-index series data. Pane identity must survive unrelated pane
removal/index shifts internally. External object IDs may repeat across attachments.
Translate supported cursor requests into typed Iced cursor values with a documented
fallback for unsupported CSS names.

```text
Chart -> primitive attachments -> mutable application primitive state
Preparation -> owned frame -> immutable render objects, hit data, labels
Widget -> borrows frame -> existing Chart messages -> owned event payload
Attachment update -> revision/invalidation -> next preparation
```

No strong back-reference to Chart. If consumer requirements justify a retained
update handle, keep it owner-independent or weak and define behavior after detach
and Chart drop. Define callback mutation/reentrancy semantics explicitly; do not
invoke arbitrary plugin code while holding conflicting mutable owner borrows.
Old retained frames keep their draw data alive without keeping attachments active.
Describe Drop versus explicit detach hooks and ensure cleanup occurs once.

## Delivery sequence

0. Complete source contract and resolve rendering, update and event identity APIs.
1. Deliver a series-attached shaded rectangle end to end with add/update/remove.
2. Extend the same path to pane attachments and complete owner lifetimes.
3. Complete layer/background ordering, clipping and viewport invalidation.
4. Integrate optional series autoscale contributions.
5. Add price labels, then time labels, then axis-area custom drawing.
6. Add primitive hit arbitration, cursor requests and owned public events.
7. Complete a shaded-region/boundary demo and source/backend/performance checks.

Each numbered stage is subdivided in the checklist to keep production slices
around five files or fewer. Exact file boundaries remain provisional. Keep each
slice buildable; do not export inactive capabilities. Checkpoints are verification
gates within authorized implementation, not permission pauses. Execute shared
ownership/frame/event changes sequentially and save logical local commits.

## Acceptance matrix

| Area | Required behavior |
| --- | --- |
| Lifecycle | Invalid/cross-chart IDs, add/update/remove, series/pane/Chart drop, clear/reload, empty panes, index shifts, source-defined hide behavior |
| Rendering | Three layers, foreground/background passes, nullable output, multiple attachments/views, stable ties, Area/Baseline raised outlines, markers/crosshair |
| Preparation | Zoom/pan/price scaling, resize/DPI, stationary pointer updates, bounded settlement, immutable retained frames, no draw-time measurement or mutation |
| Autoscale | Optional contribution, merge with built-in range/margins, visible logical interval, empty/hidden output, manual/inverted/log/percentage/indexed modes, remove contribution |
| Labels | Fixed versus adjusted placement, visibility/ticks/colors, left/right/overlay rules, edge clipping, narrow plots, measured axis size, no oscillating layout |
| Axis graphics | Price/time-axis local coordinates, clipping, source ordering, fractional ratios, hidden axes and immutable prepared output |
| Hits/events | Pane versus series ownership, logical coordinates, distance/priority, background/foreground and layer precedence, duplicate IDs, raw pointer versus magnet, cursor reset |
| Performance | No full-history work on pointer movement, cached prepared output, bounded update requests, no continuing redraw after settle/detach |

Build deterministic comparisons against actual checked-out TypeScript for
ordering, autoscale merging, label placement and hit arbitration. Synthetic
fixture plugins are acceptable; fixtures must exercise the real source pipeline,
not a duplicate implementation of the same algorithm. Record the source revision.

## Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Plugin mutates a frame through shared state | Separate prepare state from immutable owned rendering; test retained frames after updates and detach |
| Update callback captures its owner | Prefer explicit updates; if handles are required, define weak/owner-independent ownership and test drop |
| Pane hit is represented as a series hit | Resolve event owner types in task 0; test pane removal/index shifts and cross-pane lookup |
| Primitive layers conflict with hovered filled series | Compare full source pass sequence; capture translucent overlaps and markers/crosshair |
| Autoscale/layout recursively requests preparation | Define invalidation timing; test bounded settlement and no redundant continuing redraw |
| Axis labels cause narrow-window jumping | Reuse measurement/settlement rules and retain the demo's one-line status regression |
| Extension interface becomes a scene graph | Start with one complete arbitrary-draw contract; add only source-required capabilities |

## Validation and completion

Run focused behavioral tests and relevant source comparisons per slice. At final
verification run:

```sh
cargo fmt --check
cargo test --all-targets
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --examples
git diff --check
```

Run existing source comparison scripts and new primitive fixtures. Capture layers,
labels, axis graphics, overlap selection and removal on TinySkia/WGPU at ratios
1/1.25/1.5/2. Drive native widget events before first draw and after preparation;
include bounded native startup and narrow-window resizing.

Compare preparation/pointer/zoom/update workloads with `8025ca1`: no attachments,
many visible attachments and large mostly-offscreen data. Record median/p95 and
cache/projection work using identical builds and scenes. Establish any numeric
regression gate from measured baseline evidence before accepting performance;
successful screenshots do not establish frame-rate guarantees.

Provide a public-chart primitive demo with attachment, layer, update and removal
controls and a selectable boundary. Update README and the port report with exact
files, source consumers, required/discarded behavior, ownership, alternatives,
validation and remaining dependencies. Recheck real upstream consumers against
the final API. Do not claim complete extension parity with unsupported branches.

Record physical mouse/touchpad checks separately: hover boundary/region overlaps,
click versus drag, cursor transitions, zoom/pan/axis scaling, half-size windows,
live updates and removal. Prior hover/marker hardware gates remain open until
their own results are recorded; they do not prevent planning this milestone.
