# Numeric custom-series implementation checklist

Plan: [numeric-custom-series-plan.md](numeric-custom-series-plan.md).
Baseline: `2a32573`. Status: not started; planning documents only.
Use small compiling slices, focused behavioral tests and local checkpoint commits.
Likely paths below are boundaries to reuse, not mandatory new abstractions.

## 1. Freeze source behavior and compatibility evidence

Dependencies: none. Scope: 3–5 oracle/report/fixture files per slice.
Likely files: scripts/check_custom_series_reference.cjs,
scripts/check_yield_curve_reference.cjs, docs/porting/numeric-custom-series.md,
source manifest and numeric reference tests.

- [ ] Trace remaining actual custom renderer/data/reducer/event/lifecycle consumers,
  inspect upstream tests, check coverage and read gaps; pin working-tree hashes.
- [ ] Freeze numeric payload/logical index, defaults, union/whitespace, current-value,
  reducer-context and yield custom construction/settlement expectations. Separate
  native validation/bounds from source behavior; freeze case/work counts.
- [ ] Retain 24 baseline images for each of calendar built-ins, calendar custom,
  price built-ins and yield built-ins, with source/binary/fixture/font identity.
  Freeze the 240-image feature matrix before production edits.

Verification: source oracle executes actual TS files; expected outputs and old-family
fixtures are reproducible. No changed expected values to accommodate native defects.

## 2a. Generalize typed handle ownership

Dependencies: 1. Scope: 2–4 files.
Likely files: src/api/custom_series.rs, tests/public_custom_series.rs,
numeric compile/ownership test or compile-fail doctests.

- [ ] Compare associated-type-derived CustomSeriesHandle<V> with a defaulted
  behavior parameter; record final types and source-compatible signatures.
- [ ] Remove calendar-only handle storage/view bounds while preserving weak live
  view ownership, readable detached data, manual Clone/Debug and identity guards.
- [ ] Prove wrong-family views/data are rejected and old handle syntax compiles.

Verification: calendar custom tests/examples compile; drop/foreign/detached handle
checks pass. Keep still-calendar-only operations working during this slice.

## 2b. Generalize the existing cache and preparation owner

Dependencies: 2a. Scope: 3–5 files.
Likely files: src/api/custom_series.rs, custom_series/conflation.rs,
custom/conflation tests, src/ui/custom_series.rs only if a proven boundary needs it.

- [ ] Use actual item/internal row types in cache/jobs/reducer contexts and owner
  storage, with Clone bounds and the existing erased renderer interface.
- [ ] Preserve precompute budgets, fallback, original counts/colors, odd tails,
  invalidation and frozen renderer ownership; avoid a parallel numeric cache.
- [ ] Compile calendar callers and test source reducer contexts/grouping, including
  context-sensitive/non-associative cases through the actual consumer chain.

Verification: calendar custom/conflation oracles and work counters pass unchanged.

## 2c. Expose one OptionsChart custom path

Dependencies: 2b. Scope: 3–5 files.
Likely files: src/api/custom_series.rs, api/interaction.rs if needed,
tests/numeric_custom_series.rs, focused source tests and exports only if needed.

- [ ] Generalize registration, set/update/options methods onto Chart<B>; map errors
  through ChartBehavior and validate family requirements before mutation.
- [ ] Generalize PointerEvent<T> custom_data/custom_hit_data access with matching
  view items, preserving original payloads and separate renderer-hit data.
- [ ] Render one price-keyed custom series and verify autoscale, current value,
  exact-row events and logical spacing with a built-in peer.

Gate A: calendar constructors/examples/custom handles compile unchanged; one numeric
renderer works end to end without copied model/widget logic.
Verification: focused options/custom API tests and relevant source comparisons.

## 3. Complete price-chart mutations and dense rendering

Dependencies: 2c. Scope: 2–4 test/oracle files per slice; production edits only for
proven failures in the shared path.

- [ ] Cover latest/history/whitespace, ordering/duplicate/invalid writes, original
  time restoration, clear/remove/re-add, options, multiple panes/scales and exact
  payload/hit access. Rejected writes preserve canonical state and old frames.
- [ ] Cover reducer/missing-reducer behavior, dense/sparse transitions, cached tail
  updates, opt-out and invalidation during budgeted work, using actual TS output.
