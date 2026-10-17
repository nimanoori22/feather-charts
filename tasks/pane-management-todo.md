# Dynamic pane management checklist

Plan: [pane-management-plan.md](pane-management-plan.md).
Baseline: `da1d01c`. Status: proposed; implementation not started.
Likely paths are boundaries to reuse, not mandatory new abstractions. Split any
slice that grows beyond roughly five production/test files into compiling commits.

## 1. Freeze the behavioral contract

Dependencies: none. Scope: 3–5 source-oracle/fixture/report files per slice.
Likely files: scripts/check_pane_management_reference.cjs,
docs/porting/pane-management.md and source/operation fixtures.

- [ ] Read complete relevant components and direct dependencies; trace API/model/
  GUI/plugin/yield consumers, check coverage and read gaps. Pin actual source hashes.
- [ ] Execute source sequences for move/swap/transfer, new target clamping, scale
  selection, arrival order, empty/preserved/final panes and shifted target indices.
- [ ] Resolve explicit nonempty removal through actual GUI reconciliation: series
  handles/data/time union, pane primitives, custom destroy and subsequent calls.

Verification: expected states come from executed source, with stub boundaries
explicit. Do not infer lifecycle from ChartModel.removePane alone. Inspect existing
upstream pane tests and add probes for uncovered ownership cases.

## 2. Freeze design and regression baselines

Dependencies: 1. Scope: 2–4 plan/report/capture fixture files.

- [ ] Compare indexed coordination versus a universal ID registry; finalize minimal
  signatures, errors, ownership and mutation order against real consumers.
- [ ] Map every indexed owner: model panes/series, API views/primitive pane IDs,
  pending masks, input captures, crosshair, controllers and yield support ownership.
- [ ] Retain before-images and freeze operation cases/final counts, backend/DPR/size
  matrix, source/binary/fixture/font identity and existing work limits before edits.

Verification: each proposed API maps cleanly to model, UI, public and lifecycle
consumers; source removal semantics and yield support handling are explicit.

## Gate A: Contract ready

- [ ] Source operation/lifecycle fixtures and native baseline are frozen.
- [ ] No unresolved assumption about destruction or private yield membership.

## 3. Move and swap whole panes

Dependencies: 2. Scope: 3–5 files per slice.
Likely files: src/model/chart_model.rs, src/model/invalidate_mask.rs,
src/api/pane_management.rs (or chart.rs), model tests, tests/pane_management.rs.

- [ ] Add model/API move and swap, updating all indexed owners together while
  retaining each pane's scales/options/stretch and stable primitive identity.
- [ ] Preserve data/time indexing and retained frames; remap or invalidate queued
  pane work correctly, including pending autoscale and crosshair origin.
- [ ] Test both directions, first/middle/last, identity operations and rejected
  indices; rejected calls leave owners and active interactions untouched.

Verification: focused behavioral tests and all-target compile; existing multi-pane,
watermark and primitive identity regressions pass.

## 4a. Transfer built-in series

Dependencies: 3. Scope: 3–5 files.
Likely files: src/model/{chart_model,pane}.rs, src/api/pane_management.rs,
focused integration tests.

- [ ] Transfer the existing owner to the source-defined target scale/order, creating
  at most one appended pane when needed; keep SeriesId/options/data intact.
- [ ] Remove an emptied unpreserved source pane and resolve resulting target index
  correctly. Keep preserved empty panes and prevent removal of the final pane.
- [ ] Cover all six built-ins, left/right/overlay scales and populated targets;
  series markers/price lines/primitives/controllers follow the existing series.

Verification: source-state comparisons, source/destination autoscale and conversion,
same-pane no-op and invalid-ID transactional checks; no full-history conversion.

## 4b. Transfer custom series without recreation

Dependencies: 4a. Scope: 3–5 files.
Likely files: model/pane transfer boundary, src/api/custom_series.rs if required,
tests/pane_management.rs and custom support fixtures.

- [ ] Transfer existing typed model and erased view adapter without replacing the
  public handle, invoking destroy or retaining the old pane's visible-range state.
