# Feather Charts

An in-progress Rust port of TradingView Lightweight Charts, using Iced 0.14.
The reusable component renders all six built-in series: Line, Candlestick,
Histogram, Area, Bar and Baseline,
including multiple series per pane and synchronized price/volume panes.

Run the minimal application built with the public API:

```sh
cargo run --example public_chart
```

The existing interactive demos remain available: `line_chart`,
`candlestick_chart`, `histogram_chart`, and `price_volume_chart`.

## Embedding in an Iced application

Keep `feather_charts::api::Chart` in application state. Create series with
`add_series(options, pane, PriceScalePosition::Right)` and retain the returned
`SeriesId`. Use `set_data` to replace an ordered data set, or `update_data` to
append/replace the latest bar (`historical: true` updates an existing time).
Use `add_pane` and pane stretch factors for a second panel. Removing a pane's
last series also removes an unpreserved empty pane, retaining the final pane.

In `view`, embed `chart.element().map(Message::Chart)`. In `update`, pass the
received `ChartMessage` to `chart.update(message)` and handle the returned
`ChartEvent::Click` / `ChartEvent::CrosshairMoved` payloads. They contain optional
pane/point/logical/time/price and exact-index series data; whitespace has no value
row. The widget reports bounds and requests animation frames automatically.
Forward window scale-factor changes to `chart.prepare(size, ratio, now)`;
[public_chart.rs](examples/public_chart.rs) shows the complete host wiring.
After a batch of direct data/options/navigation changes, call `prepare` once.
Drawing borrows an immutable frame and never updates the model.

Hold the left mouse button in a plot to pan in both directions; wheel over a plot
to zoom. Wheel/drag on a price axis scales it; double-click restores autoscale.
Vertical plot movement explicitly enters manual price mode. Panes share time
navigation and keep independent price ranges.

This API covers built-in and typed custom series, native touch/kinetic navigation,
and the default `Time` behavior. Arbitrary pane reordering and full TradingView
API parity remain future work.
Options already represented by the engine do not imply every visual feature is
rendered. Read-only `model()` inspection remains available for advanced hosts.

Implementation contract and verification: [public chart API report](docs/porting/public-chart-api.md).

## Typed custom series

Implement `CustomSeriesPaneView<iced::widget::canvas::Frame>` and supply an owned,
cloneable renderer. `Chart::add_custom_series` returns a typed handle; use
`set_custom_data`, `update_custom_data`, `replace_custom_view_options` and
`replace_custom_series_options` with that handle. The price builder supplies
autoscale extrema and its final value supplies the current price. Explicit
whitespace participates in shared time indexing without acquiring a price.

```sh
cargo run --example custom_series
```

The box-and-whisker example includes a built-in median line and controls for
replacement, append, historical updates, whitespace, styles, clear/reload and
removal. Existing marker, price-line and primitive APIs accept the handle's
`id()`. Pointer events offer `custom_data(&handle)` and
`custom_hit_data(&handle)` for checked typed lookup. Custom data stays with its
typed owner; preparation copies only visible payloads and adjacent join rows.

Renderer clones must own immutable geometry. Sharing mutable live geometry
between clones violates retained-frame behavior. Removed handles remain readable
but cannot mutate the chart; view cleanup runs once on detachment. The public
adapter uses `Time` and unit common metadata; additional metadata belongs in
the application's datum. Full conflation rendering remains separate work.
See the [custom-series contract](docs/porting/custom-series-design.md) and
[acceptance checklist](tasks/custom-series-todo.md).

## Built-in series

All six built-in families render through the public API: Line, Candlestick,
Histogram, Area, Bar and Baseline. Area supports independent fill/outline row
colors, relative gradients and inverted fills. Bar supports open ticks and thin
or thick stems. Baseline supports a projected base price, separate above/below
fill and line colors, relative gradients, and row overrides. Filled series support
simple, stepped and curved paths and point/crosshair markers.

Try the new families:

```sh
cargo run --example public_chart -- --area
cargo run --example public_chart -- --bar
cargo run --example public_chart -- --baseline
```

The same example also accepts `--line` and `--histogram`; its default is
Candlestick. Use **Change style** to exercise styles and Baseline base changes.
All families share data updates, price decorations, zoom/pan and scale controls.
Conflation and last-price animation remain outside this milestone. Iced gradients differ slightly from browser Canvas interpolation,
especially on WGPU; paths and gradient stops are checked against source.

Contracts and validation: [Area](docs/porting/area-rendering.md),
[Bar](docs/porting/bar-rendering.md), [Baseline](docs/porting/baseline-rendering.md).
Fresh physical mouse/touchpad acceptance for these new families remains pending.

## Price lines and labels

Series defaults now draw an automatic last-price line and last-value axis label.
The label uses the last value at the visible right edge; the line uses the latest
bar overall unless `price_line_source` is `LastVisible`. Series options control
visibility, line style/color/width, titles and price/percentage display mode.

