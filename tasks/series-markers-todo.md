# Series markers: implementation checklist

Status: planned. Design: [series-markers-plan.md](series-markers-plan.md).
Execute sequentially; split any slice exceeding roughly five production files.
Paths are likely targets, to be confirmed by consumer analysis.

## 0. Source contract and design

Dependencies: none. Scope: medium investigation/report slice.

- [ ] Read complete plugin/helpers, trace attachment/data/autoscale/drawing/hit/event consumers with coverage checks and source fallback; inspect source marker tests.
- [ ] Resolve ordering, time lookup, IDs, validation, margins, layers and overlap behavior; compare direct ownership versus narrow attachments.
- [ ] Record ownership, API signatures, real consumer mappings and oracle fixtures before production edits.

Files: new `docs/porting/series-markers.md`; source/Iced references read-only.
Verify: every supported behavior has a source-derived fixture; no unresolved
attachment or event identity decision remains before task 1.

## 1a. Typed collection and transactional replacement

Dependencies: 0. Scope: medium.

- [ ] Add typed marker positions/shapes/options and chosen collection ownership using existing Time/SeriesId/error conventions.
- [ ] Implement validated replace/read/clear with ordering and identity rules; reject invalid input without losing existing markers.
- [ ] Prove removed/foreign series handling, clear/reload ownership and optional/duplicate external IDs follow the documented contract.

Files: marker types module, `src/api/chart.rs`, `src/api/mod.rs`, focused public tests.
Verify: focused lifecycle/validation tests; `cargo check --all-targets`.

## 1b. First complete public marker

Dependencies: 1a. Scope: medium.

- [ ] Prepare source-sized circle above a Candlestick row with correct time mapping and invalidation.
- [ ] Adapt owned commands into single/composed frames; public replacement/clear changes displayed output.
- [ ] Demonstrate the minimal path without claiming unsupported positions/shapes work; finalize API exposure once supported branches exist.

Files: marker pane-view/renderer modules, existing frame adapter(s), focused frame tests.
Verify: source geometry fixture, repeated immutable draw and empty/clear tests;
initial TinySkia/WGPU capture. Split model and adapter edits if needed.

## Checkpoint A: minimal marker path

- [ ] Public Candlestick circle replacement/clear works end to end.
- [ ] Focused tests, source fixture, format/check and both backends pass.

## 2a. Time mapping and complete placement

Dependencies: checkpoint A. Scope: medium.

- [ ] Complete shared-time/series-row lookup including whitespace, before/after data and multiple markers at the same resolved bar.
- [ ] Support all six positions on OHLC/scalar families with source stacking, explicit-price offsets and inverted/relative/manual scales.
- [ ] Invalidate on own/other-series updates, replacement/removal, visibility, navigation and scale changes without adding marker times to chart data.

Files: marker pane view, data/model invalidation consumers, focused model/API tests.
Verify: source placement oracle and lifecycle regressions across all six families.

## 2b. Autoscale margins

Dependencies: 2a. Scope: medium.

- [ ] Port source margin calculation/options and aggregate through existing autoscale machinery, preserving null price-range semantics.
- [ ] Invalidate on spacing/options/collection changes; preserve manual ranges and settle layout without cycles.
- [ ] Verify stacking/explicit prices do not introduce invented autoscale guarantees.

Files: marker model/view, `src/model/series.rs`, actual scale aggregation consumer,
focused autoscale tests.
Verify: source margin comparisons, enabled/disabled/mixed-scale tests and bounded
preparation; run affected existing scale/layout tests.

## Checkpoint B: complete placement

- [ ] Six positions work across all six families and supported scale modes.
- [ ] Time reindexing and source-compatible autoscale pass behavioral comparisons.

## 3a. Shapes and prepared text

Dependencies: checkpoint B. Scope: medium, subdivide shape/text work if necessary.

- [ ] Complete square/arrows/circle sizing and bitmap alignment, including zero size and alpha.
- [ ] Prepare optional text through existing font/cache mechanisms; store settled logical text bounds for later hits.
- [ ] Preserve source placement/stack spacing and clipping without projection or shaping during draw.

Files: marker renderer/view, shared Iced adapter/text integration, focused tests.
Verify: actual-source shape/path fixtures, empty/long/text-only cases and repeatable
draws; fractional-ratio captures on both backends.

## 3b. Marker drawing layers

