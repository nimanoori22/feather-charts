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
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PriceFormatBuiltInPatch {
    pub kind: Option<PriceFormatBuiltInType>,
    pub precision: Option<u32>,
    pub min_move: Option<f64>,
    pub base: Option<Option<f64>>,
}
pub enum PriceFormatPatch {
    BuiltIn(PriceFormatBuiltInPatch),
    Custom(PriceFormatCustom),
}
impl PriceFormatPatch {
    pub fn patch_precision_from_min_move(&mut self) {
        if let Self::BuiltIn(p) = self
            && p.min_move.is_some()
            && p.precision.is_none()
        {
            p.precision = p.min_move.map(precision_by_min_move);
        }
    }
    pub fn apply_to(self, target: &mut PriceFormat) {
        match self {
            Self::Custom(v) => *target = PriceFormat::Custom(v),
            Self::BuiltIn(p) => {
                if !matches!(target, PriceFormat::BuiltIn(_)) {
                    *target = PriceFormat::BuiltIn(PriceFormatBuiltIn {
                        kind: PriceFormatBuiltInType::Price,
                        precision: 2,
                        min_move: 0.01,
                        base: None,
                    });
                }
                let PriceFormat::BuiltIn(v) = target else {
                    unreachable!()
                };
                if let Some(x) = p.kind {
                    v.kind = x;
                }
                if let Some(x) = p.precision {
                    v.precision = x;
                }
                if let Some(x) = p.min_move {
                    v.min_move = x;
                }
                if let Some(x) = p.base {
                    v.base = x;
                }
            }
        }
    }
}
pub type AutoscaleInfoProvider =
    Box<dyn Fn(&dyn Fn() -> Option<AutoscaleInfo>) -> Option<AutoscaleInfo>>;
