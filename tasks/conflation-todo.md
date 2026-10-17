# Conflation execution checklist

Plan: [conflation-plan.md](conflation-plan.md). Starting checkpoint: `92d386d`.
Implementation and automated verification are complete. Mouse checks pass on both
backends after the dense software zoom fix. Historical timing acceptance stays OPEN.
Use the repository's source/consumer/design/test workflow before each port slice.

## 0. Finalize the behavioral contract

Dependencies: none. Scope: medium investigation; no production edits.
Likely files: `docs/porting/conflation-rendering.md`, this plan/checklist.

- [x] Finish full source/dependency reads, graph traces and coverage fallback;
  pin the local upstream revision plus file hashes and record consumer categories.
- [x] Resolve threshold/style/gap/custom-callback/update/precompute semantics,
  including actual factor >1 historical rejection versus mocked tests.
- [x] Compare ownership designs, settle private API signatures, map every real
  consumer and record raw/cached/frame lifetimes before implementation.

Verification: source audit supports each required behavior; unknowns are explicit.

## 1. Establish the source oracle and failing cases

Dependencies: 0. Scope: medium. Likely files: `scripts/check_conflation_reference.cjs`,
`examples/conflation_reference.rs`, focused fixtures/tests.

- [x] Run actual TS aggregation/factor/consumer behavior and persist its identity.
- [x] Cover factor 1, 2..512, non-power normalization, DPR/overrides, 0/1/3/7 rows,
  recursive remainders, style/gap identity and order-sensitive custom reducers.
- [x] Add RED cases for missing Rust aggregation/rendering and exact historical
  rejection boundaries; source discrepancies are recorded rather than guessed.

Verification: oracle self-checks pass; Rust failures demonstrate the missing behavior.

## Checkpoint A: Contract

- [x] Tasks 0–1 specify a testable, source-backed first Line slice and private API.

## 2. Implement the shared aggregation core

Dependencies: 1. Scope: medium. Likely files: `src/model/data_conflater.rs`,
`src/model/mod.rs`, `src/model/time_scale.rs`, focused tests/reference example.

- [x] Reuse factor calculation through one shared rule; support series overrides.
- [x] Aggregate OHLC/counts/identity with recursive remainder parity; canonical
  rows remain unchanged and factor 1 avoids needless copies.
- [x] Retain raw source spans needed for later style/interaction mapping.

Verification: core tests and actual-source oracle pass; format and library check.

## 3. Deliver dense Line rendering through the public chart

Dependencies: 2. Scope: medium. Likely files: `src/model/series/line_pane_view.rs`,
its model preparation boundary, `tests/conflation_line.rs`, `examples/public_chart.rs`.

- [x] Wire the selected cache ownership and shared core into cached/stateless
  preparation; replace Line's blanket rejection with supported behavior.
- [x] Match source extreme-value/tie/style rules, visible joins and curved neighbors;
  verify enabled/factor-1 and disabled output.
- [x] Provide an executable dense Line demo using existing options; counts and
  zoom/DPR transitions are observable without diagnostic engine code.

Verification: focused public/model tests, oracle and both-backend Line captures.

## 4. Make live updates and invalidation correct

Dependencies: 3. Scope: medium; split owner validation from cache repair if needed.
Likely files: `src/model/series.rs`, `src/model/data_conflater.rs`,
`src/api/chart.rs`, `tests/conflation_updates.rs` and the reference fixture.

- [x] Match warm and fresh results after latest replacement, append/remainder
  boundaries, reload/clear, reindexing, toggle, style changes and removal.
- [x] Reject prohibited historical changes before mutation at effective factor >1;
  retain historical updates at factor 1 and document any new public error variant.
- [x] Avoid warm full-history rebuilds; prove any tail repair and other-level
  invalidation, with correct fallback where chunk structure requires rebuilding.

Verification: mutation-sequence oracle and immutable-frame/unchanged-peer tests.

## Checkpoint B: First usable feature

- [x] Dense Line works through the public API and survives live feed/level changes.
- [x] Core, Line and update checks pass; unsupported families remain explicit.

## 5. Add Candlestick and Bar rendering

Dependencies: 4. Scope: medium; land Candlestick before its Bar sibling.
Likely files: the two pane views, focused OHLC tests/reference cases.

- [x] Preserve aggregate open/high/low/close and source bar/candle color lookup.
- [x] Verify wicks, borders, open ticks, inversion and visible boundary selection.

Verification: OHLC oracle, focused chart tests and both-backend captures.

## 6. Add Histogram rendering

Dependencies: 4. Scope: small/medium. Likely files: Histogram pane view and tests.

- [x] Match source representative values, colors, baseline and widths at all levels.
- [x] Preserve short-height recovery, independent volume scale and live updates.

Verification: Histogram oracle, resize regression and renderer captures.

## Checkpoint C: OHLC and volume

- [x] Tasks 5–6 pass source/rendering/update checks with Line and mixed panes.

## 7. Add Area rendering

Dependencies: 4. Scope: small/medium. Likely files: Area pane view and tests.

