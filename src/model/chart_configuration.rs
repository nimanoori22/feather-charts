//! Owned runtime configuration; construction callbacks and renderer setup stay private.
use crate::model::{
    crosshair::CrosshairOptions,
    grid::GridOptions,
    layout_options::{Background, LayoutPanesOptions},
    pane::PaneOptions,
    price_scale::PriceScaleOptionsPatch,
    time_scale_options::{HorzScaleOptions, HorzScaleOptionsPatch},
};

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutAppearance {
    pub attribution_logo: bool,
    pub color_space: crate::model::layout_options::ColorSpace,
    pub background: Background,
    pub text_color: String,
    pub font_size: f64,
    pub font_family: String,
    pub panes: LayoutPanesOptions,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaneLayoutOptionsPatch {
    pub enable_resize: Option<bool>,
    pub separator_color: Option<String>,
    pub separator_hover_color: Option<String>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutOptionsPatch {
    pub attribution_logo: Option<bool>,
    pub color_space: Option<crate::model::layout_options::ColorSpace>,
    pub background: Option<Background>,
    pub text_color: Option<String>,
    pub font_size: Option<f64>,
    pub font_family: Option<String>,
    pub panes: Option<PaneLayoutOptionsPatch>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GridLineOptionsPatch {
    pub color: Option<String>,
    pub style: Option<crate::renderers::draw_line::LineStyle>,
    pub visible: Option<bool>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GridOptionsPatch {
    pub vert_lines: Option<GridLineOptionsPatch>,
    pub horz_lines: Option<GridLineOptionsPatch>,
}
#[derive(Clone, Debug, Default)]
pub struct ChartModelOptionsPatch {
    /// Chart-wide preference for existing and future panes; series assignments stay explicit.
    pub default_visible_price_scale: Option<crate::model::pane::DefaultVisiblePriceScale>,
    pub layout: Option<LayoutOptionsPatch>,
    pub grid: Option<GridOptionsPatch>,
    pub left_price_scale: Option<PriceScaleOptionsPatch>,
    pub right_price_scale: Option<PriceScaleOptionsPatch>,
    pub overlay_price_scales: Option<PriceScaleOptionsPatch>,
    pub time_scale: Option<HorzScaleOptionsPatch>,
    pub crosshair: Option<CrosshairOptions>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeModelOptions {
    pub layout: LayoutAppearance,
    pub grid: GridOptions,
    pub pane: PaneOptions,
    pub time_scale: HorzScaleOptions,
    pub crosshair: CrosshairOptions,
}

macro_rules! merge_fields {($target:expr,$patch:expr,$($field:ident),+)=>{ $(if let Some(v)=$patch.$field {$target.$field=v;})+ };}
impl LayoutOptionsPatch {
    pub(crate) fn apply(self, target: &mut LayoutAppearance) {
        merge_fields!(
            target,
            self,
            attribution_logo,
            color_space,
            background,
            text_color,
            font_size,
            font_family
        );
        if let Some(p) = self.panes {
            merge_fields!(
                target.panes,
                p,
                enable_resize,
                separator_color,
                separator_hover_color
            );
        }
    }
}
impl GridOptionsPatch {
    pub(crate) fn apply(self, target: &mut GridOptions) {
        for (line, p) in [
            (&mut target.vert_lines, self.vert_lines),
            (&mut target.horz_lines, self.horz_lines),
        ] {
            if let Some(p) = p {
                merge_fields!(line, p, color, style, visible);
            }
        }
    }
}
impl HorzScaleOptionsPatch {
    pub(crate) fn merged(self, mut target: HorzScaleOptions) -> HorzScaleOptions {
        merge_fields!(
            target,
            self,
            right_offset,
            right_offset_pixels,
            bar_spacing,
            min_bar_spacing,
            max_bar_spacing,
            fix_left_edge,
            fix_right_edge,
            lock_visible_time_range_on_resize,
            right_bar_stays_on_scroll,
            border_visible,
            border_color,
            visible,
            time_visible,
            seconds_visible,
            shift_visible_range_on_new_bar,
            allow_shift_visible_range_on_whitespace_replacement,
            ticks_visible,
            tick_mark_max_character_length,
            uniform_distribution,
            minimum_height,
            allow_bold_labels,
            ignore_whitespace_indices,
            enable_conflation,
            conflation_threshold_factor,
            precompute_conflation_on_init,
            precompute_conflation_priority
        );
        target
    }
}

pub(crate) fn valid_scale(
    options: &crate::model::price_scale::PriceScaleOptions,
    parser: &mut crate::model::colors::ColorParser,
) -> bool {
    options.minimum_width.is_finite()
        && options.minimum_width >= 0.
        && options.minimum_width <= f64::from(f32::MAX)
        && options.tick_mark_density.is_finite()
        && options.tick_mark_density > 0.
        && parser.parse_color(&options.border_color).is_ok()
        && options
            .text_color
            .as_deref()
            .is_none_or(|c| parser.parse_color(c).is_ok())
}
pub(crate) fn scale_geometry_changed(
    a: &crate::model::price_scale::PriceScaleOptions,
    b: &crate::model::price_scale::PriceScaleOptions,
) -> bool {
    let mut a = a.clone();
    a.border_color = b.border_color.clone();
    a.text_color = b.text_color.clone();
    a.border_visible = b.border_visible;
    a.ticks_visible = b.ticks_visible;
    a != *b
}

/// Shared construction/runtime boundary; validate before invoking custom hooks.
pub(crate) fn valid_horizontal_options(t: &HorzScaleOptions) -> bool {
    [
        t.bar_spacing,
        t.min_bar_spacing,
        t.max_bar_spacing,
        t.right_offset,
        t.minimum_height,
    ]
    .into_iter()
    .all(f64::is_finite)
        && t.right_offset_pixels.is_none_or(f64::is_finite)
        && t.bar_spacing > 0.
        && t.min_bar_spacing > 0.
        && t.max_bar_spacing >= 0.
        && (t.max_bar_spacing == 0. || t.max_bar_spacing >= t.min_bar_spacing)
        && t.minimum_height >= 0.
        && t.minimum_height <= f64::from(f32::MAX)
        && t.conflation_threshold_factor
            .is_none_or(|v| v.is_finite() && v > 0.)
}
