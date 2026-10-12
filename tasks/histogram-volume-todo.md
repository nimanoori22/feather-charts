# Execution checklist: histogram and price/volume panes

Roadmap: [histogram-volume-plan.md](histogram-volume-plan.md).
Status: implemented and automatically verified; physical device acceptance pending.
Existing physical input checks remain in [candlestick-todo.md](candlestick-todo.md).

## Task 0: Establish the histogram contract and integration design

**Dependencies:** None. **Scope:** Small investigation/documentation slice.

- [x] Read complete histogram renderer/preparation sources and dependencies;
  trace consumers, source tests and visible-range/color/scale requirements.
- [x] Verify indexed coverage and source gaps. Compare two frame designs,
  describe ownership/API signatures and map important consumers before coding.
- [x] Record required behavior and discarded TS patterns in the histogram report.

**Likely files:** `docs/porting/histogram-rendering.md` (new), this checklist.
**Verification:** Contract covers geometry, whitespace, options and lifecycle;
unresolved questions have explicit investigation steps.

## Task 1: Implement histogram geometry with source comparisons

**Dependencies:** Task 0. **Scope:** Medium renderer slice.

- [x] Add backend-neutral histogram geometry/commands and renderer exports.
- [x] Preserve baseline thickness, bar bounds, pixel rounding, adjacent-index
  correction, gaps, colors and narrow-spacing behavior from the actual source.
- [x] Add behavioral tests and an actual-source oracle for boundary cases,
  invalid/empty input and fractional independent horizontal/vertical ratios.

**Likely files:** `src/renderers/histogram_renderer.rs` (new), renderer exports,
`examples/histogram_reference.rs` and `scripts/check_histogram_reference.cjs`
(new), focused tests. Split fixture expansion if needed.
**Verification:** Histogram unit tests/source comparisons pass; existing targets build.

## Task 2: Prepare histogram data through the model

**Dependencies:** Task 1. **Scope:** Medium model slice.

- [x] Reuse histogram rows/options, selected time/price scales and colorer;
  project values and configured baseline with source-compatible visible selection.
- [x] Add the smallest typed preparation entry point and owned pane view.
- [x] Test autoscale/base changes, supported scale modes, whitespace, overrides,
  updates and stale-output clearing after hide/clear/removal/preparation failure.

**Likely files:** `src/model/series/histogram_pane_view.rs` (new), model series
exports, `src/model/series.rs`, `src/model/chart_model.rs`, focused tests.
**Verification:** Connected preparation tests and all-target check pass; change
existing baseline autoscale only if source investigation demonstrates a gap.

## Checkpoint: Backend-neutral histogram

- [x] Geometry oracle and model preparation tests pass; run milestone commands.
- [x] Recheck important source consumers before frame/UI integration.

## Task 3: Integrate histogram frames and a runnable demo

**Dependencies:** Task 2. **Scope:** Medium frame slice plus small demo slice.

- [x] Extend shared frame preparation and Iced drawing without duplicating
  animation, invalidation, layout or glyph-cache logic.
- [x] Update all affected callers/exports; test clipping, settlement, resize,
  failure cleanup and retained snapshots for line, candle and histogram frames.
- [x] Add a small histogram example through shared demo support, exercising
  positive/negative/base values, overrides, whitespace and live updates.

**Likely files:** `src/ui/chart_frame.rs`, existing UI drawing adapters/exports,
`examples/histogram_chart.rs` (new), `examples/support/`, connected frame tests.
**Verification:** Build/run histogram example; headless captures and existing
line/candle comparisons pass. Split demo wiring from frame changes if needed.

## Task 4: Establish the multi-pane contract and design

**Dependencies:** Task 3. **Scope:** Medium investigation/documentation slice.

- [x] Read source chart/pane/separator/input/crosshair consumers; inspect exact
  local Iced and relevant Flowsurface examples, with coverage checks.
- [x] Resolve shared axis widths, height allocation/minima, divider options,
  time-axis ownership, active-pane routing and crosshair projection/labels.
- [x] Verify existing model pane lifecycle/index semantics and invalidation;
  do not assume stable IDs or a complete removal/movement API.
- [x] Compare both coordinated frame designs, show ownership/API signatures,
  map real consumers and record choices in the multi-pane report before coding.

**Likely files:** `docs/porting/multi-pane-rendering.md` (new), this checklist.
**Verification:** Reviewed design preserves one global animation sample and
invalidation cycle; every UI event and pane snapshot has a defined owner.

## Checkpoint: Working histogram and reviewed multi-pane design

- [x] Histogram demo and regressions pass; run milestone commands.
- [x] Source contract resolves layout, lifecycle and event-routing risks.

## Task 5: Implement pure multi-pane layout

**Dependencies:** Task 4. **Scope:** Medium layout slice.

- [x] Add pane rectangles, shared left/right axis extents, one time-axis region,
  source-defined height allocation and separator bounds.
- [x] Preserve single-pane callers where practical; update affected fixtures.
- [x] Test unequal labels, zero/tiny bounds, hidden axes, fractional ratios,
  height changes and pane insertion/removal using source-derived fixtures.

**Likely files:** `src/ui/chart_layout.rs`, connected layout tests and existing
layout reference/capture fixtures, model height handling only for demonstrated gaps.
**Verification:** Pure layout tests/source comparisons pass; plot widths align
and finite clipped rectangles fit available bounds.

