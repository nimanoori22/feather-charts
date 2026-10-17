# Dynamic pane management

Status: implemented; automated, capture and software/wgpu mouse acceptance passed
on 2026-10-17.
Baseline: `da1d01c`.
Checklist: [pane-management-todo.md](pane-management-todo.md).
These feature-specific files follow the repository's existing planning convention
and preserve earlier plans and acceptance records.

Final contract and evidence: [pane-management report](../docs/porting/pane-management.md).
The real-browser probe established a source removal cleanup bug and orphaned-map
retention. The report records the deliberate native policy: clean the requested
pane's live series/attachments and retain readable detached custom data. The frozen
source observations are preserved separately from the native cleanup assertions.

## Outcome and scope

Applications can move/swap panes, move existing built-in/custom series between
panes, and explicitly remove panes through the reusable synchronous API. Deliver
calendar, strike-price and maturity chart support, plus demo buttons. Existing
data, typed handles, rendering and attachments retain their appropriate owners.

Include target-pane creation and empty-source-pane cleanup required by source
series movement. Include ordinary/automatic markers, price lines, series/pane
primitives, watermarks, pulse state, custom conflation, autoscaling, crosshair and
gesture cancellation in integration verification. Keep yield support data private.

Mouse-driven pane rearrangement, new series drawing-order controls, new events,
export, arbitrary behavior factories and persistence are separate milestones.
Movement must still preserve source arrival order in the target pane. No dependency
or async/threading change is expected. Historical timing and unavailable-device
acceptance remain open independently.

## Source investigation and current architecture

Preliminary graph searches/traces and direct reads covered:

- Upstream src/api/{chart-api,series-api,pane-api}.ts: swap/remove, moveToPane,
  moveTo, pane queries and object identity.
- Upstream src/model/chart-model.ts: swapPanes, movePane, moveSeriesToPane,
  _getOrCreatePane, _addSeriesToPane and _cleanupIfPaneIsEmpty.
- Upstream src/model/pane.ts and gui/{chart-widget,pane-widget}.ts: pane destruction
  and GUI reconciliation, needed to interpret explicit removal correctly.
- Upstream pane graphics cases remove-pane.js and empty-pane-swap.js. Read the
  remaining graphics/coverage/interaction/type cases in the first task.
- Rust src/api/{chart,primitives,yield_curve}.rs and
  src/model/{chart_model,pane,invalidate_mask}.rs: pane vectors, associated series,
  erased custom adapters, stable primitive pane IDs and indexed invalidations.

Recorded graph coverage matched the inspected model/API and GUI lifecycle evidence
paths. Complete the consumer trace before implementation. Dynamic TypeScript
interface calls are not fully represented
by graph edges. Pin actual source hashes, not just the package version.

Source facts already observed:

1. Pane movement is remove/insert; swapping exchanges two existing panes.
2. Series movement to its current pane returns immediately. A target beyond the
   end clamps to one new appended pane, rather than allocating every missing index.
3. Series movement attaches to the configured/default price scale, then may remove
   the emptied previous pane. Resolve the target object before source cleanup;
   the destination's resulting index can differ from the requested index.
4. Unpreserved empty panes can disappear; preserved panes remain. At least one
   pane remains in the established source paths.
5. Explicit removePane returns immediately for a one-pane chart; otherwise it
   splices the model pane list and triggers GUI reconciliation. This alone does
   NOT prove how public series/data handles behave after removal. Execute the
   complete lifecycle path before choosing native cleanup semantics.

Rust already has add_pane, stretch/preservation controls, automatic empty-pane
cleanup on series removal and stable PrimitivePaneId attachments. ChartModel owns
panes and series associations; Chart also owns parallel per-pane view vectors,
primitive IDs, controllers and interaction state. Changing only the model vector
would leave these owners inconsistent.

## Required contract and decisions

- A pane's axes/options/stretch, watermarks and pane attachments follow that pane
  through reorder/swap. Existing stable primitive pane identity is retained.
- Moving a series preserves SeriesId, canonical rows, typed custom handle identity,
  custom view lifetime, series options and series-owned attachments. Moving is not
  remove_series followed by reconstruction. Pane watermarks stay with their pane.
- Determine target-scale selection and arrival z-order from real source consumers;
  recalculate destination/source scales and hit order after transfer.
- Moving/reordering does not rebuild the shared time union or clone full history.
  Frozen frames continue drawing independently of subsequent operations.
- Preflight IDs/indices and all fallible attachment requirements before mutation.
  Rejected calls preserve state, active edits, controllers and prepared frames.
  Checked no-ops do not destroy/recreate owners. Match source boundary/no-op rules.
- Valid structural changes cancel or safely retarget plot/axis/separator/touch/
  kinetic gestures and primitive edits. Prefer existing cancellation/revision
  mechanisms; no stale pane index may mutate a different pane on the next event.
- Reindex series associations, pane indices and queued per-pane invalidations
  together. Clear/recompute crosshair origin and hit data through existing paths.
- Explicit removal must define final-pane behavior, nonempty pane contents,
  retained handles, shared-time membership, callbacks and exactly-once cleanup
  from actual source evidence. Distinguish required visible behavior from incidental
  JS object retention; document any native ownership/validation policy explicitly.
- YieldCurveChart forwards operations explicitly, preserving readonly Deref,
  private scaffold ownership and source domain retention. Its support series must
  not block public empty-pane behavior or become user-movable content accidentally.

The first checkpoint must resolve explicit removal and yield support placement
before coding those operations. A model-only stub cannot establish GUI destruction
semantics; use a real browser case or reproduce and verify the actual reconciliation
chain with clearly identified boundaries.

