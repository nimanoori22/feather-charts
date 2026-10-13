# Area rendering

## Source contract and consumers

Investigated `model/series/area-pane-view.ts`, `area-series.ts`, line/pane view
bases, `renderers/area-renderer-base.ts`, `area-renderer.ts`, `walk-line.ts`,
`gradient-style-cache.ts` and `model/series-bar-colorer.ts`. Area definition creates
the view; view composes fill before outline/markers. Model and axis consumers use
the close/value with row line-color overrides. Fill top/bottom overrides are
independent of outline overrides. Normal filled paths close to pane bottom,
inverted paths to top. Relative gradient begins at the minimum visible y,
including source extended-range neighbors. Whitespace has no fulfilled row and
uses the existing source range semantics. Conflation remains explicitly unsupported
as in the existing line renderer; animation and hovered-series hit testing are out
of this milestone's scope.

Style boundaries follow walkLine: simple/curved paths finish at the new endpoint;
stepped paths finish the old horizontal leg then start the new vertical leg.
A single point spans half a bar on either side. Fill and stroke use independent
styles; curve controls retain neighboring data. Empty/hidden views clear output.

## Design and ownership

Compare owned backend-neutral fill/path/gradient commands with prebuilt Iced
geometry. Choose commands: fits existing snapshots, supports differential testing
and keeps model paths synchronous. Compare a shared segment walker with a duplicate
fill traversal; choose a narrow walker accepting style equality, returning the
source style index plus path. Existing solid-line API wraps it unchanged.

`AreaRendererData` owns projected items, line options, fill styles and viewport
parameters. `AreaPaneView` owns an optional prepared record, clears before refresh,
and borrows ChartModel only during preparation. Series/data/model ownership remains
unchanged; commands are owned snapshots and Canvas borrows them immutably. No
new Rc relationships, owner captures, manual destruction or thread-safe state.

Proposed internal API: `AreaPaneView::refresh(&mut self, &mut ChartModel, SeriesId)`;
`ChartModel::prepare_area(id) -> Result<Option<AreaRendererData>, ...>`;
`AreaRendererData::draw_commands(PixelRatio) -> ...`. Public API reuses existing
`SeriesOptionsMap::Area`, AreaData items and Chart lifecycle/options methods.
Model consumers project fulfilled values; API validates before mutation; frame
consumers convert bitmap commands once and retain independent per-series layers.

Iced's exact local `graphics/src/gradient.rs` and both geometry backends support
gradient fills/strokes. Its add_stop helper overwrites equal stops; direct ordered
stop arrays are required later for Baseline's discontinuity. Area needs two stops.
JS inheritance, lazy mutable views and CanvasGradient identity caches are replaced
with owned data and value-based style equality. Rendering remains immutable.

## Implementation and validation

Delivered public Area creation, data/options lifecycle, fills/outlines, markers,
price decorations and composed panes. Invalid visual options and row colors reject
before mutation; crosshair/magnet/event paths retain existing synchronization.
`public_chart --area` demonstrates live updates, levels and changing styles.

Source oracle: 288 Area cases and all 180 existing line cases pass. Three public
Area tests cover clear/removal, independent fill/outline visibility, rejected
options/data, row-color label precedence, scale modes and crosshair markers.
330 library tests pass. TinySkia and WGPU each passed 48 captures at ratios
1/1.25/1.5/2, including inverted/relative/curved/stepped/styled/single/empty frames.
A representative GPU capture was inspected. Native startup ran ten seconds without
errors and ended by timeout; physical acceptance remains pending.

Files: `src/renderers/area_renderer.rs`, `walk_line.rs`, `mod.rs`;
`src/model/series/area_pane_view.rs`, `series.rs`, `series_options.rs`, `chart_model.rs`;
`src/api/chart.rs`; `src/ui/line_chart.rs`, `multi_pane_frame.rs`;
`examples/area_reference.rs`, `public_chart.rs`; `scripts/check_area_reference.cjs`;
`tests/area_chart.rs`; this report and the built-in-series roadmap/checklist.
Full integrated results and performance comparison will be added at the roadmap's
final verification. No new dependencies or async state were introduced.

Reference source revision: `8a7c94ff8ddffde68955db61ef8b58098f239de0` in the local
Lightweight Charts checkout. Integrated results: [built-in series verification](built-in-series-verification.md).

Final integration extends the shared click/crosshair converter with exact typed
rows, original times, optional styles and unit custom values. Mixed-series and
whitespace tests cover these event payloads. Gradient paint adaptation lives in
`src/ui/line_chart/series_paint.rs`; `src/api/interaction.rs` is also changed.

`src/renderers/line_renderer.rs` also fixes shared negative-half-pixel marker
rounding to match source Math.round; a regression assertion covers the left edge.
