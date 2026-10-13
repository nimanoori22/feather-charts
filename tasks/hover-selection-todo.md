# Series and price-line hover selection: implementation checklist

Status: planned. Design: [hover-selection-plan.md](hover-selection-plan.md).
Foundation: `13ac49b`. Execute sequentially. Targets below are provisional;
split any slice exceeding roughly five production files before implementation.
All tasks require source-compatible behavior, focused tests and buildable output.

## 0. Source contract and Rust design

Dependencies: none. Scope: medium investigation/report slice.

- [ ] Read complete source components/imports and tests; trace geometry, pane selection, ChartModel events, public conversion and hovered drawing consumers with coverage/source fallback.
- [ ] Record source rules for all families, options, identities, price-line visibility, marker precedence, stable order, filled-series split passes and lifecycle; separate required behavior from incidental TS structure.
- [ ] Compare both frame-based and model-cache designs, describe ownership, finalize API signatures/compatibility and map them back to real consumers before production edits.

Files: new `docs/porting/hover-selection.md`; reference repositories read-only.
Verify: every supported branch has a behavioral fixture; no unresolved selection,
identity or drawing-order decision remains before task 1a.

## 1a. Source Line hit geometry

Dependencies: 0. Scope: small/medium.

- [ ] Reuse existing priority/distance types; implement source segment distance, x-bound candidate search and simple/stepped/curved Line hits.
- [ ] Preserve twelve-step curve approximation, point-marker overrides, source radius/tolerance rules, single-row output and empty/degenerate boundaries.
- [ ] Add actual-source comparison cases and behavioral boundary tests; keep the helper private until the public slice works.

Files: new `src/renderers/{hit_test_common,line_hit_test}.rs`,
`src/renderers/mod.rs`, focused inline tests and oracle fixture.
Verify: focused helper tests and actual TS differential geometry cases;
`cargo check --all-targets`.

## 1b. Settled Line hit metadata

Dependencies: 1a. Scope: medium.

- [ ] Prepare logical hit geometry with stable SeriesId/view identity from the same settled layout used for drawing; retain visible indexes and neighboring points as required by source.
- [ ] Store owned metadata in single/composed pane frames; reuse existing projection caches and extract focused helpers from large frame files.
- [ ] Verify hidden/empty output, hits before first draw and immutable retained frames at fractional display ratios.

Files: new focused pane-hit metadata/adapter module, `src/ui/{line_chart,multi_pane_frame}.rs`,
actual Line pane-view preparation consumer, focused public-frame tests.
Verify: source-equivalent positions/radii and frame tests; no draw-time projection.

## 1c. Minimal Line public events

Dependencies: 1b. Scope: medium.

- [ ] Expose the chosen optional typed target/series identity through click and crosshair events for the supported Line path; preserve exact-index series_data and raw versus magnet semantics.
- [ ] Keep existing marker APIs coherent, use owned event data and validate pane/owner identity; leave unsupported branches private.
- [ ] Drive native widget input through public Chart update and confirm Line enter/leave/click without changing click-versus-drag or axis gestures.

Files: `src/api/{mod,chart,interaction}.rs`, focused public/native tests.
Verify: focused Line integration/input tests, existing marker/event regressions,
format/check and initial renderer captures.

## Checkpoint A: first complete selection path

- [ ] Line selection works end to end through the public widget, before draw and after prepare.
- [ ] Source geometry, ownership/event tests and both renderers pass for supported targets.

## 2a. Area and Baseline primary selection

Dependencies: checkpoint A. Scope: medium.

- [ ] Share source Line hit logic across Area/Baseline without treating fill interiors as independent targets.
- [ ] Apply line/point option and base/gradient projection changes to hit metadata; preserve source hidden-line and single-row behavior.
- [ ] Test scale modes/inversion, empty/whitespace data, clipping and mixed Line/Area/Baseline panes.

Files: existing Area/Baseline pane views, common hit adapter, focused tests.
Verify: actual-source family fixtures and public events; existing filled paths stay green.

## 2b. Bar and Candlestick range selection

