# Public series data operations checklist

Status: planned, 2026-10-18. Implementation has not started.
Plan: [series-data-plan.md](series-data-plan.md). Baseline: `71c8419`.

## 1. Source contract — no dependencies

- [ ] Read complete source components/dependencies; trace public API, model, custom owners and controllers.
- [ ] Probe pop/query/whitespace/shared-time/range/numeric behavior with actual source and inspect original tests.
- [ ] Pin source states, hashes and compatibility baseline; document native policies separately.
- [ ] Freeze APIs/errors, owned-result design, alternatives, ownership and mutation/edit policy.

Verification: real-source oracle reproduces before production edits.

## 2. Built-in reads — depends on 1

- [ ] Add failing data/style/time round-trip, directional lookup and logical-range tests.
- [ ] Share existing pointer-event row conversion; implement canonical full/index/range queries.
- [ ] Preserve all six built-in kinds, fractional/gap semantics and pure read behavior.

Verification: source query comparisons and pointer-event regressions pass.

## 3. Built-in pop — depends on 2

- [ ] Add failing return-order, zero/empty/oversize, trailing-whitespace and update-after-pop regressions.
- [ ] Expose existing DataLayer pop and apply the complete response with preflight validation.
- [ ] Preserve series registration/owners and shared axis membership; protect no-ops and frozen frames.

Verification: focused public and existing model/data-layer tests pass.

## Checkpoint A

- [ ] Review source contract, API and ownership; save tested local checkpoint.

## 4. Typed custom operations — depends on 3

- [ ] Extend owned typed queries while preserving existing data_at and detached readable handles.
- [ ] Implement custom pop using old indices before reindex; reject wrong-chart/detached mutations.
- [ ] Verify whitespace, shared custom/built-in times, payload copies, view lifetime and conflation updates.

Verification: typed custom source comparisons and ownership/reentrancy tests pass.

## 5. Frontends and lifecycle — depends on 4

- [ ] Support calendar/strike/maturity; explicitly forward yield mutation and protect private support.
- [ ] Verify other-series reindexing, transferred/reordered owners, autoscale and canonical crosshair/hits.
- [ ] Verify markers, pulses, price lines, stale edits and old frames after emptying/update/drop.

Verification: focused frontend/integration suites pass at dense/sparse and short/fractional sizes.

## 6. Demo — depends on 5

- [ ] Add opt-in pop/append/reset and selected-data/index/range readouts to the existing generic host.
- [ ] Include sparse/shared-time custom and built-in data; retain pane/order/editor controls.
- [ ] Test controls, invalid selection, empty-series updates and recovery in all three modes.

Verification: example tests/build pass with clear returned-order and query readouts.

## 7. Capture and native acceptance — depends on 6

- [ ] Freeze and pass 288 feature captures with actual backend assertions and 96 unchanged baseline comparisons.
- [ ] Inspect representative software/GPU output and immutable old frames through removal/update/drop.
- [ ] Obtain software mouse results for all modes/sizes, pop/append and short-height recovery.
- [ ] Obtain matching wgpu results; retain and resolve any reported failure.

Verification: manifests, pixel comparisons and actual user responses support results.

## 8. Final verification — depends on 7

- [ ] cargo fmt check, all-target tests, doctests, cargo check, strict Clippy and diff hygiene pass.
- [ ] New source comparison and complete existing source comparison set pass unchanged.
- [ ] Review diff/recheck consumers; complete porting report, README and evidence links.
- [ ] Save local checkpoints; push only when requested.

## Feature acceptance

- [ ] Public built-in/custom pop and canonical queries work across all three chart families.
- [ ] Ownership, shared axis, data styles/time, search/range and lifecycle match the frozen contract.
- [ ] Automated, capture and available mouse acceptance pass.
- [ ] Prior drawing-order mouse acceptance, timing exceptions and unavailable-device checks retain their independent status.