Use `PriceLineOptions::new(price)` with `Chart::add_price_line(series, options)`
for custom entry/stop/target levels. Retain the returned `PriceLineId`, inspect
options with `price_line_options`, then call `replace_price_line_options` or
`remove_price_line`. Line and axis-label visibility are independent. Custom lines
use their series' scale and do not enlarge autoscale. Clearing data retains the
levels; removing the series invalidates their IDs. Invalid updates preserve options.
After a batch, call `prepare` once, as for other direct Chart mutations.

Labels align per scale without moving the line's actual price coordinate; set
`align_labels: false` on the price-axis options to disable alignment. The public
example includes append/move/remove/cluster controls and supports `--capture-dir`.
These levels support hover selection; trading actions, alerts and dragging
remain outside this milestone. Source contract and results:
[price-line report](docs/porting/price-lines.md).

## Series markers

Attach time-based annotations to any built-in series:

```rust
use feather_charts::api::{MarkerPosition, MarkerShape, SeriesMarker, SeriesMarkersOptions};

let mut marker = SeriesMarker::new(time, MarkerPosition::AboveBar, MarkerShape::ArrowUp, "green");
marker.id = Some("entry".into());
marker.text = Some("Entry".into());
let markers = chart.add_series_markers(series, vec![marker], SeriesMarkersOptions::default())?;
// Replace transactionally, clear while retaining the attachment, or detach it.
chart.set_series_markers(markers, replacement)?;
chart.set_series_markers(markers, vec![])?;
chart.remove_series_markers(markers)?;
```

Markers support circle/square/up/down arrows, six positions (including explicit
prices), size, optional text/IDs, autoscale padding and three drawing layers.
Keep each `MarkerCollectionId` to inspect or replace its markers/options.
After direct changes, call `prepare` once. Click/crosshair payloads include an
optional `marker` identifying the series, attachment, current item index and
external ID. External IDs can be absent or repeated. Clearing series data keeps
attachments for reload; removing the series releases them.

```sh
cargo run --example public_chart -- --series-markers
cargo run --example public_chart -- --series-markers --area
```

Use **Add/clear markers** and **Marker layer** to exercise replacement and layers.
Marker margins follow source rules; arbitrary text, stacked markers and explicit
prices are not guaranteed to fit inside autoscale. Editable annotations remain future work. Fresh marker mouse/
touchpad acceptance remains pending. See the [marker port report](docs/porting/series-markers.md)
and [implementation checklist](tasks/series-markers-todo.md).


## Series and price-line hover selection

All six built-in families support selection from settled logical geometry.
Click/crosshair events expose `hit: Option<ChartHit>` with the owning `SeriesId`.
`HitTarget` distinguishes series points, outlines/ranges, markers and price lines.
`PriceLineKind` distinguishes last-value, relative base and `Custom(PriceLineId)`.
External strings can be absent or repeated; typed handles remain distinct.

`chart.hit_test(pane, point)` uses raw plot coordinates, independently of crosshair
magnet snapping. The event's `marker` field describes a winning marker;
`chart.marker_hit(pane, point)` remains a separate marker-only query.
Call `prepare` after direct data/options changes to refresh the displayed frame.

Hovered series rise by default. Area/Baseline fills and normal marker attachments
keep their original order; only their outlines/points and price-line views rise.
`chart.set_hovered_series_on_top(false)` disables raising. Hit order stays stable.

```sh
cargo run --example public_chart -- --hover-selection
```

This scene combines filled series, an outline, price levels and markers. Use
**Raise hovered: on/off** to compare drawing order. The status reports the target
without resizing the chart in narrow windows.

Automated source, widget and renderer checks are recorded in the
[hover selection report](docs/porting/hover-selection.md) and
[checklist](tasks/hover-selection-todo.md). Fresh physical mouse/touchpad acceptance
is pending. Custom primitives/series, editable annotations and touch/kinetic input
remain outside this milestone.

## Custom drawing primitives

Attach application graphics to a series or a pane with
`add_series_primitive(series, value)` or `add_pane_primitive(pane, value)`.
Keep the returned `PrimitiveId` for `replace_primitive` and `remove_primitive`;
call `prepare` after direct changes. Pane ownership survives pane index shifts.
Removal and owner destruction notify `detached` once; retained frames keep their
prepared geometry.

Implement `DrawingPrimitive::prepare(&PrimitiveContext)` to return owned
`PreparedPrimitive` views. The context supplies logical/time/price conversion,
pane bounds, pixel ratio and read-only model access. `PrimitiveRenderer` draws
copied geometry through Iced Canvas on TinySkia or WGPU, with optional background
drawing and hit testing. Keep prepared renderers independent of mutable live
application state. Coordinates use logical pixels and views are clipped to their
plot or axis area. All three layers are supported.

