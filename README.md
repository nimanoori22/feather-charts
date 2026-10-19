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
calendar, strike-price, maturity and application-defined horizontal behavior.
Pane movement, series transfers and explicit pane removal are available. Exact
TradingView API parity remains incomplete; the completion report below lists
the remaining limits.
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
the application's datum. Conflation uses the view's typed reducer; views without
a reducer continue to draw their raw rows.
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
Last-price animation remains separate work. Iced gradients differ slightly from browser Canvas interpolation,
especially on WGPU; paths and gradient stops are checked against source.

Contracts and validation: [Area](docs/porting/area-rendering.md),
[Bar](docs/porting/bar-rendering.md), [Baseline](docs/porting/baseline-rendering.md).
Fresh physical mouse/touchpad acceptance for these new families remains pending.

## Last-price animation

Line, Area and Baseline support `LastPriceAnimationMode::{Disabled, Continuous,
OnDataUpdate}` through their existing `style.last_price_animation` option.
The default is Disabled. OnDataUpdate leaves a center dot after the pulse ends.
The native widget schedules redraws automatically.

```sh
cargo run --example public_chart -- --line --last-price-animation --live-feed
cargo run --example public_chart -- --area --last-price-animation-continuous --conflation
cargo run --example public_chart -- --baseline --last-price-animation --live-feed
```

See the [source contract and verification report](docs/porting/last-price-animation.md).

## Text watermarks

Attach a viewport-relative watermark to a pane using the existing primitive API:

```rust
use feather_charts::api::{TextWatermark, TextWatermarkLine, TextWatermarkOptions};

let mut options = TextWatermarkOptions {
    lines: vec![TextWatermarkLine {
        text: "BTCUSD".into(),
        ..Default::default()
    }],
    ..Default::default()
};
let id = chart.add_pane_primitive(0, TextWatermark::new(options.clone())?)?;
options.visible = false;
chart.replace_primitive(id, TextWatermark::new(options)?)?;
chart.remove_primitive(id)?;
```

Each line supports its own color, native `iced::Font` (family, weight, style),
font size and optional line height. Lines shrink independently to the pane width;
text stays anchored to the pane while data pans/zooms. Use the alignment fields
for left/center/right and top/center/bottom. Watermarks occupy the source's normal
primitive layer, so filled or densely drawn series can cover them. They add no
hit target or autoscale contribution.

```sh
cargo run --example public_chart -- --line --text-watermark --live-feed --primitives
cargo run --example public_chart -- --area --text-watermark
cargo run --example public_chart -- --line --conflation --text-watermark
```

The demo provides text, style, alignment and visibility controls. Load custom fonts
through the host's Iced setup before preparing the watermark; recreate/replace it
if font availability changes later. Native fallback and glyph metrics can differ
from browser fonts. See the [watermark port report](docs/porting/text-watermark.md).

## Image watermarks

Attach an image from decoded RGBA pixels using the existing pane primitive API:

```rust
use feather_charts::api::{ImageWatermark, ImageWatermarkOptions};

let mut options = ImageWatermarkOptions {
    alpha: 0.5,
    padding: 20.,
    max_width: Some(400.),
    ..Default::default()
};
let watermark = ImageWatermark::from_rgba(width, height, rgba_pixels, options.clone())?;
let id = chart.add_pane_primitive(0, watermark.clone())?;
options.alpha = 0.75;
chart.replace_primitive(id, watermark.with_options(options)?)?;
chart.remove_primitive(id)?;
```

Images stay centered in their pane, preserve aspect ratio and can enlarge to fit.
Optional maximum dimensions constrain fitting; zero means no maximum. Alpha zero
hides the image. Images use the bottom layer, below the grid and series, and add no
hit target, autoscale contribution or animation. Options updates share pixel data.
The host owns image fetching/decoding; the chart accepts decoded pixels only.

```sh
cargo run --example public_chart -- --line --image-watermark --text-watermark --live-feed --primitives
```

The demo provides opacity, padding, limits and image replacement controls. Append
`--wgpu` for GPU rendering. See the [port report](docs/porting/image-watermark.md)
for verification and the small local Iced rendering patches needed for draw order.

## Dense data and conflation

All six built-in families support optional conflation: aggregate drawing rows when
multiple bars occupy the same pixels. Canonical data, autoscale and exact-index
pointer payloads retain their original values. Enable `enable_conflation` and use
a sufficiently small `min_bar_spacing` to zoom out into dense history. Thresholds
can be set globally or per series; device pixel ratio affects level selection.

```sh
cargo run --example public_chart -- --line --conflation --live-feed
cargo run --example custom_series -- --conflation
```

The demo loads 8,192 bars plus a volume pane. Use the other series flags to try
Candlestick (default), Bar, Histogram, Area and Baseline. The custom demo retains
the last box with combined whisker extrema. Optional
`precompute_conflation_on_init` warms levels through bounded native chart ticks.
Custom hosts should keep ticking while `Chart::is_animating()` returns true.
Historical updates at effective factor >1 return
`ChartError::HistoricalUpdateWithConflation` before changing data. Latest updates,
appends and full replacements remain available. Custom reducers run synchronously
and must keep their work bounded.

