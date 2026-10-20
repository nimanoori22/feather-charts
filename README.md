# Feather Charts

Interactive financial charts for native Rust applications, built with Iced 0.14.

Feather Charts is an in-progress Rust port of TradingView's Lightweight Charts™.
It brings candlesticks, lines, volume, synchronized panes and extensible drawing
to an embeddable Iced widget. The port follows the original library's behavior
while using Rust ownership, typed handles and a synchronous chart engine.

[Quick start](#quick-start) · [Examples](#examples) · [Usage guide](docs/usage.md) · [Project status](#project-status) · [Contributing](#contributing)

## Features

- **Six built-in series:** Line, Candlestick, Histogram, Area, Bar and Baseline,
  with multiple series per pane.
- **Synchronized panes:** shared horizontal navigation, independent price scales,
  draggable dividers, pane reordering and series transfers.
- **Interactive navigation:** pan, zoom, crosshairs, autoscale, logarithmic and
  percentage price scales, plus native touch navigation and kinetic scrolling.
- **Live data:** full replacement, latest-bar updates, appends and historical
  updates, with optional conflation for dense histories.
- **Annotations:** price lines, series markers, hover selection, text and image
  watermarks, last-price pulses and application-owned editable primitives.
- **Extensions:** typed custom series, strike-price charts, yield curves and
  application-defined horizontal domains.
- **Application integration:** owned event payloads, custom data metadata,
  coordinate conversions, crosshair synchronization and screenshot export.
- **Two rendering backends:** TinySkia for software rendering and WGPU for GPU
  rendering.

The host application supplies data and owns feeds, networking and image decoding.
See the [usage guide](docs/usage.md) for feature-specific APIs and constraints.

## Quick start

### Install from crates.io

The first crates.io release is being prepared. After publication, add it to an
Iced application with:

```sh
cargo add feather-charts@0.1.0
```

The Cargo package name is `feather-charts`; Rust imports use `feather_charts`.
See [renderer patches when embedding](#renderer-patches-when-embedding) before
using image watermarks. [crates.io](https://crates.io/crates/feather-charts) ·
[API documentation](https://docs.rs/feather-charts)

### Run the demos

Use a current Rust toolchain with Rust 2024 edition support, Cargo and Git.
The interactive examples need a desktop session and a working Iced renderer.

```sh
git clone https://github.com/nimanoori22/feather-charts.git
cd feather-charts
cargo run --example public_chart
```

This opens a candlestick chart with a synchronized volume pane and controls for
data updates, styles and price levels. The example uses generated data; no market
account or API key is needed.

Drag inside the plot to pan and use the wheel to zoom. Drag or scroll over a
price axis to scale it; double-click the axis to restore autoscale.

To select a renderer explicitly:

```sh
ICED_BACKEND=tiny-skia cargo run --example public_chart
ICED_BACKEND=wgpu cargo run --example public_chart
```

These environment-variable examples use POSIX shell syntax. Use `--release` for
performance-sensitive runs.

## Examples

Run examples from the repository root. Flags after `--` go to the example.

| Command | What to explore |
| --- | --- |
| `cargo run --example public_chart` | Candlesticks, volume and price lines |
| `cargo run --example public_chart -- --area` | Area fills and styles; also try `--line`, `--bar`, `--baseline` or `--histogram` |
| `cargo run --example public_chart -- --line --live-feed` | A synthetic feed with interactive navigation |
| `cargo run --example public_chart -- --primitives` | Select, move and resize an application drawing |
| `cargo run --example public_chart -- --series-markers` | Time-based annotations |
| `cargo run --example public_chart -- --line --conflation --live-feed` | Dense data with rendering aggregation |
| `cargo run --example custom_series` | A typed box-and-whisker renderer |
| `cargo run --example pane_management -- --live-feed` | Pane lifecycle, resizing and series transfers |
| `cargo run --example pane_management -- --screenshot-export` | Frozen captures, layer controls and previews |
| `cargo run --example options_chart` | A strike-price horizontal axis |
| `cargo run --example yield_curve` | Maturity-based yield curves |
| `cargo run --example custom_horizontal` | An application-defined session/phase domain |

The [usage guide](docs/usage.md) includes additional flags, controls and examples
for formatting, events, scale assignment, watermarks and numeric custom series.

## Embed a chart

Keep a `Chart` in your application's state and retain the `SeriesId` returned
when adding a series. For example, create a line with ordered timestamp data:

```rust
use feather_charts::{
    api::Chart,
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
    },
};

let mut chart = Chart::default();
let series = chart.add_series(
    SeriesOptionsMap::Line(Default::default()),
    0,
    PriceScalePosition::Right,
)?;

let rows = [(1_700_000_000., 100.), (1_700_000_060., 102.), (1_700_000_120., 101.)]
    .into_iter()
    .map(|(timestamp, value)| {
        BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time: Time::from(UtcTimestamp::new(timestamp)),
            value,
            color: None,
            custom_values: None,
        }))
    })
    .collect();

chart.set_data(series, rows)?;
chart.fit_content();
```

The snippet belongs in a function returning `Result`; the complete runnable
host is [examples/public_chart.rs](examples/public_chart.rs).

1. In `view`, return `chart.element().map(Message::Chart)`.
2. In `update`, pass each `ChartMessage` to `chart.update(message)` and process
   its returned events.
3. After a batch of direct data, options or navigation changes, call
   `chart.prepare(size, pixel_ratio, now)` and handle its result. Forward window
   scale-factor changes as shown in the example. The widget reports its bounds
   and requests animation redraws.

Preparation produces an immutable frame; drawing does not mutate the model.
The chart engine stays synchronous. Applications choose their own feed/runtime
integration. If you enable optional `EventInterests`, drain events after each
batch; the event queue is lossless and uncapped.

### Renderer patches when embedding

The root [Cargo.toml](Cargo.toml) patches two Iced renderer crates with the
included [vendor sources](vendor/README.md) to preserve image/vector drawing
order. Cargo applies dependency patches from the consuming workspace root.
When using Feather Charts as a dependency, clone this repository and carry those
patches into your root manifest with paths to the checkout's vendored crates.
The crates.io package uses the standard Iced renderer dependencies; the patched
renderer crates live in the Git repository. The relevant software-renderer
development-profile settings are also in this repository's manifest.

## Project status

The public API and examples are implemented across calendar, strike-price,
maturity and custom horizontal domains. Development and compatibility work are
ongoing; the Rust API is not a drop-in replacement for the JavaScript API.

The [parity completion report](docs/porting/public-parity-completion.md) records
the completed audit scope and validation evidence. Its
[scorecard](docs/porting/public-parity-score.md) distinguishes API coverage from
behavioral parity; neither is a claim that the entire upstream library is ported.
The [runtime primary-axis follow-up](docs/porting/runtime-primary-axis.md)
supersedes the completion report's construction-only axis limitation.

Current limits include:

- Display-P3 output is unimplemented; rendering/export uses sRGB.
- Locale support covers a defined set of Gregorian/numbering profiles, with
  documented fallbacks rather than full browser `Intl` equivalence.
- Historical updates are rejected while the effective conflation factor is
  greater than one.
- A recorded timing exception and dedicated series drawing-order mouse
  acceptance remain open. Physical touchpad, touchscreen and high-DPI acceptance
  also remain open; synthetic input and DPR captures are separate evidence.

See [live-update acceptance](docs/porting/live-update-interaction-acceptance.md)
and the [bounded timing study](docs/porting/histogram-timing-load.md) for the
performance evidence and its limits.

## Development

| Path | Responsibility |
| --- | --- |
| `src/api/` | Public chart owner, widget, events and extension APIs |
| `src/model/` | Data, scales, panes and chart state |
| `src/views/` and `src/renderers/` | Prepared views and drawing |
| `src/ui/` | Frame composition, layout and input handling |
| `src/helpers/` and `src/formatters/` | Shared utilities and label formatting |
| `examples/` | Runnable hosts, feature demos and profiling tools |
| `tests/` | Behavioral and integration tests |
| `scripts/` | Source comparisons, capture checks and profiling runners |
| `docs/porting/` | Source contracts, design decisions and validation reports |
| `tasks/` | Plans and acceptance checklists |

Common checks:

```sh
cargo fmt -- --check
cargo check --all-targets
cargo test --all-targets
cargo test --doc
cargo clippy --all-targets -- -D warnings
```

Source-comparison and native-capture suites have additional requirements; follow
the relevant [porting reports](docs/porting/) for their commands and prerequisites.
Some profiling tools are Linux/Wayland-specific. Recorded native acceptance is
not a Windows or macOS validation claim.

## Contributing

Read [AGENTS.md](AGENTS.md) for the porting workflow and repository relationships.
Use Lightweight Charts as the behavioral reference, investigate its real
consumers, and check the existing Rust implementation before designing a port.
Preserve observable behavior through idiomatic Rust rather than translating
TypeScript line by line. Keep rendering and engine hot paths synchronous.

Include behavioral tests and relevant validation with changes. Keep feature
contracts and acceptance limits in their linked reports so the overview stays
useful to new readers.

## Acknowledgments and source notices

- [TradingView Lightweight Charts](https://github.com/tradingview/lightweight-charts)
  is the original library and primary behavioral reference.
- [Iced](https://github.com/iced-rs/iced) provides the native UI and rendering
  foundation.
- [Flowsurface](https://github.com/flowsurface-rs/flowsurface) provides reference
  examples for Rust/Iced application patterns.

Feather Charts is licensed under [Apache-2.0](LICENSE). See [NOTICE](NOTICE)
for upstream attribution.

Preserved upstream [LICENSE](docs/porting/attribution-source/LICENSE) and
[NOTICE](docs/porting/attribution-source/NOTICE) files accompany the attribution
assets. Iced vendor sources retain their own licenses.
