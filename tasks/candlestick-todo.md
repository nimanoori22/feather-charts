# Candlestick implementation checklist

Plan and source map: [candlestick-plan.md](candlestick-plan.md).
All tasks are pending. Existing navigation hardware checks remain in [todo.md](todo.md).
Likely files are provisional; include affected exports, callers and fixtures.

## Task 0: Establish the source contract

**Dependencies:** None. **Scope:** Small investigation/documentation slice.

- [x] Read full source components/dependencies and source tests; trace consumers
  with codebase-memory, check coverage and inspect Iced/Flowsurface where relevant.
- [x] Record geometry, colors, visible selection, autoscale, magnet and lifecycle
  requirements; distinguish existing behavior from gaps and incidental TS patterns.
- [x] Compare both frame integration designs, show ownership/API signatures and
  map them to model, renderer, UI and lifecycle consumers before implementation.

**Likely files:** `docs/porting/candlestick-rendering.md` (new), this checklist.
**Verification:** Review the contract against real consumers; every unresolved
behavior has a concrete investigation step. Do not translate from one file alone.

## Task 1: Implement candle draw geometry

**Dependencies:** Task 0. **Scope:** Medium renderer slice.

- [x] Add owned candle renderer data and backend-neutral commands implementing
  source width, wick/body/border order, colors and bitmap alignment.
- [x] Cover doji, visibility combinations, dense spacing, range boundaries,
  empty output and independent horizontal/vertical pixel ratios as supported.
- [x] Add an actual-source geometry oracle with boundary fixtures; reject invalid
  geometry through the port's established validation conventions.

**Likely files:** `src/renderers/candlesticks_renderer.rs` (new), renderer exports,
`examples/candlestick_reference.rs` (new), `scripts/check_candlestick_reference.cjs`
(new), focused renderer tests.
**Verification:** Renderer behavioral tests and source command comparisons pass;
existing targets compile. Split oracle expansion into another slice if necessary.

## Task 2: Prepare candle rows through the model

**Dependencies:** Task 1. **Scope:** Medium model slice.

- [x] Reuse typed candle rows/options and selected scales to prepare owned OHLC
  coordinates, source-compatible visible ranges and color precedence.
- [x] Expose the smallest model preparation entry point and clear stale output
  after hidden/empty/whitespace/removal states or failed refresh.
- [x] Test actual candle rows under supported scale modes, updates, first-value
  changes, colors and option replacement; resolve demonstrated autoscale gaps.

**Likely files:** `src/model/series/candlesticks_pane_view.rs` (new),
`src/model/series.rs`, `src/model/chart_model.rs`, focused model tests.
**Verification:** Connected candle preparation tests pass; compare selected
coordinates/colors with the Task 0 contract, and run all-target check.

## Checkpoint: Backend-neutral candles

- [ ] Geometry oracle and connected preparation tests pass.
- [ ] Run the plan's milestone commands; existing line behavior remains green.
- [ ] Recheck source consumers before changing UI/frame APIs.

## Task 3: Integrate candle frames with Iced

**Dependencies:** Task 2. **Scope:** Medium frame/rendering slice.

- [ ] Implement the selected bounded frame design, updating all existing callers
  and fixtures while retaining shared layout, animation and invalidation ordering.
- [ ] Draw owned candle commands through an Iced adapter with correct clipping,
  device scaling and layer order; preserve read-only draw and the axis glyph cache.
- [ ] Test fractional ratios, zero-size/resize, bounded settlement, failure cleanup
  and retained-frame immutability for both line and candle outputs.

**Likely files:** `src/ui/chart_frame.rs`, `src/ui/line_chart.rs`,
`src/ui/candlestick_chart.rs` (only if chosen design needs it), UI exports,
existing example/frame tests. Split caller migration if the slice grows.
**Verification:** Integrated headless frame tests, all-target check and existing
line captures pass; no new duplicated layout/animation loop.

## Task 4: Verify crosshair behavior on candles

**Dependencies:** Task 3. **Scope:** Medium interaction slice.

- [ ] Verify Normal/Magnet/MagnetOhlc with actual candles, including labels,
  close/OHLC candidates, source tie order and candle marker eligibility.
- [ ] Test whitespace, hidden/overlay candidates, unequal scales, option changes
  and crosshair refresh after zoom, scroll, resize and data updates.
- [ ] Change existing magnet/crosshair code only for demonstrated gaps and
  extend actual-source comparisons for the candle cases.

**Likely files:** `src/model/chart_model.rs`, focused navigation tests,
`examples/interaction_reference.rs`, `scripts/check_interaction_reference.cjs`,
the candle porting report.
**Verification:** Candle interaction differential tests and all existing line
crosshair tests pass; displayed labels refer to the applied crosshair coordinates.

## Checkpoint: Interactive candle frame

- [ ] Candles, axes, labels and crosshair share current settled scales.
- [ ] Run milestone commands; preserve line input responsiveness and geometry.

## Task 5: Deliver the runnable candlestick demo

**Dependencies:** Task 4. **Scope:** Small-to-medium application slice.

- [ ] Add a candle example reusing existing input/frame patterns, with controls
  for crosshair modes, resize, append/update, clear/reload and candle styles.
- [ ] Provide deterministic scenes with rising/falling/doji candles, whitespace
  and per-row overrides, plus headless smoke/capture paths using existing helpers.
- [ ] Confirm wheel zoom, horizontal scrolling and drag operate through the
  existing model/input lifecycle without copied interaction logic.

**Likely files:** `examples/candlestick_chart.rs` (new), existing example support,
connected demo tests and the candle report.
**Verification:** Build/run the example; headless sequences pass. Manually check
native behavior and record which input devices/platforms were exercised.

## Task 6: Validate and report the candle milestone

**Dependencies:** Task 5. **Scope:** Medium verification/documentation slice.

- [ ] Run source comparisons and affected existing oracles; inspect TinySkia/WGPU
  captures at normal/fractional ratios, narrow spacing and representative styles.
- [ ] Run connected update/history/removal/resize/navigation sequences and compare
  equivalent line zoom workloads plus measured candle workloads for regressions.
- [ ] Finish the source/consumer/design/ownership report with exact changed files,
  tests, results, measured limits and remaining dependencies.

**Likely files:** Candle reference/demo fixtures, existing verification scripts,
`docs/porting/candlestick-rendering.md`, this checklist.
**Verification:** Run all milestone commands and build examples. Raster checks
cover clipping, finite output, repeatability and owned-frame immutability; record
manual hardware results separately from synthetic events.

## Checkpoint: Verified candlestick milestone

- [ ] Correct candle geometry, colors, axes, crosshair and live updates work together.
- [ ] Source parity, behavioral tests, both raster backends and line regressions pass.
- [ ] Native responsiveness is checked; unavailable device/platform checks are listed.
- [ ] Report remaining work separately: histogram/volume, multiple panes and public API.