Source behavior, ownership, validation and performance evidence are recorded in
the [conflation report](docs/porting/conflation-rendering.md). Physical acceptance
for this feature is tracked separately from automated verification.

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
The latest [interaction acceptance report](docs/porting/live-update-interaction-acceptance.md)
records the software drag and short-height fixes, plus the remaining timing gates.
The timing exception remains open. The latest update study is inconclusive;
native acceptance stopped after 31/32 passing cohorts in its first set.
Earlier occlusion improvements and the prior 5/6-update, 63/64-frame study remain
in the [historical report](docs/porting/render-update-follow-up.md).
The [timing attribution follow-up](docs/porting/live-update-timing-attribution.md)
records further diagnostics and full normal-build mouse passes on both backends.
Touchpad, touchscreen and high-DPI checks remain pending.
See the [mouse checks](tasks/live-append-mouse-acceptance.md) for physical coverage.

The latest [bounded timing/load check](docs/porting/histogram-timing-load.md)
shows that controlled CPU contention increases elapsed time while counted
Histogram work stays fixed; it establishes no new engine defect. Further timing
investigation is deferred. Conflation rendering is implemented; the historical
timing exception and acceptance thresholds remain unchanged.

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

The [larger Histogram latency study](docs/porting/histogram-tail-follow-up.md)
passes six predefined 2,000-sample comparisons. Repeat that controlled check with
`scripts/profile_incremental_append.py --histogram-tail`; the original full-matrix
failure remains recorded separately.

The [short-window cause investigation](docs/porting/histogram-window-cause.md)
records same-history controls and hardware counters. The focused runner reports
these controls alongside every raw timing failure to qualify history attribution.

## Up/down update markers

Line and Area series support temporary circles/chevrons for managed value changes:

```rust,ignore
let markers = chart.add_up_down_markers(series, UpDownMarkersOptions::default())?;
chart.set_up_down_data(markers, initial_data)?;
chart.update_up_down_data(markers, updated_item, historical, Instant::now())?;
```

Changes compare against the last managed full data set, rather than the preceding
update. Ordinary `set_data`/`update_data` calls bypass tracking; appended keys remain
untracked until a managed full reset. The default lifetime is five seconds; zero
expires immediately. `set_up_down_markers` installs indefinite manual markers.
Colors and future update lifetimes can be replaced without changing existing deadlines.
The widget schedules idle expiration; headless hosts use `next_up_down_expiration`
and `ChartMessage::Tick(now)` or `prepare(..., now)`.

```sh
cargo run --example public_chart -- --line --up-down-markers --live-feed --primitives
ICED_BACKEND=wgpu cargo run --example public_chart -- --area --up-down-markers
```

The demo includes latest/historical changes, manual markers, clear, colors, lifetime
and pause-feed/pulse controls. Historical changes follow the existing data API's
restriction against historical edits while bars are conflated. See the
[port report](docs/porting/up-down-markers.md) for source behavior and native corrections.

## Yield curves

Use `YieldCurveChart` for numeric maturities in months (`12` means one year).
It supports Line and Area curves, shared maturity indexing, month/year labels,
historical updates and numeric up/down markers through the existing synchronous
engine and Iced widget.

```rust,ignore
let mut chart = YieldCurveChart::new(YieldCurveOptions::default())?;
let series = chart.add_line_series(Default::default(), 0, PriceScalePosition::Left)?;
// Use BuiltInSeriesDataItem<Maturity>; construct maturities with
// Maturity::from_months(months), which rejects nonfinite numbers.
chart.set_data(series, rows)?;
chart.fit_content();
```

Defaults show the left yield axis, format values as `5.378%`, and hide last-value
labels and price lines. Series patches allow explicit overrides. Use
`set_maturity_formatter` for horizontal labels and `replace_yield_curve_options`
for scaffold resolution/start/minimum. The private whitespace scaffold preserves
upstream spacing and retained extent: clearing or shortening curves does not
necessarily shrink a previously extended horizontal domain. Scaffold generation
is limited to 100,000 points; errors preserve existing data and markers.

```sh
ICED_BACKEND=tiny-skia cargo run --example yield_curve -- --live-feed
ICED_BACKEND=wgpu cargo run --example yield_curve -- --live-feed
```

The demo includes yield changes, 35-year extension, reset/empty recovery, domain
and label controls, automatic/manual markers, a feed pause and a movable box with
right-edge resizing. Escape cancels an edit. Numeric custom series and price-based
options charts remain separate work. See the [port report](docs/porting/yield-curve.md).

## Price-based horizontal charts

Use `OptionsChart` when horizontal items are prices or strikes rather than dates.
It supports all six built-in series through the ordinary chart API:

```rust
use feather_charts::{
    api::{HorizontalPrice, OptionsChart, OptionsChartOptions},
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
    },
};
let mut chart = OptionsChart::new_options(OptionsChartOptions { precision: 2 })?;
let series = chart.add_series(
    SeriesOptionsMap::Line(Default::default()), 0, PriceScalePosition::Right,
)?;
chart.set_data(series, vec![
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: HorizontalPrice::new(100.5)?, value: 4.25,
        color: None, custom_values: None,
    })),
])?;
chart.replace_options_chart_options(OptionsChartOptions { precision: 3 })?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Prices must be finite; precision accepts 0–100 decimal places and defaults to 0.
Strikes occupy ordered logical indices: 90, 100 and 150 are successive positions,
not a continuous numeric distance scale. Horizontal precision leaves vertical
series price formatting unchanged. `set_horizontal_price_formatter` overrides
crosshair labels; axis ticks continue using precision. For full chart, horizontal
scale and localization overrides use `with_options_chart_options`.

Run `cargo run --example options_chart -- --live-feed`. The demo includes a Line,
volume Histogram, precision/label controls, update markers, a pulse, watermarks and
an editable box. Its toolbar scrolls horizontally at narrow widths. Use
`ICED_BACKEND=tiny-skia` or `ICED_BACKEND=wgpu` to select the native renderer.
The source contract and validation record are in
[docs/porting/options-chart.md](docs/porting/options-chart.md).

## Custom series on numeric axes

The existing custom-series API also works on `OptionsChart` and `YieldCurveChart`.
Implement `CustomSeriesPaneView<Frame>` with `Item` and `InternalTime` both set to
`HorizontalPrice` for strikes, or both set to `Maturity` for yield curves. Keep the
existing `CustomSeriesHandle<View>` syntax; the view determines the data types.

Use `add_custom_series`, `set_custom_data`, `update_custom_data`,
`replace_custom_view_options` and `replace_custom_series_options` on either chart.
`PointerEvent<T>::custom_data` and `custom_hit_data` require a matching view. The
renderer still receives logical indices in `bar.time`, while `original_data.time`
retains the numeric price/maturity. Conflation changes drawing data; handle and
pointer access still return original canonical data.

Yield custom series retain ordinary last-value and price-line visibility defaults;
the chart-wide yield formatter still applies. Custom mutations validate the bounded
yield extent before changing data, then settle the private domain scaffold through
the existing preparation path. Chart and renderer ownership stays synchronous.

Try the same native interval renderer in both modes:

```bash
cargo run --example numeric_custom_series -- --live-feed
cargo run --example numeric_custom_series -- --yield-curve --live-feed
```

Add `--dense` to start with 8,192 items. The toolbar scrolls horizontally and offers
latest/history/whitespace writes, clear/reset/remove/re-add, styles and label controls.
Hover shows original custom data and the separate renderer hit value. Select native
software/GPU rendering with `ICED_BACKEND=tiny-skia` / `ICED_BACKEND=wgpu`.
See the [port report](docs/porting/numeric-custom-series.md) for parity and acceptance.

## Dynamic pane management

Use `move_pane(from, to)`, `swap_panes(first, second)`,
`move_series_to_pane(series, target)` and `remove_pane(index)` on calendar, strike
and maturity charts. Pane indices are current positions and can change after an
operation. A series target past the end creates one appended pane. Transferring
the last series can remove an unpreserved empty source pane.

Series IDs, custom handles, data and series attachments survive transfers. Pane
watermarks and scale settings stay with their pane. Structural changes cancel
active gestures and edits. Invalid calls and identity operations preserve them.

Explicit removal cleans the selected pane's live series and attachments, while
retained custom handles keep readable data. The final pane remains. Yield charts
retain their private horizontal support data. This native cleanup policy avoids
the local TypeScript source's orphaned-map and incorrect widget-detach behavior;
see the [source findings](docs/porting/pane-management.md).

```bash
cargo run --example pane_management -- --live-feed
cargo run --example pane_management -- --options --live-feed
cargo run --example pane_management -- --yield-curve --live-feed
```

The scrolling toolbar moves/swaps panes, transfers the blue or custom series,
creates a destination pane, removes the last pane and resets the scene. The box
and temporary update markers follow the blue series; watermarks follow panes.
Select `ICED_BACKEND=tiny-skia` or `ICED_BACKEND=wgpu` for native rendering.

## Series drawing order

Use `series_order(id)` and `set_series_order(id, order)` to read/change an existing
built-in or custom series' zero-based position within its pane. Oversized indices
clamp to the last source. Data, IDs and attachments remain owned by the same series.
Changing order cancels active gestures/edits; invalid calls and identity operations
preserve them. Hover raising affects presentation independently of stored order.

Maturity indices count the private whitespace support slot, so the first public
series normally starts at 1. That support series cannot be reordered through the
public API. Reordering can change the price-scale formatter; existing Rust policy
also refreshes overlay formatters and chooses visible leaders. See the
[source contract and verification](docs/porting/series-order.md).

```bash
cargo run --example pane_management -- --series-order --live-feed
cargo run --example pane_management -- --series-order --options --live-feed
cargo run --example pane_management -- --series-order --yield-curve --live-feed
```

Select the blue line or custom intervals, then use Backward/Forward/To back/To
front. The second toolbar displays the selected series' order and controls hover
raising. Both toolbars scroll horizontally. Pane controls and Reset remain available.

## Canonical series data and trailing removal

`series_data(id)` returns fulfilled built-in bars oldest first.
`data_by_index(id, index, direction)` searches the shared logical axis using
`MismatchDirection::{None, NearestLeft, NearestRight}`.
`bars_in_logical_range(id, range)` returns fractional before/after counts and an
optional pair of original horizontal items. Counts can be negative; whitespace
and conflated drawing samples are excluded from these data results.

`pop_data(id, count)` removes fulfilled trailing bars and returns them newest first.
Zero and empty pops do nothing; oversized counts remove all fulfilled bars.
Whitespace/shared horizontal memberships remain as required. The series, options,
pane and attachments survive, and updates can resume afterward.

Typed custom handles expose `data()`, `data_by_index` and `bars_in_logical_range`;
`pop_custom_data(&handle, count)` removes typed custom data through its owning chart.
Detached handles keep readable data but cannot mutate a chart. Nonfinite query
indices and nonfinite/reversed ranges return `ChartError::InvalidDataQuery`.
Maturity charts protect private support data and retain the established scaffold.
See the [source contract and evidence](docs/porting/series-data.md).

```bash
cargo run --example pane_management -- --series-data --live-feed
cargo run --example pane_management -- --series-data --options --live-feed
cargo run --example pane_management -- --series-data --yield-curve --live-feed
```

Use the selected-series toggle, Pop 1/3/all, Append and Inspect controls. The
readout reports removed values newest first, the canonical count, a nearest-right
lookup at logical index 0 and range information for 0..10. Toolbars scroll
horizontally. Feed ticks update surviving bars and do not resurrect an emptied
series; use Append or Reset to restore data.

## Price scales and runtime configuration

Applications can read and set a pane's vertical range with `visible_price_range`
and `set_visible_price_range`, control autoscaling with
`set_price_scale_auto_scale`, and update an existing scale (including named
overlays) with `apply_price_scale_options`. Methods take a pane index and
`&PriceScalePosition`; results and option readbacks are owned values. Manual
endpoints use scale units: normal prices, percentages/index values, or logarithmic
units in log mode, matching the source API. Invalid or overflowing ranges fail
before changing the viewport.

`apply_chart_options(ChartOptionsPatch { model: ChartModelOptionsPatch { .. }, .. })`
updates layout, grid, price/time scales and crosshair without rebuilding series.
The same patch can include mouse/touch/kinetic settings. Unspecified fields stay
unchanged; the complete patch is validated before application. `runtime_options`
returns supported presentation/default/input settings, while `price_scale_options`
returns the selected live scale's settings. Renderer construction options and
attribution-logo drawing are outside this API.

Try the scrollable configuration toolbars in any chart family:

```sh
cargo run --example pane_management -- --price-scale-configuration --live-feed
cargo run --example pane_management -- --price-scale-configuration --options --live-feed
cargo run --example pane_management -- --price-scale-configuration --yield-curve --live-feed
```

Use `ICED_BACKEND=tiny-skia` or `ICED_BACKEND=wgpu` to choose the native renderer.
See the [configuration contract and evidence](docs/porting/price-scale-configuration.md).

## Runtime formatting

`Chart::apply_localization_options(LocalizationOptionsPatch)` updates price,
percentage, tick and horizontal formatters together. Read an owned configuration
with `localization_options()`. Callback fields use `FieldPatch::Unchanged`,
`Set(Rc::new(...))` or `Clear`; omitted fields preserve settings. Clear restores
series/default formatting. Batch callbacks must return one label per input value.

For normal/log scales, chart-wide tick formatting takes precedence over chart-wide
scalar formatting, then the per-series batch/scalar formatter. Percentage scales
use the corresponding percentage hooks. Indexed-to-100 uses its own formatter.
Series formatter queries stay series-specific. Yield charts provide their default
three-decimal percentage text through chart localization.

```sh
cargo run --example pane_management -- --formatting --screenshot-export --live-feed
```

Add `--options`, `--yield-curve` or `--custom-domain` for the other domains. The
formatting controls exercise callback precedence, explicit clearing and all scale
modes. Existing export previews retain their original labels when configuration
changes; capture again to refresh them. For calendar locale controls, use `--locale`
instead of `--formatting`. Verified Gregorian profiles are en-US, fr-FR, de-DE
and ar-EG (plus language aliases), with latn/arab/arabext tick digits through the
supported Unicode extensions. Numeric date tokens and crosshair times stay ASCII.
`formatters::time_locale::time_locale_support` reports verified, best-effort or
English fallback coverage. Non-calendar axes keep their domain formatting.
See the [locale support contract](docs/porting/time-locale.md) for limits;
non-Gregorian calendars and full Intl equivalence remain outside this milestone.

See the [formatting contract, API example and acceptance record](docs/porting/formatting-parity.md).

## Configured colors and output target

```sh
cargo run --example pane_management -- --colors --screenshot-export --live-feed
```

Add `--options`, `--yield-curve` or `--custom-domain`. Palette, background, row and
extension controls use constructor `layout.color_parsers`; successful parses share
one chart cache. Invalid colors and P3 target requests are rejected atomically.
Native rendering/export remains sRGB. A callback receiving `color(...)`,
`lab()/lch()` or `oklab()/oklch()` must return the application's chosen sRGB `Rgba`.
These source-preserved functional forms go through callbacks rather than an
implicit native-library conversion.

Use `Chart::try_new` for immediate calendar option validation. Existing infallible
`Chart::new` remains available and reports unsupported targets from preparation.
Runtime `LayoutOptionsPatch.color_space` can request sRGB; DisplayP3 returns
`ChartModelError::UnsupportedColorSpace` and leaves the chart unchanged.
Parser replacement is not offered; create another chart to change its policy.

`Chart::color_parser()` returns a clone sharing the fixed callbacks/cache.
Use `TextWatermark::with_color_parser(options, chart.color_parser())` for configured
watermarks. Custom views can validate options with `validate_options` and resolve
native ink in `update_with_color_parser`; primitives use `validate_colors` and
`PrimitiveContext::color_parser`. Resolve colors before freezing geometry; raw
application Canvas drawing remains application-owned. Prepared screenshots retain
their original colors after options change or the chart is dropped.

See the [color contract and migration notes](docs/porting/configured-colors.md).

## Axis interaction controls

Drag the horizontal-axis labels left/right to change spacing about the right edge.
Double-click restores the chart's configured spacing and right offset, including
pixel offsets. Price-axis reset enables autoscaling. Both axes also support touch
scaling and double-tap reset under their respective touch-direction settings.

`AxisInteractionOptions` exposes independent `time_drag`, `time_reset`, `price_drag`
and `price_reset` flags (all default to true). Use `set_axis_interaction_options`
or `ChartOptionsPatch.axis_interaction`, and read `axis_interaction_options()` or
`runtime_options().axis_interaction`. Disabling a drag cancels that axis's capture;
unrelated axis flags preserve plot/edit gestures. Plot wheel and pinch remain separately configured; the existing native
price-axis wheel extension stays available, including with the axis flags off.

```sh
cargo run --example pane_management -- --axis-interaction --screenshot-export --live-feed
```

Add `--options`, `--yield-curve` or `--custom-domain` for strike, maturity or sessions.
Try the four independent toggles, Defaults 9 / +3, All off and All on.
All off explicitly disables plot navigation as well; boundary labels align inside
the static chart, and re-enabling navigation refreshes them. Resize/pane mutations
cancel stale captures. Reset now uses configured defaults rather than global defaults;
price-axis double-click reset occurs on the second successful release rather than
on its press. Exhaustive `ChartMessage` consumers must handle the new `TimeAxis` variant.
Explicit struct literals for the runtime patch/readback gain `axis_interaction`;
using `..Default::default()` for patches remains compatible.

See the [source contract and verification record](docs/porting/axis-interaction.md).

## Series price-scale assignment

Use `set_series_price_scale(id, PriceScalePosition::Left/Right/Overlay(name))` to
move an existing built-in or custom series within its current pane. Data, handles,
drawing order and attached annotations remain with the series. `series_price_scale`
returns a fresh checked assignment; no retained scale wrapper is required.

Creation's explicit position argument wins over a conflicting `price_scale_id`.
On option replacement, `Some(id)` requests reassignment and `None` keeps the current
scale. Stored options are normalized to actual membership. Named overlays are created
on demand, can have an empty name, and are removed when their last source leaves.
Shared scales retain manual ranges. Typed Overlay("left")/Overlay("right") aliases
are rejected; use the dedicated variants. A custom handle's `price_scale_id()` remains
readable after detachment, while the chart query rejects detached/foreign IDs.

```sh
cargo run --example pane_management -- --scale-assignment --screenshot-export --live-feed
```

Add `--options`, `--yield-curve` or `--custom-domain` for other domains. Select blue
line/custom, switch between setter/options, and try side axes, overlays and Freeze.
Use Custom→line to share a pane before assigning both series to Shared A. Plot drag
follows the pane's default scale; axis gestures control that axis independently.
See the [assignment contract and evidence](docs/porting/scale-assignment.md).

## Public application events

Enable `EventInterests` for data changes, plot double-clicks, visible logical and
original-item horizontal ranges, and horizontal-axis dimensions. These additions
default to off. Existing click, crosshair and edit events remain enabled.

```rust
# use feather_charts::api::{Chart, EventInterests};
let mut chart = Chart::default();
chart.set_event_interests(EventInterests::ALL);
// Apply a batch of setters and prepare the frame, then:
for event in chart.take_events() {
    // React after the mutable chart borrow ends.
    println!("{event:?}");
}
```

`Chart::update` returns and consumes the same queue. Process that returned batch;
do not expect it again in `take_events`. Enabled hosts must drain after every
batch: the API queue is lossless and uncapped. Events own their payloads; a data
event contains a series ID and `Full`/`Update` scope. Guard range equality when
linking charts or loading history in reaction to events.

Try event counters, family toggles, equal replacement, latest change, fractional
offset and axis visibility, with pop/append and pane controls:

```sh
cargo run --example pane_management -- --public-events --live-feed
cargo run --example pane_management -- --public-events --options --live-feed
cargo run --example pane_management -- --public-events --yield-curve --live-feed
```

The demo keeps only twelve history summaries; API delivery remains lossless.
Exhaustive matches on `ChartEvent`, `ChartMessage` and `TouchAction` must handle
the added variants. With double-click interest enabled, a successful second plot
click/tap produces `DoubleClick` instead of another `Click`; default input behavior
is unchanged. See the [event contract and evidence](docs/porting/public-events.md).

## Chart screenshot/export

`Chart::screenshot_of_prepared(layers)` captures the last successfully displayed
frame. `prepare_screenshot(layers, now)` first prepares the current viewport,
including yield-curve settlement. Both return an owned `ChartScreenshot`; queued
host events remain available. Prepared capture can deliberately precede pending
data changes. Neither method changes the live viewport for export sizing.

Initialize `ChartExporter::new(ScreenshotBackend::Software)` or `Wgpu` once through
the host runtime, then call `render(&snapshot, options)` repeatedly. Backend choice
is explicit. Output is owned, tightly packed straight-alpha sRGB RGBA8 with physical
dimensions. `ScreenshotImage::into_handle()` creates an Iced preview image; the host
owns encoding/files. `ScreenshotRasterOptions` selects a pixel ratio or an explicit
pixel canvas that uniformly fits the captured layout with padding (64 MiB output,
8192px dimension limits). Load host fonts before chart preparation.

`ScreenshotLayers::default()` excludes the top canvas and crosshair. Top primitives,
top markers and the price pulse require `include_top_layer`; crosshair lines,
markers and labels also require `include_crosshair`. Normal annotations remain.

```sh
cargo run --example pane_management -- --screenshot-export --live-feed
cargo run --example pane_management -- --screenshot-export --options --live-feed
cargo run --example pane_management -- --screenshot-export --yield-curve --live-feed
```

The toolbar offers capture, layers, ratio, uniform-fit canvas, preview and raw/PPM
saving to `target/screenshot-export`. Press **F8 while hovering the chart** to retain
a crosshair in the capture. Preview is a frozen image; Hide preview restores chart
space. Iced's readback API can panic on device-loss/map/allocation failure; invalid
user dimensions return typed errors before rendering. See the
[screenshot contract and verification](docs/porting/screenshot-export.md).

## Application-defined horizontal domains

Use `Chart::with_custom_behavior` with your own `HorzScaleBehavior` and
`ChartBehavior` implementation. Your public item can differ from its internal key;
queries, custom-series data and events retain the original item. Bars occupy logical
indices, so gaps between keys do not create proportional gaps on screen.

The factory creates two equivalent, independently mutable behaviors for data and
scale ownership, then is dropped. Neither the behavior nor its options needs
`Clone`, `Send` or `Sync`. Keep conversion, ordering and cache keys equivalent.
`ChartError::CustomBehavior(Box<dyn Error>)` preserves application-owned errors for
`Error::source` and downcasting. Exhaustive `ChartError` matches must handle this
new variant. Because its cause may be non-`Send`, convert errors to a suitable
owned payload before sending them across threads. Existing chart constructors
and their error variants are unchanged.

`horizontal_behavior_options()` reads your current options.
`configure_horizontal_behavior(make_options)` updates both owners and invalidates
formatted labels. Live formatting changes must preserve key interpretation and
stored weighting rules. Clear all shared series before changing those policies, then reload them;
alternatively, construct a new chart. The engine remains synchronous.

```sh
ICED_BACKEND=tiny-skia cargo run --example custom_horizontal -- --live-feed
ICED_BACKEND=wgpu cargo run --example custom_horizontal -- --live-feed
```

This external-consumer example defines `SessionPoint { session, phase }`, a separate
internal `SessionKey`, and custom interval data. Its scrolling toolbar exercises
updates, history, append/pop/empty/reset, labels, invalid items, pane transfers and
export. The box moves and resizes; Escape cancels edits. Capture saves raw RGBA plus
size metadata to `target/custom-horizontal/preview.{rgba,json}` and displays an
owned image. See the [contract and evidence](docs/porting/custom-horizontal.md).

## Attribution logo

Enable the source logo at construction with
`ChartModelOptions::layout.attribution_logo`, or update it through
`ChartOptionsPatch.model.layout = Some(LayoutOptionsPatch {
 attribution_logo: Some(true), ..Default::default()
})`. Runtime readback is `chart.runtime_options().model.layout.attribution_logo`.
The native default remains **off**. Runtime toggling is a native extension.

The 35×19 vector stays 10 logical pixels from the last plot's left/bottom edges.
Its theme follows text brightness. Drawing and input are clipped to that plot;
it follows pane reorder/removal and does not affect layout or autoscaling.
All screenshot layer combinations exclude it, matching TradingView's DOM overlay.

A click/tap emits `ChartEvent::AttributionActivated { url }` even with default event
interests. The fixed destination is `https://www.tradingview.com/`. The host chooses
whether to open it; the chart engine never launches a browser. Dragging, outside
release, Escape, lost focus and changed owner/bounds cancel activation. Existing
drags retain ownership when they cross the logo. Hovering it clears plot hover.

