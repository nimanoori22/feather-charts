use super::price_axis::AxisFontResolver;
use crate::{
    api::{PrimitiveRenderer, TextWatermarkOptions},
    renderers::text_watermark_renderer::layout,
};
use iced::{
    Color, Point, Size,
    advanced::{graphics::geometry::path::lyon_path::math::Transform, text},
    widget::canvas::{Frame, Path},
};
#[derive(Debug)]
pub(crate) struct PreparedTextWatermark {
    paths: Vec<(Path, Color)>,
}
impl PrimitiveRenderer for PreparedTextWatermark {
    fn draw(&self, frame: &mut Frame) {
        // The plot adapter already clips; a nested draft reorders WGPU meshes behind its background.
        for (path, color) in &self.paths {
            frame.fill(path, *color);
        }
    }
}
pub(crate) fn prepare(
    options: &TextWatermarkOptions,
    colors: &[Color],
    fonts: &AxisFontResolver,
    size: Size,
) -> PreparedTextWatermark {
    let widths = options
        .lines
        .iter()
        .map(|l| {
            if l.text.is_empty() {
                0.
            } else {
                fonts
                    .text_width(&l.text, l.font, l.font_size)
                    .expect("validated watermark font size")
            }
        })
        .collect::<Vec<_>>();
    let positions = layout(options, size, &widths);
    let alignment = match options.horizontal_alignment {
        iced::alignment::Horizontal::Left => text::Alignment::Left,
        iced::alignment::Horizontal::Center => text::Alignment::Center,
        iced::alignment::Horizontal::Right => text::Alignment::Right,
    };
    let mut paths = vec![];
    for ((line, color), p) in options.lines.iter().zip(colors).zip(positions) {
        if line.text.is_empty() || p.zoom <= 0. {
            continue;
        }
        let transform =
            Transform::new(p.zoom as f32, 0., 0., p.zoom as f32, p.x as f32, p.y as f32);
        paths.extend(
            fonts
                .glyph_paths_top(
                    &line.text,
                    line.font,
                    line.font_size,
                    alignment,
                    *color,
                    Point::ORIGIN,
                )
                .into_iter()
                .map(|(path, color)| (path.transform(&transform), color)),
        );
    }
    PreparedTextWatermark { paths }
}
