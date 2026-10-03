//! Owned glyph paths and fills; drawing neither measures text nor reads a model.
use crate::{
    model::axis_snapshots::{TimeAxisSnapshot, TimeLabelEmphasis},
    renderers::{
        grid_renderer::PixelRatio,
        price_axis_renderer::AxisBounds,
        time_axis_renderer::{
            PreparedTimeAxis, TimeAxisCommand, TimeAxisRenderError, measure_time_axis,
            prepare_time_axis,
        },
    },
    ui::{
        line_chart::{FrameError, color},
        price_axis::{
            AxisFontResolver, AxisPrimitive, IcedAxisTextMeasurer, PriceAxisUiError,
            draw_axis_primitives,
        },
    },
};
use iced::{
    Font, Point, Rectangle, Size,
    advanced::text::{Alignment, LineHeight, Shaping},
    alignment,
    widget::canvas::{Frame, Text},
};

#[derive(Debug)]
pub enum TimeAxisUiError {
    Renderer(TimeAxisRenderError),
    Font(PriceAxisUiError),
    Color(FrameError),
}

pub(crate) fn axis_font(
    fonts: &AxisFontResolver,
    family: &str,
    emphasis: TimeLabelEmphasis,
) -> Result<Font, TimeAxisUiError> {
    let mut font = fonts.resolve(family).map_err(TimeAxisUiError::Font)?;
    font.weight = match emphasis {
        TimeLabelEmphasis::Normal => iced::font::Weight::Normal,
        TimeLabelEmphasis::Emphasized => iced::font::Weight::Bold,
    };
    Ok(font)
}

#[derive(Clone, Debug, Default)]
pub struct IcedTimeAxis {
    pub required_height: f64,
    pub size: Size,
    primitives: Vec<AxisPrimitive>,
}
impl IcedTimeAxis {
    pub fn from_commands(
        commands: &PreparedTimeAxis,
        fonts: &AxisFontResolver,
    ) -> Result<Self, TimeAxisUiError> {
        let size = Size::new(commands.bounds.width as f32, commands.bounds.height as f32);
        let mut primitives = vec![];
        for command in &commands.commands {
            match command {
                TimeAxisCommand::Background(value) => primitives.push(AxisPrimitive::Fill {
                    origin: Point::ORIGIN,
                    size,
                    fill: color(value).map_err(TimeAxisUiError::Color)?.into(),
                }),
                TimeAxisCommand::Rectangle { rect, color: value } => {
                    primitives.push(AxisPrimitive::Fill {
                        origin: Point::new(rect.origin.x as f32, rect.origin.y as f32),
                        size: Size::new(rect.size.width as f32, rect.size.height as f32),
                        fill: color(value).map_err(TimeAxisUiError::Color)?.into(),
                    })
                }
                TimeAxisCommand::Text {
                    label,
                    position,
                    emphasis,
                    font_size,
                    font_family,
                    color: value,
                } => {
                    let text = Text {
                        content: label.clone(),
                        position: Point::new(position.x as f32, position.y as f32),
                        color: color(value).map_err(TimeAxisUiError::Color)?,
                        size: (*font_size).into(),
                        font: axis_font(fonts, font_family, *emphasis)?,
                        align_x: Alignment::Center,
                        align_y: alignment::Vertical::Center,
                        shaping: Shaping::Advanced,
                        line_height: LineHeight::Relative(1.2),
                        max_width: f32::INFINITY,
                    };
                    text.draw_with(|path, color| {
                        primitives.push(AxisPrimitive::Glyph { path, color })
                    });
                }
            }
        }
        Ok(Self {
            required_height: commands.required_height,
            size,
            primitives,
        })
    }
    pub fn draw(&self, frame: &mut Frame) {
        if let Some(clip) = self.clip_bounds(frame.size()) {
            draw_axis_primitives(&self.primitives, clip, frame);
        }
    }
    pub fn draw_at(
        &self,
        frame: &mut Frame,
        region: crate::renderers::price_axis_renderer::AxisRect,
    ) {
        if let Some(clip) = crate::ui::price_axis::axis_region_clip(region, frame.size()) {
            draw_axis_primitives(&self.primitives, clip, frame);
        }
    }
    fn clip_bounds(&self, available: Size) -> Option<Rectangle> {
        let size = Size::new(
            self.size.width.min(available.width),
            self.size.height.min(available.height),
        );
        (size.width > 0. && size.height > 0.).then(|| Rectangle::new(Point::ORIGIN, size))
    }
}

