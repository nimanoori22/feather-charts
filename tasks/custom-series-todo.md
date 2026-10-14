# Preparation performance, custom series and touch navigation checklist

Plan: [custom-series-plan.md](custom-series-plan.md).
Status: P/C/T implemented and automatically verified. Static preparation and
existing-time operation gates pass; append reindex/tail costs and physical
acceptance remain open. Foundation: `36a2955`.
Keep earlier hardware/performance gates intact until their own evidence is recorded.

## P0. Performance diagnosis

- [x] Trace preparation and data-update consumers; verify index coverage/source.
- [x] Separate normalization/indexing, model, geometry, layout and glyph timings.
- [x] Record debug/release baselines at fixed visible range and increasing history.
- [x] Record median/p95, operation counts, noise and budgets before optimizations.

## P1. Preparation reuse (depends on P0)

- [x] Fix the measured preparation bottleneck in small independently tested slices.
- [x] Define all data/viewport/options/layout/owner/ratio invalidators.
- [x] Verify geometry-only previews avoid history-sized built-in scans/copies.
- [x] Verify updated hits and immutable retained frames match rendered geometry.

## P2. Incremental updates (depends on P0, P1)

- [x] Avoid full normalization for existing latest-timestamp replacement.
- [x] Preserve append/historical/shared time reindexing and transactional errors.
- [x] Measure live updates and mixed sources against the recorded baseline.

## Performance checkpoint

- [x] Operation counts meet the unchanged-mapping/latest-replacement requirements.
- [x] Static preparation budget passes; record append exceptions and reindex/source-count costs.
- [x] Full Rust checks, source oracles and both-renderer regression captures pass.
- [x] Publish evidence; distinguish satisfied static-preview gates from broader append/live limits.

## C0. Custom-series contract (depends on performance checkpoint)

- [x] Read complete upstream components/dependencies and trace real consumers.
- [x] Reuse the existing Rust custom model/data/contracts where practical.
- [x] Compare two designs and record ownership, signatures and consumer mappings.
- [x] Resolve typed handles/payload events, heterogeneous storage and snapshots.

## C1. First custom renderer (depends on C0)

- [x] Register a typed custom series through the public Chart API.
- [x] Wire model/data owner to immutable custom preparation and actual price scale.
- [x] Render box-and-whisker data with correct whitespace/high/low/median semantics.
- [x] Verify mixed built-in/custom zoom/pan/resize on TinySkia and WGPU.

## C2. Custom data lifecycle (depends on C1)

- [x] Support replacement, latest append/replace and historical updates.
- [x] Preserve payload/metadata and common indexes across multiple custom types.
- [x] Validate before mutation and reject stale/foreign handles.
- [x] Verify clear/reload/removal, exactly-once cleanup and retained-frame safety.

## Checkpoint: public custom series

- [x] Public example works end to end with mixed typed series.
- [x] Existing built-in/primitive/navigation tests pass.

## C3. Chart integrations (depends on C2)

- [x] Integrate autoscale/last-value/custom price lines using builder current values.
- [x] Expose exact-index custom pointer data through the chosen typed API.
- [x] Integrate optional custom hit/hover/cursor arbitration.
- [x] Verify marker/primitive attachment support, panes and actual owner scales.
- [x] Cover all price modes, inversion/manual policy and empty/hidden/removal states.

## C4. Custom-series acceptance (depends on C3)

- [x] Add demo lifecycle/options controls and an upstream source-contract check.
- [x] Capture all custom states on both backends at four ratios.
- [x] Recheck preparation budgets with large custom payload histories.
- [x] Pass full Rust/source checks and publish API/ownership/validation report.
- [ ] Physical mouse/touchpad: data updates, zoom/pan, axis controls and hit behavior.
- [ ] Physical half-size/high-DPI: stable bounds, correct labels and no stuck cursor.

## T0. Touch contract (depends on C4 automated checks)

- [x] Trace upstream touch/pinch/tracking/kinetic behavior and native Iced delivery.
- [x] Compare designs and document ownership/options/public signatures.
- [x] Define Idle/Pan/Pinch/Tracking transitions and edit/mouse/animation precedence.
- [x] Verify native finger IDs, coordinates, loss and synthetic mouse policy.

## T1. One-finger navigation (depends on T0)

- [x] Route finger pan through existing time/price navigation and source options.
- [x] Distinguish tap from drag and prevent duplicate synthetic mouse handling.
- [x] Verify loss/focus/layout/owner changes, outside-plot moves and fresh gestures.

## T2. Pinch and tracking (depends on T1)

- [x] Implement source-based pinch scale/anchor and finger transition behavior.
- [x] Add long-press tracking/source exit modes using synchronous ticks.
- [x] Test zero-distance/nonfinite input, third fingers, loss and scale limits.
- [x] Preserve touchpad pixel-wheel behavior and normal mouse/edit routing.

## T3. Kinetic scrolling (depends on T2)

- [x] Port and test velocity sampling/decay/bounds/stopping against source.
- [x] Integrate with existing animation ticks and source mouse/touch defaults.
- [x] Stop on new pan/pinch/edit/navigation/focus/owner/layout interruption.
- [x] Verify no idle redraw loop or competing animation owner.

## T4. Navigation acceptance (depends on T3)

- [x] Pass full Rust/source checks and touch/custom/primitive renderer regressions.
- [x] Publish options, ownership, platform limits and animation measurements.
- [ ] Touchscreen: pan/tap, pinch, tracking, finger transitions and cancelled touches.
- [ ] Touchscreen: flick decay, boundaries and interruption by new input/focus loss.
- [ ] Mouse/touchpad: zoom/pan/price-axis controls and box editing after touch.
- [ ] Half-size/high-DPI/backend observations record device, scale and actual results.
- [ ] Mark each milestone accepted only after its automated and physical gates pass.

## Existing box-editing acceptance (can run alongside)

- [ ] Complete and record the physical matrix in `primitive-editing-todo.md`.
- [ ] Keep the broad per-event gate open for full-reindex append workloads;
  bounded static-preview and existing-time replacement evidence is published.
