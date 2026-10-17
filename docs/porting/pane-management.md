# Dynamic pane management

Status: implemented; automated verification, native captures and software/wgpu mouse
acceptance passed on 2026-10-17.
Baseline: da1d01c. Plan/checklist: tasks/pane-management-{plan,todo}.md.

## Source and consumer contract

The source manifest pins the actual TypeScript modules bundled from src/index.ts.
The reference runner transpiles and bundles these modules with the installed local
TypeScript/Rollup dependencies and executes them in isolated headless Chromium.
It uses real ChartApi, ChartModel, Pane, PaneWidget and GUI reconciliation, without
model/DOM stubs. Results and browser exceptions are retained under
target/pane-management; frozen observations are pane-management-states.json.
Graph API/model/GUI traces were verified with direct reads; dynamic interface edges
are incomplete. Upstream graphics/coverage cases require retained pane identity,
move/swap, transfer to existing panes and empty-source cleanup. Custom series and
plugin consumers require ownership to survive transfers without detach/recreate.

Move is remove/insert; swap exchanges two existing panes. Series movement resolves
the destination before cleanup, appends at most one missing target, preserves the
series owner/options, attaches to its configured/default scale, and may remove an
unpreserved empty source pane. Same-pane transfer is a no-op. Pane move targets
are validated after GUI settlement in the source. Rust operations stay synchronous.

## Explicit removal discrepancy and native policy

The real-browser removal probe shows that the local 5.2.1 implementation retains
removed-series data in global maps. Removing the first/middle pane calls primitive
detach on the LAST pane through widget reconciliation, rather than on the requested
pane. The optional custom view destroy callback is not called in these cases.
The frozen observations retain these facts instead of relabeling them as parity.
No inspected upstream consumer requires this stale ownership behavior.

Native explicit removal will remove the requested pane and its series from the
live chart, normalize its shared data membership and clean the appropriate live
resources once. Retained custom handles keep readable data, as existing Rust
remove_series already guarantees. The final pane remains and removal is a no-op,
including the source's early-return ordering. This is a deliberate native ownership
policy, not an exact reproduction of the source's orphaned maps/widget cleanup bug.

## Design and ownership

Keep the index-based API and coordinate existing owners rather than adding a
universal PaneId registry. The latter would migrate model/input/snapshot contracts
without improving this feature's consumers. Chart owns per-pane adapters and stable
PrimitivePaneId values; ChartModel owns panes and series associations. One operation
reorders all relevant vectors and indexed invalidations together. Existing primitive
identity follows its pane. Series attachments resolve by SeriesId and follow it.

Transfers move existing price-source adapters and rebind their pane-visible range,
preserving typed custom owners, caches and weak handles. An alternative per-series
type-erased attachment closure would add allocations/captured handles; reusing the
existing adapter is smaller. A new generic registry, async dispatch and thread-safe
ownership are unnecessary. Frames own immutable drawing state, no model borrows.

Public API: Chart<B>::move_pane(from,to), swap_panes(first,second),
move_series_to_pane(series,target), remove_pane(index), returning Result<(),ChartError>.
OptionsChart inherits these; YieldCurveChart forwards explicitly. Custom callers use
handle.id(). Current numeric pane indices are positions and may shift. Validation
precedes interaction cancellation/mutation; no-op and rejected calls preserve owners.

Yield's private whitespace series remains chart infrastructure. Moving its pane
retains it, while explicit removal must relocate it to a surviving pane first.
Public removal/movement cannot select its hidden ID. Ordinary empty-source cleanup
continues counting it, matching the upstream support series; do not shrink the
source-compatible retained scaffold domain or expose it as public content.

## Frozen native protocol

96 current before-images were retained at target/pane-management/before with source,
binary, fixture and font identities checked against the final preceding milestone.
Matrix: three chart families, two backends, three sizes, four DPR and six scenes
(initial, move, swap, built-in transfer, custom transfer, removal): 432 feature images.
Compare the 96 unchanged old-family captures exactly. Preserve frozen frames after
operations and chart drop, plus work counters for dense transfer without history copies.

## Validation

The full suite passes 709 tests across 69 suites with zero failures/ignored tests
(target/pane-management/all-targets-tests-final.log). Two compile-fail doctests pass.
All 30 source comparisons pass (oracles-final/results.json). Formatting, all-target
compilation and strict Clippy pass. Final demo tests were repeated after improving
the initial box span and selecting the visible left price axis for maturity data.
The visible-axis regression failed before that demo-only correction.

