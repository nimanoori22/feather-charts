# Baseline contract and design

Sources: model/series/baseline-pane-view.ts, baseline-series.ts,
series-bar-colorer.ts, series.ts; renderers/baseline-renderer-area.ts,
baseline-renderer-line.ts, gradient-style-cache.ts, area-renderer-base.ts,
line-renderer-base.ts, draw-series-point-markers.ts and walk-line.ts.
Graph traces locate the factory -> pane view and pane -> area/line renderer
relationships; complete source reads provide the inherited behavior. Coverage
is best effort, with changed series.ts branches read directly. Upstream relative
gradient graphic fixture includes independent row fill and line overrides.

The pane prepares extended visible single-value points and projects the base
price using the same first visible value as every point. Relative bounds are
min/max projected visible y; otherwise bounds are pane edges. Autoscale uses
closes only and does NOT force the base into range. Fill closes at the projected
base. Fill and line runs have separate style equality; line colors switch at the
gradient base, even inside curves/steps. Four ordered stops are [top1, top2,
bottom1, bottom2], with middle stops at the same clamped baseline ratio.
Source gradient multiplier and bitmap-height sentinel behavior must survive
fractional ratios. Row overrides affect geometry colors; label/crosshair fallback
bar color uses series top/bottom line defaults, equality counting as above.
Hidden line preserves fills and optional markers; markers use the same gradient.
Finite public colors/base/radii must validate before replacement.

Design alternatives: split each curve/step at baseline intersections, or preserve
full paths with gradient-painted strokes and fills. Choose gradients: avoids cubic
root solving and dash resets and preserves source traversal. Reuse owned
FilledPath/VerticalGradient, add GradientStroke (StyledStroke + VerticalGradient)
and gradient marker groups. Dedicated BaselineRendererData owns line geometry,
per-row four fill/two stroke colors, base, bounds and fill line type. Its draw
returns owned fills, strokes and marker groups. BaselinePaneView refreshes from
ChartModel and public Chart registers this view like Area. Immutable single and
composed frames convert geometry once and own backend paint values.

ChartModel -> owned series/scales; view -> owned optional renderer data; frame ->
owned converted geometry. All engine work synchronous; no references retained,
callbacks, Rc cycles, async state or new dependencies. Drop releases output.
TypeScript inheritance/cache identity/manual destroy are discarded; value equality
and local segmentation preserve style runs without shared mutable renderers.

Iced supports gradient fills and strokes on both backends. Its gradient builder
replaces equal-offset stops, so the adapter must assign the public ordered stop
array directly. Degenerate equal-endpoint gradients need a transparent finite
backend paint (Canvas gradients with identical endpoints paint nothing), avoiding
TinySkia's invalid-gradient panic. Non-finite stop ratios are rejected explicitly.
WGPU uses half precision offsets and smoothstep interpolation; rendering cannot
claim pixel-identical Canvas linear interpolation, although geometry/stops remain
source-compatible. Capture crossings and endpoints on both backends.

Proposed API: BaselineRendererData::draw_commands(PixelRatio) ->
Result<BaselineDrawCommands, BaselineRenderError>; commands contain filled paths,
gradient strokes and reverse-order marker style groups. BaselinePaneView::refresh
borrows model briefly and clears stale data. Public input/options already have
Baseline types; registration and row/style validation activate them. Existing
magnet/event consumers read typed rows and close values; marker options extend
the Line/Area branch. No base autoscale merge is introduced.

Checkpoint C: two renderer tests and four public tests cover crossings, independent
marker/line visibility, non-destructive invalid replacements, live/history/clear/
reload/removal, normal/log/percentage/indexed and manual projection and inversion, autoscale
excluding the base, source decoration-color precedence, exact click/crosshair
rows and retained mixed-series frames, plus whitespace event exclusion. Two adapter tests cover equal-offset stops,
degenerate transparent paint and invalid backend geometry. Actual upstream renders
match 1,656 command cases. Both backends pass 60 repeatable captures at ratios
1/1.25/1.5/2, including relative/curved/stepped/row styles, base outside pane,
inversion and marker-only lines. Representative captures inspected. Native startup
runs ten seconds without error and ends by timeout. Physical acceptance pending.

Degenerate relative bounds at a base equal to those bounds give upstream a NaN
stop offset, which browser Canvas rejects. The Rust adapter intentionally paints
that zero-length gradient transparently with finite stops instead of crashing.
All nondegenerate oracle cases and finite degenerate ratios preserve source stops.

Files: src/renderers/baseline_renderer.rs, src/renderers/mod.rs,
src/model/series/baseline_pane_view.rs, src/model/series.rs,
src/model/series_options.rs, src/model/chart_model.rs, src/api/chart.rs,
src/api/interaction.rs, src/ui/line_chart.rs, src/ui/line_chart/series_paint.rs,
src/ui/multi_pane_frame.rs, tests/baseline_chart.rs,
examples/baseline_reference.rs, scripts/check_baseline_reference.cjs,
examples/public_chart.rs. Shared Area fill commands avoid preparing outlines twice.
No new dependency, async engine state or shared lifecycle machinery.

Remaining dependencies: conflation, animated last prices and series-specific
hover/hit-test behavior; arbitrary custom-series rendering and full API parity
are later milestones. Integrated verification and timings are linked below.

Reference source revision: `8a7c94ff8ddffde68955db61ef8b58098f239de0` in the local
Lightweight Charts checkout. Integrated results: [built-in series verification](built-in-series-verification.md).
