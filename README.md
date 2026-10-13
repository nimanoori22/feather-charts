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

This API covers the currently rendered built-in series and the default `Time`
behavior. Custom-series rendering, arbitrary pane reordering,
touch/kinetic input and full TradingView API parity remain future work.
Options already represented by the engine do not imply every visual feature is
rendered. Read-only `model()` inspection remains available for advanced hosts.

Implementation contract and verification: [public chart API report](docs/porting/public-chart-api.md).

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
Conflation, last-price animation and hovered-series hit testing remain outside
this milestone. Iced gradients differ slightly from browser Canvas interpolation,
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
These levels are visual annotations; trading actions, alerts, dragging and hit
testing are outside this milestone. Source contract and results:
[price-line report](docs/porting/price-lines.md).
