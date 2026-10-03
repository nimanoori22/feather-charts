//! Owned ordinary axis ticks. Coordinates and metrics are logical pixels;
//! measurement, positioning corrections, overlap resolution and drawing are deferred.

use crate::{
    helpers::make_font::make_font,
    model::{
        ihorz_scale_behavior::{HorzScaleBehavior, TimeMark},
        layout_options::{Background, ColorSpace, LayoutOptions},
        price_scale::{PriceMark, PriceScale, PriceScaleOptionsError},
        time_scale_options::HorzScaleOptions,
    },
    renderers::{
        iprice_axis_view_renderer::PriceAxisViewRendererOptions,
        price_axis_renderer_options_provider::{
            PriceAxisRendererOptionsProvider, PriceAxisRendererStyle,
        },
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PriceAxisSide {
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PriceAxisSnapshot {
    pub side: PriceAxisSide,
    pub visible: bool,
    pub marks: Vec<PriceMark>,
    pub background: Background,
    pub color_space: ColorSpace,
    pub border_visible: bool,
    pub border_color: String,
    pub ticks_visible: bool,
    pub minimum_width: f64,
    /// Reuses typography/spacing derivation, not the floating-label interface.
    /// `color` is the effective ordinary-tick text color. Tick length remains
    /// reserved for text positioning even when no tick stroke is drawn.
    pub renderer_options: PriceAxisViewRendererOptions,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeLabelEmphasis {
    Normal,
    Emphasized,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimeAxisTick {
    pub mark: TimeMark,
    /// Priority labels paint after other labels, even when bold is disabled.
    pub priority: bool,
    pub emphasis: TimeLabelEmphasis,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimeAxisRendererOptions {
    pub font: String,
    pub bold_font: String,
    pub font_family: String,
    pub font_size: f32,
    pub text_color: String,
    pub border_size: f32,
    pub tick_length: f32,
    pub padding_top: f32,
    pub padding_bottom: f32,
    pub padding_horizontal: f32,
    pub baseline_offset: f32,
    pub label_bottom_offset: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimeAxisSnapshot {
    pub visible: bool,
    pub ticks: Vec<TimeAxisTick>,
    pub background: Background,
    pub color_space: ColorSpace,
    pub border_visible: bool,
    pub border_color: String,
    pub ticks_visible: bool,
    pub minimum_height: f64,
    pub renderer_options: TimeAxisRendererOptions,
}
impl TimeAxisSnapshot {
    /// The source time widget fills with the bottom color of a gradient,
    /// unlike price widgets, which paint the entire gradient.
    pub fn background_fill_color(&self) -> &str {
        bottom_color(&self.background)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AxisSnapshots {
    pub left: PriceAxisSnapshot,
    pub right: PriceAxisSnapshot,
    pub time: TimeAxisSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AxisSnapshotError {
    InvalidPane(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PriceAxisOptionsError {
    InvalidPane(usize),
    Options(PriceScaleOptionsError),
}

fn top_color(background: &Background) -> &str {
    match background {
        Background::Solid { color } => color,
        Background::VerticalGradient { top_color, .. } => top_color,
    }
}
fn bottom_color(background: &Background) -> &str {
    match background {
        Background::Solid { color } => color,
        Background::VerticalGradient { bottom_color, .. } => bottom_color,
    }
}

pub(crate) fn price_snapshot(
    scale: &mut PriceScale,
    side: PriceAxisSide,
    layout: &LayoutOptions,
    plot_has_width: bool,
    provider: &mut PriceAxisRendererOptionsProvider,
) -> PriceAxisSnapshot {
    let marks = if plot_has_width {
        scale.marks().to_vec()
    } else {
        vec![]
    };
    let options = scale.options();
    let renderer_options = provider
        .options(PriceAxisRendererStyle {
            font_size: layout.font_size as f32,
            font_family: &layout.font_family,
            text_color: options.text_color.as_deref().unwrap_or(&layout.text_color),
            pane_background_color: top_color(&layout.background),
        })
        .clone();
    PriceAxisSnapshot {
        side,
        visible: options.visible,
        marks,
        background: layout.background.clone(),
        color_space: layout.color_space,
        border_visible: options.border_visible,
        border_color: options.border_color.clone(),
        ticks_visible: options.ticks_visible,
        minimum_width: options.minimum_width,
        renderer_options,
    }
}

pub(crate) fn time_snapshot<B: HorzScaleBehavior>(
    marks: Vec<TimeMark>,
    behavior: &B,
    options: &HorzScaleOptions,
    layout: &LayoutOptions,
) -> TimeAxisSnapshot {
    // Some behaviors deliberately choose a threshold below the largest weight.
    let threshold = (!marks.is_empty()).then(|| behavior.max_tick_mark_weight(&marks));
    let ticks = marks
        .into_iter()
        .map(|mark| {
            let priority = threshold.is_some_and(|weight| mark.weight >= weight);
            TimeAxisTick {
                mark,
                priority,
                emphasis: if priority && options.allow_bold_labels {
                    TimeLabelEmphasis::Emphasized
                } else {
                    TimeLabelEmphasis::Normal
                },
            }
        })
        .collect();
    let size = layout.font_size as f32;
    TimeAxisSnapshot {
        visible: options.visible,
        ticks,
        background: layout.background.clone(),
        color_space: layout.color_space,
        border_visible: options.border_visible,
        border_color: options.border_color.clone(),
        ticks_visible: options.ticks_visible,
        minimum_height: options.minimum_height,
        renderer_options: TimeAxisRendererOptions {
            font: make_font(size, Some(&layout.font_family), None),
            bold_font: make_font(size, Some(&layout.font_family), Some("bold")),
            font_family: layout.font_family.clone(),
            font_size: size,
            text_color: layout.text_color.clone(),
            border_size: 1.,
            tick_length: 5.,
            padding_top: 3. * size / 12.,
            padding_bottom: 3. * size / 12.,
            padding_horizontal: 9. * size / 12.,
            baseline_offset: 0.,
            label_bottom_offset: 4. * size / 12.,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        horz_scale_behavior_time::{
            horz_scale_behavior_time::HorzScaleBehaviorTime, types::TickMarkWeight,
        },
        layout_options::LayoutPanesOptions,
        time_data::TickMarkWeightValue,
    };

    #[test]
    fn axis_snapshots_use_behavior_threshold_instead_of_numeric_maximum() {
        let layout = LayoutOptions {
            background: Background::Solid {
                color: "white".into(),
            },
            text_color: "black".into(),
            font_size: 12.,
            font_family: "Inter".into(),
            panes: LayoutPanesOptions {
                enable_resize: true,
                separator_color: String::new(),
                separator_hover_color: String::new(),
            },
            attribution_logo: false,
            color_space: ColorSpace::Srgb,
            color_parsers: vec![],
        };
        let marks = [
            TickMarkWeight::Minute30,
            TickMarkWeight::Hour1,
            TickMarkWeight::Hour6,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, weight)| TimeMark {
            coordinate: index as f64 * 50.,
            label: index.to_string(),
            weight: TickMarkWeightValue::from(weight),
            need_align_coordinate: index == 0,
        })
        .collect::<Vec<_>>();
        let snapshot = time_snapshot(
            marks.clone(),
            &HorzScaleBehaviorTime::default(),
            &HorzScaleOptions::default(),
            &layout,
        );
        assert_eq!(
            snapshot
                .ticks
                .iter()
                .map(|tick| tick.priority)
                .collect::<Vec<_>>(),
            [false, true, true]
        );
        assert_eq!(
            snapshot
                .ticks
                .iter()
                .map(|tick| tick.mark.clone())
                .collect::<Vec<_>>(),
            marks
        );
        assert_eq!(snapshot.ticks[1].emphasis, TimeLabelEmphasis::Emphasized);
    }
}
