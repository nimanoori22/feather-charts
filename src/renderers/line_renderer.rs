//! Logical-coordinate line data and backend-neutral bitmap drawing commands.
use crate::{
    model::time_data::{SeriesItemsIndexesRange, TimePointIndex},
    renderers::{
        draw_line::{LineStyle, LineType, LineWidth, dash_pattern},
        grid_renderer::PixelRatio,
    },
};

#[derive(Clone, Debug, PartialEq)]
pub struct LinePoint {
    pub index: TimePointIndex,
    pub x: f64,
    pub y: f64,
    pub color: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    pub fn distance(self, other: Self) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }
    pub fn lerp(self, other: Self, t: f64) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PathOperation {
    MoveTo(Point),
    LineTo(Point),
    CubicTo {
        control1: Point,
        control2: Point,
        end: Point,
    },
}

/// All coordinates and distances are bitmap pixels. Caps are butt, joins round.
#[derive(Clone, Debug, PartialEq)]
pub struct StyledStroke {
    pub path: Vec<PathOperation>,
    pub color: String,
    pub width: f64,
    pub dash_pattern: Vec<f64>,
    pub dash_offset: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PointMarker {
    pub center: Point,
    pub radius: f64,
    pub color: String,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LineDrawCommands {
    pub strokes: Vec<StyledStroke>,
    pub markers: Vec<PointMarker>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LineRendererData {
    /// Full logical data, including neighbors needed by cubic controls.
    pub items: Vec<LinePoint>,
    pub visible_range: SeriesItemsIndexesRange,
    pub bar_width: f64,
    pub line_type: Option<LineType>,
    pub line_width: LineWidth,
    pub line_style: LineStyle,
    pub point_markers_radius: Option<f64>,
}
impl LineRendererData {
    pub fn draw_commands(&self, ratio: PixelRatio) -> LineDrawCommands {
        if !ratio.horizontal.is_finite()
            || !ratio.vertical.is_finite()
            || ratio.horizontal <= 0.0
            || ratio.vertical <= 0.0
            || self.visible_range.from >= self.visible_range.to
            || self.visible_range.to > self.items.len()
        {
            return LineDrawCommands::default();
        }
        let items: Vec<_> = self
            .items
            .iter()
            .map(|p| LinePoint {
                x: p.x * f64::from(ratio.horizontal),
                y: p.y * f64::from(ratio.vertical),
                ..p.clone()
            })
            .collect();
        if items.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
            return LineDrawCommands::default();
        }
        let width = f64::from(self.line_width.pixels()) * f64::from(ratio.vertical);
        let pattern = dash_pattern(self.line_style, width as f32)
            .into_iter()
            .map(f64::from)
            .collect::<Vec<_>>();
        let mut commands = LineDrawCommands::default();
        if let Some(kind) = self.line_type {
            commands.strokes = super::walk_line::walk_line(
                &items,
                self.visible_range,
                kind,
                self.bar_width * f64::from(ratio.horizontal),
                width,
                &pattern,
            );
        }
        if let Some(radius) = self
            .point_markers_radius
            .filter(|r| r.is_finite() && *r > 0.0)
        {
            let correction = (f64::from(ratio.horizontal).floor().max(1.0) % 2.0) / 2.0;
            commands.markers = items[self.visible_range.from..self.visible_range.to]
                .iter()
                .rev()
                .map(|p| PointMarker {
                    center: Point {
                        x: p.x.round() + correction,
                        y: p.y,
                    },
                    radius: radius * f64::from(ratio.vertical) + correction,
                    color: p.color.clone(),
                })
                .collect();
        }
        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn markers_are_independent_reverse_order_and_bitmap_snapped() {
        let data = LineRendererData {
            items: vec![
                LinePoint {
                    index: 0.0.into(),
                    x: 10.2,
                    y: 4.0,
                    color: "red".into(),
                },
                LinePoint {
                    index: 1.0.into(),
                    x: 20.2,
                    y: 8.0,
                    color: "blue".into(),
                },
            ],
            visible_range: SeriesItemsIndexesRange { from: 0, to: 2 },
            bar_width: 6.0,
            line_type: None,
            line_width: LineWidth::Two,
            line_style: LineStyle::Solid,
            point_markers_radius: Some(3.0),
        };
        let commands = data.draw_commands(PixelRatio {
            horizontal: 1.5,
            vertical: 2.0,
        });
        assert!(commands.strokes.is_empty());
        assert_eq!(
            commands.markers[0],
            PointMarker {
                center: Point { x: 30.5, y: 16.0 },
                radius: 6.5,
                color: "blue".into()
            }
        );
        assert_eq!(commands.markers[1].center.x, 15.5);
        let mut hidden = data;
        hidden.point_markers_radius = None;
        assert_eq!(
            hidden.draw_commands(PixelRatio {
                horizontal: 1.0,
                vertical: 1.0
            }),
            LineDrawCommands::default()
        );
    }
}