```sh
cargo run --example pane_management -- --attribution --screenshot-export --live-feed
cargo run --example pane_management -- --attribution --screenshot-export --options --live-feed
cargo run --example pane_management -- --attribution --screenshot-export --yield-curve --live-feed
cargo run --example pane_management -- --attribution --screenshot-export --custom-domain --live-feed
```

Set `ICED_BACKEND=tiny-skia` or `ICED_BACKEND=wgpu` to select the renderer. Logo
visibility, background and text/theme controls are independent. The demo displays
a persistent activation counter and host link action without opening a URL. Pane
controls and Reset exercise last-pane ownership; screenshot preview/file controls
exercise source export exclusion. Press F8 while hovering the chart to capture it.

Migration: exhaustive matches need the new `ChartMessage::AttributionActivated` and
`ChartEvent::AttributionActivated` arms. Complete layout/frame struct literals need
the new attribution fields (`None` for a manually constructed frame); prefer
`..Default::default()` where applicable. Existing constructors keep their defaults.
See the [source contract and verification](docs/porting/attribution.md) and the
[preserved source notices](docs/porting/attribution-source/NOTICE).

## Public parity audit

The [local 5.2.1 parity audit](docs/porting/public-parity-audit.md) records the
remaining API, option-routing and edge-case differences after the feature roadmap.
It includes a 374-member inventory and reproducible source/native diagnostics.
The original audit identifies gaps; its successful diagnostic command is not a
parity pass. The [combined completion record](docs/porting/public-parity-completion.md)
tracks milestones A–I, final validation and the remaining explicit limitations.

