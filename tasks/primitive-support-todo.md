# Custom drawing primitives: implementation checklist

Status: planned. Design: [primitive-support-plan.md](primitive-support-plan.md).
Foundation: `8025ca1`. Execute sequentially. Files below are likely targets;
split any slice exceeding roughly five production files before implementation.
Module exports and focused tests belong to the slice that introduces behavior.

## 0. Source contract and Rust design

Dependencies: none. Scope: medium investigation/report slice.

- [ ] Read complete source components/imports/tests and trace real attachment, drawing, autoscale, axis, hit and plugin consumers with coverage/source fallback.
- [ ] Compare immutable-render-object and command-output designs; resolve ownership, update/reentrancy, cleanup, pane identity and event compatibility before production edits.
- [ ] Record important Rust signatures and consumer mappings in the port report; identify fixtures for every supported capability.

Files: new `docs/porting/primitive-support.md`; reference repositories read-only.
Verify: exact Iced API evidence, no unresolved ownership/render/event contract,
and a testable source-derived acceptance matrix.

## 1a. Prepare a custom series-attached rectangle

Dependencies: 0. Scope: medium.

- [ ] Define the minimum owned mutable primitive/prepared immutable renderer contract and typed identity, reusing existing layer types where appropriate.
- [ ] Prepare one series-attached rectangle using the current scale and settled plot geometry; render it through the existing adapter with normal-layer clipping.
- [ ] Test empty/whitespace data, multiple scales, preparation before first draw and retained-frame immutability.

Files: focused new model primitive module, UI primitive adapter, existing frame
integration, focused tests. Keep temporary capabilities private.
Verify: renderer tests, fractional-ratio capture and all-target check.

## 1b. Expose the first public attachment path

Dependencies: 1a. Scope: medium.

- [ ] Add validated public series attachment, explicit update/replacement and removal operations with typed IDs and transactional errors.
- [ ] Wire revision/invalidation and owner scale changes so preparation displays updated geometry and removal clears it.
- [ ] Exercise the full path in a minimal public widget scene; test stale/cross-chart IDs and rejected changes preserving existing output.

Files: new focused `src/api/primitives.rs`, API exports/Chart integration,
focused public tests and minimal example helper.
Verify: public add/update/remove tests, existing API tests and both renderers.

## Checkpoint A: first complete extension

- [ ] An application can supply custom series graphics through the public widget.
- [ ] Updates/removal affect the next settled frame; retained frames remain immutable.
- [ ] Rendering has no mutable model/plugin access or new asynchronous machinery.

## 2a. Add pane attachment ownership

Dependencies: checkpoint A. Scope: medium.

- [ ] Reuse the attachment/render contract for pane primitives without inventing a SeriesId or series scale.
- [ ] Maintain genuine internal pane ownership through index shifts and unrelated pane removal; define the public attachment context from source consumers.
- [ ] Test independent panes, empty/preserved panes and source-defined pane lifetime when its last series disappears.

Files: focused model/API primitive modules, actual pane/frame owner integration,
focused lifecycle tests.
Verify: pane widget rendering and owner identity tests before/after pane removal.

## 2b. Complete cleanup and update lifetime

Dependencies: 2a. Scope: small/medium.

- [ ] Implement source-required detach notifications and Rust Drop behavior exactly once across explicit removal and owner destruction.
- [ ] Define safe update behavior after detach/Chart drop; test request coalescing or explicit revision updates and any supported callback mutation.
- [ ] Preserve attachments on data clear/reload where required and apply source visibility behavior without stale output.

Files: primitive owner/context modules, series/pane cleanup integration, tests.
Verify: lifetime counters, repeated cleanup, retained-frame drop and no owner cycle.

## 3a. Complete pane rendering passes

Dependencies: 2b. Scope: medium.

- [ ] Implement bottom/normal/top placement and separate background/foreground passes with stable source/view ordering and absent renderers.
- [ ] Integrate hovered Area/Baseline split drawing, normal/top/bottom markers and crosshair according to the source pass sequence.
- [ ] Capture translucent overlaps and assert backgrounds are neither duplicated nor moved into the wrong pass.

Files: primitive rendering adapter, `src/ui/line_chart/series_drawing.rs`, actual
frame/plot drawing consumer, focused source/capture fixtures.
Verify: real-source pass oracle and TinySkia/WGPU layered scenes.

## 3b. Settle viewport changes and clipping

Dependencies: 3a. Scope: small/medium.

- [ ] Refresh prepared output on zoom/pan, owner scale changes, resize/DPI and application updates using existing invalidation/settlement machinery.
- [ ] Clip to the correct pane and keep coordinate units consistent across fractional ratios and independent panes.
- [ ] Test failed preparation, stationary changes and bounded redraw with no new draw-time measurement or mutation.

Files: primitive preparation/context module, existing frame owner integration,
focused complete-display tests.
Verify: finite geometry, retained frames, bounded redraw and viewport sequences.

## Checkpoint B: stable custom graphics

- [ ] Series/pane ownership, lifetime, all pane passes and viewport changes pass.
- [ ] Existing six-family rendering, hover selection, markers and navigation remain green.

## 4. Optional series autoscale contribution

Dependencies: checkpoint B. Scope: medium.

- [ ] Merge optional primitive price ranges/margins with the owner's built-in autoscale using the visible logical interval and source eligibility rules.
- [ ] Preserve manual, inverted, logarithmic, percentage/indexed and empty/hidden behavior; define validation for invalid extension output.
- [ ] Test contribution update/removal restoring the original range and bounded preparation without autoscale feedback loops.