Dependencies: 2a. Scope: medium.

- [ ] Implement source high/low interval hits with time-slot boundaries, adjacent-index rules and horizontal/vertical tolerances.
- [ ] Prepare both OHLC families using the shared helper; preserve inversion, gaps, equal-distance behavior and visible-range boundaries.
- [ ] Test thin/thick stems, candle options, empty/single rows and logical gaps without inventing painted-pixel-only selection.

Files: new `src/renderers/range_hit_test.rs`, existing Bar/Candlestick pane views,
common hit adapter, focused tests.
Verify: source range oracle and both families' public selection tests.

## 2c. Histogram interval selection

Dependencies: 2b. Scope: small/medium.

- [ ] Reuse the range helper with the projected value/base interval and source time-slot rules.
- [ ] Cover positive/negative values, changed base, inversion, relative/manual scales, whitespace and visibility changes.
- [ ] Verify independent price/volume pane identities and no cross-pane hits.

Files: Histogram pane view, common hit adapter, focused public tests.
Verify: source interval oracle and all six families through the public widget.

## Checkpoint B: all built-in series

- [ ] Six families follow source geometry/options at ratios 1/1.25/1.5/2.
- [ ] Existing rendering, markers, scale/layout and navigation checks remain green.

## 3a. Price-line identity and prepared metadata

Dependencies: checkpoint B. Scope: medium.

- [ ] Preserve custom PriceLineId and built-in line kind alongside each settled price-line geometry record; SeriesId alone must not identify a custom line.
- [ ] Support optional external IDs through existing validated options if the audited contract needs them; keep absent/duplicate IDs distinct from internal handles.
- [ ] Verify replacement/removal/clear/reload invalidation and unchanged automatic/custom line/label rendering.

Files: `src/model/price_line.rs`, `src/model/chart_model/price_lines.rs`,
focused hit adapter, actual snapshot consumer, lifecycle tests.
Verify: ownership/transactional option tests and existing price-line oracle/captures.

## 3b. Price-line public selection

Dependencies: 3a. Scope: medium.

- [ ] Implement source line-width-plus-seven hit tolerance, visibility gates and relevant built-in horizontal line behavior.
- [ ] Expose the correct typed price-line target through public events and settled-frame lookup without conflating labels with plot-line hits.
- [ ] Test automatic/custom lines, label-only options, styles/widths, multiple lines, external IDs and no stale target after mutation/removal.

Files: horizontal hit helper/adapter, public target/event consumers, focused tests.
Verify: actual-source hit/miss boundary oracle and native-input-to-public-event tests.

## Checkpoint C: price-line selection

- [ ] Public events identify individual custom levels and source-supported built-in lines.
- [ ] Existing line/label output and all previously supported hit paths pass.

## 4a. Complete pane overlap selection

Dependencies: checkpoint C. Scope: medium.

- [ ] Port stable source/view ordering, distance/point override and primitive foreground/background precedence for the currently supported targets.
- [ ] Integrate markers with series and price-line candidates while preserving marker attachment/item ordering and reported hit layers.
- [ ] Verify point versus range/line, distance differences, equal ties, same-series view order and cross-series marker/price-line conflicts against actual source execution.

Files: focused `src/model/pane_hit_test.rs` or frame equivalent chosen in task 0,
`src/model/internal_hit_test.rs`, marker hit adapter, event consumer, focused tests.
Verify: source pane-selection oracle and marker-versus-price-line e2e intent;
selection must not use transient hovered drawing order.

## 4b. Unified hover lifecycle

Dependencies: 4a. Scope: medium.

- [ ] Reconcile the winning target after zoom/pan, scale/options/data updates, resize/DPI, hide/removal and pointer pane transitions/exit.
- [ ] Detect target changes even if snapped crosshair/time is unchanged; reconcile stationary-pointer mutations with bounded redraw requests.
- [ ] Preserve raw-pointer hits, exact-index event rows, click/drag/axis behavior and no continuing redraw loop; reuse geometry for hover-only messages.