- [ ] Verify immutable old frames after mutation/removal/drop, exactly-once cleanup,
  mixed built-in overlays and bounded visible-copy/update work at 100,000 rows.

Gate B: numeric price behavior and calendar regressions pass; no strengthened
Send/Sync/Copy bounds or full-history hover/axis-drag work introduced.

## 4a. Protect yield custom mutation and settlement

Dependencies: 3. Scope: 3–5 files.
Likely files: src/api/yield_curve.rs, shared custom preflight if needed,
tests/numeric_custom_series.rs, yield/custom source reference tests.

- [ ] Add explicit custom registration/set/update/options forwarding with matching
  Maturity views; preserve readonly Deref and private support ownership.
- [ ] Preflight custom extent/resource bounds before canonical writes; successful
  domain mutations mark dirty and settle once. Failed writes preserve domain,
  caches/handles, deadlines and preparable existing frames.
- [ ] Verify custom-only growth, mixed curve/custom union, scaffold-only crosshair
  exclusion, batched writes and retained source extent on shorten/clear/remove.

Verification: actual-source settlement sequences and unsafe-extent rejection tests;
no scaffold rebuild on hover/axis drag or same-domain value-only writes.

## 4b. Preserve yield custom defaults and lifecycle

Dependencies: 4a. Scope: 2–4 test/report files plus narrowly required fixes.
Likely files: yield/custom tests, source oracle, src/api/yield_curve.rs if necessary.

- [ ] Compare inherited addCustomSeries with overridden addSeries; retain source
  custom common defaults, chart-wide yield formatting and explicit overrides.
- [ ] Verify yield price conversion/autoscale/hits/payloads and custom cleanup,
  frozen frames and handle guards alongside private support-series lifecycle.

Gate C: both numeric families pass the source contract and preserve old behavior.

## 5a. Deliver the numeric custom-renderer demo

Dependencies: 4b. Scope: 3–5 files.
Likely files: examples/numeric_custom_series.rs and a small support module,
existing custom renderer/demo support where reusable, README.md, focused demo tests.

- [ ] Use one range/distribution renderer with price and maturity modes, typed
  payloads, mixed built-in peer, live/history/reset/remove/re-add/options controls.
- [ ] Expose hover/hit data and dense/sparse behavior; keep narrow controls usable
  and reuse existing chart gesture/editing support without a second event loop.
- [ ] Verify demo geometry/options/short-height recovery and startup on both explicit
  native backends; no option valuation or unsupported custom animation plugins.

Verification: focused example tests, all-target compile and deterministic fixtures.

## 5b. Run frozen native regression captures

Dependencies: 5a and protocol frozen in 1. Scope: 3–5 capture/fixture/manifest files.
Likely files: numeric capture module/helper, reference fixtures and evidence report.

- [ ] Capture 240 feature images: two numeric modes, two backends, four DPR values,
  three sizes and five scenes; inspect spacing, labels, clipping and overlays.
- [ ] Compare all 96 baseline images exactly, and prove retained frames draw the
  same pixels after mutation/removal/drop. Save failures without weakening gates.
- [ ] Record final source/binary/fixture/font identity, counts and actual renderer.

Verification: capture runner exits successfully with all expected images/checks.

## 6. Validate, review and record physical acceptance

Dependencies: 5b. Scope: docs/checklist; fixes return to their implementation slice.

- [ ] Run cargo fmt --check, cargo test --all-targets, cargo check --all-targets,
  cargo clippy --all-targets -- -D warnings and affected actual-source oracles.
- [ ] Record software mouse acceptance for both numeric modes at regular/narrow/
  earlier problem sizes: controls, hover/payloads, dense zoom, diagonal pan,
  vertical axes, clear/reset/remove/re-add and short-height recovery; box editing
  and built-in peer overlays where exposed. Repeat explicitly on wgpu.
- [ ] Review compatibility/ownership/work bounds and write exact source/consumer
  contract, alternatives, discarded TS patterns, files, test/results and remaining
  dependencies. Keep unavailable device checks and historical timing status open.

Gate D: all planned source/automated/native/mouse checks pass with evidence. No push
unless separately requested; no checkpoint requires another implementation approval.
