# Price lines and last-value labels

## Source and consumers

Investigated source Series lastValueData/createPriceLine/removePriceLine,
SeriesPriceAxisView, SeriesPriceLinePaneView, CustomPriceLine and both custom views,
PriceLineOptions/defaults, PriceLine API, PriceAxisView, PanePriceAxisView,
HorizontalLineRenderer, PriceAxisViewRenderer and the axis widget/alignment helper.
The source series/colorer connects fulfilled close values with row color overrides.
API consumers mutate options synchronously; series own custom lines; pane consumers
render automatic/custom lines and titles; axis consumers measure all back labels,
resolve overlap, then paint the crosshair separately. Source autoscale considers
series data/baseline/primitives, not custom price lines.

Required: global-last versus visible-right nearest-left selection, first visible
base, scale-specific formatting/projection, source color precedence, independent
visibility, titles/display modes, left/right/overlay placement, source overlap
ordering and edge correction, invalidation and series-removal cleanup. Empty or
hidden data clears prepared output. Horizontal lines retain their actual projected
coordinate even when labels move. No annotation hit testing or animation is added.

## Design, alternatives and ownership

Series-owned Vec<(PriceLineId, PriceLineOptions)> is chosen over a Chart-owned map:
it preserves creation order and automatic cleanup while avoiding duplicate state.
ChartModel supplies synchronous access/mutation and invalidation; Chart forwards it.
IDs are process-global/non-recycled; options are cloned for public inspection so
no RefCell guard escapes or blocks subsequent mutation. JS wrapper identity,
manual destroy, inheritance, callbacks and mutable renderer references are replaced
by owned records and immutable frame snapshots. Existing model handles do not point
back to Chart, so no new Rc cycle or destruction ordering requirement arises.

Application -> Chart -> model -> series -> custom line records
Preparation -> owned axis labels/line commands -> Canvas borrows immutably

Compare immutable snapshot label collections with boxed mutable PriceAxisView
objects. Choose collections: existing frame settlement already measures owned
snapshots, rendering is synchronous and there is no need for JS lazy shared views.
Actual coordinate, render coordinate and pane-title text are distinct fields.
Axis/pane commands reuse existing text measurement, glyph cache and paint clipping.

Public API: add_price_line(series, PriceLineOptions) -> Result<PriceLineId,
ChartError>; replace_price_line_options(id, options) -> Result<(), ChartError>;
price_line_options(id) -> Result<PriceLineOptions, ChartError>; remove_price_line(id)
-> Result<(), ChartError>. PriceLineOptions::new(price) supplies source defaults.
Full replacement matches existing options APIs. Invalid price/colors preserve state;
removed/foreign IDs return typed errors. Dropping/removing series drops its lines;
clearing its data retains records but suppresses output until values return.

Consumer mapping: Chart API owns valid mutation, model assembles settled decorations,
axis preparation measures/aligns/draws labels, plot preparation draws price lines
once per pane then pane labels, and crosshair remains the final top layer. Labels
contribute to shared axis widths, pane titles do not. Custom lines never enlarge
source autoscale ranges. Price-axis hidden state suppresses axis output but preserves
pane-title placement as source permits.

## Implementation and validation

Implemented for the currently rendered line, candlestick and histogram series.
No dependency was added. Full options replacement validates finite bounded price
and CSS colors before changing records; typed line width/style prevent invalid values.
The model's existing effects invalidate frame preparation synchronously.

### Files changed

- Model: `src/model/price_line.rs`, `series.rs`, `price_scale.rs`, `mod.rs`,
  `axis_snapshots.rs`, `chart_model.rs`, `chart_model/price_lines.rs`.
- Public interface: `src/api/chart.rs`, `src/api/mod.rs`.
- Rendering: `src/renderers/horizontal_line_renderer.rs`, `price_axis_labels.rs`,
  `price_axis_renderer.rs`, `price_axis_renderer/labels.rs`, `mod.rs`.
- Frame integration: `src/ui/line_chart.rs`, `chart_frame.rs`, `multi_pane_frame.rs`.
- Behavioral tests: `tests/chart_api.rs`,
  `src/model/chart_model/tests/price_axis.rs`,
  `src/model/chart_model/tests/complete_display.rs`,
  `src/renderers/price_axis_renderer/tests.rs`.
