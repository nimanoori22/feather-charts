//! Backend text measurement and immutable, clipped price-axis drawing.
use crate::{
    model::{
        axis_snapshots::{AxisSnapshots, PriceAxisSnapshot},
        layout_options::Background,
        text_width_cache::{TextMeasurer, TextMetrics},
    },
    renderers::{
        grid_renderer::PixelRatio,
        iprice_axis_view_renderer::PriceAxisViewRendererOptions,
        price_axis_renderer::{
            AxisBounds, AxisTextAlignment, PreparedPriceAxis, PriceAxisCommand,
            PriceAxisRenderError, measure_price_axis, prepare_price_axis,
        },
    },
    ui::line_chart::{FrameError, color},
};
use iced::{
    Color, Font, Point, Rectangle, Size,
    advanced::{
        graphics::text::Paragraph,
        text::{self, LineHeight, Paragraph as _, Shaping, Wrapping},
    },
    alignment,
    widget::canvas::{self, Frame, Path, gradient},
};

#[derive(Debug)]
pub enum PriceAxisUiError {
    Renderer(PriceAxisRenderError),
    UnsupportedFontFamily(String),
    Color(FrameError),
}

/// Named families in Iced 0.14 require static strings. Registration avoids
/// leaking arbitrarily changing layout strings to satisfy that requirement.
#[derive(Default)]
pub struct AxisFontResolver {
    named: Vec<(&'static str, Font)>,
}
impl AxisFontResolver {
    pub fn register(&mut self, name: &'static str, font: Font) {
        if let Some(entry) = self.named.iter_mut().find(|(family, _)| *family == name) {
            entry.1 = font;
        } else {
            self.named.push((name, font));
        }
    }
    pub fn resolve(&self, families: &str) -> Result<Font, PriceAxisUiError> {
        for family in families
            .split(',')
            .map(|name| name.trim().trim_matches(['\'', '"']))
        {
            if let Some((_, font)) = self
                .named
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(family))
            {
                return Ok(*font);
            }
            let generic = match family.to_ascii_lowercase().as_str() {
                "sans-serif" | "system-ui" => Some(iced::font::Family::SansSerif),
                "serif" => Some(iced::font::Family::Serif),
                "monospace" => Some(iced::font::Family::Monospace),
                "cursive" => Some(iced::font::Family::Cursive),
                "fantasy" => Some(iced::font::Family::Fantasy),
                _ => None,
            };
            if let Some(family) = generic {
                return Ok(Font {
                    family,
                    ..Font::DEFAULT
                });
            }
        }
        Err(PriceAxisUiError::UnsupportedFontFamily(families.into()))
    }
}

