# Yield-curve implementation checklist

Plan: [yield-curve-plan.md](yield-curve-plan.md). Baseline `24bd3ba`.
Status: complete for planned scope; automated and software/GPU mouse acceptance pass. Each task needs a focused behavioral check and a compiling commit.
File names below are expected boundaries, not instructions to create parallel
abstractions. Keep each compiling slice within roughly 3–5 files; split further
when trait/type changes cross more callers. No checkpoint requires extra permission.

## 0. Freeze the source contract

Dependencies: none. Scope: 3–4 oracle/report files.
Likely files: scripts/check_yield_curve_reference.cjs,
docs/porting/yield-curve-source.json, docs/porting/yield-curve.md, reference tests.

- [x] Pin actual TS working-tree hashes, resolve direct/indirect consumers and read
  coverage gaps. Preserve existing modified source checkout.
- [x] Execute scaffold/default/formatter/weighting/mutation sequences with controlled
  microtask flushing; characterize off-grid, fractional/negative values, reset and
  removal, options updates and self-influenced extents before finalizing policy.
- [x] Freeze case counts, native validation/correction expectations, scaffold resource
  bound and 24 pre-feature calendar captures with verified binary identity.

Verification: actual TS oracle runs; every contract row has an explicit expected case.

## 1a. Introduce the narrow behavior-neutral boundary

Dependencies: 0. Scope: 3–5 files, then follow-up compiling slices if needed.
Likely files: src/api/chart.rs, model/ihorz_scale_behavior.rs, model/data_layer.rs,
API errors/validation helper, compatibility test.

- [x] Compare shared generic core versus separate frontend; settle ownership,
  key/error hooks and typed frontend signatures from real consumer mappings.
- [x] Separate calendar conversion from common row validation without relaxing
  transactional replacement/order/value checks; keep calendar constructors/types usable.

Verification: compile existing Chart callers and run focused data/model tests.

## 1b. Reuse generic preparation through the core

Dependencies: 1a. Scope: 3–5 files per compiling slice.
Likely files: src/api/chart.rs, src/ui/chart_frame.rs, multi_pane_frame.rs,
API exports/core module, preparation compatibility test.

- [x] Thread behavior through only the state/preparation boundary that needs it;
  reuse existing generic model/frame owners and owned axis strings/geometry.
- [x] Preserve retained-frame identity, invalidation, axis stabilization and calendar
  aliases; avoid renaming unrelated TimeAxis renderer types just for terminology.

Verification: calendar frames/default pixels unchanged; existing rendering tests pass.

## 1c. Preserve typed events and the widget

Dependencies: 1b. Scope: 3–5 files per compiling slice.
Likely files: src/api/interaction.rs, widget.rs, touch.rs, UI hit payloads/tests.

- [x] Make item-dependent event payloads support numeric maturities with calendar
  defaults; keep geometric gesture messages and navigation logic shared.
- [x] Compile the existing Iced element/update/subscription examples without a second
  event loop, runtime or copied mouse/touch controller.

Verification: existing interaction/widget/touch tests and examples compile unchanged.

## 1d. Keep extension compatibility

Dependencies: 1c. Scope: 3–5 files per compiling slice.
Likely files: src/api/primitives.rs, primitive_editing.rs, custom_series.rs,
text/image watermark implementations and external-implementation compile tests.

- [x] Preserve ordinary calendar DrawingPrimitive implementations and typed custom
  series on the calendar specialization; make numeric preparation possible safely.
- [x] Reuse viewport watermarks and frozen primitive geometry; avoid Rc cycles or
  retaining mutable chart state through prepared frames.

Verification: existing primitive/custom/watermark tests; compile representative old API implementations.

## Gate A — compatibility boundary

- [x] All existing examples compile and affected calendar source comparisons pass.
- [x] Default calendar pixels are unchanged; final public/ownership/error design recorded.

## 2. Implement numeric maturity behavior

Dependencies: Gate A. Scope: 3–5 files.
Likely files: model/yield_curve_horz_scale_behavior module, model export,
API numeric options/types, focused tests.

- [x] Add validated maturity/key semantics, source label formatting, weights and
  options; use localization callbacks without calendar conversion.
- [x] Add pure scaffold-state/count/key generation, hash/equality and dirty-work
  coalescing rules; reject unsafe counts before allocation/mutation.

Verification: source oracle cases and deterministic invalid/empty/boundary tests pass.

## 3a. Deliver one public Line yield curve

Dependencies: 2. Scope: 3–5 files per compiling slice.
Likely files: api/yield_curve.rs, API exports/core hooks, numeric fixture test.

- [x] Expose YieldCurveChart with one Line, source defaults and private support
  ownership; record visible-output RED then prove the native curve/axis is drawn.
- [x] Settle multiple writes synchronously, hide scaffold ownership from public
  events/iteration and keep minimum-domain behavior source-compatible.

Verification: one sparse curve matches source coordinates, labels, axes and defaults.

## 3b. Complete data/domain lifecycle

Dependencies: 3a. Scope: 3–5 files.
Likely files: yield API/scaffold module, options handling and lifecycle tests.

- [x] Add Area/multiple curves, historical/latest/append/whitespace/full reset,
  longest-owner removal, empty recovery and option/default overrides.
