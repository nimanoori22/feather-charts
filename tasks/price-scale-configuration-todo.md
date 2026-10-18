# Price-scale controls and runtime configuration checklist

Status: implementation and verification in progress, 2026-10-18.
Plan: [price-scale-configuration-plan.md](price-scale-configuration-plan.md).
Baseline: `ea7198a`. Earlier feature/timing/device acceptance records are independent.

## 1. Source contract — no dependencies

- [x] Complete source/dependency reads and real consumer traces; inspect original tests.
- [x] Freeze source ranges/units/rounding/mode transitions and invalid-input policy.
- [x] Inventory runtime fields, defaults, readbacks, validation and affected caches.
- [x] Confirm API alternatives/ownership, atomic patch and interaction policy.
- [x] Pin actual-source results/hashes and unchanged compatibility baseline.

Verification: reproducible real-source probes; documented native deviations before production edits.

## 2. Visible range — depends on 1

- [x] Add failing public read/set tests, including empty/invalid/log/special-mode cases.
- [x] Reuse existing range/scale types and model invalidation; preserve target scale ownership.
- [x] Verify fixed ranges survive feed/navigation/resize with both sides and existing overlays.

Verification: focused tests and actual-source comparisons pass; no mutation on errors.

## 3. Autoscale and frontends — depends on 2

- [x] Expose explicit autoscale/readback and YieldCurveChart mutation forwarding.
- [x] Test custom-range release, disable/freeze, mode transitions and double-click restore.
- [x] Verify pop/empty/append and private maturity support across all three frontends.

Verification: source transitions and focused frontend tests pass.

## Checkpoint A

- [x] Review API/error/ownership contract and save a tested local checkpoint.

## 4. Appearance patches — depends on 1 and 3

- [x] Add typed partial layout/grid options and owned readbacks.
- [x] Validate before committing; propagate colors/typography/grid to affected snapshots/defaults.
- [x] Test partial/no-op/invalid mixed updates and immutable retained frames.

Verification: configuration tests and actual rendering agree with readback.

## 5. Aggregate routing — depends on 4

- [x] Integrate supported price/time/crosshair patches, retaining existing public setters.
- [x] Prevalidate complete patches; preserve unspecified fields and batch effects.
- [x] Verify existing/new pane defaults and per-pane targeting against source.

Verification: aggregate/individual equivalence and failure-atomicity tests pass.

## 6. Input and pane configuration — depends on 5

- [x] Reuse navigation options; propagate supported pane styling/resize fields to widgets.
- [x] Test appearance-only preservation, viewport-change cancellation and stale tokens.
- [x] Test disable/re-enable gestures and stop stale kinetic state without affecting data.

Verification: focused Iced input/widget tests pass with actual effects for every exposed field.

## Checkpoint B

- [x] Review the complete field inventory and save tested local checkpoints; no successful no-op fields.

## 7. Lifecycle — depends on 3, 5 and 6

- [x] Test pane reorder/removal, series transfer/order and scale ownership.
- [x] Test built-in/custom pop/empty/update, autoscale, overlays/crosshair and box edits.
- [x] Verify retained frames, short-height recovery and no unnecessary data copying/recreation.

Verification: focused integrations and unchanged compatibility tests pass.

## 8. Demo — depends on 7

- [x] Add opt-in controls and readouts to the shared calendar/strike/maturity demo.
- [x] Test range/auto/appearance/navigation/pane controls, reset and invalid-input recovery.
- [x] Document runnable commands and public examples; build both renderers.

Verification: generic demo tests pass in all three modes.

## 9. Acceptance — depends on 8

- [ ] Freeze and pass feature captures across three families, two asserted backends, sizes and DPRs.
- [ ] Inspect real output; pass unchanged compatibility images and all source comparisons.
- [ ] Obtain software mouse results at regular/narrow/problem sizes and short-height recovery.
- [ ] Obtain matching GPU results; resolve reported defects and retain actual responses/exits.

Verification: protocol, comparisons and physical user reports support every accepted claim.

## 10. Final verification — depends on 9

- [x] Formatting, all-target tests, doctests, cargo check, strict Clippy and diff hygiene pass.
- [x] Review consumers/diff and complete source/design/files/evidence report and README.
- [ ] Save final local commits; push only on request.

## Feature acceptance

- [ ] Read/manual/auto price-scale controls work across all three chart families.
- [ ] Every inventoried supported runtime field updates state, rendering and future defaults correctly.
- [ ] Ownership, validation, gestures and immutable frame contracts are preserved.
- [ ] Automated/source/capture and available software/GPU mouse checks pass unchanged.
- [ ] Historical timing, drawing-order and unavailable-device records retain independent status.
