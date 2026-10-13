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

Validation results will be recorded after integration.