- Examples/fixtures: `examples/public_chart.rs`, `price_lines_reference.rs`,
  `price_axis_reference.rs`, `interaction_reference.rs`.
- Source oracle: `scripts/check_price_lines_reference.cjs`.
- Documentation: `README.md`, this report, `tasks/price-lines-plan.md`,
  `tasks/price-lines-todo.md`.

The existing tick tests retain exact ordinary-tick/grid assertions and explicitly
account for default-enabled additional labels. New public tests cover CRUD,
foreign/stale handles, non-destructive invalid updates, data clear, series removal,
visible/global differences, no autoscale expansion, independent visibility/color
fallback, live updates, pane isolation, clustered/disabled alignment and repeated
preparation. Scale/display-mode tests cover normal/log/percentage/indexed output
and row versus explicit price-line color precedence.

### Validation

- `cargo fmt --check`, `git diff --check`: passed.
- `cargo test --all-targets`: 406 passed, including 329 library, 13 public API and
  64 native demo interaction tests.
- `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`,
  `cargo build --examples`: passed.
- All ten `scripts/check_*_reference.cjs` comparisons passed: 19,515 cases.
  The new oracle contributes 282 cases by executing actual upstream TypeScript
  `Series.lastValueData`, horizontal-line drawing, axis/pane label geometry and
  axis overlap correction. It includes sparse fulfilled/whitespace rows, ties,
  short axes, offscreen edges, left/right labels, widths/styles and ratios
  1/1.25/1.5/2. Geometry uses fixed text metrics; it does not claim browser font parity.
- Mutation check: removing line-color validation makes the public invalid-options
  test fail. Restoring validation passes; no mutation is retained.
- `public_chart --smoke`: passed append, move, cluster, visibility, remove/recreate.
- TinySkia and WGPU captures cover default, cluster, hidden, live, left-axis/long
  title and empty-data frames at ratios 1/1.25/1.5/2, with repeatability and full
  background-coverage assertions. Artifacts are local under
  `/tmp/feather-price-lines-public-{tiny,wgpu}-final`; 24 captures per backend.
- Bounded native `public_chart` startup ran for 10 seconds with no errors and was
  stopped by the timeout (exit 124); this is startup evidence, not hardware acceptance.

### Performance

Equivalent existing navigation profiling fixtures used the same debug profile,
data, 120 updates per mode, and baseline `1fc36a9`. Baseline and current runs were
sequential with no concurrent cargo build. Their internal fixture tests run
concurrently, so these are local diagnostic measurements, not a precise benchmark.

| Workload/stage (ms) | Before median / p95 | After median / p95 |
| --- | --- | --- |
| Continuous line engine/measurement | 19.614 / 37.204 | 20.393 / 41.567 |
| Continuous line glyph preparation | 0.511 / 0.571 | 0.538 / 0.594 |
| Alternating line engine/measurement | 24.337 / 35.703 | 22.747 / 44.034 |
| Alternating line glyph preparation | 0.556 / 0.580 | 0.581 / 0.611 |
| Continuous price/volume complete update | 11.443 / 28.551 | 12.240 / 29.445 |
| Alternating price/volume complete update | 35.267 / 57.730 | 36.813 / 70.773 |

An initial implementation remeasured the full axis while converting pane titles
to glyphs, adding roughly 8 ms. The final implementation prepares title commands
from the existing settled measurement and converts only those commands to glyphs.
Existing glyph cache/retained-frame paths remain in use. Additional labels introduce
modest work; the measurements do not establish a smoothness guarantee.

## Remaining dependencies and acceptance

Fresh physical mouse/touchpad navigation and crowded-label acceptance remain
unchecked for this milestone, despite earlier milestones' physical checks.
Run `cargo run --example public_chart`, cluster levels, scroll into history, pan
both axes, append data, move/remove/recreate levels and toggle the last label.
Verify labels stay readable and price lines retain their true coordinates.

Other series families, last-price animation, draggable annotations/hit testing,
trading actions and alerts remain separate work. This milestone covers source
algorithms for the supported synchronous model and owned frames; it does not
claim full TradingView API or browser font parity. Source graph findings were
best-effort and relevant stale/missing call sites were checked directly.
