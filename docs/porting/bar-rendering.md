# Bar series contract and design

Source: `model/series/bar-series.ts`, `bars-pane-view.ts`, `bars-pane-view-base.ts`,
`renderers/bars-renderer.ts`, `optimal-bar-width.ts`, and `series-bar-colorer.ts`.
The factory creates a pane view; the view projects visible OHLC rows using the
series first visible close and current price/time scales. The renderer draws a
high/low stem, an optional left open tick and a right close tick. Whitespace has
no OHLC geometry. The colorer also supplies last-value labels and price lines.
Actual source graphic fixtures cover whitespace and per-row colors.

Required behavior: nonextended visible range; open <= close uses up color;
row color wins; OHLC high/low participate in autoscale. Width is
max(floor(horizontal ratio), floor(spacing * .3 * ratio)), parity adjusted to
crosshair width. Thin stems use at most floor(ratio). At compressed spacing,
open/close ticks disappear together. Each tick is clamped to the stem vertically.
Inverted scales reorder projected high/low before drawing. Preserve JS rounding,
paint order, dojis, signed/zero rectangle extents and independent horizontal and
vertical ratios. Hidden/empty views clear their previous owned output. Invalid
colors/options/data are rejected before replacing working public state.

Two designs considered: share a generalized OHLC view with candlesticks, or a
dedicated typed Bar view using the established preparation pattern and rectangle
adapter. Choose the latter: Bar has one color and no wick/border fields, and
sharing a hierarchy now would add branching and unrelated candle changes.
Geometry remains small and synchronous. Reuse CandleRectangle as the existing
backend-neutral rectangle command, rather than add an identical type.

Ownership: ChartModel owns series handles and scales; BarsPaneView owns optional
BarsRendererData; immutable frames own converted rectangles. Preparation borrows
model state temporarily and stores no references, callbacks or cycles. Dropping
a view/frame releases its output; refresh clears stale output even on error.

Proposed API: BarPoint { x, open_y, high_y, low_y, close_y, color };
BarsRendererData { items, visible_range, bar_spacing, open_visible, thin_bars };
draw_commands(PixelRatio) -> Result<Vec<CandleRectangle>, BarRenderError>.
BarsPaneView::refresh(&mut ChartModel, SeriesId) prepares owned data, exactly as
existing candle views. Chart::add_series accepts typed Bar options and existing
BarData. Single and composed frames borrow the same view; labels use the shared
series color helper; OHLC event payloads and magnet already consume typed rows.
There is no Bar crosshair point marker in source.

Discard source inheritance, mutable renderer cache and manual destruction:
local draw state and Rust ownership preserve the observable result. Existing
conflation remains explicitly unsupported, consistent with the other views.
Graph coverage is best effort; clean paths were also read. Source series.ts had
changed metadata; its autoscale/marker branches were read directly.

Checkpoint B: focused renderer and public lifecycle tests pass; 7,776 actual-source
rectangle cases pass; strict all-target Clippy passes; 44 repeatable captures per
backend at ratios 1/1.25/1.5/2 pass. Captures cover thick stems, hidden open ticks,
compression, overrides and single/empty states. Bounded native startup succeeds.
Full integrated tests and fresh physical acceptance are recorded at the final
milestone checkpoint.

Files: src/renderers/bars_renderer.rs, src/renderers/mod.rs,
src/model/series/bars_pane_view.rs, src/model/series.rs,
src/model/series_options.rs, src/model/chart_model.rs, src/api/chart.rs,
src/ui/line_chart.rs, src/ui/multi_pane_frame.rs, tests/bar_chart.rs,
examples/bar_reference.rs, scripts/check_bar_reference.cjs,
examples/public_chart.rs. No dependencies added. Baseline is the next family;
conflation and series-specific hit testing remain outside this rendering milestone.

Reference source revision: `8a7c94ff8ddffde68955db61ef8b58098f239de0` in the local
Lightweight Charts checkout. Integrated results: [built-in series verification](built-in-series-verification.md).

Final integration extends the shared click/crosshair converter with exact typed
rows, original times, optional styles and unit custom values. Mixed-series and
whitespace tests cover these event payloads. Gradient paint adaptation lives in
`src/ui/line_chart/series_paint.rs`; `src/api/interaction.rs` is also changed.
