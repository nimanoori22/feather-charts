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
use std::{cell::RefCell, collections::VecDeque};

struct CachedAxisText {
    content: String,
    font: Font,
    size: f32,
    alignment: text::Alignment,
    color: Color,
    paths: Vec<(Path, Color)>,
}

#[derive(Debug)]
pub enum PriceAxisUiError {
    Renderer(PriceAxisRenderError),
    UnsupportedFontFamily(String),
    Color(FrameError),
}

/// Named families in Iced 0.14 require static strings. Registration avoids
/// leaking arbitrarily changing layout strings to satisfy that requirement.
/// Bounded label paths are stored at the origin and translated during preparation.
#[derive(Default)]
pub struct AxisFontResolver {
    named: Vec<(&'static str, Font)>,
    glyphs: RefCell<VecDeque<CachedAxisText>>,
    widths: RefCell<VecDeque<(String, Font, f32, f64)>>,
}
impl AxisFontResolver {
    pub fn register(&mut self, name: &'static str, font: Font) {
        self.glyphs.get_mut().clear();
        self.widths.get_mut().clear();
        if let Some(entry) = self.named.iter_mut().find(|(family, _)| *family == name) {
            entry.1 = font;
        } else {
            self.named.push((name, font));
        }
    }
    pub(crate) fn text_width(
        &self,
        content: &str,
        font: Font,
        size: f32,
    ) -> Result<f64, PriceAxisUiError> {
        let mut widths = self.widths.borrow_mut();
        if let Some((_, _, _, width)) = widths
            .iter()
            .find(|(s, f, z, _)| s == content && *f == font && *z == size)
        {
            return Ok(*width);
        }
        let width = f64::from(
            IcedAxisTextMeasurer::with_font(font, size)?
                .measure_text(content)
                .width,
        );
        if widths.len() == 256 {
            widths.pop_front();
        }
        widths.push_back((content.into(), font, size, width));
        Ok(width)
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
    pub(crate) fn glyph_paths(
        &self,
        content: &str,
        font: Font,
        size: f32,
        alignment: text::Alignment,
        color: Color,
        position: Point,
    ) -> Vec<(Path, Color)> {
        let mut cache = self.glyphs.borrow_mut();
        let index = cache.iter().position(|entry| {
            entry.content == content
                && entry.font == font
                && entry.size == size
                && entry.alignment == alignment
                && entry.color == color
        });
        let index = match index {
            Some(index) => index,
            None => {
                let text = canvas::Text {
                    content: content.into(),
                    position: Point::ORIGIN,
                    font,
                    size: size.into(),
                    align_x: alignment,
                    align_y: alignment::Vertical::Center,
                    line_height: LineHeight::Relative(1.2),
                    shaping: Shaping::Advanced,
                    max_width: f32::INFINITY,
                    color,
                };
                let mut paths = vec![];
                text.draw_with(|path, color| paths.push((path, color)));
                if cache.len() == 256 {
                    cache.pop_front();
                }
                cache.push_back(CachedAxisText {
                    content: content.into(),
                    font,
                    size,
                    alignment,
                    color,
                    paths,
                });
                cache.len() - 1
            }
        };
        let translation =
            iced::advanced::graphics::geometry::path::lyon_path::math::Transform::translation(
                position.x, position.y,
            );
        cache[index]
            .paths
            .iter()
            .map(|(path, color)| (path.transform(&translation), *color))
            .collect()
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
                PriceAxisCommand::RoundedRectangle {
                    rect,
                    radii,
                    color: value,
                } => {
                    primitives.push(AxisPrimitive::Glyph {
                        path: Path::rounded_rectangle(
                            Point::new(rect.origin.x as f32, rect.origin.y as f32),
                            Size::new(rect.size.width as f32, rect.size.height as f32),
                            iced::border::Radius {
                                top_left: radii[0],
                                top_right: radii[1],
                                bottom_right: radii[2],
                                bottom_left: radii[3],
                            },
                        ),
                        color: color(value).map_err(PriceAxisUiError::Color)?,
                    });
                }
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
                    let paths = fonts.glyph_paths(
                        label,
                        fonts.resolve(font_family)?,
                        *font_size,
                        match align {
                            AxisTextAlignment::Left => text::Alignment::Left,
                            AxisTextAlignment::Right => text::Alignment::Right,
                        },
                        color(value).map_err(PriceAxisUiError::Color)?,
                        Point::new(position.x as f32, position.y as f32),
                    );
                    // Cached Canvas text escapes clipping on the 0.14 TinySkia
                    // backend. Prepared paths preserve clip and paint order.
                    primitives.extend(
                        paths
                            .into_iter()
                            .map(|(path, color)| AxisPrimitive::Glyph { path, color }),
                    );
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
    fn cached_text_preserves_glyphs_and_reuses_positions_without_mixing_styles() {
        let fonts = AxisFontResolver::default();
        let text = canvas::Text {
            content: "−123.45%".into(),
            position: Point::ORIGIN,
            color: Color::WHITE,
            font: Font::DEFAULT,
            size: 12.0.into(),
            align_x: text::Alignment::Right,
            align_y: alignment::Vertical::Center,
            line_height: LineHeight::Relative(1.2),
            shaping: Shaping::Advanced,
            max_width: f32::INFINITY,
        };
        let mut expected = vec![];
        text.draw_with(|path, color| expected.push((path, color)));
        for position in [Point::ORIGIN, Point::new(30.5, 20.25)] {
            let cached = fonts.glyph_paths(
                &text.content,
                text.font,
                12.,
                text.align_x,
                text.color,
                position,
            );
            let transform =
                iced::advanced::graphics::geometry::path::lyon_path::math::Transform::translation(
                    position.x, position.y,
                );
            assert_eq!(cached.len(), expected.len());
            for ((path, color), (original, expected_color)) in cached.iter().zip(&expected) {
                assert_eq!(*color, *expected_color);
                assert_eq!(
                    path.raw().iter().collect::<Vec<_>>(),
                    original
                        .transform(&transform)
                        .raw()
                        .iter()
                        .collect::<Vec<_>>()
                );
            }
        }
        assert_eq!(fonts.glyphs.borrow().len(), 1);
        for (font, size, alignment, color) in [
            (Font::MONOSPACE, 12., text.align_x, text.color),
            (text.font, 24., text.align_x, text.color),
            (text.font, 12., text::Alignment::Center, text.color),
            (text.font, 12., text.align_x, Color::BLACK),
        ] {
            fonts.glyph_paths(&text.content, font, size, alignment, color, Point::ORIGIN);
        }
        assert_eq!(fonts.glyphs.borrow().len(), 5);
        let mut fonts = fonts;
        fonts.register("New", Font::DEFAULT);
        assert!(fonts.glyphs.borrow().is_empty());
    }
    #[test]
    fn text_path_cache_evicts_old_labels_and_keeps_prepared_geometry_owned() {
        let fonts = AxisFontResolver::default();
        let retained = fonts.glyph_paths(
            "123",
            Font::DEFAULT,
            12.,
            text::Alignment::Left,
            Color::WHITE,
            Point::ORIGIN,
        );
        for i in 0..256 {
            fonts.glyphs.borrow_mut().push_back(CachedAxisText {
                content: format!("old-{i}"),
                font: Font::DEFAULT,
                size: 12.,
                alignment: text::Alignment::Left,
                color: Color::WHITE,
                paths: vec![],
            });
        }
        fonts.glyphs.borrow_mut().pop_front();
        fonts.glyph_paths(
            "new",
            Font::DEFAULT,
            12.,
            text::Alignment::Left,
            Color::WHITE,
            Point::ORIGIN,
        );
        assert_eq!(fonts.glyphs.borrow().len(), 256);
        assert!(
            fonts
                .glyphs
                .borrow()
                .iter()
                .all(|entry| entry.content != "old-0")
        );
        assert!(!retained.is_empty());
        assert!(
            retained
                .iter()
                .all(|(path, _)| path.raw().iter().next().is_some())
        );
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
