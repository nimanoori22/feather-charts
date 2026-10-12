# Mouse navigation

## Investigation and contract

The source navigation components are `ChartWidget` wheel normalization,
`MouseEventHandler` mouse drag dispatch, `PaneWidget` scrolling, and
`ChartModel`/`TimeScale` navigation. Wheel values use 32 pixels per DOM line,
divide by 100, and cap zoom magnitude at one. Native Iced positive wheel values
have the opposite sign from DOM wheel values. Both vertical zoom and horizontal
wheel scrolling are supported, with independent option gates.

The model clamps the zoom anchor to [1, width]. Scale zoom adjusts bar spacing
by one tenth per unit; offset compensation is conditional on
`rightBarStaysOnScroll`. Scrolling uses the saved offset plus displacement divided
by current spacing. The source mouse handler requires five logical pixels of
Manhattan movement; PaneWidget begins its scrolling transition after that
threshold and the first subsequent movement. Release is handled outside the
pane through root events. Touch, vertical price dragging and kinetic motion are
separate source behavior outside this milestone.

Consumers impose synchronous mutation, pane recalculation and redraw after
navigation. Source calls in chart/pane widgets were traced through codebase-memory;
method resolution is incomplete, so direct call-site reads supplement traces.
Crosshair updates use saved raw coordinates after recalculation.

## Design and ownership

Canvas messages plus model operations reuse the demo's existing ownership and
frame preparation. A custom Iced widget would also work but adds a widget owner
and event plumbing without a demonstrated need. The chosen canvas uses local
gesture state and publishes typed messages; it never borrows the model mutably.
Iced canvas dispatch forwards events to Program::update, and Action::and_capture
prevents bubbling, not OS pointer capture. Window leave/focus loss cancels the
gesture; movement/release outside the plot but inside the window remains handled.
Flowsurface's indicator canvas uses the same message/capture pattern.

```text
Application owns model, animation controller, frame owner and owned frame
Canvas owns gesture state; borrows frame; publishes navigation messages
Model owns scale transition; no canvas/model back-reference or Rc cycle
```

Model API (chosen before implementation):

```rust
pub fn zoom_time(&mut self, point: Coordinate, scale: f64);
pub fn start_scroll_time(&mut self, point: Coordinate);
pub fn scroll_time_to(&mut self, point: Coordinate);
pub fn end_scroll_time(&mut self);
```

Wheel handling maps to zoom plus an instantaneous scroll transition. Drag maps
to start/move/end. Model entry points validate finite inputs, consume scale
effects, recalculate panes and cancel incompatible pending/active animation.
No DOM callback identity, manual destruction, shared widget/model state, async
engine work, locks or additional dependencies are needed.

## Validation

Implemented 2026-10-12. Model navigation tests cover anchor preservation,
right-bar policy, invalid/empty input, recalculation and ended transitions.
`ChartInput` tests cover line/pixel units, independent option gates, axis exclusion,
the drag threshold, outside movement/release, focus loss during resize, repeated
cancellation and disabled redraws. The canvas/demo test passes real Iced events
through `Program::update`, applies model messages and checks prepared labels,
markers, stable axis width and animation cancellation at ratios 1, 1.25, 1.5, 2.
Existing demo sequences exercise append/prepend/replace, resize, remove/reload.

`interaction_reference` plus `check_interaction_reference.cjs` execute the
original TimeScale methods against 32 Rust zoom/scroll cases, covering narrow/wide
plots, both fixed-edge settings and both right-bar policies. The same harness
checks 16 crosshair-line, 36 floating-label and 5 magnet-selection cases.
The full milestone validation is recorded in [crosshair.md](crosshair.md).

Exact implementation files: `src/model/chart_model.rs`,
`src/model/time_scale.rs`, `src/ui/chart_input.rs`, `src/ui/mod.rs`,
`examples/line_chart.rs`, `src/model/chart_model/tests.rs`,
`src/model/chart_model/tests/navigation.rs`, `examples/interaction_reference.rs`,
`scripts/check_interaction_reference.cjs`. Roadmap status is in
`tasks/plan.md` and `tasks/todo.md`.

## Remaining work and limits

The one-pane demo supports wheel zoom, horizontal wheel scrolling and left-button
horizontal drag. Physical mouse/touchpad feel and actual window-system event
routing have not been manually exercised; synthetic native events and headless
Iced drawing do not prove those properties. Iced capture means event consumption,
not OS pointer capture. Leaving the window cancels a drag deliberately.
Touch/pinch, kinetic scrolling, vertical price dragging, axis manipulation,
multi-pane UI and public chart API integration remain separate milestones.

Pointer movement is idempotent: repeated native cursor coordinates do not
publish another UI message. The desktop freeze regression and follow-up
validation are recorded in [crosshair.md](crosshair.md#desktop-freeze-regression-2026-10-12).

After the freeze and glyph-cache fixes, the user confirmed native responsiveness
and smooth zoom. Full mouse gesture lifecycle and touchpad-specific checks remain
unverified; the earlier automated validation does not cover those checks.