## Rust API proposal

Reuse the existing index-based API and ChartError; add only necessary error cases.
These signatures are provisional until source probes resolve boundaries:

```rust
impl<B: ChartBehavior> Chart<B> {
    pub fn move_pane(&mut self, from: usize, to: usize) -> Result<(), ChartError>;
    pub fn swap_panes(&mut self, first: usize, second: usize) -> Result<(), ChartError>;
    pub fn move_series_to_pane(
        &mut self, series: SeriesId, target: usize,
    ) -> Result<(), ChartError>;
    pub fn remove_pane(&mut self, pane: usize) -> Result<(), ChartError>;
}
```

YieldCurveChart gets explicit equivalents. OptionsChart inherits the generic API.
Custom callers pass handle.id(); handles remain typed and valid after movement.
Existing model() queries can inspect the resulting layout; add a public query only
if a real demo/host consumer cannot reasonably use an existing one. Document that
pane indices are current positions, not durable identities, and may shift.

Compare two designs before implementation:

| Design | Benefits | Costs |
| --- | --- | --- |
| Keep current pane storage; model operations plus coordinated Chart mutations | Small additive API, reuses stable primitive IDs and current ownership | Must update all indexed owners and pending invalidations together |
| Introduce a universal PaneId/registry and make all owners address panes by ID | Central identity for every subsystem | Broad migration across model, input, snapshots and API; compatibility and allocation cost |

Prefer the first design. If actual transfer consumers expose an ownership obstacle,
add the smallest internal transfer/reindex helper rather than a second pane registry.
Compare reuse of an existing erased price source with a narrow transfer method for
custom series: the destination must use its own visible-range state. Never introduce
an adapter that keeps borrowing the old pane after movement.

Ownership remains Chart -> ChartModel/panes and Chart -> view adapters/controllers.
Series-owned attachments resolve by SeriesId; pane-owned attachments resolve by
PrimitivePaneId. External custom handles retain model data and a weak live view;
prepared frames own immutable drawing state. Movement introduces no new shared
owner or callback cycle. Removal drops the appropriate live resources once, while
old frames remain drawable. Preserve existing readable detached custom handles
where consistent with the removal contract established in the first task.

## Implementation order and gates

1. Freeze source behavior, consumer analysis, lifecycle rules and native baselines.
2. Deliver pane move/swap end to end through model, public API and regression tests.
3. Deliver built-in series transfer, then custom transfer without rebuilding owners.
4. Deliver explicit pane removal with proven lifecycle and empty-pane rules.
5. Complete yield-wrapper/private-support integration and cross-feature regressions.
6. Add one demo with calendar/strike/maturity modes and operation buttons; run native
   captures and physical mouse acceptance on tiny-skia and wgpu.

Each slice compiles and has behavioral tests plus a local checkpoint. Shared owners
make the mutation work sequential. Tasks and acceptance criteria are in the linked
checklist. Keep existing plan thresholds and historical failures intact.

## Verification protocol

Before production changes, freeze source operation tables and before-images for
representative existing multi-pane calendar/price/yield/custom charts. Preserve
binary/source/fixture/font/backend identity. Freeze exact final capture counts once
the source cases are known, before implementing renderer/model changes.

Exercise first/middle/last, same-index, invalid indices/IDs, missing-target creation,
source cleanup shifting destination indices, empty/preserved/final panes, repeated
operations, all built-ins and distinct custom payload types. Test both visible
price scales and overlay scales, target attach order and automatic marker deadlines.
Verify no custom destroy on move; exactly-once cleanup on proven removal paths;
old-frame pixels after move/swap/transfer/removal/chart drop.

Use row-copy/conversion/reducer counters on dense histories: structural movement
may process pane/series metadata, but must not copy or reconvert canonical history
or restart custom caches merely because pane index changes. Reprojection needed
for a new price transform is expected. Keep hot hover/drag paths unchanged.

Run focused tests for each slice, then cargo fmt --check, cargo test --all-targets,
cargo test --doc, cargo check --all-targets and strict Clippy. Run the new source
comparison and affected existing layout/interaction/custom/yield/plugin oracles.
Use deterministic captures on software/GPU at DPR 1/1.25/1.5/2 and regular, narrow
and short sizes. Compare unaffected frozen fixtures exactly; inspect changed layouts.

User mouse acceptance covers all three families on both explicit backends at
regular/narrow/earlier problem sizes: operation buttons, live feed, payloads,
wheel zoom, diagonal pan, price-axis manipulation, box move/resize/Escape and
release outside the chart where the demo exposes edits. Shrink/enlarge for recovery.
Replay structural mutations during gestures automatically, since clicking a toolbar
button alone cannot prove cancellation of an already captured mouse drag.

## Risks and remaining questions

| Risk | Mitigation |
| --- | --- |
| Model/API pane lists drift | One verified mapping; assert owner counts/associations after every operation |
| Wrong owner is destroyed during cleanup | Trace real GUI lifecycle; resource counters and retained handles/frames |
| Destination index shifts after source cleanup | Resolve target identity first; explicit source-before-target cases |
| Custom source retains old visible range | Verify target autoscale/conversion and custom transfer ownership |
| Yield scaffold prevents cleanup or is removed incorrectly | Dedicated wrapper cases and public/private membership checks |
| Stale input or invalidation targets another pane | Revision/cancellation and event replay with queued work |

No user preference blocks planning. Source lifecycle questions are implementation
investigation tasks, not reasons to invent new behavior. The final report must list
the exact source contract, discarded TS patterns, design alternatives, ownership,
files, validation, physical results and remaining dependencies. No push is included.
