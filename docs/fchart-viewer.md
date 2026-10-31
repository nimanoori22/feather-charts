# Feather Charts documents and native viewer

To have an AI agent create and open charts for you, start with the
[AI agent charting guide and reusable prompt](ai-agent-charting.md).

Install the released viewer: `cargo install feather-charts-app --version 0.1.0 --locked`.
Build from this checkout: `cargo build -p feather-charts-app --release`.
Run: `target/release/feather-charts results.fchart`, or run without a path for the
welcome screen. `--help` prints usage. A loaded document fills the window with the
chart and its compact in-chart value readout: no permanent toolbar, file path,
legend panel, title, status row or axis-label widgets surround it.
Ctrl+O reveals a temporary path field, Enter opens it, and Escape dismisses it.
Ctrl+R reloads, Ctrl+0 fits, and OS file drop opens a replacement. Errors appear
in a dismissible overlay and preserve the previous chart. The newest request wins.
File reads/JSON parsing run on Iced's native executor, with no new Tokio dependency.
Construction stays synchronous in the chart engine.

Drag to pan, wheel to zoom about the pointer, and Fit to restore the data extent.
Resize preserves proportional numeric geometry. Numeric ticks need not coincide
with samples; the crosshair axis label shows the continuous x coordinate while
sample selection retains original data. Columns use explicit numeric widths.
Series names and authored numeric precision appear in the in-chart readout. It
shows the selected original sample, latest values when the crosshair is absent,
and a dash when a series has no sample at the selected key. Document title,
axis labels/units and metadata remain in the file; they do not add viewer panels.
Empty data is displayed explicitly. The application title is Feather
Charts; the launcher/executable is `feather-charts`; the internal Cargo package is
`feather-charts-app`. The library remains package `feather-charts`.

## Write from any language

See [v1 contract](fchart-format.md). No GUI is needed to produce documents:

```sh
python examples/fchart/write_benchmark.py /tmp/python-results.fchart
cargo run -p feather-chart-file --example write_benchmark -- /tmp/rust-results.fchart
cargo run -p feather-chart-file --example write_benchmark -- --check /tmp/python-results.fchart
```

These examples label synthetic data explicitly and refuse to overwrite files.
Rust benchmarks can depend only on `feather-chart-file`, construct a
`ChartDocument`, and call `write_document` after the timed section. The crate's
read/write functions validate; directly deriving/using Serde is not sufficient.

Convert retained profiler output after collection:

```sh
python scripts/export_profile_fchart.py target/profile/frames.csv /tmp/profile.fchart
# Optional: --columns prepare_ns work_cpu_ns --include-warmups
```

The converter excludes `warmup=true` rows by default, retains original frame keys,
separates phases and original column names, converts ns to ms, embeds adjacent
`metadata.txt` run context and preserves raw CSV/metadata unchanged. Work/present
call timings are CPU/work measurements, not GPU execution or presentation latency.
The profiler measurement paths are unchanged. Use fewer columns if phase/column
combinations exceed the 32-series budget. Python uses only the standard library.

See [Linux installation](../packaging/linux/README.md), [numeric API/design](fchart-numeric-axis.md)
and [acceptance record](fchart-acceptance.md). v1 supports one pane, one y-unit,
line and histogram series, numeric linear axes, and bounded embedded JSON only.
No editing, live reload watcher, external data, dates, log x, additional series,
Windows/macOS installers or package publication are included.

## Reusable chart readout

Library hosts can use `chart.element_with_readout()` instead of `chart.element()`
to show a passive overlay in the plot's top-left corner. `crosshair_readout()`
returns the same formatted lines for a custom host layout. Candlestick/bar series
show O/H/L/C with their series formatter; line, area, baseline and histogram
series show their value. Hidden series are omitted. The widget reads current
canonical rows without subscriptions or extra state, and does not intercept pan,
zoom or crosshair input. Existing built-in demos now enable it, including
`cargo run --example candlestick_chart` for an OHLC demonstration.

The `.fchart` v1 format still supports line/histogram only; the reusable library
readout supports existing OHLC chart types without extending that file format.
The readout is a host widget overlay, like the upstream legend examples;
chart-only screenshot exports contain engine geometry, excluding host widgets.
