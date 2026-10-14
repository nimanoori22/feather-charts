# Implementation roadmap: preparation performance, custom series and touch navigation

Date: 2026-10-13. Updated 2026-10-14: P/C/T implementation and automated
verification complete. Physical acceptance is pending; append reindex/tail
costs remain documented follow-up work.
Foundation: `36a2955` (interactive primitive editing).
Execution checklist: [custom-series-todo.md](custom-series-todo.md).

This is a new roadmap in the repository's separate-milestone convention. Earlier
plans, including their pending physical acceptance, remain intact. Implement the
three stages in order, with independently reviewable commits and validation at
each checkpoint. Planning does not mark any implementation or hardware gate done.

## Outcome and dependency order

1. Repeated preparation and geometry-only editing avoid rebuilding unchanged
   history. Incremental data updates retain their incremental behavior.
2. An application can add a typed custom data series through the public API,
   update it and render it beside built-in series using the same scales, panes,
   decorations and events. A box-and-whisker example proves the complete path.
3. Native touch can pan, pinch to zoom and track the crosshair, with source-based
   kinetic scrolling and reliable cancellation. Existing mouse/touchpad controls
   retain their behavior.

```text
P0 measurements -> P1 preparation -> P2 incremental updates -> performance gate
    -> C0 source/API design -> C1 custom renderer -> C2 data lifecycle
    -> C3 chart integrations -> C4 renderer/performance gate
    -> T0 native/source design -> T1 touch pan -> T2 pinch/tracking
    -> T3 kinetic scrolling -> T4 final acceptance
```

Physical box-editing acceptance can run alongside these stages using
`tasks/primitive-editing-todo.md`. A real touch device is needed for the touch
hardware gate; a touchpad's wheel events do not prove touchscreen behavior.

## Evidence and architectural direction

The current editing report records debug preview medians of 13.014 ms for 1,200
rows, 17.123 ms for 20,000 rows, and 40.039 ms with live 20,000-row updates. These
are whole routing/update/preparation measurements, not evidence that a particular
function causes the history cost. Profile before choosing a cache or refactor.

Graph lookup and direct reads identify `Chart::prepare_snapshot` as the public
composition entry point, delegating to `MultiPaneChartFrameOwner` and primitive
preparation. The Rust port already has `CustomSeries`, `CustomDataLayerAdapter`,
`CustomData`, `CustomSeriesPaneView`, `CustomSeriesPaneRenderer`, custom price
projections and options. Extend these where practical; do not introduce a second
custom-series model or store arbitrary custom data in the built-in enum.

The upstream `src/model/icustom-series.ts` specifies original renderer data,
horizontal positions, visible range, bar spacing, price conversion, whitespace,
price-value building, optional hit testing and lifecycle. The price builder's
last value is the current value; extrema determine autoscale. The local
box-and-whisker plugin supplies high/low/median values, so it exercises more of
this contract than a custom renderer that merely duplicates a line series.

The pinned Iced touch events contain stable finger IDs and positions for press,
move, lift and loss. Upstream pane handlers distinguish touch scrolling, pinch
and long-press crosshair tracking. These native events and source behavior must
drive the design rather than treating touch as synthetic mouse input.

Evidence checks covered the relevant Rust model/API/layout paths, upstream
custom interfaces/API/plugin/pane/kinetic paths and Iced `core/src/touch.rs`.
Graph coverage is best effort. Upstream `icustom-series.ts` had changed metadata;
its complete current source was read directly. Interface/dynamic graph callers
are incomplete. This planning investigation is not the complete consumer audit
required before production implementation.

## Design rules and boundaries

- Keep the engine synchronous. Reuse ownership, invalidation and the existing
  tick/animation path. No new async runtime, locks or dependencies without an
  evidenced need.
- Frames own immutable render geometry. Old frames remain drawable after data
  updates or owner removal. Cache reuse must never retain stale hit geometry,
  price transforms or layout, or accidentally share mutable renderer state.
- Typed custom data stays with its typed owner; heterogeneous chart composition
  may use a small object-safe adapter at the rendering boundary. Compare that
  approach against a generic Chart before selecting final API signatures.
- Custom renderers use the existing Iced Canvas path on both backends. They may
  expose prepared snapshots through an adapter to the existing traits. Do not
  copy JavaScript callbacks, DOM destruction or FancyCanvas mechanically.
- Preserve shared time indexing, actual owner scales and source-defined current
  values. Validate before mutation; failed updates preserve the displayed data.