Files: `src/api/{chart,interaction}.rs`, existing frame-revision/input consumers,
focused public/native lifecycle tests.
Verify: source lifecycle fixtures, stale-hit tests and existing interaction/narrow-window tests.

## Checkpoint D: complete public target selection

- [ ] Series, price lines and markers select the same winning supported target as source.
- [ ] Mutation/navigation/exit does not leave stale identity or redraw loops.

## 5a. Hovered-series option and ordinary series drawing

Dependencies: checkpoint D. Scope: medium; split option plumbing if constructors exceed the file budget.

- [ ] Add the source-compatible hovered-series-on-top option/default through the chosen public/model option surface.
- [ ] Derive temporary render order for Line/OHLC/Histogram from the winning source without mutating stable model order or hit metadata.
- [ ] Verify disabled/default behavior, price decorations/marker layers/crosshair order and non-sticky overlap crossover.

Files: chosen chart options owner/API, focused drawing-order module,
actual single/composed frame adapters, focused tests.
Verify: source render-order oracle, mixed-series captures and stable overlap-event sequences.

## 5b. Area and Baseline split drawing passes

Dependencies: 5a. Scope: medium.

- [ ] Preserve normal fill ordering while raising only source-defined outlines/points and price-line views; keep source-defined normal marker attachments in their correct pass.
- [ ] Handle fill-only, hidden outlines, point-only and mixed translucent fills without double drawing/blending.
- [ ] Verify fixed hit order, retained frame immutability and source crosshair/annotation/axis layering.

Files: Area/Baseline preparation/drawing helpers, focused pass adapter,
actual frame consumer, focused geometry/layer tests.
Verify: source split-view fixtures and TinySkia/WGPU overlap captures at four ratios.

## Checkpoint E: complete hovered rendering

- [ ] All six families support the option with source-correct fills, strokes and decorations.
- [ ] Raised rendering cannot make hover stick to an earlier winner.

## 6a. Demonstration and documentation

Dependencies: checkpoint E. Scope: medium.

- [ ] Extend the public demo with crowded mixed-series/price-line/marker scenes, selected target display and an option toggle; retain one-line status and stable narrow-window layout.
- [ ] Add reproducible overlap, crossover, split-fill, price-line and lifecycle capture/input fixtures without private-engine manipulation in the host example.
- [ ] Finish README and port report with source contract, consumer audit, alternatives/ownership/API, exact files, behavior limits and dependencies.

Files: `examples/public_chart.rs`, focused helper/test files,
`README.md`, `docs/porting/hover-selection.md`.
Verify: public smoke, real widget input tests, bounded native startup and renderer captures.

## 6b. Integrated verification and performance

Dependencies: 6a. Scope: medium verification/report slice.

- [ ] Run fmt/full tests/check/strict Clippy/example builds/diff checks and every new/affected actual-source comparison; review the connected code and regression coverage.
- [ ] Inspect both renderer matrices at ratios 1/1.25/1.5/2 for selection, crossover, fill/outline/price-line/marker layers and navigation/resize behavior.
- [ ] Compare identical workloads against `13ac49b`: median/p95 for hover/zoom/append, candidate work and shaping/cache behavior on empty/many-series/large-offscreen scenes; investigate regressions and recheck source consumers.

Files: focused verification fixtures where needed, report and this checklist.
Verify: recorded commands/results/limits, no weakened assertions/gates and no
full-history projection or text shaping introduced solely for pointer movement.

## Final checkpoint and physical acceptance

Dependencies: 6b. Scope: small manual acceptance/report slice.

- [ ] Implementation and automated/native evidence are complete and saved in buildable local commits.
- [ ] Fresh mouse/touchpad tests cover overlap switching, target identification, click-versus-drag, zoom/pan/axis scaling, half-size windows, live updates and hide/removal in crowded panes.
- [ ] Record observed hardware/backend/scale results separately; retain prior milestone hardware gates until independently completed.

Files: report/checklist only unless acceptance finds a defect.
Verify: actual user-observed behavior. Synthetic input and previous mouse checks
cannot close this gate. GitHub push remains a separate user action.