## Task 6: Prepare and draw coordinated multi-pane frames

**Dependencies:** Task 5. **Scope:** Medium slices: settlement, then drawing/callers.

- [x] Prepare pane-local series/axes with globally settled widths and one sampled
  animation/invalidation cycle; keep settlement bounded and failure cleanup defined.
- [x] Draw owned pane snapshots with correct clipping/layers and one time axis;
  retain glyph-cache behavior and immutable previously retained frames.
- [x] Test line/candle/histogram combinations, height/width changes, empty panes
  and source-supported lifecycle operations without stale snapshots.

**Likely files:** `src/ui/chart_frame.rs`, UI draw adapters/exports, frame tests;
model invalidation/lifecycle code only where the contract demonstrates missing behavior.
**Verification:** Static two-pane captures align at normal/fractional ratios;
all existing single-pane frame tests and source comparisons pass.

## Checkpoint: Aligned static price/volume frame

- [x] Candles and histogram share time coordinates with separate price scales.
- [x] Shared settlement, clipping and lifecycle tests pass; run milestone commands.

## Task 7: Route navigation and crosshair by pane

**Dependencies:** Task 6. **Scope:** Medium slices: input routing, then crosshair.

- [x] Add explicit pane targeting and one gesture owner; wheel/drag navigation
  from either pane changes the shared time scale once per event.
- [x] Preserve coalescing, stationary-pointer suppression and source cancellation
  behavior through pane transitions, outside drags, layout changes and focus loss.
- [x] Implement source-defined vertical projection across panes and active-pane
  horizontal/price labels, including magnet modes and a shared time label.
- [x] Extend actual-source interaction comparisons and synthetic event sequences
  for unequal scales, whitespace, hidden series and data/navigation refresh.

**Likely files:** `src/ui/chart_input.rs`, application input callers, crosshair
model/view/rendering paths, interaction reference fixtures and tests.
**Verification:** Each event produces at most one navigation operation; crosshair
geometry/labels match source and existing single-pane interaction remains correct.

## Task 8: Implement basic pane-divider resizing

**Dependencies:** Task 7. **Scope:** Medium interaction/layout slice.

- [x] Connect divider hit regions/drag ownership to source-defined height updates,
  minimum sizes and resize-enabled behavior without triggering plot navigation.
- [x] Preserve coherent frames and crosshair refresh through drag, window resize,
  focus loss and layout changes; maintain the established global time width.
- [x] Test minimum-size clamps, fractional bounds, disabled resizing and gesture
  cancellation using source behavior and Iced event sequences.

**Likely files:** Existing layout/input/frame modules, shared demo event handling,
focused resize tests. Add a widget only if the inspected Iced design requires it.
**Verification:** Divider gestures change pane heights only; chart stays aligned
and ordinary navigation still works before and after resizing.

## Checkpoint: Synchronized interactive panes

- [x] Both panes navigate together, with source-correct crosshair and resizing.
- [x] Input, layout and frame regressions pass; run milestone commands.

## Task 9: Deliver the price/volume demo and live lifecycle

**Dependencies:** Task 8. **Scope:** Medium application slice.

- [x] Add a runnable candlestick/volume example using shared support and existing
  typed rows, with independent scale/volume formatting and aligned timestamps.
- [x] Exercise missing volume as whitespace, append/same-time updates, history
  replacement/prepend, clear/reload, styles, series removal and pane-height changes.
- [x] Apply paired demo data changes before preparing the next frame; reuse the
  existing synchronous lifecycle without adding speculative batching infrastructure.
- [x] Add deterministic captures and connected tests; document run/control commands.

**Likely files:** `examples/price_volume_chart.rs` (new), `examples/support/`,
connected demo tests and the two porting reports.
**Verification:** Build/run example and headless update sequences; no stale bars,
axes or crosshair labels after lifecycle changes.

## Task 10: Validate and report the milestone

**Dependencies:** Task 9. **Scope:** Medium verification/documentation slice.

- [x] Run all affected source oracles and TinySkia/WGPU captures for histogram,
  multi-pane layout, interaction, fractional ratios and clipping.
- [x] Measure equivalent existing single-pane navigation before/after; record
  separate two-pane median/p95 costs and investigate material regressions.
- [x] Run final format/tests/check/Clippy/example builds specified in the roadmap.
- [x] Complete source/consumer/contract/design/alternatives/ownership reports with
  exact files changed, evidence, limitations and remaining dependencies.

**Likely files:** Verification fixtures/scripts, navigation benchmark fixtures,
`docs/porting/histogram-rendering.md`, `docs/porting/multi-pane-rendering.md`, checklist.
**Verification:** Record exact commands/results and inspected captures. Synthetic
events and native startup remain distinct from physical device acceptance.

## Checkpoint: Verified price/volume milestone

- [x] Runnable two-pane demo, source parity, behavioral tests and both backends pass.
- [x] Existing single-pane behavior/performance is preserved; new costs are measured.
- [ ] Manually exercise physical mouse/touchpad zoom, scroll, crosshair, divider
  dragging, window resizing and live-data controls; record devices/results.
  Also check price-axis wheel/drag scaling and double-click reset as described
  in [price-axis interaction](../docs/porting/price-axis-interaction.md), and
  free diagonal/vertical plot dragging in [plot panning](../docs/porting/plot-panning.md).
- [x] Report pending hardware checks honestly and identify public chart/widget API
  as subsequent work, without treating it as part of this milestone.
