use crate::{
    model::series_options::*,
    renderers::draw_line::{LineStyle, LineType, LineWidth},
};

impl Default for SeriesOptionsCommon {
    fn default() -> Self {
        Self {
            last_value_visible: true,
            title: String::new(),
            price_scale_id: None,
            series_last_value_mode: None,
            visible: true,
            hit_test_tolerance: 3.,
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
            base_line_color: "#B2B5BE".into(),
            base_line_width: LineWidth::One,
            base_line_style: LineStyle::Solid,
            autoscale_info_provider: None,
            conflation_threshold_factor: None,
        }
    }
}
impl<S: Default> Default for SeriesOptions<S> {
    fn default() -> Self {
        Self {
            common: Default::default(),
            style: Default::default(),
        }
    }
}
impl Default for LineStyleOptions {
    fn default() -> Self {
        Self {
            color: "#2196f3".into(),
            line_style: LineStyle::Solid,
            line_width: LineWidth::Three,
            line_type: LineType::Simple,
            line_visible: true,
            point_markers_visible: false,
            point_markers_radius: None,
            crosshair_marker_visible: true,
            crosshair_marker_radius: 4.,
            crosshair_marker_border_color: String::new(),
            crosshair_marker_background_color: String::new(),
            crosshair_marker_border_width: 2.,
            last_price_animation: LastPriceAnimationMode::Disabled,
        }
    }
}
impl Default for CandlestickStyleOptions {
    fn default() -> Self {
        Self {
            up_color: "#26a69a".into(),
            down_color: "#ef5350".into(),
            wick_visible: true,
            border_visible: true,
            border_color: "#378658".into(),
            border_up_color: "#26a69a".into(),
            border_down_color: "#ef5350".into(),
            wick_color: "#737375".into(),
            wick_up_color: "#26a69a".into(),
            wick_down_color: "#ef5350".into(),
        }
    }
}
impl Default for HistogramStyleOptions {
    fn default() -> Self {
        Self {
            color: "#26a69a".into(),
            base: 0.,
        }
    }
}
impl Default for crate::model::chart_model::ChartModelOptions {
    fn default() -> Self {
        use crate::model::layout_options::*;
        Self {
            layout: LayoutOptions {
                background: Background::Solid {
                    color: "#ffffff".into(),
                },
                text_color: "#191919".into(),
                font_size: 12.,
                font_family: "sans-serif".into(),
                panes: LayoutPanesOptions {
                    enable_resize: true,
                    separator_color: "#E0E3EB".into(),
                    separator_hover_color: "rgba(178, 181, 189, 0.2)".into(),
                },
                attribution_logo: false,
                color_space: ColorSpace::Srgb,
                color_parsers: vec![],
            },
            pane: crate::model::pane::PaneOptions {
                left_price_scale: crate::model::price_scale::PriceScaleOptions {
                    visible: false,
                    ..Default::default()
                },
                ..Default::default()
            },
            grid: Default::default(),
            add_default_pane: true,
        }
    }
}
