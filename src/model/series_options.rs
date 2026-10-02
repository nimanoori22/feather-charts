//! Typed options and option algorithms for every built-in series kind.
pub use crate::model::autoscale_info_impl::{
    AutoScaleMargins as SeriesAutoScaleMargins, AutoscaleInfo as SeriesAutoscaleInfo,
    PriceRange as SeriesPriceRange,
};
use crate::{
    model::{
        autoscale_info_impl::AutoscaleInfo,
        price_formatter_fn::{PriceFormatterFn, TickmarksPriceFormatterFn},
    },
    renderers::draw_line::{LineStyle, LineType, LineWidth},
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LastPriceAnimationMode {
    #[default]
    Disabled,
    Continuous,
    OnDataUpdate,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PriceAxisLastValueMode {
    #[default]
    LastPriceAndPercentageValue,
    LastValueAccordingToScale,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PriceLineSource {
    #[default]
    LastBar,
    LastVisible,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BaseValuePrice {
    pub price: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub enum BaseValueType {
    Price(BaseValuePrice),
}
#[derive(Clone, Debug, PartialEq)]
pub struct PriceFormatBuiltIn {
    pub kind: PriceFormatBuiltInType,
    pub precision: u32,
    pub min_move: f64,
    pub base: Option<f64>,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PriceFormatBuiltInType {
    #[default]
    Price,
    Volume,
    Percent,
}
pub struct PriceFormatCustom {
    pub formatter: PriceFormatterFn,
    pub tickmarks_formatter: Option<TickmarksPriceFormatterFn>,
    pub min_move: f64,
    pub base: Option<f64>,
}
pub enum PriceFormat {
    BuiltIn(PriceFormatBuiltIn),
    Custom(PriceFormatCustom),
}
pub type AutoscaleInfoProvider =
    Box<dyn Fn(&dyn Fn() -> Option<AutoscaleInfo>) -> Option<AutoscaleInfo>>;

#[derive(Clone, Debug, PartialEq)]
pub struct CandlestickStyleOptions {
    pub up_color: String,
    pub down_color: String,
    pub wick_visible: bool,
    pub border_visible: bool,
    pub border_color: String,
    pub border_up_color: String,
    pub border_down_color: String,
    pub wick_color: String,
    pub wick_up_color: String,
    pub wick_down_color: String,
}
pub fn fill_up_down_candlestick_colors(options: &mut CandlestickStyleOptionsPatch) {
    if let Some(color) = options.border_color.clone() {
        options.border_up_color = Some(color.clone());
        options.border_down_color = Some(color)
    }
    if let Some(color) = options.wick_color.clone() {
        options.wick_up_color = Some(color.clone());
        options.wick_down_color = Some(color)
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CandlestickStyleOptionsPatch {
    pub border_color: Option<String>,
    pub border_up_color: Option<String>,
    pub border_down_color: Option<String>,
    pub wick_color: Option<String>,
    pub wick_up_color: Option<String>,
    pub wick_down_color: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BarStyleOptions {
    pub up_color: String,
    pub down_color: String,
    pub open_visible: bool,
    pub thin_bars: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct HistogramStyleOptions {
    pub color: String,
    pub base: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CustomStyleOptions {
    pub color: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LineStyleOptions {
    pub color: String,
    pub line_style: LineStyle,
    pub line_width: LineWidth,
    pub line_type: LineType,
    pub line_visible: bool,
    pub point_markers_visible: bool,
    pub point_markers_radius: Option<f64>,
    pub crosshair_marker_visible: bool,
    pub crosshair_marker_radius: f64,
    pub crosshair_marker_border_color: String,
    pub crosshair_marker_background_color: String,
    pub crosshair_marker_border_width: f64,
    pub last_price_animation: LastPriceAnimationMode,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AreaStyleOptions {
    pub top_color: String,
    pub bottom_color: String,
    pub relative_gradient: bool,
    pub invert_filled_area: bool,
    pub line: LineStyleOptions,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BaselineStyleOptions {
    pub base_value: BaseValueType,
    pub relative_gradient: bool,
    pub top_fill_color1: String,
    pub top_fill_color2: String,
    pub top_line_color: String,
    pub bottom_fill_color1: String,
    pub bottom_fill_color2: String,
    pub bottom_line_color: String,
    pub line: LineStyleOptions,
}
pub struct SeriesOptionsCommon {
    pub last_value_visible: bool,
    pub title: String,
    pub price_scale_id: Option<String>,
    pub series_last_value_mode: Option<PriceAxisLastValueMode>,
    pub visible: bool,
    pub hit_test_tolerance: f64,
    pub price_line_visible: bool,
    pub price_line_source: PriceLineSource,
    pub price_line_width: LineWidth,
    pub price_line_color: String,
    pub price_line_style: LineStyle,
    pub price_format: PriceFormat,
    pub base_line_visible: bool,
    pub base_line_color: String,
    pub base_line_width: LineWidth,
    pub base_line_style: LineStyle,
    pub autoscale_info_provider: Option<AutoscaleInfoProvider>,
    pub conflation_threshold_factor: Option<f64>,
}
pub struct SeriesOptions<S> {
    pub common: SeriesOptionsCommon,
    pub style: S,
}
pub type AreaSeriesOptions = SeriesOptions<AreaStyleOptions>;
pub type BaselineSeriesOptions = SeriesOptions<BaselineStyleOptions>;
pub type BarSeriesOptions = SeriesOptions<BarStyleOptions>;
pub type CandlestickSeriesOptions = SeriesOptions<CandlestickStyleOptions>;
pub type HistogramSeriesOptions = SeriesOptions<HistogramStyleOptions>;
pub type CustomSeriesOptions = SeriesOptions<CustomStyleOptions>;
pub type LineSeriesOptions = SeriesOptions<LineStyleOptions>;
pub fn precision_by_min_move(mut min_move: f64) -> u32 {
    if min_move >= 1.0 {
        return 0;
    }
    for i in 0..8 {
        if (min_move.round() - min_move).abs() < 1e-8 {
            return i;
        }
        min_move *= 10.0;
    }
    8
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn precision_matches_min_move_contract() {
        assert_eq!(precision_by_min_move(1.0), 0);
        assert_eq!(precision_by_min_move(0.01), 2);
        assert_eq!(precision_by_min_move(0.05), 2);
    }
    #[test]
    fn shared_candlestick_colors_override_directional_colors() {
        let mut p = CandlestickStyleOptionsPatch {
            border_color: Some("a".into()),
            wick_color: Some("b".into()),
            ..Default::default()
        };
        fill_up_down_candlestick_colors(&mut p);
        assert_eq!(p.border_down_color.as_deref(), Some("a"));
        assert_eq!(p.wick_up_color.as_deref(), Some("b"));
    }
}