- [x] Reuse source line-family selection with fill/outline styles and curve neighbors.
- [x] Preserve gradient/inverted fill, row overrides and updates across levels.

Verification: Area source, focused regression and both-backend capture checks.

## 8. Add Baseline rendering

Dependencies: 7. Scope: small/medium. Likely files: Baseline pane view and tests.

- [x] Preserve source representative values and above/below-base fill/line behavior.
- [x] Verify base changes, curves, manual scales and mixed-series updates.

Verification: Baseline oracle and renderer/regression checks; all six built-ins work.

## 9. Integrate typed custom reducers

Dependencies: 4. Scope: medium. Likely files: `src/api/custom_series.rs`,
`src/model/icustom_series.rs`, core typed support and `tests/custom_conflation.rs`.

- [x] Invoke existing reducer/price builder with source contexts and reduction order;
  no-reducer views retain raw data and source renderer-factor semantics.
- [x] Verify singleton/remainder/replacement/removal, retained typed payloads and
  callback borrowing; update/historical validation matches the public contract.

Verification: custom source oracle, order-sensitive reducers and custom demo/captures.

## Checkpoint D: Series coverage

- [x] All built-ins and reducer/no-reducer custom series pass automated parity checks.

## 10. Verify interaction and data consumers

Dependencies: 5–9. Scope: medium regression slice; change shared code only if needed.
Likely files: focused integration tests, `src/ui/hit_testing.rs`, custom adapter
or affected crosshair/marker consumer identified by the contract.

- [x] Verify autoscale, first/last price, exact-index event data, magnet/hits/hover,
  markers, price lines, primitives and editing against their actual source paths.
- [x] Cover multi-pane zoom/DPR/level changes, live feed, clipping and retained frames;
  distinguish aggregate hit geometry from canonical data access.

Verification: integration/source regressions and mixed-scene backend captures.

## 11. Implement bounded optional precomputation

Dependencies: 10. Scope: medium; split core jobs from host scheduling if needed.
Likely files: cache/job owner, existing Chart scheduling adapter and focused tests.

- [x] Inspect exact Iced/Flowsurface scheduling APIs, then record native priority
  mapping while keeping engine work synchronous and per-step work bounded.
- [x] Honor options without stale results; cancel on generation/removal changes,
  bound storage and verify reducers cannot outlive their owners.

Verification: deterministic job lifecycle/work-bound tests and live-input checks.

## Checkpoint E: Complete behavior

- [x] Series, interactions and optional warmup satisfy recorded contracts.

## 12. Verify reduced work with a bounded performance study

Dependencies: 11. Scope: medium tooling/measurement slice using existing fixtures.
Likely files: `examples/preparation_profile.rs`, focused count/reference fixture
and a bounded runner/report where existing tools cannot be reused directly.

- [x] Prove exact aggregate/projection counts, no warm rebuild and bounded storage.
- [x] Freeze the plan's 24-run dense Line/Histogram protocol before measuring;
  record cold/warm/latest/append costs and all controls/raw evidence serially.
- [x] Retain failures/inconclusive timings; claim speed only beyond measured noise.
  Historical acceptance criteria/results are not altered or silently rerun.

Verification: artifact/sample/count self-checks; matched work and timing report.

## 13. Validate, review and document the delivered scope

Dependencies: 12. Scope: medium validation/documentation slice; no unrelated fixes.
Likely files: port report, README, this checklist and the dense demo instructions.

- [x] Pass fmt, all-target tests/check/strict Clippy, affected source scripts and
  backend captures; review ownership, memory, update and error paths.
- [x] Record physical live-feed/zoom/pan/axis checks at regular/narrow sizes on
  both backends; leave unavailable device coverage explicitly pending.
- [x] Publish source/consumer/API/design/alternative/test/performance results and
  remaining limits; distinguish implementation from hardware/timing acceptance.

Verification: reviewable report with exact commands/results and evidence links.

## Completion

- [x] Implementation tasks pass with no hidden unsupported paths or stale caches.
- [x] Automated parity/work checks pass; physical results and missing coverage recorded.
- [x] Historical timing exception remains separately tracked until its own policy passes.

## Execution evidence (2026-10-17)

See [the port report](../docs/porting/conflation-rendering.md) for the source
contract, design change to Series-owned caches, exact scope and limitations.
Final checks: 601 all-target tests, strict Clippy/check/fmt, source oracles, the
frozen 24-process performance batch, 192 byte-identical before/after native
renderer captures and 16 additional mixed-scene captures.
Physical mouse checks pass at regular/narrow sizes with live feed on TinySkia
and WGPU. The first software zoom failure was reproduced, profiled and resolved
with dependency-only dev build settings; canonical engine behavior is unchanged.

Hardware coverage outside this completed implementation checklist remains open:

- [ ] Touchpad physical acceptance.
- [ ] Touchscreen physical acceptance.
- [ ] High-DPI physical-device acceptance (simulated DPR checks passed).

The historical timing exception remains OPEN/deferred. This feature's successful
work reduction, timing comparison and mouse checks do not close that exception.