- [ ] Keep original/hit payloads, options, annotations and reducer/cache ownership;
  verify destination price projection and canonical data after subsequent writes.
- [ ] Test distinct payload types, retained handles/frames and dense custom data;
  counters prove bounded preparation and no canonical-history copy on transfer.

Verification: source custom contract and existing calendar/numeric custom suites
pass; moving followed by history/latest/clear/remove still uses the same owner.

## Gate B: Reorganization works

- [ ] Pane move/swap and built-in/custom transfer work end to end.
- [ ] Stable owners/attachments and queued work survive repeated sequences.

## 5. Explicit pane removal

Dependencies: 4b. Scope: 3–5 files per slice.
Likely files: model/API pane management, primitive/controller cleanup and tests.

- [ ] Implement the frozen empty/nonempty/final-pane behavior, reconciling series,
  data-layer membership, API adapters and pane attachments according to the contract.
- [ ] Exactly-once resource cleanup, detached-handle behavior and time membership
  match the established policy; later updates/re-adds cannot access stale owners.
- [ ] Test first/middle/last, preserved panes, invalid/repeated calls, populated
  custom panes, pending work and old frame drawing after removal/drop.

Verification: lifecycle counters and source sequence comparisons. Do not implement
removal as repeated ordinary series deletion if automatic pane cleanup changes
which pane an index identifies halfway through the operation.

## 6. Complete numeric wrappers and interaction integration

Dependencies: 5. Scope: 3–5 files per slice.
Likely files: src/api/yield_curve.rs, src/api/{interaction,primitive_editing}.rs,
tests/pane_management.rs and existing numeric tests.

- [ ] Expose explicit YieldCurveChart operations; verify OptionsChart inheritance.
  Preserve private scaffold, read-only core, domain settlement and public membership.
- [ ] Replay operation calls during plot/axis/separator drags, touch/kinetic motion
  and primitive edits; subsequent move/release/Escape cannot target the wrong pane.
- [ ] Test watermarks stay with panes; series attachments, pulse/marker deadlines,
  crosshair/payloads and autoscale remain correct across all three families.

Verification: connected event/lifecycle tests, including failed-call preservation,
idle marker expiry and short-height recovery; unaffected timing gates stay unchanged.

## 7. Deliver demo controls and native captures

Dependencies: 6. Scope: 3–5 files per slice.
Likely files: examples/pane_management.rs plus small support/capture modules,
README.md and scripts/capture_pane_management.py only if existing tools need extension.

- [ ] One demo offers calendar/strike/maturity modes, mixed built-in/custom panes,
  move up/down, swap, series-to-existing/new-pane, remove and reset controls.
- [ ] Reuse existing live feed, watermarks, peer animation/markers and editable
  annotation support; expose current pane identity/position and useful errors.
- [ ] Run the frozen software/GPU DPR/size matrix and exact unchanged baselines;
  inspect moved layouts and retained-frame pixels after each structural operation.

Verification: focused example control tests, actual renderer selection, immutable
frame comparisons and retained source/binary/fixture/font manifests.

## 8. Acceptance and final report

Dependencies: 7. Scope: documentation/checklist; fixes return to their slice.

- [ ] Run formatting, full all-target tests, doctests, all-target check and strict
  Clippy; run new source comparisons and affected existing oracles unchanged.
- [ ] Record user mouse checks for all three families on explicit tiny-skia/wgpu:
  operation buttons, live updates, hover, navigation/axes, editing where exposed,
  three window sizes and short-height recovery. Record actual answers separately.
- [ ] Review ownership/error atomicity/work bounds and finish the port report;
  record exact evidence and unavailable devices/historical timing status honestly.

## Gate C: Planned scope complete

- [ ] All source, automated, capture and software/GPU mouse gates pass.
- [ ] Existing source callers remain supported without new ownership cycles,
  unsafe code, dependencies or asynchronous chart-engine machinery.
- [ ] Local checkpoint commits are reviewable; no push unless separately requested.
