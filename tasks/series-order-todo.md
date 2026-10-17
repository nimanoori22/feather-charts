# Series drawing order checklist

Status: implemented; automated checks passed 2026-10-18. Source and capture verification passed; physical mouse acceptance is pending.
Plan: [series-order-plan.md](series-order-plan.md).

## 1. Freeze source contract and baseline

- [x] Read complete source components/dependencies and trace real API/model/render/hit consumers.
- [x] Inspect original change-series-order test and existing Rust mixed-source ordering tests.
- [x] Probe actual source ordering, formatter changes, lifecycle, custom series and maturity support slots.
- [x] Pin source hashes, source observations and unchanged capture baselines.
- [x] Document contract, two viable designs, ownership, chosen API and capture/edit policy.

Verification: repeat the real-browser probe; review recorded states and unresolved
source discrepancies before production edits. Dependencies: none.

## 2. Calendar public API through rendering

- [x] Add a failing public overlap/order regression.
- [x] Add query/setter, model effect delivery and view-order coordination using existing pane logic.
- [x] Cover middle/front/back, oversized/clamped no-op, invalid/removed ID and next-prepare rendering.

Verification: focused Rust tests and source-state comparison pass; no canonical
data copies or new series ownership. Dependency: 1.

## 3. Custom/numeric lifecycle integration

- [x] Support built-in/custom series in calendar/strike/maturity charts with explicit yield forwarding.
- [x] Preserve handles, data, updates and scale assignment; protect private support IDs.
- [x] Verify indices after add/remove/transfer/pane move/swap/reset and reorder again afterward.

Verification: ordering and existing pane-management tests pass for all families.
Dependency: 2.

## Checkpoint A

- [x] Review API, ownership and source-state parity; document deliberate native differences.
- [x] Save a local tested implementation checkpoint.

## 4. Rendering, selection and edit regression

- [x] Test ties/non-ties, hover raising on/off and filled background/foreground behavior.
- [x] Test normal/top/bottom primitives, markers, price lines and layer reassignment.
- [x] Test warmed formatter/layout caches on both side scales and overlays, including hidden/empty data.
- [x] Test active capture/edit policy, stale tokens, exactly-once cancellation and immutable old frames.

Verification: focused UI/interaction suites and affected source comparisons pass;
fix only demonstrated shared-path defects. Dependency: 3.

## 5. Existing demo controls

- [x] Add selected-series order display and backward/forward/back/front controls.
- [x] Add useful overlap/formatter scenes and hover-raising toggle without a new host/editor.
- [x] Test controls with live feed after transfer/removal/reset across all three families.

Verification: example tests/build pass and controls handle missing selections.
Dependency: 4.

## 6. Capture and physical mouse acceptance

- [x] Freeze and run the 288-image feature matrix with explicit software/wgpu backend assertions.
- [x] Pass the 96 unchanged before-image comparisons; inspect overlap/formatting/clipping samples.
- [ ] Obtain user software mouse results in all three chart modes and sizes, including short-height recovery.
- [ ] Obtain matching wgpu results; fix and repeat only affected checks if problems occur.

Verification: retained manifests, representative visual review and actual user
responses support acceptance. Dependency: 5. Device checks require the user's
available hardware; record outstanding checks accurately.

## 7. Final verification and report

- [x] Formatting, all-target tests, doctests, all-target check, strict Clippy and diff hygiene pass.
- [x] New source probe and complete existing source comparison set pass unchanged.
- [x] Complete porting report, README and evidence links; recheck real source consumers.
- [x] Review final diff and save local checkpoints; leave push for explicit instruction.

## Checkpoint B — Feature accepted

- [x] Public ordering works across all chart families for built-in/custom series.
- [x] Rendering, hit arbitration, formatters and ownership satisfy the frozen contract.
- [ ] Automated, capture and available mouse acceptance pass.
- [x] Historical timing and unavailable-device exceptions remain separately recorded.

Dependency: 6. Do not mark this checkpoint complete from compilation or captures alone.
