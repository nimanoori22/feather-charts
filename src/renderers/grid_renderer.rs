//! Backend-neutral, bitmap-snapped grid-line command generation.
//!
//! The Iced adapter will translate these commands into canvas paths. Keeping
//! the pixel-coordinate conversion here preserves Lightweight Charts' grid
//! geometry without coupling the chart model to Fancy Canvas or Iced.

use crate::{
    model::{coordinate::Coordinate, price_scale::PriceMark},
    renderers::draw_line::LineStyle,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BitmapSize {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelRatio {
    pub horizontal: f32,
    pub vertical: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridMark {
    pub coordinate: Coordinate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GridRendererData {
    pub vertical_lines_visible: bool,
    pub vertical_line_color: String,
    pub vertical_line_style: LineStyle,
    pub time_marks: Vec<GridMark>,
    pub horizontal_lines_visible: bool,
    pub horizontal_line_color: String,
    pub horizontal_line_style: LineStyle,
    pub price_marks: Vec<PriceMark>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GridDrawCommand {
    VerticalLines {
        x_positions: Vec<f32>,
        color: String,
        style: LineStyle,
        width: f32,
        y_start: f32,
        y_end: f32,
    },
    HorizontalLines {
        y_positions: Vec<f32>,
        color: String,
        style: LineStyle,
        width: f32,
        x_start: f32,
        x_end: f32,
    },
}

/// Converts cached scale marks into pixel-snapped drawing commands.
#[derive(Clone, Debug, Default)]
pub struct GridRenderer {
    data: Option<GridRendererData>,
}

impl GridRenderer {
    pub fn set_data(&mut self, data: Option<GridRendererData>) {
        self.data = data;
    }

    pub fn data(&self) -> Option<&GridRendererData> {
        self.data.as_ref()
    }

    pub fn draw_commands(&self, size: BitmapSize, pixel_ratio: PixelRatio) -> Vec<GridDrawCommand> {
        let Some(data) = &self.data else {
            return vec![];
        };
        let width = pixel_ratio.horizontal.floor().max(1.0);
        let mut commands = Vec::with_capacity(2);
        if data.vertical_lines_visible {
            let x_positions = data
                .time_marks
                .iter()
                .map(|mark| (mark.coordinate.value() as f32 * pixel_ratio.horizontal).round())
                .filter(|coordinate| coordinate.is_finite())
                .collect::<Vec<_>>();
            if !x_positions.is_empty() {
                commands.push(GridDrawCommand::VerticalLines {
                    x_positions,
                    color: data.vertical_line_color.clone(),
                    style: data.vertical_line_style,
                    width,
                    y_start: -width,
                    y_end: size.height + width,
                });
            }
        }
        if data.horizontal_lines_visible {
            let y_positions = data
                .price_marks
                .iter()
                .map(|mark| (mark.coord.value() as f32 * pixel_ratio.vertical).round())
                .filter(|coordinate| coordinate.is_finite())
                .collect::<Vec<_>>();
            if !y_positions.is_empty() {
                commands.push(GridDrawCommand::HorizontalLines {
                    y_positions,
                    color: data.horizontal_line_color.clone(),
                    style: data.horizontal_line_style,
                    width,
                    x_start: -width,
                    x_end: size.width + width,
                });
            }
        }
        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> GridRendererData {
        GridRendererData {
            vertical_lines_visible: true,
            vertical_line_color: "vertical".into(),
            vertical_line_style: LineStyle::Dashed,
            time_marks: vec![
                GridMark {
                    coordinate: Coordinate::new(10.4),
                },
                GridMark {
                    coordinate: Coordinate::new(20.6),
                },
            ],
            horizontal_lines_visible: true,
            horizontal_line_color: "horizontal".into(),
            horizontal_line_style: LineStyle::Dotted,
            price_marks: vec![PriceMark {
                coord: Coordinate::new(4.25),
                label: String::new(),
                logical: 0.0,
            }],
        }
    }

    #[test]
    fn emits_pixel_snapped_vertical_and_horizontal_commands() {
        let mut renderer = GridRenderer::default();
        renderer.set_data(Some(data()));
        assert_eq!(
            renderer.draw_commands(
                BitmapSize {
                    width: 300.0,
                    height: 200.0,
                },
                PixelRatio {
                    horizontal: 1.5,
                    vertical: 2.0,
                },
            ),
            vec![
                GridDrawCommand::VerticalLines {
                    x_positions: vec![16.0, 31.0],
                    color: "vertical".into(),
                    style: LineStyle::Dashed,
                    width: 1.0,
                    y_start: -1.0,
                    y_end: 201.0,
                },
                GridDrawCommand::HorizontalLines {
                    y_positions: vec![9.0],
                    color: "horizontal".into(),
                    style: LineStyle::Dotted,
                    width: 1.0,
                    x_start: -1.0,
                    x_end: 301.0,
                },
            ]
        );
    }

    #[test]
    fn omits_disabled_or_empty_line_directions() {
        let mut renderer = GridRenderer::default();
        let mut data = data();
        data.vertical_lines_visible = false;
        data.price_marks.clear();
        renderer.set_data(Some(data));
        assert!(
            renderer
                .draw_commands(
                    BitmapSize {
                        width: 1.0,
                        height: 1.0,
                    },
                    PixelRatio {
                        horizontal: 2.0,
                        vertical: 2.0,
                    },
                )
                .is_empty()
        );
    }
}
