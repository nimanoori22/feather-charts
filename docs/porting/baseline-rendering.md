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

Validation and exact changed files will be recorded after implementation.
