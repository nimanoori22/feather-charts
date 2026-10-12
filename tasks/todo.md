# Mouse navigation and crosshair checklist

Design, scope and source references: [plan.md](plan.md).
Implementation tasks 0–8 are delivered. Automated checks and both raster backends pass.
The user confirmed native responsiveness and smooth zoom. Full mouse/touchpad
gesture validation remains unchecked; see the porting reports.
The likely-file lists below preserve the original planning notes.
Include necessary module exports; split work if the actual change becomes large.

## Task 0: Establish the navigation contract

**Dependencies:** None. **Scope:** Small investigation/documentation slice.

- [x] Read the complete navigation source components, trace real consumers with
  codebase-memory, check coverage, and inspect Iced dispatch plus Flowsurface examples.
- [x] Record wheel units/options, plot coordinates, drag lifecycle, animation
  ordering, and effect delivery; resolve horizontal wheel handling explicitly.
- [x] Compare the two integration designs in the plan; document ownership,
  proposed Rust signatures, and mappings for model/UI consumers before coding.

**Likely files:** `docs/porting/mouse-navigation.md` (new), this checklist.
**Verification:** Review the contract against source call sites and local Iced
APIs. Every unresolved behavior has an explicit investigation step.

## Task 1: Deliver wheel zoom

**Dependencies:** Task 0. **Scope:** Medium vertical slice.

- [x] Add the minimum model-owned zoom entry point and audit/correct existing
  scale semantics and effects only where demonstrated by source behavior.
- [x] Wire Iced line/pixel wheel events in the demo, using the settled plot
  rectangle, deliberate normalization and source-compatible option gates.
- [x] Test anchor preservation within source limits, delta direction, boundaries,
  empty data, invalid inputs, recalculation, and animation interruption.

**Likely files:** `src/model/chart_model.rs`, `src/model/time_scale.rs`,
`src/model/chart_model/tests.rs`, `examples/line_chart.rs`.
**Verification:** Focused navigation tests and `cargo check --all-targets` pass;
native zoom follows the pointer and does not intercept unrelated UI events.

## Task 2: Deliver horizontal drag scrolling

**Dependencies:** Task 1. **Scope:** Medium vertical slice.

- [x] Add model-owned start/move/end operations using existing scale machinery;
  deliver effects and preserve animation cancellation and source boundary rules.
- [x] Add explicit canvas gesture state with source-compatible button/threshold
  rules, outside-plot continuation, release and cancellation handling.
- [x] Test outside release, window leave/focus loss, repeated end/cancel, resize,
  clear/removal during drag, and subsequent gestures without stale state.

**Likely files:** `src/model/chart_model.rs`, `src/model/time_scale.rs`,
`src/model/chart_model/tests.rs`, `examples/line_chart.rs`.
**Verification:** Focused model/event-sequence tests pass; native dragging remains
responsive and never sticks after release or cancellation.

## Task 3: Verify the connected navigation path

**Dependencies:** Task 2. **Scope:** Medium verification slice.

- [x] Add deterministic source comparisons for zoom anchors, scroll offsets,
  boundaries and ordered input sequences using equivalent normalized input.
- [x] Extend integrated tests for navigation during layout/animation/data changes,
  fractional scaling, empty/whitespace/reload, and preserved prepared frames.
- [ ] Exercise available mouse/touchpad hardware and record validation plus native
  input differences in the navigation report.

**Likely files:** `src/model/chart_model/tests/complete_display.rs`,
`examples/line_chart.rs`, a focused reference fixture/script if needed,
`docs/porting/mouse-navigation.md`.
**Verification:** Source comparisons and connected tests pass; existing display
regressions stay green. Keep fixtures minimal and reuse existing harness patterns.

## Checkpoint: Usable mouse navigation

- [x] Wheel zoom and horizontal drag work end to end in the line-chart demo.
- [x] Run the milestone commands from `plan.md`; record results and limitations.
- [x] Recheck original navigation consumers; complete the porting report before
  starting crosshair work.

## Task 4: Establish crosshair state and pointer updates

**Dependencies:** Navigation checkpoint. **Scope:** Medium vertical slice.

- [x] Investigate complete crosshair source and consumers; compare Rust designs,
  document state ownership/signatures, visibility modes and scale selection.
- [x] Introduce typed model-owned crosshair state and pointer messages, separating
  raw pointer coordinates from source-defined applied coordinates.
