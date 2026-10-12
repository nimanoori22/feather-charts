# Implementation roadmap: mouse navigation and crosshair

Date: 2026-10-12. Status: implemented and verified automatically; the user confirmed native responsiveness and smooth zoom. Full mouse/touchpad gesture validation remains unperformed.
Execution checklist: [todo.md](todo.md).

## Outcome and scope

Make the existing one-pane line-chart demo interactive: wheel zoom around the
pointer, horizontal drag scrolling, then a crosshair with time/price labels and
magnet snapping. Deliver navigation before beginning crosshair work.

This roadmap covers the steps recommended after the
[complete-display milestone](../docs/porting/complete-display-verification.md).
It does not include new series renderers, multi-pane UI, the full public chart
API, touch/pinch gestures, kinetic scrolling, axis dragging, or general series
hit testing. These remain separate milestones. Basic crosshair lines precede
point markers; add markers only if the traced crosshair contract requires them
for this line-chart scope.

## Current foundation

- `src/model/time_scale.rs` already provides `zoom`, `start_scroll`, `scroll_to`,
  and `end_scroll`. Audit their source semantics and effect delivery before reuse;
  their existence does not prove that the complete interaction path is ready.
- `ChartModel` owns recalculation and pending invalidation. Its public
  `time_scale()` access is immutable; navigation belongs through model operations.
- `examples/line_chart.rs` currently handles canvas redraws for layout and
  animation, with drawing borrowing an owned prepared frame.
- The [layout contract](../docs/porting/axis-aware-layout.md) currently excludes
  floating crosshair labels from axis sizing. Label support must explicitly
  extend the measurement and settlement contract.
- Iced's local canvas `Program::update` can publish messages and capture events.
  Wheel input distinguishes `Lines` and `Pixels`; treat them as distinct units.

These findings were checked using codebase-memory and direct source reads.
Coverage metadata reported no recorded issues for the inspected files, but is
best-effort. Full consumer investigation remains an implementation prerequisite.

## Implementation order

```text
0. Source contract and interaction design
   -> 1. Wheel zoom through model and demo
   -> 2. Drag scrolling through model and demo
   -> 3. Navigation regression and source comparisons
   -> checkpoint: usable mouse navigation
   -> 4. Crosshair state and pointer updates
   -> 5. Crosshair lines in prepared frames
   -> 6. Floating time/price labels and layout
   -> 7. Magnet snapping
   -> 8. Integrated regression and milestone report
   -> checkpoint: interactive line chart complete
```

Task 4 begins with its own source/design investigation rather than treating this
roadmap as a settled crosshair API. Task 6 should be split by time/price axis if
its measured scope exceeds one focused implementation session.

## Source investigation map

Before each component is implemented, follow the repository's porting workflow:
read the complete source component and semantic dependencies, trace real
consumers across Lightweight Charts using codebase-memory, derive required
behavior, compare at least two viable Rust designs, describe ownership and
signatures, then check the design against consumers. Record the result in that
milestone's porting report before changing engine code.

| Work | Primary source under `../lightweight-charts/src/` | Questions to resolve |
| --- | --- | --- |
| Wheel navigation | `gui/chart-widget.ts`, `model/chart-model.ts`, `model/time-scale.ts` | Direction, speed adjustment, pointer anchor, clamping, option gates, animation cancellation, horizontal delta policy |
| Drag lifecycle | `gui/pane-widget.ts`, `gui/mouse-event-handler.ts`, model navigation methods | Movement threshold, button rules, coordinates, outside release, cancellation, resize/data changes during a drag |
| Crosshair state | `model/crosshair.ts`, `model/chart-model.ts`, pointer consumers | Raw versus applied position, visibility/modes, selected pane/scale, data lookup, updates during navigation |
| Crosshair drawing | `views/pane/crosshair-pane-view.ts`, `renderers/crosshair-renderer.ts`, `views/pane/crosshair-marks-pane-view.ts` | Layer order, clipping, source visibility/options, bitmap alignment, marker dependency |
| Crosshair labels | `views/price-axis/crosshair-price-axis-view.ts`, `views/time-axis/crosshair-time-axis-view.ts`, axis widgets/renderers | Formatting, backgrounds, edge placement, scale selection, width contributions |
| Magnet | `model/magnet.ts` and its consumers | Eligible series, nearest-value selection, ties, whitespace, hidden sources, selected price scale |

For Iced integration, inspect the indexed local `iced/` canvas event routing,
`Action` capture semantics, cursor coordinates, and window focus events. Inspect
`flowsurface/` for actual drag/zoom examples after locating them with graph
queries; it is an example, not the behavioral specification. Do not equate event
capture with OS pointer capture without checking the runtime behavior.