### Public queries and crosshair synchronization

The reusable owner exposes `visible_logical_range`, `visible_horizontal_range`,
`set_visible_horizontal_range`, `reset_time_scale`, `scroll_position` and
`scroll_to_real_time` (the configured right offset, animated over 400 ms).
Horizontal range endpoints are inclusive. Queries resolve current lazy range
caches without consuming events or preparing a frame. Queued navigation settles at the next `prepare`.

Use `series_pane`, `series_type`, `series_price_scale`, `price_lines` and
`last_value_data` for checked current ownership/readbacks. IDs and returned values
are owned; removed/foreign series are rejected. `with_series_options` and
`with_series_formatter` provide scoped reads without copying application callbacks.
Typed custom handles expose `with_options`, `with_formatter` and
`with_view_options(&chart, ...)`; model reads survive detachment, view reads do not.
Do not retain references from these scoped reads.

`price_to_coordinate` and `coordinate_to_price` use the selected series' actual
scale and visible first value, including log/percentage modes. Coordinates are
pane-local; prepare after data, layout or scale changes. Empty/unavailable
conversions return `None`. Horizontal conversion uses `horizontal_to_index`,
`horizontal_to_coordinate`, `coordinate_to_horizontal`, `logical_to_coordinate`
and `coordinate_to_logical`. The last two preserve source integer-index rounding;
use primitive transforms for continuous drawing positions.