Seven pane-management integration tests cover source state sequences, move/swap,
custom owner and attachment identity, zero payload copies on transfer, source
cleanup, new-target clamping, requested-pane removal and yield infrastructure.
Five connected interaction tests cover all six built-ins on left/right/overlay
scales, canonical storage identity, real Canvas plot/axis/divider capture,
rejected/no-op preservation, kinetic stopping and update-marker deadlines/expiry.
The existing primitive-edit suite includes one new structural cancellation/stale
token regression. An invalidation test checks autoscale mapping and the unchanged
stop-animation/offset command sequence.

The visible-range regression is mutation-checked: temporarily removing the source
adapter rebind made first_value return 1 instead of 101 after deleting the source
pane and scrolling. The binding was restored before the full passing suite. This
protects target-pane percentage/indexed formatting and autoscale first-value state.
Earlier failing test/build logs are retained; a new invalidation test initially
omitted the existing StopAnimation command from its expectation. No command or
acceptance threshold was removed to satisfy that test.

The final native matrix passes: 432 feature images and 96 exact before-image
comparisons. Frozen snapshots retain their pixels after swap, transfer, removal
and chart drop. Software regular/narrow/short captures and a fractional-DPR wgpu
capture were visually inspected. Per-image hashes, renderer assertions and
source/binary/fixture/font identity are in
target/pane-management/final/{protocol,results}.json.

Physical mouse acceptance on 2026-10-17:

| Backend | Calendar | Strike | Maturity |
| --- | --- | --- | --- |
| tiny-skia | User: all checks passed; normal exit | User: all checks passed; normal exit | User: all checks passed; normal exit |
| wgpu | User: all checks passed; normal exit | User: all checks passed; normal exit | User: all checks passed; normal exit |

Checks include toolbar operations, stable pane labels, series-following box and
markers, hover data, live-feed zoom/pan, price-axis wheel/drag, box move/resize/Escape,
regular/narrow/earlier problem size and short-height recovery. Native process logs
are target/pane-management/mouse-{calendar,price,yield}-{tiny-skia,wgpu}.log.
Historical timing and touchpad/touchscreen/physical high-DPI gates remain open
separately. No push is included.

## Files and design verification

Production Rust:

- src/api/pane_management.rs: checked move/swap/transfer/remove, coordinated views,
  stable pane attachments, cancellation and old-hover clearing.
- src/api/{mod,chart}.rs: module registration and reuse of existing pane-view cleanup.
- src/api/yield_curve.rs: explicit forwarding, empty preservation controls and private
  support relocation before explicit pane removal.
- src/model/chart_model/pane_management.rs: model association/index coordination,
  transfer and empty-pane cleanup; chart_model.rs reuses this cleanup boundary.
- src/model/invalidate_mask.rs: indexed request remapping and regression.
- src/model/{iprice_data_source,pane,series}.rs: reuse/rebind existing erased adapters
  rather than reconstructing custom owners or cloning their history. The new trait
  method has a default, preserving existing external/static source implementors.

Examples/tests/evidence:

- examples/pane_management.rs and pane_management/{host,captures}.rs: three modes,
  native controls, live updates, typed hover, mixed series and immutable captures.
- examples/support/editable_box.rs: existing options demo box extracted and made
  behavior-generic; examples/options_chart/region.rs retains its original editor tests.
  options_chart.rs keeps its original span; the new demo chooses a domain-sized span.
- examples/numeric_custom_series/view.rs: existing renderer accepts a distinct
  internal item type for calendar reuse; numeric callers keep the default type.
- tests/pane_management.rs, pane_management_interaction.rs and primitive_editing.rs.
- scripts/{check_pane_management_reference.cjs,capture_pane_management.py}, source
  manifest/state fixtures, README and the feature-specific plan/checklist.

No dependency, vendor, Iced runtime or chart widget rewrite was needed. Callback
identity APIs and JS orphan retention were discarded; owned typed adapters, weak
custom handles and frozen frames retain their existing semantics. Mutation remains
synchronous. No extra strong owner back-reference or new Rc cycle was introduced.

Public/model/UI/lifecycle consumers use the proposed four signatures directly.
Pane primitives resolve via stable identity; series primitives resolve the current
series association. Native cancellation uses the existing revision/touch epochs,
so widget captures cannot keep operating on a reassigned numeric index. Yield core
access remains read-only. More drawing-order APIs, mouse-driven pane rearrangement,
export and broader event APIs remain separate features.