- [x] Test movement, visibility clearing, empty/whitespace data, removal, resize
  and navigation updates without introducing general hit-testing infrastructure.

**Likely files:** `src/model/crosshair.rs` (new), `src/model/mod.rs`,
`src/model/chart_model.rs`, `examples/line_chart.rs`,
`docs/porting/crosshair.md` (new). Keep focused tests beside their implementation.
**Verification:** Crosshair state tests and all-target check pass; consumer mapping
supports later rendering and labels without exposing mutable shared state.

## Task 5: Render crosshair lines

**Dependencies:** Task 4. **Scope:** Medium rendering slice.

- [x] Trace pane-view/renderer contracts and add crosshair geometry to owned
  prepared frames, with source-compatible options, clipping and layer order.
- [x] Render finite, correctly aligned horizontal/vertical lines; clear them
  immediately when crosshair state becomes hidden.
- [x] Test fractional ratios, resize, visibility and repeated read-only drawing;
  investigate point-marker dependencies and document their scope decision.

**Likely files:** `src/renderers/crosshair_renderer.rs` (new), renderer exports,
`src/ui/chart_frame.rs`, focused crosshair/display tests.
**Verification:** Renderer/source geometry checks pass and native lines track the
pointer over the existing series without stale frames or clipping seams.

## Task 6: Add floating time and price labels

**Dependencies:** Task 5. **Scope:** Two small-to-medium slices if needed:
time label first, then price label/layout.

- [x] Trace both label views and axis consumers; document sizing/edge alignment
  rules and extend snapshots using existing formatting/measurement abstractions.
- [x] Render labels with correct time/price formatting, selected scale, backgrounds
  and clipping; preserve percentage/indexed/logarithmic behavior where supported.
- [x] Test axis visibility, narrow plots, fractional ratios and bounded layout
  settlement when labels change; drawing performs no new measurement or mutation.

**Likely areas:** `src/model/axis_snapshots.rs`, `src/ui/chart_frame.rs`,
time/price axis renderers and adapters, focused axis tests. Split by axis rather
than changing all areas in one large patch.
**Verification:** Focused source label/layout comparisons and axis regressions pass;
native labels remain legible and pointer movement does not cause layout oscillation.

## Task 7: Add magnet snapping

**Dependencies:** Task 6. **Scope:** Medium behavioral slice.

- [x] Investigate `magnet.ts` and real consumers, then define the minimum typed
  selection operation needed by supported line-series data.
- [x] Apply source-compatible snapping to crosshair position and labels while
  retaining raw pointer state and the independently tested unsnapped behavior.
- [x] Test eligible-series selection, unequal scales, ties, whitespace/missing
  values, hidden sources, empty data and changes during navigation/data updates.

**Likely files:** `src/model/magnet.rs` (new), model exports,
`src/model/crosshair.rs`, `src/model/chart_model.rs`, focused tests.
**Verification:** Deterministic source selection comparisons pass; native snapped
lines and labels refer to the same selected value.

## Task 8: Verify and report the interactive milestone

**Dependencies:** Task 7. **Scope:** Medium verification/documentation slice.

- [x] Run connected sequences combining zoom, drag, crosshair, labels, snapping,
  animation cancellation, append/prepend, clear/reload, removal and resize.
- [x] Reuse raster/source comparison tools to check clipping, finite geometry,
  label placement and prepared-frame immutability at representative scale factors.
- [x] Complete the crosshair porting report with exact changes, consumers,
  alternatives, ownership, test results and remaining dependencies.

**Likely files:** `src/model/chart_model/tests/complete_display.rs`,
`examples/line_chart.rs`, existing or focused reference/capture fixtures,
`docs/porting/crosshair.md`.
**Verification:** Run milestone commands and affected existing comparison scripts;
inspect native mouse interactions and representative TinySkia/WGPU output.

## Checkpoint: Interactive line chart complete

- [x] Wheel zoom, drag scrolling, crosshair lines/labels and magnet work together.
- [x] Automated milestone validation passes, with hardware and platform limits
  explicitly recorded.
- [ ] Manually exercise physical mouse/touchpad navigation in a desktop window.
- [x] Re-read important source consumers and list follow-up work separately:
  touch/kinetic gestures, axis manipulation, multi-pane UI, additional series and
  public APIs remain outside this roadmap.
