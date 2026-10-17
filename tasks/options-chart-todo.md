# Price-based horizontal chart checklist

Plan: [options-chart-plan.md](options-chart-plan.md). Baseline: `53ed228`.
Status: source contract, pure behavior and public API implemented; demo and acceptance in progress.

Each task needs a focused behavioral check and a compiling slice. Listed files
are likely boundaries, not a requirement to create parallel abstractions. Split
slices exceeding about five files. Dependencies are sequential unless stated.

## 1. Contract, oracle and baselines

Dependencies: none. Scope: oracle, reference test, source manifest and port report.
Likely files: scripts/check_options_chart_reference.cjs,
tests/options_chart_reference.rs, docs/porting/options-chart-source.json,
docs/porting/options-chart.md.

- [x] Read complete price behavior/dependencies and trace remaining formatter,
  data, options and event consumers; check coverage and read gaps directly.
- [x] Pin actual upstream working-tree hashes, preserve its local modifications,
  and execute actual TS behavior rather than a rewritten expected algorithm.
- [x] Freeze cases/counts for identity, weights, toFixed default/range/rounding,
  large magnitudes, signed zero, irregular spacing and multi-series whitespace.
- [x] Characterize public factory defaults/options replacement using the actual
  source construction path; distinguish narrow stubs from browser integration.
- [x] Record finite-value validation and integer precision policy separately from
  source coercions; choose formatting implementation from measured parity cases.
- [x] Save 24 calendar and 24 yield pre-change images with binary/source/font
  identity; freeze the feature capture protocol and counts.

Gate A: executable source expectations and recoverable baselines exist before
production changes. A formatting mismatch stays open until explained and resolved.

## 2a. Price item and pure behavior

Dependencies: 1. Scope: 3–5 files.
Likely files: src/model/horz_scale_behavior_price.rs, model/mod.rs,
precision formatter helper if needed, focused behavior/reference tests.

- [x] Write failing behavior tests, then implement finite HorizontalPrice,
  signed-zero normalization, consistent Eq/Ord/Hash and identity conversion.
- [x] Implement precision validation/defaults, source-compatible item/tick labels,
  exact weight predicates and incremental append behavior.
- [x] Test negative/fractional/large values, partial weight filling, empty mark
  handling and replacement/append equivalence without unnecessary allocations.

Verification: pure tests and actual-source comparisons pass; no renderer dependency.

## 2b. Public behavior/error integration

Dependencies: 2a. Scope: 3–5 files.
Likely files: src/api/behavior.rs, chart.rs error variants, api/options_chart.rs,
api/mod.rs, constructor compatibility test (split exports if necessary).

- [x] Compare alias plus named constructor against wrapper; record final API and
  ownership. Prove calendar Chart::new and YieldCurveChart callers compile.
- [x] Implement ChartBehavior validation/error hooks and typed public construction
  with ordinary chart defaults and full chart/localization override access.
- [x] Replace horizontal options transactionally through the shared model/data
  configuration path; invalidate labels without altering vertical price format.

Verification: one public Line prepares numeric labels/events; precision changes
update both behavior owners and cached labels; invalid options preserve old state.

## 3. Shared feature integration

Dependencies: 2b. Scope: 2–4 tests/oracle files per slice; production changes only
where a failing contract test proves a missing generic boundary.

- [ ] Verify all six built-in families, multiple panes/series, fit/ranges,
  coordinate round trips and source index spacing with irregular/missing strikes.
- [ ] Verify set/latest/historical/whitespace updates, duplicates/order errors,
  empty/reset/remove recovery and old-frame independence.
- [ ] Verify typed pointer/crosshair data, annotation and managed markers, manual
  marker persistence, explicit-clock idle expiry and historical marker behavior.
- [ ] Verify drawing/edit contexts, watermarks, appropriate price pulses and
  existing interactions through the shared widget. Do not copy yield scaffolding.
- [ ] Check precision refresh, hover and same-domain updates avoid full-history
  work where the existing incremental paths support it.

Gate B: API, model semantics and extensions pass focused comparisons; no second
engine/event loop and no new async or numeric custom-series framework.

## 4a. Interactive example

Dependencies: 3. Scope: 3–5 files.
Likely files: examples/options_chart.rs, examples/options_chart/main.rs,
Cargo.toml example entry if needed, README.md, example test/fixture.

- [ ] Build a strike-price Line + Histogram demo using existing support/runtime.
- [ ] Expose precision, latest/history changes, reset/empty and feed/pulse controls;
  include representative markers, watermarks and editable box.
- [ ] Keep demo controls usable at narrow widths; numeric labels stay distinct
  from vertical price formatting and no option-pricing functionality is implied.

Verification: all-target compile; deterministic preparation fixture and basic
startup on explicit software/GPU backends.

## 4b. Frozen backend captures

Dependencies: 4a and frozen protocol from 1. Scope: capture helper, fixture module,
protocol/manifest and focused pixel assertions; reuse existing capture machinery.

- [ ] Capture proposed 120 feature images across 2 backends, 4 DPR values, 3 sizes
  and 5 frozen scenes; check dimensions, clipping, spacing and labels.
- [ ] Compare 24 calendar and 24 yield baseline images unchanged; inspect key
  feature scenes, empty recovery and frozen frames after mutation/drop.
- [ ] Record source/binary/fixture/font hashes, actual backend and exact outcomes.

Gate C: no unexplained source or native compatibility failures; preserve failures
rather than modifying expected counts or thresholds after a failed run.

## 5. Acceptance and final report

Dependencies: 4b. Scope: docs/report/checklist; fixes return to the relevant task.

- [ ] Run cargo fmt --check, cargo test --all-targets, cargo check --all-targets,
  cargo clippy --all-targets -- -D warnings and applicable source oracles.
- [ ] Ask for actual software mouse checks at regular/narrow/problem sizes:
  zoom/pan, price-axis wheel/drag, numeric crosshair, precision/live updates,
  box move/resize/Escape, idle markers, pulse visibility and short-height recovery.
- [ ] Repeat physical checks with ICED_BACKEND=wgpu; record user results separately.
- [ ] Write source/consumer contract, discarded TS conventions, chosen ownership,
  alternatives, exact files, tests/check results and remaining dependencies in the
  port report. Document public constructors and horizontal precision usage.
- [ ] Keep historical timing exceptions and untested touchpad/touchscreen/high-DPI
  physical checks explicit. Do not mark them resolved by this feature.

Gate D: planned behavior, source comparisons, backend regressions and recorded
mouse acceptance pass. Push only when requested separately.
