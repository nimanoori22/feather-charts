# Synchronized pane rendering

## Source investigation and contract

Read ChartWidget sizing/synchronization consumers, complete PaneSeparator,
CrosshairPaneView and CrosshairRenderer, Crosshair pane/axis view consumers,
PaneWidget pointer/scroll handlers, CrosshairMarksPaneView, VolumeFormatter and
existing Rust pane/removal/axis code.
Graph traces connect chart synchronization -> sizing/separator construction,
and Crosshair -> per-pane view. Coverage checks reported no recorded gaps;
this remains best-effort and direct reads establish the behavior used here.
Local Iced Canvas Program exposes captured actions and read-only drawing;
Flowsurface dashboard uses explicit resize messages/state updates. Its PaneGrid
is for independent dashboard panels; the chart instead needs globally aligned
axes/time coordinates, so we keep the existing single Canvas approach.

Required behavior: shared maximum left/right widths, one bottom time axis,
weighted pane heights, source round-to-device-pixel for intermediate panes and
ceil remainder for the last, minimum 2px layout panes and 1px separators. Divider
handles extend 4px on either side; resizing preserves adjacent total stretch and
clamps both panes to 30px, refusing pairs already too small. Window resizing
retains stretch proportions. Vertical crosshair appears in every pane; horizontal
line/price labels belong to the origin pane; the time label is chart-wide. Line
crosshair markers follow the applied time in every pane, including inactive panes.
Series removal may delete an empty pane and shift later indexes; callers rebuild
pane selections from model membership rather than retaining stable-ID assumptions.

The existing Rust navigation contract (wheel coalescing, 5px drag threshold,
focus/layout cancellation) stays intact. This milestone shares horizontal time
navigation; source vertical price dragging, touch and kinetic scroll are separate
unported capabilities, not silently introduced here.

## Alternatives and design

1. Extend ChartFrameOwner into a monolithic single/multi-pane assembler.
2. Add MultiPaneChartFrameOwner coordinating pane preparation through the existing
   borrowed dispatch and owned chart/plot snapshots, retaining one global cycle.

Choose 2 to preserve tested single-pane callers and performance. It is a chart
coordinator, not independent competing pane frame owners. It owns allocation and
retained-mask state; no pane preparer drains invalidation or advances animation.
The limited duplication of settlement orchestration avoids destabilizing the
existing path; geometry, axes, measurement, commands and drawing stay shared.

Public types: `SeriesPaneView` (owned Line/Candlesticks/Histogram variants),
`MultiPaneChartFrameInput` (one optional SeriesId per model pane, bounds/ratio/
background), `MultiPaneChartFrameSnapshot`, `IcedMultiPaneChartFrame`, and
`MultiPaneChartFrameOwner::prepare`. `allocate_multi_pane_layout` takes positive
stretch factors and returns shared chart regions plus per-pane rectangles.
The known one-selected-series-per-pane limitation matches the current single
frame API; generalized multi-series composition awaits the public widget facade.

```text
Application -> strong owned model/controller/coordinator/Vec<SeriesPaneView>
Coordinator borrows model + views -> owned vector of pane frame snapshots
Canvas borrows immutable snapshots -> typed pane/divider actions
Application updates model synchronously -> prepares next chart frame
```

No back-references, Rc cycles, async machinery or manual destruction. Views and
snapshots drop normally. Input owns an active plot gesture or divider gesture,
never both; pane membership/layout changes cancel obsolete gestures. Preparation
validates pane count/selections before changing dimensions or consuming commands.

## Verification and remaining dependencies

Run `cargo run --example price_volume_chart`. Wheel/drag either plot to navigate;
drag the divider to resize panes. Controls exercise append, same-time replacement,
older/latest navigation, prepend, clear/reload, style changes and series removal.
Missing volume is whitespace; candle and volume changes are applied synchronously
before publishing the next frame. Source-compatible volume formatting preserves
integer trailing zeroes (`40K`, `0`), correcting a pre-existing formatter bug.

