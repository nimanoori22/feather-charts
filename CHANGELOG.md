# Changelog

## feather-charts 0.2.0 — 2026-10-31

### Added

- Opt-in continuous linear numeric axes with proportional point spacing,
  numeric ticks, cursor-anchored zoom and numeric histogram widths.
- Optional top-left crosshair readout for built-in series, including O/H/L/C
  for candlestick/bar charts and latest-value fallback.
- A Cargo workspace containing the new `feather-chart-file` 0.1.0 headless codec
  and `feather-charts-app` 0.1.0 native viewer (`feather-charts` executable).
- Version-1 `.fchart` fixtures, Rust/Python producers, profiler CSV export,
  optional Linux desktop/MIME assets and an AI-agent charting guide/prompt.

### Compatibility and migration from 0.1

- Update the library dependency to `feather-charts = "0.2"`.
- Low-level `HistogramRendererData` struct literals need `numeric_width: None`
  to retain index-based rendering. Numeric chart preparation supplies its width.
- Exhaustive `HitGeometry` matches must handle the new `Columns` variant.
- Existing horizontal domains retain their default logical-index spacing.
  Use `OptionsChart::new_numeric(precision)` for continuous numeric distances.
- `Chart::element()` retains its plain presentation. Choose
  `element_with_readout()` to enable the optional host-widget readout.
- The library package's placeholder automatic binary is disabled; the installable
  viewer package owns the `feather-charts` executable name.

The `.fchart` wire version remains 1. It supports line/histogram documents;
OHLC documents, dates, categorical axes and multiple panes are outside version 1.
Numeric inertia, custom-series readout formatting and separate per-pane readouts
remain unavailable in the supplied convenience widget.

## feather-charts 0.1.0

Initial published Rust/Iced chart library.
