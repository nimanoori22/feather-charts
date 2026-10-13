# Area, Bar and Baseline: implementation checklist

Status: in progress; Area delivered, Bar next. Design: [built-in-series-plan.md](built-in-series-plan.md).
Execute in order; keep each slice buildable and split beyond roughly five production
files. Paths below are likely targets, not permission to rewrite adjacent systems.
Existing price-line and interaction hardware acceptance remains separately pending.

## 0. Area contract and fill design

Dependencies: none. Scope: medium investigation/documentation slice.

- [x] Read complete Area/shared-path/gradient components, trace consumers and source tests; record coverage fallbacks.
- [x] Compare fill/path designs, verify exact Iced gradient APIs and record ownership/internal signatures in `docs/porting/area-rendering.md`.
- [x] Define source behavior for styles, inversion, relative gradients, whitespace, markers, validation and autoscale.

Files: new Area port report; relevant source/Iced files are read-only evidence.
Verify: contract maps to public/model/frame consumers; gradient representation can
preserve stop ordering, alpha and style boundaries. Resolve feasibility before coding.

## 1. Area render commands

Dependencies: 0. Scope: medium.

- [x] Add minimal owned fill/gradient commands and source-compatible area closure/path traversal.
- [x] Preserve independent fill/line row styles, simple/stepped/curved paths, single-point width and inversion.
- [x] Add focused geometry/gradient oracle cases executing actual source, including fractional ratios and degenerate ranges.

Files: `src/renderers/area_renderer.rs` (new), existing path helper/exports,
new `examples/area_reference.rs`, `scripts/check_area_reference.cjs`.
Verify: focused renderer tests and oracle pass; existing line oracle still passes.

## 2. Area model preparation

Dependencies: 1. Scope: medium.

- [x] Prepare typed projected Area items from existing data/options/visible range without duplicate storage.
- [x] Preserve source color overrides, relative gradient bounds and visibility/empty invalidation.
- [x] Test updates/options/scale projection and document the supported option boundary.

Files: `src/model/series/area_pane_view.rs` (new), `src/model/series.rs`,
`src/model/chart_model.rs`, focused model tests.
Verify: focused tests and all-target check; projected items match source fixtures.

## 3. Area frame adaptation

Dependencies: 2. Scope: medium.

- [x] Convert owned fill/stroke commands to Iced once with finite geometry and correct ratio conversion.
- [x] Add the Area pane-view branch to single/composed frame paths; preserve fill/outline/decoration layering.
- [x] Test mixed series, clipping, repeatable drawing and empty/hidden output.

Files: `src/ui/line_chart.rs`, `src/ui/multi_pane_frame.rs`,
`src/ui/chart_frame.rs` only if needed, focused frame tests.
Verify: frame tests/all-target check and initial TinySkia/WGPU gradient captures.

## 4. Area public and interaction integration

Dependencies: 3. Scope: medium; split decoration and input wiring if needed.

- [x] Enable Area creation through Chart, retaining typed validation/data lifecycle and non-destructive invalid replacements.
- [x] Extend last-value color/labels, automatic/custom lines, crosshair markers, magnet and exact event rows wherever current matches need it.
- [x] Test public creation/update/history/clear/reload/removal and interaction without regressing existing types.

Files: `src/api/chart.rs`, `src/model/series.rs`,
`src/model/chart_model/price_lines.rs`, actual crosshair/magnet consumers,
`tests/chart_api.rs`. Locate consumers before editing; subdivide rather than exceed scope.
Verify: focused API/interaction tests and affected source comparisons pass.

## Checkpoint A: usable Area

- [x] Area works through a public example, including gradients/inversion, live changes and existing price labels/levels.
- [x] Formatting/check, focused tests/oracles and both-backend captures at 1/1.25/1.5/2 pass; native startup succeeds.
- [x] Recheck consumers, record results/limits in Area report and commit buildable slices.

## 5. Bar contract

Dependencies: checkpoint A for delivery order. Scope: small investigation.

- [ ] Trace Bar pane/renderer/colorer/API consumers and tests, including OHLC projection and autoscale.
- [ ] Record width/parity/tick rules, row color precedence, option validation and empty/hidden behavior.
- [ ] Compare shared OHLC preparation versus a dedicated view; record chosen design/API/ownership before edits.

Files: new `docs/porting/bar-rendering.md`; source files are read-only evidence.
Verify: distinguish Bar geometry from candlesticks; every exposed style has a testable contract.

## 6. Bar geometry and preparation

Dependencies: 5. Scope: two medium slices, geometry first then model preparation.

- [ ] Implement/test source Bar stem/tick geometry, width parity, open visibility and thin/compressed bars.
- [ ] Prepare Bar OHLC/colors using existing data and scale machinery with correct lifecycle invalidation.
- [ ] Add source comparisons for zero-height/inverted/fractional/compressed cases and overrides.

Geometry files: new `src/renderers/bars_renderer.rs`, renderer exports,
`examples/bar_reference.rs`, `scripts/check_bar_reference.cjs`.
Preparation files: new `src/model/series/bars_pane_view.rs`, `src/model/series.rs`,
`src/model/chart_model.rs`, focused model tests.
Verify each slice: focused tests/oracle and all-target check.

## 7. Bar complete integration

Dependencies: 6. Scope: two medium slices, frame adaptation then public behavior.

