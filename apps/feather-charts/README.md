# Feather Charts viewer

Native, read-only viewer for self-contained `.fchart` numeric chart documents.
Cargo package: `feather-charts-app`. Installed executable: `feather-charts`.

```sh
cargo install feather-charts-app --version 0.1.0 --locked
feather-charts results.fchart
```

The chart fills the window with an in-chart crosshair value readout. Drag to pan,
wheel to zoom, Ctrl+0 to Fit, Ctrl+R to Reload, or Ctrl+O to reveal the temporary
path field. Enter opens a document; Escape dismisses the opener/error overlay.
OS file drop is supported. Invalid replacements preserve the last valid chart.
Run without a path for the opener, or use `--help` for CLI usage.

Version 1 supports line/histogram series, one pane, one shared y-unit and
proportional linear numeric axes. Data is embedded; documents contain no scripts,
external references or renderer plugins. Produce documents using the headless
`feather-chart-file` crate or any language's JSON writer.

A graphical desktop session, system fonts and Iced's platform libraries are
required. Linux desktop/MIME assets are optional and are available in the
[repository](https://github.com/nimanoori22/feather-charts/tree/v0.2.0/packaging/linux).
The viewer never installs associations automatically. Windows/macOS installers
are not provided.

See the [agent workflow and reusable prompt](https://github.com/nimanoori22/feather-charts/blob/v0.2.0/docs/ai-agent-charting.md).
Licensed under Apache-2.0.