`set_crosshair_at(series, &item, price)` and `clear_crosshair_position` emit no
pointer event, so hosts can synchronize charts without event feedback. As in the
source, the series selects the pane, its **default price scale** supplies Y, and
horizontal lookup uses the next available item. This differs from series-specific
price conversion when a pane has multiple scales. Placement returns `false` when
no usable domain or scale exists.

```sh
cargo run --example pane_management -- --public-helpers --series-order --series-data --scale-assignment --screenshot-export --live-feed
# Add --options, --yield-curve or --custom-domain for the other domains.
```

Select blue/custom, then try Items 4..18, Reset axis, Go live, Inspect,
Add/remove line, Place/Clear crosshair, Invalid range and Sync. Inspect reports
current ranges/ownership/formatting and a price round trip. Move the pointer on
either chart to synchronize crosshairs; deliveries should stop while idle. Data,
pane/scale transfers and export controls remain available. Reset the application
between destructive sequences. Crosshair buttons target the main chart; linked
updates follow actual pointer events.

### Persistent pane sizing

`chart.set_pane_height(pane, height)?` requests logical pixels from the last
settled layout. Prepare once before sizing and again to apply it. The request
changes stretch factors, so window resizing scales the resulting proportions;
it does not pin an absolute height. Other panes share the difference in source
pane order, with 30px clamps. Non-finite inputs and invalid panes are errors.
Single panes and collapsed layouts preserve their allocation.