#[derive(Default)]
pub enum FieldPatch<T> {
    #[default]
    Unchanged,
    Set(T),
    Clear,
}

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
    pub up_color: Option<String>,
    pub down_color: Option<String>,
    pub wick_visible: Option<bool>,
    pub border_visible: Option<bool>,
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
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BarStyleOptionsPatch {
    pub up_color: Option<String>,
    pub down_color: Option<String>,
    pub open_visible: Option<bool>,
    pub thin_bars: Option<bool>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct HistogramStyleOptions {
    pub color: String,
    pub base: f64,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HistogramStyleOptionsPatch {
    pub color: Option<String>,
    pub base: Option<f64>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CustomStyleOptions {
    pub color: String,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CustomStyleOptionsPatch {
    pub color: Option<String>,
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
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LineStyleOptionsPatch {
    pub color: Option<String>,
    pub line_style: Option<LineStyle>,
    pub line_width: Option<LineWidth>,
    pub line_type: Option<LineType>,
    pub line_visible: Option<bool>,
    pub point_markers_visible: Option<bool>,
    pub point_markers_radius: Option<Option<f64>>,
    pub crosshair_marker_visible: Option<bool>,
    pub crosshair_marker_radius: Option<f64>,
    pub crosshair_marker_border_color: Option<String>,
    pub crosshair_marker_background_color: Option<String>,
    pub crosshair_marker_border_width: Option<f64>,
    pub last_price_animation: Option<LastPriceAnimationMode>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AreaStyleOptions {
    pub top_color: String,
    pub bottom_color: String,
    pub relative_gradient: bool,
    pub invert_filled_area: bool,
    pub line_color: String,
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
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AreaStyleOptionsPatch {
    pub top_color: Option<String>,
    pub bottom_color: Option<String>,
    pub relative_gradient: Option<bool>,
    pub invert_filled_area: Option<bool>,
    pub line_color: Option<String>,
    pub line_style: Option<LineStyle>,
    pub line_width: Option<LineWidth>,
    pub line_type: Option<LineType>,
    pub line_visible: Option<bool>,
    pub point_markers_visible: Option<bool>,
    pub point_markers_radius: Option<Option<f64>>,
    pub crosshair_marker_visible: Option<bool>,
    pub crosshair_marker_radius: Option<f64>,
    pub crosshair_marker_border_color: Option<String>,
    pub crosshair_marker_background_color: Option<String>,
    pub crosshair_marker_border_width: Option<f64>,
    pub last_price_animation: Option<LastPriceAnimationMode>,
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
    pub line_width: LineWidth,
    pub line_style: LineStyle,
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
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BaselineStyleOptionsPatch {
    pub base_value: Option<BaseValueType>,
    pub relative_gradient: Option<bool>,
    pub top_fill_color1: Option<String>,
    pub top_fill_color2: Option<String>,
    pub top_line_color: Option<String>,
    pub bottom_fill_color1: Option<String>,
    pub bottom_fill_color2: Option<String>,
    pub bottom_line_color: Option<String>,
    pub line_width: Option<LineWidth>,
    pub line_style: Option<LineStyle>,
    pub line_type: Option<LineType>,
    pub line_visible: Option<bool>,
    pub point_markers_visible: Option<bool>,
    pub point_markers_radius: Option<Option<f64>>,
    pub crosshair_marker_visible: Option<bool>,
    pub crosshair_marker_radius: Option<f64>,
    pub crosshair_marker_border_color: Option<String>,
    pub crosshair_marker_background_color: Option<String>,
    pub crosshair_marker_border_width: Option<f64>,
    pub last_price_animation: Option<LastPriceAnimationMode>,
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
pub struct SeriesOptionsCommonPatch {
    pub last_value_visible: Option<bool>,
    pub title: Option<String>,
    pub price_scale_id: Option<Option<String>>,
    pub series_last_value_mode: Option<Option<PriceAxisLastValueMode>>,
    pub visible: Option<bool>,
    pub hit_test_tolerance: Option<f64>,
    pub price_line_visible: Option<bool>,
    pub price_line_source: Option<PriceLineSource>,
    pub price_line_width: Option<LineWidth>,
    pub price_line_color: Option<String>,
    pub price_line_style: Option<LineStyle>,
    pub price_format: Option<PriceFormatPatch>,
    pub base_line_visible: Option<bool>,
    pub base_line_color: Option<String>,
    pub base_line_width: Option<LineWidth>,
    pub base_line_style: Option<LineStyle>,
    pub autoscale_info_provider: FieldPatch<AutoscaleInfoProvider>,
    pub conflation_threshold_factor: Option<Option<f64>>,
}
impl Default for SeriesOptionsCommonPatch {
    fn default() -> Self {
        Self {
            last_value_visible: None,
            title: None,
            price_scale_id: None,
            series_last_value_mode: None,
            visible: None,
            hit_test_tolerance: None,
            price_line_visible: None,
            price_line_source: None,
            price_line_width: None,
            price_line_color: None,
            price_line_style: None,
            price_format: None,
            base_line_visible: None,
            base_line_color: None,
            base_line_width: None,
            base_line_style: None,
            autoscale_info_provider: FieldPatch::Unchanged,
            conflation_threshold_factor: None,
        }
    }
}
impl SeriesOptionsCommonPatch {
    pub fn apply_to(self, target: &mut SeriesOptionsCommon) {
        if let Some(v) = self.last_value_visible {
            target.last_value_visible = v;
        }
        if let Some(v) = self.title {
            target.title = v;
        }
        if let Some(v) = self.price_scale_id {
            target.price_scale_id = v;
        }
        if let Some(v) = self.series_last_value_mode {
            target.series_last_value_mode = v;
        }
        if let Some(v) = self.visible {
            target.visible = v;
        }
        if let Some(v) = self.hit_test_tolerance {
            target.hit_test_tolerance = v;
        }
        if let Some(v) = self.price_line_visible {
            target.price_line_visible = v;
        }
        if let Some(v) = self.price_line_source {
            target.price_line_source = v;
        }
        if let Some(v) = self.price_line_width {
            target.price_line_width = v;
        }
        if let Some(v) = self.price_line_color {
            target.price_line_color = v;
        }
        if let Some(v) = self.price_line_style {
            target.price_line_style = v;
        }
        if let Some(mut v) = self.price_format {
            v.patch_precision_from_min_move();
            v.apply_to(&mut target.price_format);
        }
        if let Some(v) = self.base_line_visible {
            target.base_line_visible = v;
        }
        if let Some(v) = self.base_line_color {
            target.base_line_color = v;
        }
        if let Some(v) = self.base_line_width {
            target.base_line_width = v;
        }
        if let Some(v) = self.base_line_style {
            target.base_line_style = v;
        }
        match self.autoscale_info_provider {
            FieldPatch::Unchanged => {}
            FieldPatch::Set(v) => target.autoscale_info_provider = Some(v),
            FieldPatch::Clear => target.autoscale_info_provider = None,
        }
        if let Some(v) = self.conflation_threshold_factor {
            target.conflation_threshold_factor = v;
        }
    }
}
pub struct SeriesOptions<S> {
    pub common: SeriesOptionsCommon,
    pub style: S,
}
pub struct SeriesOptionsPatch<S> {
    pub common: SeriesOptionsCommonPatch,
    pub style: S,
}
pub type AreaSeriesOptions = SeriesOptions<AreaStyleOptions>;
pub type BaselineSeriesOptions = SeriesOptions<BaselineStyleOptions>;
pub type BarSeriesOptions = SeriesOptions<BarStyleOptions>;
pub type CandlestickSeriesOptions = SeriesOptions<CandlestickStyleOptions>;
pub type HistogramSeriesOptions = SeriesOptions<HistogramStyleOptions>;
pub type CustomSeriesOptions = SeriesOptions<CustomStyleOptions>;
pub type LineSeriesOptions = SeriesOptions<LineStyleOptions>;
pub type AreaSeriesOptionsPatch = SeriesOptionsPatch<AreaStyleOptionsPatch>;
pub type BaselineSeriesOptionsPatch = SeriesOptionsPatch<BaselineStyleOptionsPatch>;
pub type BarSeriesOptionsPatch = SeriesOptionsPatch<BarStyleOptionsPatch>;
pub type CandlestickSeriesOptionsPatch = SeriesOptionsPatch<CandlestickStyleOptionsPatch>;
pub type HistogramSeriesOptionsPatch = SeriesOptionsPatch<HistogramStyleOptionsPatch>;
pub type CustomSeriesOptionsPatch = SeriesOptionsPatch<CustomStyleOptionsPatch>;
pub type LineSeriesOptionsPatch = SeriesOptionsPatch<LineStyleOptionsPatch>;
pub type SeriesPartialOptions<S> = SeriesOptionsPatch<S>;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum SeriesType {
    Bar,
    Candlestick,
    Area,
    Baseline,
    #[default]
    Line,
    Histogram,
    Custom,
}
pub enum SeriesStyleOptionsMap {
    Bar(BarStyleOptions),
    Candlestick(CandlestickStyleOptions),
    Area(AreaStyleOptions),
    Baseline(BaselineStyleOptions),
    Line(LineStyleOptions),
    Histogram(HistogramStyleOptions),
    Custom(CustomStyleOptions),
}
pub enum SeriesOptionsMap {
    Bar(BarSeriesOptions),
    Candlestick(CandlestickSeriesOptions),
    Area(AreaSeriesOptions),
    Baseline(BaselineSeriesOptions),
    Line(LineSeriesOptions),
    Histogram(HistogramSeriesOptions),
    Custom(CustomSeriesOptions),
}
pub enum SeriesPartialOptionsMap {
    Bar(BarSeriesOptionsPatch),
    Candlestick(CandlestickSeriesOptionsPatch),
    Area(AreaSeriesOptionsPatch),
    Baseline(BaselineSeriesOptionsPatch),
    Line(LineSeriesOptionsPatch),
    Histogram(HistogramSeriesOptionsPatch),
    Custom(CustomSeriesOptionsPatch),
}

macro_rules! set_patch {
    ($target:expr, $patch:expr, $field:ident) => {
        if let Some(value) = $patch.$field {
            $target.$field = value;
        }
    };
}
impl CandlestickStyleOptionsPatch {
    fn apply_to(self, t: &mut CandlestickStyleOptions) {
        set_patch!(t, self, up_color);
        set_patch!(t, self, down_color);
        set_patch!(t, self, wick_visible);
        set_patch!(t, self, border_visible);
        set_patch!(t, self, border_color);
        set_patch!(t, self, border_up_color);
        set_patch!(t, self, border_down_color);
        set_patch!(t, self, wick_color);
        set_patch!(t, self, wick_up_color);
        set_patch!(t, self, wick_down_color);
    }
}
impl BarStyleOptionsPatch {
    fn apply_to(self, t: &mut BarStyleOptions) {
        set_patch!(t, self, up_color);
        set_patch!(t, self, down_color);
        set_patch!(t, self, open_visible);
        set_patch!(t, self, thin_bars);
    }
}
impl HistogramStyleOptionsPatch {
    fn apply_to(self, t: &mut HistogramStyleOptions) {
        set_patch!(t, self, color);
        set_patch!(t, self, base);
    }
}
impl CustomStyleOptionsPatch {
    fn apply_to(self, t: &mut CustomStyleOptions) {
        set_patch!(t, self, color);
    }
}
impl LineStyleOptionsPatch {
    fn apply_to(self, t: &mut LineStyleOptions) {
        set_patch!(t, self, color);
        set_patch!(t, self, line_style);
        set_patch!(t, self, line_width);
        set_patch!(t, self, line_type);
        set_patch!(t, self, line_visible);
        set_patch!(t, self, point_markers_visible);
        set_patch!(t, self, point_markers_radius);
        set_patch!(t, self, crosshair_marker_visible);
        set_patch!(t, self, crosshair_marker_radius);
        set_patch!(t, self, crosshair_marker_border_color);
        set_patch!(t, self, crosshair_marker_background_color);
        set_patch!(t, self, crosshair_marker_border_width);
        set_patch!(t, self, last_price_animation);
    }
}
impl AreaStyleOptionsPatch {
    fn apply_to(self, t: &mut AreaStyleOptions) {
        set_patch!(t, self, top_color);
        set_patch!(t, self, bottom_color);
        set_patch!(t, self, relative_gradient);
        set_patch!(t, self, invert_filled_area);
        set_patch!(t, self, line_color);
        set_patch!(t, self, line_style);
        set_patch!(t, self, line_width);
        set_patch!(t, self, line_type);
        set_patch!(t, self, line_visible);
        set_patch!(t, self, point_markers_visible);
        set_patch!(t, self, point_markers_radius);
        set_patch!(t, self, crosshair_marker_visible);
        set_patch!(t, self, crosshair_marker_radius);
        set_patch!(t, self, crosshair_marker_border_color);
        set_patch!(t, self, crosshair_marker_background_color);
        set_patch!(t, self, crosshair_marker_border_width);
        set_patch!(t, self, last_price_animation);
    }
}
impl BaselineStyleOptionsPatch {
    fn apply_to(self, t: &mut BaselineStyleOptions) {
        set_patch!(t, self, base_value);
        set_patch!(t, self, relative_gradient);
        set_patch!(t, self, top_fill_color1);
        set_patch!(t, self, top_fill_color2);
        set_patch!(t, self, top_line_color);
        set_patch!(t, self, bottom_fill_color1);
        set_patch!(t, self, bottom_fill_color2);
        set_patch!(t, self, bottom_line_color);
        set_patch!(t, self, line_width);
        set_patch!(t, self, line_style);
        set_patch!(t, self, line_type);
        set_patch!(t, self, line_visible);
        set_patch!(t, self, point_markers_visible);
        set_patch!(t, self, point_markers_radius);
        set_patch!(t, self, crosshair_marker_visible);
        set_patch!(t, self, crosshair_marker_radius);
        set_patch!(t, self, crosshair_marker_border_color);
        set_patch!(t, self, crosshair_marker_background_color);
        set_patch!(t, self, crosshair_marker_border_width);
        set_patch!(t, self, last_price_animation);
    }
}

macro_rules! option_apply {
    ($style:ident,$patch:ident) => {
        impl SeriesOptions<$style> {
            pub fn apply_patch(&mut self, patch: SeriesOptionsPatch<$patch>) {
                patch.common.apply_to(&mut self.common);
                patch.style.apply_to(&mut self.style);
            }
        }
    };
}
option_apply!(CandlestickStyleOptions, CandlestickStyleOptionsPatch);
option_apply!(BarStyleOptions, BarStyleOptionsPatch);
option_apply!(HistogramStyleOptions, HistogramStyleOptionsPatch);
option_apply!(CustomStyleOptions, CustomStyleOptionsPatch);
option_apply!(LineStyleOptions, LineStyleOptionsPatch);
option_apply!(AreaStyleOptions, AreaStyleOptionsPatch);
option_apply!(BaselineStyleOptions, BaselineStyleOptionsPatch);
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

    fn line_options() -> LineSeriesOptions {
        SeriesOptions {
            common: SeriesOptionsCommon {
                last_value_visible: true,
                title: String::new(),
                price_scale_id: None,
                series_last_value_mode: None,
                visible: true,
                hit_test_tolerance: 3.0,
                price_line_visible: true,
                price_line_source: PriceLineSource::LastBar,
                price_line_width: LineWidth::One,
                price_line_color: String::new(),
                price_line_style: LineStyle::Dashed,
                price_format: PriceFormat::BuiltIn(PriceFormatBuiltIn {
                    kind: PriceFormatBuiltInType::Price,
                    precision: 2,
                    min_move: 0.01,
                    base: None,
                }),
                base_line_visible: true,
                base_line_color: String::new(),
                base_line_width: LineWidth::One,
                base_line_style: LineStyle::Solid,
                autoscale_info_provider: None,
                conflation_threshold_factor: None,
            },
            style: LineStyleOptions {
                color: "a".into(),
                line_style: LineStyle::Solid,
                line_width: LineWidth::One,
                line_type: LineType::Simple,
                line_visible: true,
                point_markers_visible: false,
                point_markers_radius: None,
                crosshair_marker_visible: true,
                crosshair_marker_radius: 4.0,
                crosshair_marker_border_color: String::new(),
                crosshair_marker_background_color: String::new(),
                crosshair_marker_border_width: 2.0,
                last_price_animation: LastPriceAnimationMode::Disabled,
            },
        }
    }
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
    #[test]
    fn runtime_patch_merges_fields_derives_precision_and_clears_provider() {
        let mut options = line_options();
        options.common.autoscale_info_provider = Some(Box::new(|base| base()));
        options.apply_patch(LineSeriesOptionsPatch {
            common: SeriesOptionsCommonPatch {
                price_format: Some(PriceFormatPatch::BuiltIn(PriceFormatBuiltInPatch {
                    min_move: Some(0.005),
                    ..Default::default()
                })),
                autoscale_info_provider: FieldPatch::Clear,
                ..Default::default()
            },
            style: LineStyleOptionsPatch {
                color: Some("b".into()),
                point_markers_radius: Some(Some(3.0)),
                ..Default::default()
            },
        });
        assert_eq!(options.style.color, "b");
        assert_eq!(options.style.point_markers_radius, Some(3.0));
        assert!(options.common.autoscale_info_provider.is_none());
        match options.common.price_format {
            PriceFormat::BuiltIn(ref f) => assert_eq!((f.min_move, f.precision), (0.005, 3)),
            PriceFormat::Custom(_) => panic!("unexpected custom format"),
        }
    }
}
