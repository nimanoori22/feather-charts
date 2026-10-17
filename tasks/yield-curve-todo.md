# Yield-curve implementation checklist

Plan: [yield-curve-plan.md](yield-curve-plan.md). Baseline `24bd3ba`.
Status: planned. Each task needs a focused behavioral check and a compiling commit.
File names below are expected boundaries, not instructions to create parallel
abstractions. Keep each compiling slice within roughly 3–5 files; split further
when trait/type changes cross more callers. No checkpoint requires extra permission.

## 0. Freeze the source contract

Dependencies: none. Scope: 3–4 oracle/report files.
Likely files: scripts/check_yield_curve_reference.cjs,
docs/porting/yield-curve-source.json, docs/porting/yield-curve.md, reference tests.

- [ ] Pin actual TS working-tree hashes, resolve direct/indirect consumers and read
  coverage gaps. Preserve existing modified source checkout.
- [ ] Execute scaffold/default/formatter/weighting/mutation sequences with controlled
  microtask flushing; characterize off-grid, fractional/negative values, reset and
  removal, options updates and self-influenced extents before finalizing policy.
- [ ] Freeze case counts, native validation/correction expectations, scaffold resource
  bound and 24 pre-feature calendar captures with verified binary identity.

Verification: actual TS oracle runs; every contract row has an explicit expected case.

## 1a. Introduce the narrow behavior-neutral boundary

Dependencies: 0. Scope: 3–5 files, then follow-up compiling slices if needed.
Likely files: src/api/chart.rs, model/ihorz_scale_behavior.rs, model/data_layer.rs,
API errors/validation helper, compatibility test.

- [ ] Compare shared generic core versus separate frontend; settle ownership,
  key/error hooks and typed frontend signatures from real consumer mappings.
- [ ] Separate calendar conversion from common row validation without relaxing
  transactional replacement/order/value checks; keep calendar constructors/types usable.

Verification: compile existing Chart callers and run focused data/model tests.

## 1b. Reuse generic preparation through the core

Dependencies: 1a. Scope: 3–5 files per compiling slice.
Likely files: src/api/chart.rs, src/ui/chart_frame.rs, multi_pane_frame.rs,
API exports/core module, preparation compatibility test.

- [ ] Thread behavior through only the state/preparation boundary that needs it;
  reuse existing generic model/frame owners and owned axis strings/geometry.
- [ ] Preserve retained-frame identity, invalidation, axis stabilization and calendar
  aliases; avoid renaming unrelated TimeAxis renderer types just for terminology.

Verification: calendar frames/default pixels unchanged; existing rendering tests pass.

## 1c. Preserve typed events and the widget

Dependencies: 1b. Scope: 3–5 files per compiling slice.
Likely files: src/api/interaction.rs, widget.rs, touch.rs, UI hit payloads/tests.

- [ ] Make item-dependent event payloads support numeric maturities with calendar
  defaults; keep geometric gesture messages and navigation logic shared.
- [ ] Compile the existing Iced element/update/subscription examples without a second
  event loop, runtime or copied mouse/touch controller.

Verification: existing interaction/widget/touch tests and examples compile unchanged.

## 1d. Keep extension compatibility

Dependencies: 1c. Scope: 3–5 files per compiling slice.
Likely files: src/api/primitives.rs, primitive_editing.rs, custom_series.rs,
text/image watermark implementations and external-implementation compile tests.

- [ ] Preserve ordinary calendar DrawingPrimitive implementations and typed custom
  series on the calendar specialization; make numeric preparation possible safely.
- [ ] Reuse viewport watermarks and frozen primitive geometry; avoid Rc cycles or
  retaining mutable chart state through prepared frames.

Verification: existing primitive/custom/watermark tests; compile representative old API implementations.

## Gate A — compatibility boundary

- [ ] All existing examples compile and affected calendar source comparisons pass.
- [ ] Default calendar pixels are unchanged; final public/ownership/error design recorded.

## 2. Implement numeric maturity behavior

Dependencies: Gate A. Scope: 3–5 files.
Likely files: model/yield_curve_horz_scale_behavior module, model export,
API numeric options/types, focused tests.

- [ ] Add validated maturity/key semantics, source label formatting, weights and
  options; use localization callbacks without calendar conversion.
- [ ] Add pure scaffold-state/count/key generation, hash/equality and dirty-work
  coalescing rules; reject unsafe counts before allocation/mutation.

Verification: source oracle cases and deterministic invalid/empty/boundary tests pass.

## 3a. Deliver one public Line yield curve