## Proposed architecture and ownership

Prefer a small canvas adapter that publishes typed interaction messages to the
application. The application invokes synchronous `ChartModel` operations and
prepares the next owned frame through the existing frame owner.

```text
Application owns DataLayer, ChartModel, animation controller, frame owner
Canvas state owns gesture state and redraw timing
Canvas borrows prepared frame -> publishes interaction message
Application -> ChartModel mutation/effects -> prepare owned frame -> draw
ChartModel owns crosshair state once introduced
```

No new shared model handle or owner back-reference is needed. Gesture state is
cleared on completion/cancellation; model crosshair state drops with the model.
Drawing remains read-only and cannot sample animations or measure text.

Two integration designs are viable:

1. **Canvas messages plus model-owned operations:** fits the current demo,
   keeps borrows short, and makes event sequences testable. Start here.
2. **A custom Iced widget directly coordinating model input:** offers more control
   over event routing, but broadens ownership and widget integration. Use only if
   source/runtime investigation demonstrates that canvas routing cannot deliver
   the required lifecycle; do not maintain both paths.

Exact model method signatures and the location of reusable input helpers are
decided in Task 0. Keep transient gesture state in the existing canvas initially;
extract only the small logic needed for reuse or behavioral tests. No Tokio,
async channels, locks, subscriptions, new dependencies, or general event bus.

## Required behavioral checks

- Convert canvas-local positions to plot-local logical coordinates using the
  settled plot rectangle. Axis widths and device pixel ratios must not shift
  the interaction anchor. A gesture begins in the plot; outside continuation
  follows the investigated drag contract.
- Normalize wheel units deliberately. Preserve source direction, anchor and
  scale limits, with documented native-platform differences. Vertical wheel
  zoom is this milestone; resolve and document horizontal wheel behavior before
  intercepting an event containing both deltas.
- Consume every scale effect required for recalculation and invalidation.
  Audit zoom's spacing and offset effects as well as source options before
  assuming the existing scale method is complete.
- Navigation stops incompatible animation work, including pending continuation;
  resize/redraw must not revive cancelled animation or completed gestures.
- Empty, whitespace-only, cleared, removed and reloaded data remain safe. Invalid
  coordinates must not create non-finite state or frame geometry.
- Crosshair is refreshed against current scales/data on navigation, resize and
  updates. Leaving the plot/window or losing focus cannot leave stale overlays.
- Floating labels use current formatting and explicit axis measurement rules;
  pointer movement must not cause an unstable resize/measurement feedback loop.
- Magnet tests preserve source selection behavior, including ties and missing
  data. Basic crosshair behavior remains independently testable.

## Validation and completion

Each task has focused tests and a visible demo acceptance check in `todo.md`.
Run source differential checks for deterministic navigation/crosshair geometry
and inspect representative native scenes; browser/Iced font pixels need not be
identical. Compare equivalent normalized inputs when browser and Iced input
units differ. Reuse existing reference scripts and display scenarios where
practical; add only the fixtures needed for the new behavioral checks.

At each milestone checkpoint run:

```sh
cargo fmt --check
cargo test --all-targets
cargo check --all-targets
cargo clippy --lib --examples -- -D warnings
git diff --check
```

Run relevant existing source comparison scripts after renderer/layout changes.
Preserve owned-frame immutability, read-only drawing, finite geometry, and
fractional-scale clipping tests. Manually exercise a mouse and, if available,
a touchpad; explicitly report unavailable hardware or untested platforms.

Record source consumers, required semantics, discarded TypeScript conventions,
chosen design/alternatives, ownership, exact changed files, tests, validation,
and remaining dependencies in the milestone report. No milestone is complete
on compilation alone.

## Risks and decisions still to resolve

| Risk | Resolution before implementation |
| --- | --- |
| Native wheel units differ from browser deltas | Define normalization from local APIs; compare deterministic values and check available hardware |
| Canvas misses release or focus events | Inspect actual dispatch and test the lifecycle; use the smallest runtime-supported cancellation path |
| Existing zoom/scroll methods omit needed effects or options | Compare source methods and consumer expectations before wiring input |
| Deferred navigation and layout replay change the anchor | Establish ordering and test wheel/drag during resize and animation |
| Crosshair labels change plot width while pointer is stationary | Derive source sizing policy and test settlement for bounded convergence |
| Magnet needs broader hit-testing machinery | Implement only the selection contract needed by the supported line-series path |

No calendar estimate is assigned before Task 0. Scope labels in the checklist
are provisional and include module exports and tests; split a task if actual
investigation grows it beyond a focused slice.