```sh
cargo run --example pane_management -- --pane-sizing --screenshot-export --live-feed
# Add --options, --yield-curve or --custom-domain.
```

Use Next pane and 80/150/300px, Minimum/Maximum, Save/Restore layout and Invalid
height. Readouts show actual settled heights. Saved layouts retain stretch factors
by pane slot, so restore needs the same pane count; after removal, Reset and save
again. Pane moves keep their allocation; restoring assigns the saved proportions
to the current slots. Divider resizing and export remain available.

### Application metadata

`Chart<B, M = ()>` retains `custom_values: Option<M>` through built-in data
updates, queries, pop and `PointerEvent<T, M>::series_data`. M needs only Clone
and a static lifetime. Use `Rc<AppRecord>` for large records or shared identity;
the record itself needs neither Clone, Debug, equality nor Send/Sync. Owned
readbacks/events remain readable after a series or chart is removed. A plain M
uses its own Clone policy; the engine does not compare or serialize metadata.

Existing unit constructors and aliases keep working. Choose metadata explicitly:

```rust
let mut chart = Chart::<_, std::rc::Rc<AppRecord>>::new_with_metadata(Default::default());
// Strikes: Chart::<HorzScaleBehaviorPrice, M>::new_options_with_metadata(options)?
// Maturities: YieldCurveChart::<M>::new_with_metadata(options)?
// Custom domains: Chart::<B, M>::with_custom_behavior_and_metadata(factory, chart, scale, locale)?
```