- Initial custom rendering uses the existing `Time` behavior. Arbitrary horizontal
  scale behavior, plugin discovery/JavaScript compatibility and full conflation
  rendering are separate work. Existing reducer contracts must remain usable.
- Touch is navigation/tracking in this roadmap. Touch annotation editing,
  snapping, undo, multi-selection, additional drawing tools and pane reordering
  are outside scope. Do not promise full TradingView web-application parity.

## Stage P: preparation performance

### P0. Measure and identify the history cost

Extend repeatable profiling to separate data normalization/indexing, model
settlement, visible geometry, layout/labels, primitives and glyph conversion.
Compare 1,200, 20,000 and 100,000 rows at a fixed visible range, for one and 16
sources, with geometry-only edits, hover/zoom, latest-bar replacement and append.
Run debug for diagnosis and release for performance acceptance; preserve raw
timings, warmup/sample counts, backend, scale factor and baseline revision.

Acceptance: identify measured dominant work and invalidation causes; record
baseline median/p95 and operation counts; write budgets before optimizing.
Verification: three alternating baseline/current runs, 20 warmups and at least
100 measured samples, without competing builds; confirm identical final frames.
Likely files: `examples/public_api_profile.rs`,
`examples/public_chart/primitive_editing_capture.rs`, a new performance report.
Dependencies: none. Scope: small.

### P1. Reuse unchanged preparation work

Fix the bottleneck found by P0 in one preparation subsystem at a time. Reuse
existing generations/invalidation where available; separate stable data geometry
from viewport projection or decorations only where measurements justify it.
Define all invalidators, including source updates/options, shared time reindexing,
owner price base/range, pane layout, ratio and primitive preview changes.

Acceptance: a geometry-only preview with unchanged mapping performs no
history-sized built-in scan/copy; changed geometry and hits refresh together;
retained frames and all built-in render behavior remain correct.
Verification: operation-count regressions plus renderer/source comparisons;
repeat P0 static previews with fixed visible rows as history increases.
Likely files: the measured subset of `src/ui/multi_pane_frame.rs`, series view
preparation, `src/api/chart.rs`, and focused tests. Split changes exceeding five
files or independent subsystems into separate slices.
Dependencies: P0. Scope: medium per slice.

### P2. Preserve incremental data-update work

Follow latest-bar replacement/append through validation, DataLayer, indexing and
view invalidation. Avoid routing an incremental mutation through full replacement
or cloning all prior rows. Separate unavoidable shared reindexing for new or
historical time insertion from replacing an existing latest timestamp.

Acceptance: existing-timestamp replacement does not renormalize unchanged
history; append/reindex behavior remains source-correct across multiple series;
rejected and historical updates preserve transactional behavior.
Verification: mixed-series/whitespace update tests and operation counts, then P0
live workloads. A genuine required reindex is documented rather than skipped.
Likely files: `src/api/chart.rs`, `src/model/data_layer.rs`, affected series/view
update methods, focused API tests. Scope: medium per slice.
Dependencies: P0, P1.

### Performance checkpoint

Require no history-proportional scan/copy on unchanged-mapping geometry previews
and no full normalization on existing latest-bar replacement. As a default timing
screen, static-preview median and p95 should grow by at most 10% between 1,200
and 100,000 rows at the same visible range; investigate any reproducible increase
over 10% on existing workloads. P0 must record timing noise and final budgets
before code edits; do not relax them after a failing result without documenting
why. Work legitimately proportional to source count or required reindexing is
reported separately. Do not infer a 60 fps guarantee from CPU preparation alone.

Run the full Rust checks, existing source oracles and both-backend captures.
Update the original editing performance gate only when its actual requirement
is met; hardware gates remain pending until recorded observations exist.

## Stage C: custom data-series rendering

### C0. Derive the source contract and settle the public API

Complete the AGENTS workflow: read complete source components and semantic
dependencies; trace real plugins, model, API, rendering, hit and event consumers
with coverage/source fallback. Inspect the Rust custom model/data contracts,
Iced rendering and relevant Flowsurface examples. Compare at least two designs,
record ownership/drop behavior and show important signatures before production
edits. Resolve typed custom handles, mixed data ownership and snapshot rendering.

Acceptance: all important consumers map to the proposed API; distinguish required
source behavior from incidental TS patterns; no duplicate custom abstraction.
Verification: review the consumer map, ownership diagram and API examples.
Likely files: `docs/porting/custom-series.md`, this plan/checklist.
Dependencies: performance checkpoint. Scope: small.

### C1. Deliver one complete custom rendering slice