pub fn prepare_iced_time_axis(
    snapshot: &TimeAxisSnapshot,
    bounds: AxisBounds,
    ratio: PixelRatio,
    fonts: &AxisFontResolver,
) -> Result<IcedTimeAxis, TimeAxisUiError> {
    let options = &snapshot.renderer_options;
    let mut normal = IcedAxisTextMeasurer::with_font(
        axis_font(fonts, &options.font_family, TimeLabelEmphasis::Normal)?,
        options.font_size,
    )
    .map_err(TimeAxisUiError::Font)?;
    let mut bold = IcedAxisTextMeasurer::with_font(
        axis_font(fonts, &options.font_family, TimeLabelEmphasis::Emphasized)?,
        options.font_size,
    )
    .map_err(TimeAxisUiError::Font)?;
    let measurement =
        measure_time_axis(snapshot, &mut normal, &mut bold).map_err(TimeAxisUiError::Renderer)?;
    let commands =
        prepare_time_axis(&measurement, bounds, ratio).map_err(TimeAxisUiError::Renderer)?;
    IcedTimeAxis::from_commands(&commands, fonts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::text_width_cache::TextMeasurer, renderers::time_axis_renderer::tests::snapshot,
    };

    #[test]
    fn actual_measurement_and_glyph_paths_share_normal_and_bold_fonts() {
        let fonts = AxisFontResolver::default();
        for emphasis in [TimeLabelEmphasis::Normal, TimeLabelEmphasis::Emphasized] {
            let font = axis_font(&fonts, "sans-serif", emphasis).unwrap();
            assert_eq!(
                font.weight,
                if emphasis == TimeLabelEmphasis::Normal {
                    iced::font::Weight::Normal
                } else {
                    iced::font::Weight::Bold
                }
            );
            let mut measurer = IcedAxisTextMeasurer::with_font(font, 12.).unwrap();
            let metrics = measurer.measure_text("Jan 2025");
            assert!(metrics.width > 0. && metrics.width.is_finite());
            assert!(metrics.actual_bounding_box_ascent.is_none());
            let mut larger = IcedAxisTextMeasurer::with_font(font, 24.).unwrap();
            assert!(larger.measure_text("Jan 2025").width > metrics.width);
        }
    }

    #[test]
    fn immutable_clipped_glyphs_remain_logical_at_fractional_scales() {
        let fonts = AxisFontResolver::default();
        let mut previous = None;
        for r in [1., 1.25, 1.5, 2.] {
            let axis = prepare_iced_time_axis(
                &snapshot(),
                AxisBounds {
                    width: 100.,
                    height: 40.,
                },
                PixelRatio {
                    horizontal: r,
                    vertical: r,
                },
                &fonts,
            )
            .unwrap();
            let paths = axis
                .primitives
                .iter()
                .filter_map(|p| match p {
                    AxisPrimitive::Glyph { path, .. } => Some(format!("{:?}", path.raw())),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(!paths.is_empty());
            if let Some(prior) = previous {
                assert_eq!(paths, prior);
            }
            previous = Some(paths);
            assert_eq!(axis.size, Size::new(100., 40.));
            assert_eq!(axis.required_height, 28.);
            assert_eq!(
                axis.clip_bounds(Size::new(8., 20.)),
                Some(Rectangle::new(Point::ORIGIN, Size::new(8., 20.)))
            );
            assert!(axis.clip_bounds(Size::ZERO).is_none());
        }
    }

    #[test]
    fn empty_hidden_zero_bounds_and_font_errors_do_not_keep_glyphs() {
        let fonts = AxisFontResolver::default();
        let mut s = snapshot();
        let bounds = AxisBounds {
            width: 100.,
            height: 40.,
        };
        let ratio = PixelRatio {
            horizontal: 1.,
            vertical: 1.,
        };
        s.ticks.clear();
        let axis = prepare_iced_time_axis(&s, bounds, ratio, &fonts).unwrap();
        assert!(
            axis.primitives
                .iter()
                .all(|p| !matches!(p, AxisPrimitive::Glyph { .. }))
        );
        s.visible = false;
        assert!(
            prepare_iced_time_axis(&s, bounds, ratio, &fonts)
                .unwrap()
                .primitives
                .is_empty()
        );
        s.visible = true;
        assert!(
            prepare_iced_time_axis(&s, AxisBounds::default(), ratio, &fonts)
                .unwrap()
                .primitives
                .is_empty()
        );
        s.renderer_options.font_family = "unregistered".into();
        assert!(matches!(
            prepare_iced_time_axis(&s, bounds, ratio, &fonts),
            Err(TimeAxisUiError::Font(_))
        ));
    }
}