Custom views still own their typed V::Data; their whitespace V::Metadata matches
the chart's M. A primitive that reads metadata implements DrawingPrimitive<B, M>
and receives PrimitiveContext<B, M>. Built-in snapshots own geometry, not live
metadata; custom renderers may retain the original data they explicitly capture.
Use Weak for application payloads that refer back to their owning chart.

```sh
cargo run --example pane_management -- --application-metadata --series-data --series-order --scale-assignment --screenshot-export --live-feed
# Add --options, --yield-curve or --custom-domain.
```

Pause the feed to inspect records. The Blue records toolbar replaces latest/history
metadata without changing price, clears a record, reports popped records and
retains data across Clear blue data. Inspect retained and Restore data use the
original owned records; Release retained drops that observation. Hover reports
the captured record ID/revision. Resume feed for interaction checks. Reset clears
the example's retained data. Existing custom intervals, pane/scale controls, box
edits and export stay available.

### Native pointer metadata

`PointerEvent<T, M>::source_event` is an optional owned `PointerInput` snapshot.
It reports mouse/touch identity, event kind, changed mouse button, chart-local
and window-content-relative positions, and the latest keyboard modifiers observed
by the Canvas. Positions use logical pixels; `PointerEvent.point` remains local
to the pane's plot. DOM page/screen coordinates and event/window references are
not exposed. Modifiers are unknown (`None`) until observed and after focus changes.
Mouse identifies Iced's event family, which also carries touchpad wheel/cursor
input; it does not identify physical mouse hardware. Touch IDs identify contacts.

The widget samples input before dispatch. Queued messages and retained pointer
payloads keep their original metadata. Touch tracking initiated by a redraw uses
its originating press snapshot. Existing synthetic-mouse suppression remains in
place. Ordinary programmatic messages and animation/viewport replays have no
native source; a host can explicitly attach a snapshot with
`ChartMessage::with_pointer_input`. `into_pointer_parts` unwraps the flat envelope
for adapters with exhaustive message matching. Such adapters must handle the new
`ChartMessage::PointerInput` variant; no per-movement Box allocation is needed.

```sh
cargo run --example pane_management -- --pointer-input --application-metadata --series-data --series-order --scale-assignment --screenshot-export --live-feed
# Add --options, --yield-curve or --custom-domain.
```

Ctrl-click pins an owned inspection snapshot; Shift-click clears it (including
when a rapid second release is recognized as a double-click). The panel displays
source coordinates/modifiers and counts clicks/double-clicks. Clear pin and
Programmatic click are available in the horizontal toolbar; the latter reports
no source and leaves the pin intact. Pause feed for stable inspection. Move focus
away and back to verify modifiers do not remain stuck. Both readout rows scroll
horizontally at narrow sizes. Reset clears the pin while preserving pointer mode.
