//! Logical hit distances matching the source's line and point geometry.
use super::{
    draw_line::LineType,
    line_renderer::{LinePoint, Point},
    walk_line::control_points,
};
use crate::model::{internal_hit_test::HitTestPriority, time_data::SeriesItemsIndexesRange};

pub fn distance_to_segment(x: f64, y: f64, a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    if dx == 0. && dy == 0. {
        return (x - a.x).hypot(y - a.y);
    }
    let t = (((x - a.x) * dx + (y - a.y) * dy) / (dx * dx + dy * dy)).clamp(0., 1.);
    (x - a.x - t * dx).hypot(y - a.y - t * dy)
}
fn point(p: &LinePoint) -> Point {
    Point { x: p.x, y: p.y }
}
fn curve_point(a: Point, b: Point, c: Point, d: Point, t: f64) -> Point {
    let u = 1. - t;
    Point {
        x: u * u * u * a.x + 3. * u * u * t * b.x + 3. * u * t * t * c.x + t * t * t * d.x,
        y: u * u * u * a.y + 3. * u * u * t * b.y + 3. * u * t * t * c.y + t * t * t * d.y,
    }
}
#[allow(clippy::too_many_arguments)]
pub fn line_hit(
    items: &[LinePoint],
    range: SeriesItemsIndexesRange,
    x: f64,
    y: f64,
    kind: LineType,
    width: f64,
    point_radius: Option<f64>,
    spacing: f64,
    tolerance: f64,
) -> Option<(f64, HitTestPriority)> {
    if range.from >= range.to || range.to > items.len() {
        return None;
    }
    let radius = (width / 2.).max(point_radius.unwrap_or(0.)) + tolerance;
    let slice = &items[range.from..range.to];
    let mut point_distance = f64::INFINITY;
    if let Some(r) = point_radius {
        let r = r + tolerance;
        let from = slice.partition_point(|p| p.x < x - r);
        let to = slice.partition_point(|p| p.x <= x + r);
        for p in &slice[from..to] {
            let d = (x - p.x).hypot(y - p.y);
            if d <= r {
                point_distance = point_distance.min(d);
            }
        }
    }
    if slice.len() == 1 {
        let p = &slice[0];
        let half = (spacing / 2.).max(radius);
        let d = distance_to_segment(
            x,
            y,
            Point {
                x: p.x - half,
                y: p.y,
            },
            Point {
                x: p.x + half,
                y: p.y,
            },
        );
        if d <= radius {
            point_distance = point_distance.min(d);
        }
        return point_distance
            .is_finite()
            .then_some((point_distance, HitTestPriority::Point));
    }
    let from = range.from + slice.partition_point(|p| p.x < x - radius);
    let to = range.from + slice.partition_point(|p| p.x <= x + radius);
    let mut line_distance = f64::INFINITY;
    for i in (range.from + 1).max(from)..range.to.min(to + 1) {
        let a = point(&items[i - 1]);
        let d = point(&items[i]);
        let distance = match kind {
            LineType::Simple => distance_to_segment(x, y, a, d),
            LineType::WithSteps => {
                let b = Point { x: d.x, y: a.y };
                distance_to_segment(x, y, a, b).min(distance_to_segment(x, y, b, d))
            }
            LineType::Curved => {
                let (b, c) = control_points(items, i - 1, i);
                let min_x = a.x.min(b.x).min(c.x).min(d.x);
                let max_x = a.x.max(b.x).max(c.x).max(d.x);
                if x < min_x - radius || x > max_x + radius {
                    continue;
                }
                let mut previous = a;
                let mut closest = f64::INFINITY;
                for step in 1..=12 {
                    let current = curve_point(a, b, c, d, f64::from(step) / 12.);
                    closest = closest.min(distance_to_segment(x, y, previous, current));
                    previous = current;
                }
                closest
            }
        };
        if distance <= radius {
            line_distance = line_distance.min(distance);
        }
    }
    if point_distance.is_finite() {
        Some((point_distance, HitTestPriority::Point))
    } else {
        line_distance
            .is_finite()
            .then_some((line_distance, HitTestPriority::Line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::time_data::SeriesItemsIndexesRange,
        renderers::{draw_line::LineType, line_renderer::LinePoint},
    };
    fn point(x: f64, y: f64) -> LinePoint {
        LinePoint {
            index: x.into(),
            x,
            y,
            color: String::new(),
        }
    }
    #[test]
    fn source_segments_points_and_single_row_boundaries() {
        let items = vec![point(0., 0.), point(20., 20.)];
        let range = SeriesItemsIndexesRange { from: 0, to: 2 };
        assert!(line_hit(&items, range, 10., 10., LineType::Simple, 2., None, 6., 0.).is_some());
        assert!(line_hit(&items, range, 10., 0., LineType::Simple, 2., None, 6., 0.).is_none());
        assert!(
            line_hit(
                &items,
                range,
                10.,
                0.,
                LineType::WithSteps,
                2.,
                None,
                6.,
                0.
            )
            .is_some()
        );
        let hit = line_hit(
            &items,
            range,
            0.,
            2.,
            LineType::Simple,
            2.,
            Some(3.),
            6.,
            0.,
        )
        .unwrap();
        assert_eq!(hit.1, HitTestPriority::Point);
        let one = SeriesItemsIndexesRange { from: 0, to: 1 };
        assert!(line_hit(&items, one, 3., 1., LineType::Simple, 2., None, 6., 0.).is_some());
        assert!(line_hit(&items, one, 3., 1.01, LineType::Simple, 2., None, 6., 0.).is_none());
    }
}
