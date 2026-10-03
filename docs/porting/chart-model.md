# Chart-model core

## Source and scope

Investigated `lightweight-charts/src/model/chart-model.ts`, with the data-update
and removal consumers in `src/api/chart-api.ts` and the frame consumer in
`src/gui/chart-widget.ts`. The complete ChartModel source was inspected; the Rust
implementation ports its synchronous ownership, dimensions, data fan-out,
viewport preservation, recalculation, and deferred invalidation core.

Crosshair, magnet snapping, hit testing, primitives, background-gradient APIs,
pane movement/resizing gestures, complete chart-option patches, and rendering
remain subsequent work. There are no placeholder implementations of them.

## Behavioral contract and consumers

The API data path invalidates affected Series caches before changing logical
indexes. It updates the shared time scale, applies rows, refreshes data-bearing
indices, and recalculates panes. Removal obtains a DataLayer response before
detaching the source, then applies that response to surviving series.

`ChartModel.updateTimeScale` distinguishes right-hand appends from prepended
history. After updating points and before replacing the base index, it checks
whether the previous last bar is visible. Right offset is compensated when the
viewport must remain stationary, including when whitespace replacement is not
allowed to shift the view. Both shifting flags are preserved.

`ChartWidget` batches masks, applies layout first, performs momentary autoscaling
before ordered time-scale commands, and can replay the retained mask after layout
changes. Work generated during processing becomes a separate pending mask.
Cursor-only masks do not execute deferred scale commands.

The first series requests full redraw. New panes request momentary autoscaling
even when persistent autoscaling is disabled. Empty, unpreserved panes are cleaned
up after removing their final series when another pane remains. That cleanup
reindexes pane objects, registry associations, and pending pane invalidations.

## Rust design and alternatives

`ChartModel<B, M>` directly owns the configured `TimeScale<B>`, panes, registry,
`ChartDataCoordinator`, and pending `Option<InvalidateMask>`. DataLayer stays
outside the model, as in the original API/model separation.

The alternative was a combined chart/data facade owning DataLayer as well. It
would offer convenient data mutation but combine API and model responsibilities
before those APIs are ported. The selected design reuses the existing response
types and coordinator with fewer architectural changes.

```text
Application / future chart facade
  +-- owns DataLayer
  +-- owns ChartModel
        +-- owns TimeScale
        +-- owns Panes / price-scale adapters -- strong --> Series state
        +-- owns registry --------------------- strong --> same Series state
        +-- owns coordinator / update adapters - strong --> same Series state
        +-- owns pending invalidation

UI frame owner
  +-- owns animation controller / timing
```

Built-in state uses existing `Rc<RefCell<Series>>` handles. Its update adapter
borrows briefly, mutates the same state seen by the Pane, and releases the borrow
before effects are processed. Public built-in queries return an immutable `Ref`
tied to the model borrow; callers cannot mutate the registry or clone its handles.

The registry uses an enum to distinguish built-in and custom entries. A custom
handle is retained as `Rc<dyn Any>` only at this heterogeneous owner boundary,
where typed update handoffs verify the actual registered instance. Custom data
remains typed and is moved without cloning its payload. Removing or dropping the
model releases adapters; externally owned custom handles may outlive registration.

Library adapters have no owner back-reference and no cycle. User-provided custom
formatters/autoscale callbacks must not capture their own Series strongly; use
Weak references where necessary. No particular drop order is required.

An operation coalesces recalculation effects, snapshots the visible strict range
once, and visits every pane. Recalculation effects emitted because a pane's range
changed are already satisfied by that pass. Redraw effects remain pending rather
than causing recursive model calls. Options-applied work is explicitly exposed
as a coalesced UI flag; TimeScale's existing delegate notifications remain intact.

No Tokio, tasks, Iced subscriptions, locks, or thread-safe ownership are introduced.
Optional TypeScript references become enums/Options; callback-driven invalidation
delivery becomes an owned pending mask; explicit destroy methods become RAII.

## Main API and usage

The initial options contain the existing layout, pane/price-scale, and grid
options plus `add_default_pane`. Pane indexes refer to the current layout;
retained masks can be replayed while that pane structure is unchanged.

