# Dynamic pane management

Status: source/design checkpoint complete; implementation in progress.
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

Pending implementation, source/native comparisons, full tests, native captures and
software/GPU mouse acceptance. Historical timing and untested physical device gates
remain open separately. No push is included.