pub struct IcedAxisTextMeasurer {
    font: Font,
    size: f32,
}
impl IcedAxisTextMeasurer {
    pub fn with_font(font: Font, size: f32) -> Result<Self, PriceAxisUiError> {
        if !size.is_finite() || size <= 0. {
            return Err(PriceAxisUiError::Renderer(
                PriceAxisRenderError::InvalidSnapshot,
            ));
        }
        Ok(Self { font, size })
    }
    pub fn new(
        options: &PriceAxisViewRendererOptions,
        fonts: &AxisFontResolver,
    ) -> Result<Self, PriceAxisUiError> {
        if !options.font_size.is_finite() || options.font_size <= 0. {
            return Err(PriceAxisUiError::Renderer(
                PriceAxisRenderError::InvalidSnapshot,
            ));
        }
        Ok(Self {
            font: fonts.resolve(&options.font_family)?,
            size: options.font_size,
        })
    }
}
impl TextMeasurer for IcedAxisTextMeasurer {
    fn measure_text(&mut self, content: &str) -> TextMetrics {
        let paragraph = Paragraph::with_text(text::Text {
            content,
            bounds: Size::INFINITE,
            size: self.size.into(),
            line_height: LineHeight::Relative(1.2),
            font: self.font,
            align_x: text::Alignment::Left,
            align_y: alignment::Vertical::Center,
            shaping: Shaping::Advanced,
            wrapping: Wrapping::None,
        });
        TextMetrics {
            width: paragraph.min_width(),
            // Paragraph height is a line box, not glyph ink ascent/descent.
            actual_bounding_box_ascent: None,
            actual_bounding_box_descent: None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum AxisPrimitive {
    Fill {
        origin: Point,
        size: Size,
        fill: canvas::Fill,
    },
    Glyph {
        path: Path,
        color: Color,
    },
}
#[derive(Clone, Debug, Default)]
pub struct IcedPriceAxis {
    pub required_width: f64,
    pub size: Size,
    primitives: Vec<AxisPrimitive>,
}
impl IcedPriceAxis {
    pub fn from_commands(
        commands: &PreparedPriceAxis,
        fonts: &AxisFontResolver,
    ) -> Result<Self, PriceAxisUiError> {
        let size = Size::new(commands.bounds.width as f32, commands.bounds.height as f32);
        let mut primitives = vec![];
        for command in &commands.commands {
            match command {
                PriceAxisCommand::Background(background) => {
                    let fill = match background {
                        Background::Solid { color: value } => {
                            color(value).map_err(PriceAxisUiError::Color)?.into()
                        }
                        Background::VerticalGradient {
                            top_color,
                            bottom_color,
                        } => gradient::Linear::new(Point::ORIGIN, Point::new(0., size.height))
                            .add_stop(0., color(top_color).map_err(PriceAxisUiError::Color)?)
                            .add_stop(1., color(bottom_color).map_err(PriceAxisUiError::Color)?)
                            .into(),
                    };
                    primitives.push(AxisPrimitive::Fill {
                        origin: Point::ORIGIN,
                        size,
                        fill,
                    });
                }
                PriceAxisCommand::Rectangle { rect, color: value } => {
                    primitives.push(AxisPrimitive::Fill {
                        origin: Point::new(rect.origin.x as f32, rect.origin.y as f32),
                        size: Size::new(rect.size.width as f32, rect.size.height as f32),
                        fill: color(value).map_err(PriceAxisUiError::Color)?.into(),
                    })
                }
                PriceAxisCommand::Text {
                    label,
                    position,
                    alignment: align,
                    font_size,
                    font_family,
                    color: value,
                } => {
                    let text = canvas::Text {
                        content: label.clone(),
                        position: Point::new(position.x as f32, position.y as f32),
                        color: color(value).map_err(PriceAxisUiError::Color)?,
                        size: (*font_size).into(),
                        line_height: LineHeight::Relative(1.2),
                        font: fonts.resolve(font_family)?,
                        align_x: match align {
                            AxisTextAlignment::Left => text::Alignment::Left,
                            AxisTextAlignment::Right => text::Alignment::Right,
                        },
                        align_y: alignment::Vertical::Center,
                        shaping: Shaping::Advanced,
                        max_width: f32::INFINITY,
                    };
                    // Cached Canvas text escapes clipping on the 0.14 TinySkia
                    // backend. Prepared paths preserve clip and paint order.
                    text.draw_with(|path, color| {
                        primitives.push(AxisPrimitive::Glyph { path, color })
                    });
                }
            }
        }
        Ok(Self {
            required_width: commands.required_width,
            size,
            primitives,
        })
    }
    pub fn draw(&self, frame: &mut Frame) {
        let Some(clip) = self.clip_bounds(frame.size()) else {
            return;
        };
        draw_axis_primitives(&self.primitives, clip, frame);
    }
    pub fn draw_at(
        &self,
        frame: &mut Frame,
        region: crate::renderers::price_axis_renderer::AxisRect,
    ) {
        if let Some(clip) = axis_region_clip(region, frame.size()) {
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

pub(crate) fn draw_axis_primitives(
    primitives: &[AxisPrimitive],
    clip: Rectangle,
    frame: &mut Frame,
) {
    frame.with_clip(clip, |frame| {
        // Frame::draft does not inherit its parent's transform in Iced 0.14.
        frame.translate(iced::Vector::new(clip.x, clip.y));
        for primitive in primitives {
            match primitive {
                AxisPrimitive::Fill { origin, size, fill } => {
                    frame.fill_rectangle(*origin, *size, *fill)
                }
                AxisPrimitive::Glyph { path, color } => frame.fill(path, *color),
            }
        }
    });
}

pub(crate) fn axis_region_clip(
    region: crate::renderers::price_axis_renderer::AxisRect,
    available: Size,
) -> Option<Rectangle> {
    let x = region.origin.x as f32;
    let y = region.origin.y as f32;
    let size = Size::new(
        (region.size.width as f32).min((available.width - x).max(0.)),
        (region.size.height as f32).min((available.height - y).max(0.)),
    );
    (size.width > 0. && size.height > 0.).then(|| Rectangle::new(Point::new(x, y), size))
}

#[derive(Clone, Debug, Default)]
pub struct IcedPriceAxes {
    pub left: IcedPriceAxis,
    pub right: IcedPriceAxis,
}
pub fn prepare_iced_price_axis(
    snapshot: &PriceAxisSnapshot,
    bounds: AxisBounds,
    ratio: PixelRatio,
    fonts: &AxisFontResolver,
) -> Result<IcedPriceAxis, PriceAxisUiError> {
    let mut measurer = IcedAxisTextMeasurer::new(&snapshot.renderer_options, fonts)?;
    let measurement =
        measure_price_axis(snapshot, &mut measurer).map_err(PriceAxisUiError::Renderer)?;
    let commands =
        prepare_price_axis(&measurement, bounds, ratio).map_err(PriceAxisUiError::Renderer)?;
    IcedPriceAxis::from_commands(&commands, fonts)
}

/// Uses the already-settled frame's snapshots, never another model read/drain.
pub fn prepare_iced_price_axes(
    snapshots: &AxisSnapshots,
    left: AxisBounds,
    right: AxisBounds,
    ratio: PixelRatio,
    fonts: &AxisFontResolver,
) -> Result<IcedPriceAxes, PriceAxisUiError> {
    Ok(IcedPriceAxes {
        left: prepare_iced_price_axis(&snapshots.left, left, ratio, fonts)?,
        right: prepare_iced_price_axis(&snapshots.right, right, ratio, fonts)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::axis_snapshots::PriceAxisSide, renderers::price_axis_renderer::tests::snapshot,
    };

    fn ratio(value: f32) -> PixelRatio {
        PixelRatio {
            horizontal: value,
            vertical: value,
        }
    }
    #[test]
    fn real_iced_measurement_uses_font_size_and_no_fake_ink_metrics() {
        let mut snapshot = snapshot(PriceAxisSide::Right);
        let fonts = AxisFontResolver::default();
        let mut small = IcedAxisTextMeasurer::new(&snapshot.renderer_options, &fonts).unwrap();
        let first = small.measure_text("−123.45%");
        assert!(first.width.is_finite() && first.width > 0.);
        assert!(
            first.actual_bounding_box_ascent.is_none()
                && first.actual_bounding_box_descent.is_none()
        );
        snapshot.renderer_options.font_size = 24.;
        let mut large = IcedAxisTextMeasurer::new(&snapshot.renderer_options, &fonts).unwrap();
        assert!(large.measure_text("−123.45%").width > first.width);
        assert_eq!(small.measure_text("−123.45%"), first);
    }

    #[test]
    fn named_font_registration_and_css_generic_fallbacks_do_not_leak_names() {
        let mut fonts = AxisFontResolver::default();
        assert!(matches!(
            fonts.resolve("unregistered"),
            Err(PriceAxisUiError::UnsupportedFontFamily(_))
        ));
        assert_eq!(
            fonts.resolve("missing, monospace").unwrap(),
            Font::MONOSPACE
        );
        fonts.register("Demo", Font::with_name("DejaVu Sans"));
        assert_eq!(
            fonts.resolve("'Demo', sans-serif").unwrap(),
            Font::with_name("DejaVu Sans")
        );
        fonts.register("Demo", Font::MONOSPACE);
        assert_eq!(fonts.resolve("Demo").unwrap(), Font::MONOSPACE);
    }

    #[test]
    fn adapter_prepares_paths_once_and_keeps_text_logical_at_fractional_ratios() {
        let snapshot = snapshot(PriceAxisSide::Right);
        let fonts = AxisFontResolver::default();
        let bounds = AxisBounds {
            width: 100.,
            height: 80.,
        };
        let mut previous = None;
        for r in [1., 1.25, 1.5, 2.] {
            let axis = prepare_iced_price_axis(&snapshot, bounds, ratio(r), &fonts).unwrap();
            let glyphs = axis
                .primitives
                .iter()
                .filter_map(|primitive| match primitive {
                    AxisPrimitive::Glyph { path, .. } => Some(format!("{:?}", path.raw())),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(!glyphs.is_empty());
            if let Some(prior) = previous {
                assert_eq!(glyphs, prior);
            }
            previous = Some(glyphs);
            assert_eq!(axis.size, Size::new(100., 80.));
        }
    }

    #[test]
    fn adapter_clips_allocated_bounds_without_cropping_glyph_paths() {
        let snapshot = snapshot(PriceAxisSide::Right);
        let axis = prepare_iced_price_axis(
            &snapshot,
            AxisBounds {
                width: 8.,
                height: 80.,
            },
            ratio(1.),
            &AxisFontResolver::default(),
        )
        .unwrap();
        assert_eq!(
            axis.clip_bounds(Size::new(6., 50.)),
            Some(Rectangle::new(Point::ORIGIN, Size::new(6., 50.)))
        );
        assert!(axis.clip_bounds(Size::ZERO).is_none());
        assert!(
            axis.primitives
                .iter()
                .any(|item| matches!(item, AxisPrimitive::Glyph { .. }))
        );
        assert!(axis.required_width > 8.); // requested size never overwrites allocation
    }

    #[test]
    fn adapter_preserves_gradient_background_and_empty_hidden_output() {
        let mut snapshot = snapshot(PriceAxisSide::Left);
        snapshot.background = Background::VerticalGradient {
            top_color: "#fff".into(),
            bottom_color: "#000".into(),
        };
        let bounds = AxisBounds {
            width: 100.,
            height: 80.,
        };
        let fonts = AxisFontResolver::default();
        let axis = prepare_iced_price_axis(&snapshot, bounds, ratio(1.), &fonts).unwrap();
        assert!(matches!(
            axis.primitives[0],
            AxisPrimitive::Fill {
                fill: canvas::Fill {
                    style: canvas::Style::Gradient(_),
                    ..
                },
                ..
            }
        ));
        snapshot.visible = false;
        let hidden = prepare_iced_price_axis(&snapshot, bounds, ratio(1.), &fonts).unwrap();
        assert!(hidden.primitives.is_empty());
        assert_eq!(hidden.required_width, 0.);
        snapshot.visible = true;
        snapshot.marks.clear();
        let empty = prepare_iced_price_axis(&snapshot, bounds, ratio(1.), &fonts).unwrap();
        assert!(
            empty
                .primitives
                .iter()
                .all(|item| !matches!(item, AxisPrimitive::Glyph { .. }))
        );
        assert_eq!(empty.required_width, 56.);
    }
}