- [ ] Adapt Bar rectangles through the shared immutable frame path without duplicating composed geometry.
- [ ] Enable public Bar data/options/lifecycle and source-compatible last values, labels, levels and crosshair/magnet/events.
- [ ] Test mixed Bar/candlestick panes, live/history/clear/hidden/removal and existing navigation.

Frame files: `src/ui/line_chart.rs`, `src/ui/multi_pane_frame.rs`, focused frame tests.
API/behavior files: `src/api/chart.rs`, `src/model/series.rs`, actual affected
decoration/crosshair consumers, `tests/chart_api.rs`; subdivide as necessary.
Verify: focused public/frame tests and affected existing source comparisons.

## Checkpoint B: usable Bar

- [ ] Public Bar example demonstrates open/thin options, color overrides, live data and price decorations.
- [ ] Formatting/check, focused tests/oracles, fractional captures on both backends and native startup pass.
- [ ] Bar report records contract/design/files/results and verified scope; Area remains green.

## 8. Baseline contract

Dependencies: checkpoint B; Area fill infrastructure. Scope: medium investigation.

- [ ] Read/trace Baseline area/line/gradient/colorer/model consumers and original tests.
- [ ] Record base projection/autoscale, gradient discontinuities, relative bounds, equality/offscreen cases and row overrides.
- [ ] Compare gradient strokes with any viable clipping/splitting design against curves/steps and exact Iced support; record ownership/API before coding.

Files: new `docs/porting/baseline-rendering.md`; relevant source/Iced files read-only.
Verify: selected design handles crossings without assigning a whole segment one endpoint's color.

## 9. Baseline render commands

Dependencies: 8. Scope: medium.

- [ ] Reuse proven fill/path commands for closure at the projected base and source top/bottom fill gradients.
- [ ] Preserve line-gradient discontinuity at the base, row overrides, dash continuity and supported markers.
- [ ] Add actual-source oracles for crossings, equality, base outside pane, relative/degenerate ranges and fractional ratios.

Files: new Baseline renderer module(s), minimal shared gradient/path changes,
new `examples/baseline_reference.rs`, `scripts/check_baseline_reference.cjs`.
Verify: focused renderer tests/oracle; existing Area/line comparisons still pass.

## 10. Baseline model preparation

Dependencies: 9. Scope: medium.

- [ ] Project existing Baseline data/base through the associated scale and visible first-value rules.
- [ ] Preserve source autoscale participation, gradient bounds, colors and invalidation on data/base/options changes.
- [ ] Test normal/log/percentage/indexed/inverted/manual projection and hidden/empty/reload states.

Files: new `src/model/series/baseline_pane_view.rs`, `src/model/series.rs`,
`src/model/chart_model.rs`, focused model tests.
Verify: source projection/autoscale fixtures and focused tests/all-target check.

## 11. Baseline complete integration

Dependencies: 10. Scope: two medium slices, frame adaptation then public behavior.

- [ ] Integrate prepared Baseline fills/strokes in single/multi-pane frames with correct layering/clipping.
- [ ] Enable public lifecycle/options and correct last-value colors, labels, levels, crosshair/magnet/events.
- [ ] Test live/history/base changes, multiple scales, clear/hidden/removal and repeated immutable frames.

Frame files: existing `src/ui/line_chart.rs`, `src/ui/multi_pane_frame.rs`, tests.
Public files: `src/api/chart.rs`, `src/model/series.rs`, affected decoration/input
consumers, `tests/chart_api.rs`; subdivide if more than roughly five production files.
Verify: focused API/frame tests and affected source comparisons.

## Checkpoint C: usable Baseline

- [ ] Public Baseline demo shows above/below styles and base changes with working interaction/decorations.
- [ ] Formatting/check, focused tests/oracles, both-backend fractional captures and native startup pass.
- [ ] Baseline report records results/limits; Area, Bar and previous series remain green.

## 12. Public demonstration and documentation

Dependencies: checkpoint C. Scope: medium.

- [ ] Extend a public example to exercise all six built-in families and live/options/visibility/level controls.
- [ ] Provide reproducible mixed-pane, row-style, inversion, compressed-Bar and Baseline-crossing capture scenarios.
- [ ] Update README supported scope and finish each family report with exact files, contracts, ownership, alternatives and evidence.

Files: `examples/public_chart.rs` or a focused public example using Chart,
existing capture support where suitable, `README.md`, family reports.
Verify: headless smoke plus bounded native startup; host controls stay outside the engine.

## 13. Integrated verification

Dependencies: 12. Scope: medium verification/report slice.

- [ ] Run full formatting/tests/check/strict Clippy/example builds/diff checks and all new/affected source oracles.
- [ ] Inspect TinySkia/WGPU captures at ratios 1/1.25/1.5/2 for gradients, alpha, crossings, ticks, clipping, layers and repeatability.
- [ ] Compare identical pre/post navigation/data-update workloads and glyph preparation; investigate regressions and record limits.

Files: focused fixtures if gaps emerge, family reports and this checklist.
Verify: commands in the plan pass with recorded results; no skipped assertions or weakened existing checks.

## Final checkpoint

- [ ] Area, Bar and Baseline each work through the public API with all in-scope behaviors.
- [ ] Automated/native evidence and reports are complete; implementation saved in buildable local commits.
- [ ] Complete fresh physical mouse/touchpad checks: zoom/pan/axis scaling, crosshair, live updates, changing styles/base and crowded labels.

Automated completion and physical acceptance are reported separately. Unavailable
hardware stays unchecked; prior milestones' acceptance does not substitute for this one.
No automatic GitHub push is part of this roadmap.