Files: primitive contract, existing series/price-scale autoscale consumer,
focused behavioral tests and differential fixture.
Verify: real-source autoscale oracle and navigation with an out-of-range region.

## 5a. Series primitive price labels

Dependencies: 4. Scope: medium.

- [ ] Prepare owned label data, including fixed/adjusted coordinates, text/colors, visibility and tick behavior.
- [ ] Integrate existing price-label placement/measurement across supported sides/scales without changing the primitive's actual coordinate.
- [ ] Test collisions with built-in/custom price labels, edges, hidden axes and half-size windows without layout oscillation.

Files: primitive label module, axis snapshot/price-axis integration, tests/fixture.
Verify: source label placement cases and fractional-ratio price-axis captures.

## 5b. Series primitive time labels

Dependencies: 5a. Scope: medium.

- [ ] Reuse owned label data for time-axis placement, fixed coordinates, visibility and ticks.
- [ ] Integrate axis measurement/clipping with current Time behavior and preserve shared time-axis semantics across panes.
- [ ] Test edges, collisions with the crosshair label, narrow windows and zoom/resize updates.

Files: primitive label module, time-axis snapshots/adapter, tests/fixture.
Verify: source time-label cases and bounded layout settlement.

## 5c. Custom drawing inside axis areas

Dependencies: 5b. Scope: medium; split price/time adapters if necessary.

- [ ] Prepare series primitive price/time-axis pane graphics independently of axis labels using the same immutable renderer contract.
- [ ] Apply axis-local coordinates, source layer/background order, clipping and hidden-axis behavior.
- [ ] Capture custom axis graphics mixed with labels at fractional ratios and retain old frames across updates.

Files: primitive adapter, actual price/time-axis drawing consumers, focused tests.
Verify: source pass comparison and both renderer axis-area scenes.

## Checkpoint C: chart-aware primitives

- [ ] Autoscale, both label axes and axis graphics work without preparation loops.
- [ ] Label changes do not make the public demo's chart bounds jump on hover.

## 6a. Prepare primitive hits and arbitration

Dependencies: checkpoint C. Scope: medium.

- [ ] Prepare immutable hit data for the same revision as displayed graphics; carry attachment/owner/object identity and source distance/priority/layer/background metadata.
- [ ] Extend existing arbitration with pane and series primitives while preserving stable ties, marker precedence and raw coordinates independent of magnet snapping.
- [ ] Test hidden/missing views, duplicate external IDs, overlaps and removal/failed preparation without sticky winners.

Files: primitive hit contract/adapter, `src/ui/hit_testing.rs`, behavioral tests
and source arbitration fixture.
Verify: real-source overlap oracle and hit lookup before first draw.

## 6b. Public events and cursor behavior

Dependencies: 6a. Scope: medium.

- [ ] Expose the audited owned primitive identity through click/crosshair events with explicit pane/series owner semantics and compatible existing marker/series payloads.
- [ ] Map supported cursor requests to Iced and reset on leave, target change, removal and focus loss; document unsupported cursor fallback.
- [ ] Drive native widget input for enter/leave/click/drag, independent panes, axis gestures and stationary mutation without altering navigation.

Files: API event/interaction modules, widget cursor adapter, focused native tests.
Verify: native input sequences, stale-ID rejection and existing event regressions.

## 7a. Public shaded-region demonstration

Dependencies: 6b. Scope: small/medium.

- [ ] Add a public-chart primitive scene showing a shaded time/price region and selectable boundary plus a pane-attached graphic.
- [ ] Provide attachment/update/removal and layer controls; exercise autoscale, labels and axis graphics without adding editing behavior.
- [ ] Keep status on one line and include mixed filled series, price levels and markers for overlap checks.

Files: new `examples/public_chart/primitives.rs`, demo host/CLI integration,
focused host regression test.
Verify: native startup, half-size window behavior and capture scenarios.

## 7b. Final automated evidence and documentation

Dependencies: 7a. Scope: medium verification/report slice.

- [ ] Run final fmt/tests/check/strict Clippy/example builds, existing comparison scripts and all primitive source fixtures.
- [ ] Inspect TinySkia/WGPU captures at ratios 1/1.25/1.5/2; compare matched preparation/pointer/zoom/update workloads against `8025ca1`, including empty, crowded and mostly-offscreen scenes.
- [ ] Recheck real upstream consumers and finish README/report/checklist with exact files, ownership, alternatives, tested limits and remaining dependencies.

Files: capture/reference tools as required, README, port report and this checklist.
Verify: commands/results recorded; no weakened checks or unsupported parity claims.

## Checkpoint D: automated milestone delivered

- [ ] Public lifecycle, drawing, autoscale, labels, axis graphics and selection pass.
- [ ] Existing chart behavior remains green; performance evidence and limitations are recorded.

## 7c. Physical acceptance

Dependencies: checkpoint D. Scope: manual verification.

- [ ] Exercise mouse/touchpad overlap selection, clicks versus drag, cursor transitions, zoom/pan/axis scaling and half-size window resizing.
- [ ] Exercise live updates, attachment removal and owner removal; record device/backend/scale and check for stale graphics, sticky hits or continuing redraw.
- [ ] Record observed results separately from synthetic checks and earlier pending hardware milestones.

Files: port report and this checklist.
Verify: only mark complete after actual hardware results; unavailable checks stay
explicitly pending.