Series attachments can also contribute autoscale ranges/margins, price/time
labels and separate graphics inside either axis. Pane attachments only contribute
plot graphics and hits. Fixed price coordinates bypass collision adjustment;
the upstream time-label renderer ignores fixed coordinates. Generic primitive
visibility is controlled by its prepared output, including on hidden series.

Click/crosshair payloads include `primitive: Option<PrimitiveHit>` with typed
attachment and series/pane ownership, external object ID and optional Iced cursor.
`chart.primitive_hit(pane, point)` queries the same winner using raw plot
coordinates. Primitive and built-in hits share source ordering, layer, background,
distance and priority rules. Strings may repeat without losing owner identity.

```sh
cargo run --example public_chart -- --primitives
```

Use **Add/remove region**, **Move region** and **Primitive layer** to exercise a
shaded price/time region, selectable boundary, autoscale, labels and axis graphics.
The volume pane also has an independent graphic. This scene mixes filled series,
price levels and markers, and keeps chart bounds stable when status changes.

Click the box to select it. Drag inside it to move it, or drag an edge/corner to
resize it using the eight handles. Click outside to deselect. Escape, focus loss
or leaving the window cancels the current drag and restores its starting bounds.
Dragging outside the box pans the chart. During an edit, wheel/axis navigation
is held; it resumes after release or cancellation.

Set `PrimitiveHitCandidate::editable = true` to opt an application drawing into
the widget's synchronous `ChartEvent::PrimitiveEdit` events. The external hit ID
names your application's edit target. Store the drawing state in the application
and copy its geometry during preparation. `request_primitive_update(id)` notifies
external changes; `preview_primitive_update(edit_id)` notifies an active editor
without detach/reattach. Apply an edit event to your state, then call `prepare`
once. Edit messages return owned events without preparing a discarded frame.
`PrimitiveContext::coordinate_transform()` copies the time/owner price mapping
for start-relative previews, including fractional logical positions.

The demo holds autoscale and allocated axis sizes during a drag, restoring their
prior policy afterward. External replacement/update cancels without rolling back
newer geometry; removal never recreates an attachment. Adding hit fields/event
variants requires downstream struct literals and exhaustive matches to adapt.

Editing captures and profiling:

```sh
cargo run --example public_chart -- --primitives --primitive-edit-captures-only --capture-dir /tmp/feather-editing
cargo run --example public_chart -- --primitive-edit-profile
```

See the [primitive port report](docs/porting/primitive-support.md) and
[checklist](tasks/primitive-support-todo.md) for source comparisons and validation.
See the [editing report](docs/porting/primitive-editing.md) and
[editing checklist](tasks/primitive-editing-todo.md) for the interaction contract
and acceptance evidence. Physical mouse/touchpad editing acceptance is pending.
More drawing tools, touch editing and JavaScript plugin compatibility remain future work.

## Native touch and inertia

All chart demos accept native touchscreen input. Drag one finger to pan; use
two fingers to zoom around their initial midpoint. Hold for 240 ms to track
the crosshair. Tracking persists until the next unmoved tap by default; choose
`TrackingExitMode::OnTouchEnd` to clear it on release.

`Chart::set_touch_navigation_options` controls horizontal/vertical drag
delegation, pinch and tracking exit. `Chart::set_kinetic_scroll_options` controls
inertia: touch is enabled and mouse disabled by default. Flicking animates time
offset; vertical price movement remains manual without inertia. New navigation,
focus loss, owner removal and incompatible layout changes stop the animation.
Touchpad wheel input continues through the wheel path.

Try `cargo run --example custom_series` for mixed custom/built-in data or
`cargo run --example public_chart -- --primitives` for mouse box editing alongside
touch navigation. Touch does not edit annotations. Physical touchscreen and
fresh mouse/touchpad acceptance remain pending. See the
[touch report](docs/porting/touch-navigation.md) and
[roadmap checklist](tasks/custom-series-todo.md).

Earlier preparation/update measurements are recorded in the
[performance report](docs/porting/preparation-performance.md).

## Incremental live updates

Native time-series updates now append one shared time point and one changed row
for built-in and typed custom series. Unchanged peer histories stay in place;
shared middle insertions retain full reindexing. Custom live updates preserve
mouse drags. See the [append report](docs/porting/incremental-append.md) and
[acceptance checklist](tasks/incremental-append-todo.md) for validation and limits.
The strict timing screen and physical-device acceptance remain open.

```sh
cargo run --example public_chart -- --line --append-captures-only --capture-dir target/append-captures
cargo run --release --example preparation_profile -- --rows 100000 --series 16 --kind line --phase append --fixed-last-price
```

For hands-free interaction checks, both demos have a **Start/Stop live feed**
control. Start with the feed already running:

```sh
cargo run --example public_chart -- --line --primitives --live-feed
cargo run --example custom_series -- --live-feed
```

The synthetic feed appends four bars per second. You can zoom, pan, use the price
axis, or edit a box while it runs. See the [acceptance investigation](docs/porting/live-append-acceptance.md)
for diagnostic evidence and remaining checks.