Add a custom series through the public Chart API, supply typed box-and-whisker
data, and prepare/draw it beside a built-in series. Wire the existing custom model
and data adapter to the actual owner scale and immutable prepared drawing path.
The renderer receives logical positions/bar spacing and an owner-correct price
converter without holding a live Chart borrow during drawing.

Acceptance: the public example renders finite and whitespace data with correct
high/low autoscale and median current value; zoom/pan/resize work on both
backends; drawing does not mutate the model or run a second data update.
Verification: public integration test, upstream plugin geometry comparison and
one capture per backend before broadening the feature.
Likely files: `src/api/custom_series.rs` (if needed), `src/api/mod.rs`, a custom
plot adapter, `examples/custom_series.rs`, `tests/custom_series.rs`.
Dependencies: C0. Scope: medium; split model registration from draw adapter if
the chosen design requires more than five files.

### C2. Complete custom data lifecycle

Support validated full replacement, latest append/replace, historical update,
empty/whitespace transitions and removal. Preserve original custom payload and
metadata while DataLayer supplies common time indexes and price projections.
Allow multiple different custom types in the same Chart. Respect series options,
pane ownership, stale/foreign handles and exactly-once lifecycle cleanup.

Acceptance: mixed custom/built-in time indexes stay synchronized; failed updates
are non-destructive; removal leaves retained frames drawable and invalidates IDs.
Verification: transactional data/lifecycle tests, mixed-type/whitespace reindex
cases, drop counters and retained snapshot tests.
Likely files: custom API adapter, existing custom DataLayer/Series methods and
`tests/custom_series.rs`. Dependencies: C1. Scope: medium.

Checkpoint: one custom series is usable end to end through public APIs; all
existing built-in and primitive tests pass before adding decorations/events.

### C3. Integrate decorations and pointer events

Use projected extrema/current values for owner autoscale, last-value labels,
automatic/custom price lines and crosshair. Integrate optional custom hits with
existing arbitration/hover, cursor and typed event ownership. Decide in C0 how
exact-index custom payloads are exposed without unchecked downcasts. Support
existing marker and primitive attachments where their source contracts allow.

Acceptance: decorations use the final builder value and row color; exact-index
events preserve original custom data; overlap ordering and owner identity remain
correct across left/right/overlay scales and multiple panes.
Verification: all scale modes/inversion/manual policy, mixed overlaps, missing
hits, repeated object IDs, empty/hidden series and owner removal.
Likely files: split API event/hit changes and decoration preparation into separate
medium slices, each with focused custom integration tests.
Dependencies: C2. Scope: medium per slice.

### C4. Validate the extension boundary

Expand the example with replace/append/historical update/options/clear/remove
controls. Capture normal, hovered, whitespace, updated, empty, mixed-pane and
narrow-window states at ratios 1, 1.25, 1.5 and 2 on TinySkia and WGPU. Profile
custom series at fixed visible counts and large offscreen histories against
the post-P baseline; custom payloads must not restore the full-history preview
cost removed in P1.

Acceptance: source geometry/price-contract tests and immutable captures pass;
all Rust/source checks pass; API/lifecycle/performance limits are documented.
Verification: dedicated source-reference script, complete automated matrix,
release median/p95 comparison and physical mouse/touchpad demo checklist.
Likely files: custom example/capture helpers, source-reference script, custom
report, README and this checklist. Dependencies: C3. Scope: small per verification
or documentation slice. Automated readiness and physical acceptance are separate.

## Stage T: touch and kinetic navigation

### T0. Derive the native touch contract

Read complete upstream pane touch/pinch/tracking handlers, mouse-event helper,
kinetic algorithm/options and real consumers. Verify pinned Iced Canvas/winit
touch delivery, coordinate spaces, finger IDs, synthetic mouse behavior and
focus/window-exit semantics. Map them to existing ChartInput, multi-pane routing,
edit capture, controller/tick and animation ownership. Inspect Flowsurface where
relevant without treating it as the specification.

Acceptance: document Idle/Pan/Pinch/Tracking transitions, option defaults,
gesture precedence and every cancellation path; compare designs and settle
signatures and ownership before implementation.
Verification: event-delivery/source evidence and native sequence fixtures.
Likely files: `docs/porting/touch-navigation.md`, plan/checklist.
Dependencies: C4 automated checkpoint. Scope: small.

### T1. Deliver one-finger pan and tap

Recognize one stable finger in the owning pane, route pan through existing time
and price navigation, and distinguish tap from drag. Respect source touch-scroll
options, plot/axis regions and synchronous capture. Clear state on loss, focus
loss, owner/layout invalidation and conflicts with mouse/primitive capture.