Dependencies: 2. Scope: 3–5 files per compiling slice.
Likely files: api/yield_curve.rs, API exports/core hooks, numeric fixture test.

- [ ] Expose YieldCurveChart with one Line, source defaults and private support
  ownership; record visible-output RED then prove the native curve/axis is drawn.
- [ ] Settle multiple writes synchronously, hide scaffold ownership from public
  events/iteration and keep minimum-domain behavior source-compatible.

Verification: one sparse curve matches source coordinates, labels, axes and defaults.

## 3b. Complete data/domain lifecycle

Dependencies: 3a. Scope: 3–5 files.
Likely files: yield API/scaffold module, options handling and lifecycle tests.

- [ ] Add Area/multiple curves, historical/latest/append/whitespace/full reset,
  longest-owner removal, empty recovery and option/default overrides.
- [ ] Preserve error atomicity and old frames; support only promised built-in types
  and keep scaffold ownership inaccessible to public removal.

Verification: replay the upstream graphics/coverage update sequences and contract oracle.

## Gate B — source-compatible curves

- [ ] Line/Area domain, mutation and formatting sequences pass against settled TS.
- [ ] Calendar regressions remain green; unsupported numeric custom-series scope documented.

## 4a. Complete numeric interaction

Dependencies: Gate B. Scope: 3–5 files.
Likely files: numeric API event bridge, interaction/hit/grid membership and tests.

- [ ] Return numeric maturity/item data from pointer events and crosshair labels;
  support points never become meaningful hover/grid selections by default.
- [ ] Reuse wheel/diagonal/price-axis navigation, multi-pane cancellation, resize
  and short-height recovery. Honor explicit whitespace/default overrides.

Verification: numeric event/exclusion RED→GREEN plus existing interaction tests.

## 4b. Add numeric up/down markers

Dependencies: 4a. Scope: 3–5 files per compiling slice.
Likely files: api/up_down_markers.rs, private marker tests, yield API bridge,
marker annotations/time-item validation, numeric integration tests.

- [ ] Reuse typed marker data with numeric keys and managed seed/update commands;
  keep seed-relative signs, manual persistence, options and transactional errors.
- [ ] Prove exact/idle expiry and pulse fast-path coexistence through shared Tick/
  widget scheduling, including controller/series removal and frozen frames.

Verification: numeric idle-expiry RED→GREEN; existing time marker oracle remains unchanged.

## 4c. Verify overlays and bounded work

Dependencies: 4b. Scope: 2–4 test/instrumentation files.
Likely files: numeric integration tests, software_canvas_damage.rs, narrow work counters.

- [ ] Verify numeric curve markers, watermarks, price lines/pulse and an editable
  box through the chosen primitive boundary; no new drawing-tool feature.
- [ ] Assert zero scaffold rebuilds on hover/pan/zoom/same-domain value updates,
  bounded batched rebuilds, no stale storage and one software damage region.

Verification: deterministic work/storage/damage checks; no new broad timing campaign.

## Gate C — integrated numeric chart

- [ ] Numeric navigation/events, update markers, overlays and bounded work pass.

## 5. Add the interactive demo and frozen fixtures

Dependencies: Gate C. Scope: 3–5 files.
Likely files: examples/yield_curve.rs, small example helper/capture module, README,
scripts/capture_yield_curve.py.

- [ ] Show two Line/Area curves and controls for historical changes, extension/reset,
  domain/formatter changes, idle/manual markers and live-feed pause.
- [ ] Run 120 feature captures and 24 unchanged calendar default-off comparisons
  with actual backend assertions, frozen state, hashes and retained-frame checks.
- [ ] Inspect label/spacing/clipping/mixed examples; retain failures and their cause.

Verification: finite capture runner succeeds on both renderers; demo controls have focused tests.

## 6. Validate and record native acceptance

Dependencies: 5. Scope: validation plus 3–4 docs/evidence files.

- [ ] Pass fmt, all-target tests/check/strict Clippy and yield/affected source oracles.
- [ ] Review against the contract and real consumers; record exact API/files,
  ownership, alternatives, deliberate source corrections and compatibility limits.
- [ ] Complete mouse checks at regular/narrow/problem sizes, short-height recovery,
  numeric selection and idle/manual marker behavior on explicitly selected software
  and GPU windows; retain other physical device/timing gaps separately.

Verification: docs/porting/yield-curve.md, source manifest/evidence and this checklist reflect actual results.

## Gate D — completion

- [ ] Feature, compatibility, source comparisons and native mouse gates pass.
- [ ] Unsupported features/device gaps and historical timing status are explicit.
- [ ] Changes are reviewable local commits; no push without user instruction.
