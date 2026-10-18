# Additional public events checklist

Status: implementing, 2026-10-18. Core API checkpoint: `f17b20d`.
Plan: [public-events-plan.md](public-events-plan.md). Baseline: `dd3f1c8`.
Previous timing, drawing-order and untested-device records stay independent.

## 1. Source contract — no dependencies

- [x] Complete source/dependency/consumer audit and original event tests.
- [x] Probe data scopes/count/order, double-clicks, ranges, hidden/visible dimensions and original horizontal items.
- [x] Freeze delivery/interests, initial events, snapshot/error/queue semantics and native timing policy.
- [x] Pin source hashes/results and unchanged compatibility baseline.

Verification: reproducible real-source evidence and reviewed API/ownership before production edits.

## 2. Delivery — depends on 1

- [x] Add typed variants/interests and one consuming stream for setters/prepare/update.
- [x] Preserve existing signatures, default input events and exactly-once delivery on all returns.
- [x] Update exhaustive enum consumers and document source compatibility/migration.

Verification: default-disabled, pending drain, interest changes, fast paths and rejected-operation tests pass.

## 3. Built-in data — depends on 2

- [x] Emit full/update after successful public set/update/nonempty pop commits.
- [x] Cover equal/empty/whitespace/historical/shared-index cases and wrapper deduplication.
- [x] Keep queries/options/peer reindexing free of false data events and full-data copies.

Verification: native/source sequences and rejection/no-op cases pass.

## 4. Custom data and frontends — depends on 3

- [x] Apply the same rules to typed custom handles and all three frontends.
- [x] Protect foreign/removed owners, private yield support and existing plugin lifecycles.
- [x] Verify pane transfers, empty owners and deferred owned payload lifetimes.

Verification: built-in/custom sequences match across calendar/strike/maturity.

## Checkpoint A

- [x] Review source semantics, API and queue ownership; save tested local checkpoints.

## 5. Double-clicks — depends on 2

- [x] Recognize successful same-pane plot sequences and reuse typed pointer payloads.
- [x] Preserve one Click plus one DoubleClick; test triple/timing/distance/cancellation.
- [x] Preserve edit/axis/divider priority, axis reset and applicable touch tap behavior.

Verification: deterministic native input tests and source sequences pass without sleeps.

## 6. Horizontal ranges — depends on 2 and 4

- [x] Bridge existing strict/logical causes to owned optional logical/original-item events.
- [x] Cover setters/data/prepare/animations, fractional moves, gaps and None transitions.
- [x] Avoid duplicate redraw events, endpoint-equality suppression and scaffold leaks.

Verification: exact trigger/payload sequences pass in all three families.

## 7. Horizontal-axis size — depends on 6

- [x] Emit settled logical dimensions; hide produces 0x0 and restore recovers.
- [x] Define displayed versus snapshot-only preparation and failure behavior.
- [x] Cover fonts/price-axis width/resize/short charts, DPR and repeated unchanged preparation.

Verification: correct size counts/order; no events from drawing retained frames.

## Checkpoint B

- [x] Review ordering, cost, early returns and layout/input boundaries; save tested checkpoints.

## 8. Reactions and lifecycle — depends on 3–7

- [x] Test host reactions producing later batches without recursion or duplicate consumption.
- [x] Verify removal/reorder/transfer, plugin updates and safe historical event identities.
- [x] Exercise sustained feed with regular drains and disabled interests without full-data cloning.

Verification: behavioral lifetime/order/load tests and existing gesture/overlay suites pass.

## 9. Demo — depends on 8

- [x] Add opt-in monitoring, interest controls, counters and bounded display history to the generic host.
- [x] Exercise programmatic and input-return delivery in calendar/strike/maturity.
- [x] Test clear/disable/reset, data controls, double-click and range/size readouts; document commands.

Verification: three-family demo tests/builds pass; no duplicate event handling.

## 10. Acceptance — depends on 9

- [ ] Freeze exact event/raster matrix and pass new plus all 33 existing source comparisons.
- [ ] Pass both asserted renderers, sizes/DPRs and unchanged compatibility images; inspect real output.
- [ ] Obtain software mouse results in all modes, including idle counters and short-height recovery.
- [ ] Obtain matching GPU results and normal exits; retain and resolve any failure.

Verification: manifests, event assertions and actual physical user responses support acceptance.

## 11. Final validation — depends on 10

- [ ] Formatting, all-target tests, doctests, cargo check, strict Clippy and diff hygiene pass.
- [ ] Recheck consumers and complete source/design/files/test/evidence report and README.
- [ ] Save final local commits with clean status; push only when requested.

## Feature acceptance

- [ ] All five event families reach applications across the three chart frontends.
- [ ] Ordering, scopes, original items, size units and single delivery match the frozen contract.
- [ ] Existing input/edit/plugin behavior and ownership remain correct.
- [ ] Automated/source/capture and available software/GPU mouse checks pass unchanged.
- [ ] Earlier timing, drawing-order and unavailable-device exceptions retain independent status.
