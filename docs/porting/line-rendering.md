# Line rendering

## Source contract

Inspected complete `model/series/{line-pane-view,line-pane-view-base,series-pane-view-base}.ts`
and `renderers/{line-renderer,line-renderer-base,walk-line,draw-series-point-markers}.ts`.
Graph tracing connects the line-series definition to its pane view, and the shared
walker to line/area renderers. Markers use the same visible range and point colors.
`model/time-data.ts` had changed index metadata; its current source was read directly.

| Behavior | Source | Test obligation |
|---|---|---|
| One available neighbor on each visible edge, but no extension for a wholly disjoint range | `time-data.ts:visibleTimedValues` | edge and disjoint ranges |
| Value rows connect across whitespace; time-axis spacing remains | `line-pane-view-base.ts:_fillRawPoints` | sparse row coordinates |
| First value is the first row at/after the strict range's left edge | `series-pane-view-base.ts:_makeValidImpl`, Series firstValue | percentage/indexed coordinates |
| Row color overrides series color | `series-bar-colorer.ts:Line` | override/fallback |
| Simple/curved segment uses its starting color; change occurs at its endpoint | `walk-line.ts:walkLine` | styled path boundaries |
| Stepped horizontal leg uses old color, vertical leg uses new color | `walk-line.ts:WithSteps` | leg boundary and dash phase |
| A single selected item spans half a bar on either side | `walk-line.ts` | isolated segment |
| Cubic controls use adjacent full-data neighbors, tension six, clamped endpoints | `walk-line.ts:getControlPoints` | interior/boundary controls |
| Dash phase tracks bitmap distance; curves use mean of chord/control-polygon lengths | `walk-line.ts` | fractional phase continuity |
| Butt caps, round joins, vertically scaled width/dashes | `line-renderer-base.ts` | command and adapter attributes |
| Markers draw backwards; x/radius include half-pixel correction based on horizontal ratio | `draw-series-point-markers.ts` | marker order/snapping |
| Marker radius uses explicit nonzero radius or width/2+2; markers can draw without a line | `line-pane-view.ts` | defaults/visibility |
| Hidden/empty/zero-size/missing-first-value output is empty | pane view bases | stale snapshot cleanup |
| Conflation chooses farther high/low extreme, with low on ties | `line-pane-view-base.ts` | explicitly unsupported here |

## Design and ownership

Selected backend-neutral logical-coordinate preparation and bitmap-coordinate
commands, followed by an Iced adapter. Direct Iced path preparation was viable
but would couple model tests to the GUI. Existing line enums, timed-range selection,
PixelRatio, and grid commands are reused. No inheritance/callback identity APIs,
NaN initialization, model back-references, or shared mutable renderer data are needed.

The application owns DataLayer, ChartModel, the animation controller, line view,
and render snapshot. The view borrows model inputs only during refresh. Canvas
reads an owned snapshot; update messages carry actual bounds and frame time back
to the application. There are no new Rc edges, cycles, clocks in the model, or
async engine paths. Refresh clears old data before fallible preparation.

Implemented API: `ChartModel::prepare_line(id) -> Result<Option<LineRendererData>,
LinePreparationError>`, `LinePaneView::refresh(model,id)`,
`LineRendererData::draw_commands(pixel_ratio)`, and UI-owned frame preparation.
The registry stays private. Full point coordinates remain available for cubic
neighbors, while only the extended visible range produces draw commands.

Model coordinates are logical f64. Commands apply horizontal/vertical pixel ratios
once, including stroke width, dash lengths, and marker correction. The adapter
divides bitmap coordinates back to Iced logical units; the runtime performs device
scaling. Grid commands retain their existing bitmap contract. Clipping is a real
Canvas frame clip, not curve endpoint truncation.
The Iced adapter requires a uniform device ratio (as reported by its window);
backend-neutral commands independently support horizontal and vertical ratios.
Same-color marker runs are filled together, retaining the source's alpha-overlap
semantics rather than repeatedly blending individual circles.

Iced 0.14's LineDash offset is usize, and WGPU interprets it as a pattern index
while tiny-skia treats it as distance. The adapter therefore splits dashed paths
using fractional distance itself. Solid cubics stay cubic; dashed cubics are
adaptively flattened with a small bitmap tolerance before splitting.