- [x] Preserve error atomicity and old frames; support only promised built-in types
  and keep scaffold ownership inaccessible to public removal.

Verification: replay the upstream graphics/coverage update sequences and contract oracle.

## Gate B — source-compatible curves

- [x] Line/Area domain, mutation and formatting sequences pass against settled TS.
- [x] Calendar regressions remain green; unsupported numeric custom-series scope documented.

## 4a. Complete numeric interaction

Dependencies: Gate B. Scope: 3–5 files.
Likely files: numeric API event bridge, interaction/hit/grid membership and tests.

- [x] Return numeric maturity/item data from pointer events and crosshair labels;
  support points never become meaningful hover/grid selections by default.
- [x] Reuse wheel/diagonal/price-axis navigation, multi-pane cancellation, resize
  and short-height recovery. Honor explicit whitespace/default overrides.

Verification: numeric event/exclusion RED→GREEN plus existing interaction tests.

## 4b. Add numeric up/down markers

Dependencies: 4a. Scope: 3–5 files per compiling slice.
Likely files: api/up_down_markers.rs, private marker tests, yield API bridge,
marker annotations/time-item validation, numeric integration tests.

- [x] Reuse typed marker data with numeric keys and managed seed/update commands;
  keep seed-relative signs, manual persistence, options and transactional errors.
- [x] Prove exact/idle expiry and pulse fast-path coexistence through shared Tick/
  widget scheduling, including controller/series removal and frozen frames.

Verification: numeric idle-expiry RED→GREEN; existing time marker oracle remains unchanged.

## 4c. Verify overlays and bounded work

Dependencies: 4b. Scope: 2–4 test/instrumentation files.
Likely files: numeric integration tests, software_canvas_damage.rs, narrow work counters.

- [x] Verify numeric curve markers, watermarks, price lines/pulse and an editable
  box through the chosen primitive boundary; no new drawing-tool feature.
- [x] Assert zero scaffold rebuilds on hover/pan/zoom/same-domain value updates,
  bounded batched rebuilds, no stale storage and one software damage region.

Verification: deterministic work/storage/damage checks; no new broad timing campaign.

## Gate C — integrated numeric chart

- [x] Numeric navigation/events, update markers, overlays and bounded work pass.

## 5. Add the interactive demo and frozen fixtures

Dependencies: Gate C. Scope: 3–5 files.
Likely files: examples/yield_curve.rs, small example helper/capture module, README,
scripts/capture_yield_curve.py.

- [x] Show two Line/Area curves and controls for historical changes, extension/reset,
  domain/formatter changes, idle/manual markers and live-feed pause.
- [x] Run 120 feature captures and 24 unchanged calendar default-off comparisons
  with actual backend assertions, frozen state, hashes and retained-frame checks.
- [x] Inspect label/spacing/clipping/mixed examples; retain failures and their cause.

Verification: finite capture runner succeeds on both renderers; demo controls have focused tests.

## 6. Validate and record native acceptance

Dependencies: 5. Scope: validation plus 3–4 docs/evidence files.

- [x] Pass fmt, all-target tests/check/strict Clippy and yield/affected source oracles.
- [x] Review against the contract and real consumers; record exact API/files,
  ownership, alternatives, deliberate source corrections and compatibility limits.
- [x] Complete mouse checks at regular/narrow/problem sizes, short-height recovery,
  numeric selection and idle/manual marker behavior on explicitly selected software
  and GPU windows; retain other physical device/timing gaps separately.

Verification: docs/porting/yield-curve.md, source manifest/evidence and this checklist reflect actual results.

## Gate D — completion

- [x] Feature, compatibility, source comparisons and native mouse gates pass.
- [x] Unsupported features/device gaps and historical timing status are explicit.
- [x] Changes are reviewable local commits; no push without user instruction.

## Execution record

- `6fabf77`: pinned actual-source contract and initial oracle.
- `5416b2e`, `1136375`: compiling calendar-compatible type/storage slices.
- `835d760`: shared owner methods, numeric maturity behavior and yield frontend.
- Source oracle: 20 labels/weights/custom labels, eight scaffolds, nine lifecycle
  states. Constructor boundary also runs actual factory/YieldChartApi with a narrow
  base-chart stub; actual DataLayer is used for lifecycle/index state comparisons.
- The oracle startup fixture was corrected to seed the scaffold hash before the
  first microtask, matching the real constructor. Counts/keys are unchanged.
- Calendar compatibility: 654 existing tests passed after generic owner changes;
  initial software/GPU calendar pixel matrix passed, final matrix passed.
- Numeric data/events/markers/work checks, box move/resize/Escape routing and
  one-region software damage pass. Headless snapshot expiry and support-owner
  percent formatting have behavioral REDs and fixes; final validation passed.
- First numeric capture failed because multi_frame() only returns a multi-pane
  frame; fixture now uses frame(). Logs retained under target/yield-curve/after.
- Final validation: 668 passing tests in 62 suites, zero failures/ignored; format,
  check and strict Clippy pass. Final captures: 120 numeric images and 24 unchanged
  calendar images on actual software/GPU backends; source hashes match.
- Software/GPU mouse acceptance passed at all requested sizes, including idle
  expiry, manual persistence, editing/cancel and short-height recovery. Both
  windows exited normally. Other devices remain untested and historical timing
  acceptance remains OPEN/deferred.
