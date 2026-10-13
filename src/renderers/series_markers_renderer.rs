//! Backend-independent marker placement output and bitmap shape commands.
use super::line_renderer::Point;
use crate::model::{
    data_layer::SeriesId,
    series_markers::{
        MarkerCollectionId, MarkerHit, MarkerShape, MarkerZOrder, ceiled_odd, shape_size,
    },
    time_data::TimePointIndex,
};

#[derive(Clone, Debug, PartialEq)]
pub struct MarkerText {
    pub content: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedMarker {
    pub index: TimePointIndex,
    pub center: Point,
    pub size: f64,
    pub shape: MarkerShape,
    pub color: String,
    pub hit: MarkerHit,
    pub text: Option<MarkerText>,
}
#[derive(Clone, Debug)]
pub struct PreparedMarkerCollection {
    pub series_layer: usize,
    pub series: SeriesId,
    pub id: MarkerCollectionId,
    pub z_order: MarkerZOrder,
    pub items: Vec<PreparedMarker>,
    pub font_size: f64,
    pub font_family: String,
}
#[derive(Clone, Debug, PartialEq)]
pub enum MarkerShapeCommand {
    Circle { center: Point, radius: f64 },
    Square { origin: Point, size: f64 },
    Polygon(Vec<Point>),
}
impl PreparedMarker {
    pub fn shape_command(&self, ratio: f64) -> Option<MarkerShapeCommand> {
        if self.size == 0. {
            return None;
        }
        let c = Point {
            x: (self.center.x * ratio + 0.5).floor() + ratio.floor().max(1.) % 2. / 2.,
            y: self.center.y * ratio,
        };
        let size = shape_size(self.shape, self.size);
        let half = (size - 1.) * ratio / 2.;
        Some(match self.shape {
            MarkerShape::Circle => MarkerShapeCommand::Circle {
                center: c,
                radius: half,
            },
            MarkerShape::Square => MarkerShapeCommand::Square {
                origin: Point {
                    x: c.x - half,
                    y: c.y - half,
                },
                size: size * ratio,
            },
            MarkerShape::ArrowUp | MarkerShape::ArrowDown => {
                let base = (ceiled_odd(self.size / 2.) - 1.) * ratio / 2.;
                let sign = if self.shape == MarkerShape::ArrowUp {
                    -1.
                } else {
                    1.
                };
                MarkerShapeCommand::Polygon(vec![
                    Point {
                        x: c.x - half,
                        y: c.y,
                    },
                    Point {
                        x: c.x,
                        y: c.y + sign * half,
                    },
                    Point {
                        x: c.x + half,
                        y: c.y,
                    },
                    Point {
                        x: c.x + base,
                        y: c.y,
                    },
                    Point {
                        x: c.x + base,
                        y: c.y - sign * half,
                    },
                    Point {
                        x: c.x - base,
                        y: c.y - sign * half,
                    },
                    Point {
                        x: c.x - base,
                        y: c.y,
                    },
                ])
            }
        })
    }
    pub fn hit_test(&self, x: f64, y: f64) -> bool {
        if self.text.as_ref().is_some_and(|t| {
            x >= t.x && x <= t.x + t.width && y >= t.y - t.height / 2. && y <= t.y + t.height / 2.
        }) {
            return true;
        }
        if self.size == 0. {
            return false;
        }
        let size = shape_size(self.shape, self.size);
        let half = (size - 1.) / 2.;
        let dx = x - self.center.x;
        let dy = y - self.center.y;
        match self.shape {
            MarkerShape::Circle => dx.hypot(dy) <= 2. + size / 2.,
            MarkerShape::Square => {
                dx >= -half && dx <= -half + size && dy >= -half && dy <= -half + size
            }
            MarkerShape::ArrowUp | MarkerShape::ArrowDown => {
                // Source renderer passes the opposite orientation to its arrow hit helper.
                let up = self.shape == MarkerShape::ArrowDown;
                let base = (ceiled_odd(self.size / 2.) - 1.) / 2.;
                let (top, bottom) = if up { (0., half) } else { (-half, 0.) };
                if dx >= -base - 2. && dx <= base + 2. && dy >= top - 2. && dy <= bottom + 2. {
                    return true;
                }
                let (top, bottom) = if up {
                    (-half - 3., 0.)
                } else {
                    (0., half + 3.)
                };
                dx.abs() <= half + 3. && dy >= top && dy <= bottom && dy.abs() + 3. >= dx.abs() / 2.
            }
        }
    }
}