## Scope

Rebuild each relevant light/full frame, with no geometry cache. Conflation is not
ported: enabled conflation returns a typed error rather than silently rendering
unconflated data. Hit testing, crosshair, price lines, last-price animation, full
axes, gestures, and other series remain deferred. The demo is plot-only with grid.

## Model/UI handoff

`ChartModel::replace_series_options` replaces built-in options, refreshes price
formatting sources, and requests recalculation/full redraw. Scale attachment stays
where registration placed it; moving sources is not part of this options operation.
`Pane::price_scale_for_source` is crate-private; registry handles remain private.

`ui::line_chart::prepare_frame` validates layout/series selection, applies actual
plot dimensions, processes the animation controller, refreshes the line view,
and snapshots grid/line commands. It returns the retained invalidation mask for
optional layout replay; newly generated requests are not drained. The demo needs
only one layout pass. Its Canvas update publishes actual bounds and scheduled
frame messages; draw reads the snapshot only. Zero dimensions clear geometry.
Color/layout errors clear demo output without starting a redraw-message loop.

The adjacent Iced checkout is newer than the released APIs resolved by Cargo.
Those released source files were inspected directly after compilation exposed the
differences: startup scale factor uses a window Task, and animation redraw timing
uses Canvas Action::request_redraw_at separately from message publication. These
UI runtime operations do not introduce asynchronous chart-engine work.

## Files changed

Created:

- `src/model/series/line_pane_view.rs`
- `src/renderers/line_renderer.rs`
- `src/renderers/walk_line.rs`
- `src/ui/mod.rs`
- `src/ui/dashed_path.rs`
- `src/ui/line_chart.rs`
- `examples/line_chart.rs`
- `examples/line_reference.rs`
- `scripts/check_line_reference.cjs`
- `docs/porting/line-rendering.md`

Modified:

- `src/lib.rs`
- `src/model/chart_model.rs`
- `src/model/chart_model/tests.rs`
- `src/model/pane.rs`
- `src/model/series.rs`
- `src/renderers/mod.rs`

No Cargo dependencies or reference repositories were modified.

## Tests and validation

Eighteen new library tests cover styled paths, source curve controls/distance,
fractional dash splitting, marker order/defaults/fill runs, pixel conversion,
selected price-scale coordinates (normal, percentage, indexed, logarithmic),
whitespace, visible neighbors, stale-output clearing, resize/history/append,
configuration rejection, layout validation, removal, and animation/layout replay.
The example adds a connected headless smoke test.

`cargo test --all-targets` passes all 216 library tests and the demo smoke test.
`cargo check --all-targets` and strict Clippy for library/examples pass. All-targets
Clippy reports only the five existing test-code warnings documented in the
chart-model report. Formatting and whitespace checks are clean.

`cargo run --example line_chart -- --smoke` passes. The desktop demo was launched
and its initial fitted colored line/grid visually inspected. Resizing, appends,
removal/reload, styles, and cancellation are also exercised headlessly. A compositor
API mismatch prevented the attempted automated desktop resize; an interactive
resize check and exhaustive backend pixel-golden testing remain manual checks.

The read-only reference oracle executes the original TypeScript walker and marker
implementation, not a rewritten JavaScript substitute. All 180 combinations of
line type, dash style, device ratio, and selected item range match Rust commands.
An optional browser comparison checks simple/stepped/curved dashed paths and
markers against the original implementation. Dashed curves in the Iced adapter
are an adaptive polyline approximation; cross-backend antialiasing is not promised
to be pixel-identical to Canvas2D.

## Running

```sh
cargo run --example line_chart
cargo run --example line_chart -- --smoke
cargo build --example line_reference
node scripts/check_line_reference.cjs
node scripts/check_line_reference.cjs --visual
```

The oracle needs the sibling Lightweight Charts checkout and its installed
TypeScript package; visual mode also uses its Puppeteer package and local Chromium.
It does not require network access or modify that checkout. Screenshots are placed
in `/tmp`, not committed. Standard Cargo tests do not require Node or a desktop.

## Next dependencies

The runnable load/fit/draw/resize/append/remove slice is implemented. Full axes,
chart gestures/crosshair, other series, conflation, custom color-parser integration,
and measured caching are follow-on work rather than placeholders in this slice.