Dependencies: 3a. Scope: medium.

- [ ] Integrate normal/above-series/top ordering into existing single/composed frames without a general plugin registry.
- [ ] Preserve relative series, grid, price-decoration and crosshair layers as established by source consumers.
- [ ] Cover mixed families/panes, hidden collections and retained immutable frames.

Files: `src/ui/line_chart.rs`, `src/ui/multi_pane_frame.rs`, marker options/view,
focused frame tests.
Verify: source layer fixtures plus mixed-series TinySkia/WGPU overlap captures.

## Checkpoint C: complete visuals

- [ ] Shapes/text/size/layers render through the public API with documented limits.
- [ ] Existing built-in paths/decorations remain green; font work is prepared once.

## 4. Marker hit selection

Dependencies: checkpoint C. Scope: medium.

- [ ] Implement source shape/text hit tolerances from settled frame metadata using logical coordinates and raw pointer positions.
- [ ] Resolve overlapping markers/collections/series according to traced source selection rules; do not infer priority from drawing order.
- [ ] Exclude hidden/offscreen/clipped candidates as required by the source contract and test hits before first draw.

Files: marker hit/renderer module, existing frame hit metadata, internal hit-test
helper only if relevant, focused tests.
Verify: source hit oracle, boundary/text-only/zero-size/duplicate-ID cases and
overlaps across panes. Full price-line/series hit testing remains deferred.

## 5. Public events and hover lifecycle

Dependencies: 4. Scope: medium; split payload and lifecycle wiring if necessary.

- [ ] Extend click/crosshair payloads with typed marker/series identity while preserving exact-index series rows and magnet behavior.
- [ ] Reconcile hits after navigation, resize/DPI changes, data/options/marker replacement, clear/hide/removal and pointer exit; detect hit changes even if snapped crosshair stays fixed.
- [ ] Preserve click-versus-drag and axis gestures; avoid reprojecting/shaping markers solely for each hover message.

Files: `src/api/interaction.rs`, `src/api/widget.rs` only if needed, Chart/frame
revision consumers, focused public/input tests.
Verify: native input-to-public-event tests, hit-after-time-change, lifecycle
regressions and unchanged navigation/event assertions.

## Checkpoint D: complete public behavior

- [ ] Shape/text hover and click identify markers correctly on settled frames.
- [ ] No stale identity after replacement/removal/exit; existing gestures pass.

## 6a. Demonstration and documentation

Dependencies: checkpoint D. Scope: medium.

- [ ] Add public marker replace/clear/options controls and show owned hover/click identity with price/volume and mixed-family examples.
- [ ] Add reproducible stacking, text, explicit-price/inverted, overlap/layer and hit-after-navigation capture fixtures.
- [ ] Finish README and port report with source contracts, alternatives, ownership/API, changed files and supported limits.

Files: `examples/public_chart.rs`, focused capture/oracle support,
`README.md`, `docs/porting/series-markers.md`.
Verify: public headless smoke and bounded native startup; example imports public API.

## 6b. Integrated verification and performance

Dependencies: 6a. Scope: medium verification/report slice.

- [ ] Run full fmt/tests/check/strict Clippy/example build/diff checks and all new/affected source oracles.
- [ ] Inspect repeatable TinySkia/WGPU captures at ratios 1/1.25/1.5/2 for shape/text geometry, clipping, layers and hits after navigation.
- [ ] Compare identical pre/post zoom/append/hover fixtures against `2718f18`: empty-marker overhead and large mostly-offscreen collections, median/p95 and text shaping counts; investigate regressions.

Files: focused verification fixtures if needed, port report, this checklist.
Verify: record exact commands/results and limitations; no skipped assertions or
weakened quality gates. Recheck important source consumers before completion.

## Final checkpoint and physical acceptance

Dependencies: 6b. Scope: small manual acceptance/report slice.

- [ ] Implementation and automated/native evidence are complete and saved in buildable commits.
- [ ] Fresh physical mouse/touchpad checks cover marker shape/text hover, click versus drag, zoom/pan/axis scaling, live updates and marker replacement/clear in crowded panes.
- [ ] Record hardware results separately; keep prior built-in-series hardware checks pending until independently completed.

Files: report and checklist only unless testing discovers a defect.
Verify: user-observed physical behavior; synthetic input and old acceptance cannot
close this gate. GitHub push is a separate user action.
