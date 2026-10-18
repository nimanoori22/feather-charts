# Public series data operations checklist

Status: complete, 2026-10-18. Automated/source/capture verification and available software/wgpu mouse acceptance passed in all three chart modes.
Plan: [series-data-plan.md](series-data-plan.md). Baseline: `71c8419`.

## 1. Source contract — no dependencies

- [x] Read complete source components/dependencies; trace public API, model, custom owners and controllers.
- [x] Probe pop/query/whitespace/shared-time/range/numeric behavior with actual source and inspect original tests.
- [x] Pin source states, hashes and compatibility baseline; document native policies separately.
- [x] Freeze APIs/errors, owned-result design, alternatives, ownership and mutation/edit policy.

Verification: real-source oracle reproduces before production edits.

## 2. Built-in reads — depends on 1

- [x] Add failing data/style/time round-trip, directional lookup and logical-range tests.
- [x] Share existing pointer-event row conversion; implement canonical full/index/range queries.
- [x] Preserve all six built-in kinds, fractional/gap semantics and pure read behavior.

Verification: source query comparisons and pointer-event regressions pass.

## 3. Built-in pop — depends on 2

- [x] Add failing return-order, zero/empty/oversize, trailing-whitespace and update-after-pop regressions.
- [x] Expose existing DataLayer pop and apply the complete response with preflight validation.
- [x] Preserve series registration/owners and shared axis membership; protect no-ops and frozen frames.

Verification: focused public and existing model/data-layer tests pass.

## Checkpoint A

- [x] Review source contract, API and ownership; save tested local checkpoint.

## 4. Typed custom operations — depends on 3

- [x] Extend owned typed queries while preserving existing data_at and detached readable handles.
- [x] Implement custom pop using old indices before reindex; reject wrong-chart/detached mutations.
- [x] Verify whitespace, shared custom/built-in times, payload copies, view lifetime and conflation updates.

Verification: typed custom source comparisons and ownership/reentrancy tests pass.

## 5. Frontends and lifecycle — depends on 4

- [x] Support calendar/strike/maturity; explicitly forward yield mutation and protect private support.
- [x] Verify other-series reindexing, transferred/reordered owners, autoscale and canonical crosshair/hits.
- [x] Verify markers, pulses, price lines, stale edits and old frames after emptying/update/drop.

Verification: focused frontend/integration suites pass at dense/sparse and short/fractional sizes.

## 6. Demo — depends on 5

- [x] Add opt-in pop/append/reset and selected-data/index/range readouts to the existing generic host.
- [x] Include sparse/shared-time custom and built-in data; retain pane/order/editor controls.
- [x] Test controls, invalid selection, empty-series updates and recovery in all three modes.

Verification: example tests/build pass with clear returned-order and query readouts.

## 7. Capture and native acceptance — depends on 6

- [x] Freeze and pass 288 feature captures with actual backend assertions and 96 unchanged baseline comparisons.
- [x] Inspect representative software/GPU output and immutable old frames through removal/update/drop.
- [x] Obtain software mouse results for all modes/sizes, pop/append and short-height recovery.
- [x] Obtain matching wgpu results; retain and resolve any reported failure.

Verification: manifests, pixel comparisons and actual user responses support results.

## 8. Final verification — depends on 7

- [x] cargo fmt check, all-target tests, doctests, cargo check, strict Clippy and diff hygiene pass.
- [x] New source comparison and complete existing source comparison set pass unchanged.
- [x] Review diff/recheck consumers; complete porting report, README and evidence links.
- [x] Save local checkpoints; push only when requested.

## Feature acceptance

- [x] Public built-in/custom pop and canonical queries work across all three chart families.
- [x] Ownership, shared axis, data styles/time, search/range and lifecycle match the frozen contract.
- [x] Automated, capture and available mouse acceptance pass.
- [x] Prior drawing-order mouse acceptance, timing exceptions and unavailable-device checks retain their independent status.