Behavioral tests cover aligned time coordinates and independent scales, global
axis settlement, bounded failure cleanup (including a later pane failing),
immutable retained snapshots, one animation sample across multiple settlement
passes, pane removal, cross-pane line markers and origin-pane price labels.
Synthetic Iced events cover one navigation per event, drag ownership across pane
boundaries, stationary-pointer suppression, focus/layout/membership cancellation,
divider priority, minimum heights and disabled resizing.

### Validation

- `cargo fmt --check`, `cargo test --all-targets --quiet`,
  `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings` and
  `cargo build --examples` pass. Tests: 318 library tests and 9 shared demo tests
  in each of four example targets (354 executions).
- Nine actual-source comparison scripts pass: 9,900 histogram, 288 pane layout,
  112 pane crosshair/divider/volume formatter, 7,776 candle, 131 interaction,
  180 line, 128 price axis, 192 time axis, 192 chart layout and 64 corner cases:
  **18,963 comparisons**. Run the corresponding `scripts/check_*_reference.cjs`
  files with Node; each invokes its Rust reference example and the local TS source.
- Histogram and price/volume `--smoke` runs pass. Each demo produced 28 TinySkia
  and 28 WGPU captures at ratios 1, 1.25, 1.5 and 2: **112 captures** with finite
  geometry, clipping/seam and repeatability assertions. Output is in
  `/tmp/feather-{histogram,price-volume}-final-{tiny,wgpu}`. Inspected the final
  1.25x GPU images, including correctly formatted volume labels and aligned
  candle/volume crosshair. The native price/volume window opened without reported
  errors and was stopped intentionally after the startup check.

### Controlled performance

Debug build, 120 points, 917 × 870.8 logical bounds, 20 warm-up iterations then
100 measured samples, serial execution. Baseline is commit `4602736` (histogram
integration before multi-pane work). These are CPU preparation measurements,
not physical input latency or GPU presentation timings.

| Alternating zoom | Before median / p95 | After median / p95 |
| --- | --- | --- |
| Line engine | 8.905 / 9.214 ms | 9.010 / 9.225 ms |
| Candle engine | 7.966 / 8.471 ms | 8.317 / 8.604 ms |
| Complete price/volume model + frame + glyph preparation | — | 12.306 / 12.986 ms |

Continuous zoom line engine: 8.036 → 8.096 ms median; candle engine:
7.037 → 7.180 ms. Complete price/volume continuous zoom: 11.817 ms median,
13.835 ms p95. Existing single-pane timing remained close to baseline. These
measurements preceded the final small pane-membership gesture guard.

### Files changed

- Engine/crosshair: `src/model/chart_model.rs`,
  `src/ui/line_chart.rs`; tests in `src/model/chart_model/tests/multi_pane.rs`,
  `histogram.rs`, `chart_layout.rs` and their registration in `tests.rs`.
- Layout/coordinator/input: `src/ui/multi_pane_layout.rs`,
  `src/ui/multi_pane_frame.rs`, `src/ui/chart_input.rs`, `src/ui/mod.rs`.
- Formatting: `src/formatters/volume_formatter.rs`.
- Demo: `examples/price_volume_chart.rs`, `examples/support/chart_demo.rs`,
  `examples/support/chart_demo/{series,capture,tests}.rs`.
- Source fixtures: `examples/multi_pane_{layout,interaction}_reference.rs`,
  `scripts/check_multi_pane_{layout,interaction}_reference.cjs`.
- Reports/checklists: this report, `histogram-rendering.md`,
  `tasks/histogram-volume-{plan,todo}.md`.

A separate small validation commit fixed five existing strict Clippy warnings
without suppressions in `src/formatters/format_date.rs`, `src/model/get_series_plot_row_creator.rs`,
`src/model/horz_scale_behavior_time/horz_scale_behavior_time.rs` and
`src/model/price_scale.rs`.

### Remaining dependencies and unchecked acceptance

Physical mouse/touchpad zoom, scrolling, divider dragging and window resizing
remain unchecked on real devices; native startup and synthetic events do not
establish this acceptance. Public chart/widget facade, arbitrary multi-series
composition/reordering, source touch/kinetic gestures and vertical price dragging
remain subsequent work. Histogram conflation is explicitly unsupported.