```rust
let mut model = ChartModel::new(configured_time_scale, model_options);
model.register_series(id, SeriesType::Line, line_options, pane, position)?;
model.set_width(plot_width)?;
model.set_pane_height(pane, plot_height)?;

let update = data_layer.set_series_data(id, SeriesType::Line, input)?;
model.apply_data_update(update)?;
model.fit_content();

// The UI supplies time and applies layout dimensions before this call.
if let Some(mask) = animation_controller.draw_frame(&mut model, now) {
    // Render according to the retained mask and current model state.
}

let update = data_layer.update_series_data(id, next_bar, false)?;
model.apply_data_update(update)?;

let update = data_layer.remove_series(id)?;
model.remove_series(id, update)?;
```

Registration requires an explicitly selected price-scale position and an existing
pane. `add_pane()` creates panes. Duplicate IDs, incorrect panes, mismatched series
options, unknown updates, wrong update categories, and mismatched custom handles
return typed errors before model mutation. ID allocation and selection of default
series options belong to the future facade.

Advanced frame consumers can use `take_invalidation()` and
`apply_invalidation(&mask)` separately. The mask stays readable for layout replay.
`TimeScaleAnimationController::apply_frame` samples animation and carries surviving
requests into pending work before later navigation can cancel them. Its existing
terminal-frame behavior is retained. `draw_frame` also adopts new scroll descriptors
once; navigation cancels descriptors awaiting adoption as well as active continuation.

## Coherent horizontal behavior

DataLayer and TimeScale own separate B values. `apply_time_scale_options` accepts
the external DataLayer and updates their common scale context together. The
behavior trait's `update_scale_options` hook lets the built-in time behavior keep
its formatting options synchronized. Its default does nothing for behaviors that
do not consume that context.

`configure_horizontal_behavior(data_layer, make_options)` builds equivalent owned
behavior options twice. This supports options containing boxed callbacks without
requiring B or B::Options to implement Clone. The factory must produce equivalent
configuration on each invocation. TimeScale's scale and localization context is
authoritative. Initial custom behaviors should be created with equivalent
configuration; changing time-key interpretation after loading data requires
rebuilding that data, rather than relabeling existing indexes.

## Integration corrections

The real pipeline exposed three dependencies needing correction:

- `Pane::momentary_auto_scale` now evaluates both left and right recalculations;
  boolean short-circuiting previously skipped the right scale.
- DataLayer's base index now follows the latest value-bearing point, preserving
  the source's zero base for whitespace-only data and no base for empty data.
- `CustomDataUpdateResponse` now carries affected built-in rows as well as custom
  payload/index updates. Custom changes to the shared time axis previously dropped
  that built-in fan-out. Existing custom response literals must include `series`.

TimeScale additionally supports resizing to zero width and exposes the narrow
context/accessors needed by its owner.

## Files and tests

Created:

- `src/model/chart_model.rs`
- `src/model/chart_model/tests.rs`
- `docs/porting/chart-model.md`

Modified:

- `src/model/mod.rs`
- `src/model/chart_data_coordinator.rs`
- `src/model/series.rs`
- `src/model/pane.rs`
- `src/model/data_layer.rs`
- `src/model/time_scale.rs`
- `src/model/ihorz_scale_behavior.rs`
- `src/model/horz_scale_behavior_time/horz_scale_behavior_time.rs`
- `src/model/invalidate_mask.rs`
- `src/views/time_scale_animation.rs`
- `docs/porting/invalidate-mask.md`

Connected tests use real DataLayer, TimeScale, Series, Pane, and ChartModel values.
They cover fit/resize and coordinate round trips, append/prepend/history behavior,
both whitespace-shifting flags, shared-axis reindexing/removal, left/right manual
autoscaling, zero-width/empty/whitespace-only states, failed registration and
invalid inputs, pane cleanup, command replay, typed custom updates, model drop,
configuration synchronization, animation progress/cancellation, and grid snapshots.

## Validation

`cargo fmt --check`, all 26 focused chart-model tests, the full 198-test suite,
`cargo check`, and `cargo clippy --lib -- -D warnings` pass. `cargo clippy
--all-targets` completes with five existing warnings in test code (date formatting,
plot-row creation, time behavior, and price-scale tests). `git diff --check` passes.
The API update/removal ordering and widget command/autoscale replay were rechecked
against the original consumers after implementation.

## Remaining dependencies

The next visible milestone needs a line pane view/renderer and an Iced widget or
demo. Crosshair/magnet/hit-testing and kinetic-animation algorithms are later ports.
The public chart facade will own DataLayer and model together, provide defaults and
ID allocation, and expose public data mutation APIs. Full chart options, pane
movement, primitives, and render-provider integration remain separate work.