Acceptance: tap emits one event, pan does not click or double-handle synthetic
mouse events, and every terminal path resumes ordinary navigation.
Verification: native press/move/lift/lost sequences across multiple panes,
outside-plot movement, disabled options, resize and interruption.
Likely files: a touch input helper, widget/message routing and focused tests.
Dependencies: T0. Scope: medium.

### T2. Add pinch and crosshair tracking

Use two stable finger IDs and the source pinch scale/anchor behavior. Transition
between one/two fingers without jumps; define third-finger and finger-loss policy
from source/native evidence. Add long-press tracking and source exit modes using
the synchronous tick path, not an async timer. Keep touchpad pixel-wheel behavior
unchanged unless the native platform demonstrably supplies equivalent gestures.

Acceptance: pinch zoom preserves its anchor; tracking moves the crosshair with
correct exact-index data; cancellation/zero-distance/nonfinite input cannot leave
stuck capture or an unintended pan/edit.
Verification: deterministic multi-finger and long-press time sequences, scale
limits, crosshair modes, pane ownership and transitions back to mouse navigation.
Likely files: touch recognizer, chart input/options routing and focused tests.
Dependencies: T1. Scope: medium; split pinch and tracking if needed.

### T3. Add source-based kinetic scrolling

Port sampled velocity, decay, bounds and stopping behavior from the kinetic
source, then integrate with existing time-scale animation/ticks. Preserve source
touch/mouse option defaults; avoid inventing vertical inertia where the source
only animates horizontal scrolling. A new press/pinch/edit, explicit navigation,
focus loss, removal or incompatible layout change must stop the animation.

Acceptance: deterministic sampled gestures decay and settle correctly; no idle
redraw loop or duplicate animation owner remains; new input interrupts promptly.
Verification: algorithm boundary/source-oracle tests, simulated monotonic time,
short/slow/reversing drags, limits and all interruption paths.
Likely files: `src/model/kinetic_animation.rs` (if no equivalent), existing
controller/animation integration, touch routing and tests.
Dependencies: T2. Scope: medium per algorithm/integration slice.

### T4. Final automated and physical acceptance

Replay touch/navigation captures and rerun all built-in/custom/primitive checks,
including pending edit cancellation and ordinary wheel/price-axis behavior.
Document public options, platform event limits and measured animation costs.

Acceptance: all automated checks pass; real-device observations are recorded for
finger pan, pinch, tracking, flick/stop/interruption and mouse/touchpad controls
afterward, at half-size and high DPI on both backends when available.
Verification: full validation below and device/backend/scale/result checklist.
Likely files: captures/source checks, touch report, README and checklist.
Dependencies: T3. Scope: small per validation/documentation slice.
If hardware is unavailable, record it as pending and leave that gate unchecked.

## Shared validation and completion

Add behavioral tests before changing core behavior. At each stage checkpoint run
`cargo fmt --check`, `cargo test --all-targets`, `cargo check --all-targets`,
`cargo clippy --all-targets -- -D warnings`, `cargo build --examples`, and every
existing `scripts/check_*_reference.cjs` plus new stage-specific checks. Recheck
source consumers against the final Rust API and review connected code, not only
the diff. Record exact files, chosen design/alternatives, ownership, tests,
renderer evidence, measured performance and remaining dependencies in each report.

Earlier baseline: 478 tests and 16 source-reference scripts pass at `36a2955`.
Counts can grow; do not use a test count or file count as a percentage of parity.
No calendar estimate or new completion percentage is assigned by this roadmap.

## Risks and controls

| Risk | Control |
| --- | --- |
| An optimization hides stale layout or hit geometry | Explicit invalidators, retained-frame tests and pixel/source comparisons |
| Measured cost lies outside the assumed bottleneck | P0 phase timings and operation counts before design |
| Shared reindexing is accidentally skipped | Mixed custom/built-in insertion and whitespace regressions |
| Typed data leaks through unsafe/unchecked erasure | Compare typed handles and a private object-safe boundary at C0 |
| Custom renderer mutates an old frame | Owned immutable preparation and lifecycle/snapshot tests |
| Touch, mouse, edit and animation compete for capture | One documented state/precedence model with interruption tests |
| Touchscreen behavior is inferred from touchpad input | Separate native touch and physical-device acceptance |

No missing product choice blocks planning. Exact custom API signatures,
performance budgets and native gesture defaults are implementation investigation
outputs, to be recorded before their respective production changes.
