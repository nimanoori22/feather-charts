//! Ordered styled paths matching Lightweight Charts' walk-line behavior.
use super::{
    draw_line::LineType,
    line_renderer::{LinePoint, PathOperation, Point, StyledStroke},
};
use crate::model::time_data::SeriesItemsIndexesRange;

fn point(item: &LinePoint) -> Point {
    Point {
        x: item.x,
        y: item.y,
    }
}
pub fn control_points(items: &[LinePoint], from: usize, to: usize) -> (Point, Point) {
    let a = point(&items[from]);
    let b = point(&items[to]);
    let before = point(&items[from.saturating_sub(1)]);
    let after = point(&items[(to + 1).min(items.len() - 1)]);
    (
        Point {
            x: a.x + (b.x - before.x) / 6.0,
            y: a.y + (b.y - before.y) / 6.0,
        },
        Point {
            x: b.x - (after.x - a.x) / 6.0,
            y: b.y - (after.y - a.y) / 6.0,
        },
    )
}
pub fn walk_line(
    items: &[LinePoint],
    range: SeriesItemsIndexesRange,
    kind: LineType,
    bar_width: f64,
    width: f64,
    pattern: &[f64],
) -> Vec<StyledStroke> {
    if range.from >= range.to || range.to > items.len() {
        return vec![];
    }
    let first = &items[range.from];
    let make_stroke = |color: &str, offset| StyledStroke {
        path: vec![],
        color: color.into(),
        width,
        dash_pattern: pattern.to_vec(),
        dash_offset: offset,
    };
    let mut current = make_stroke(&first.color, 0.0);
    if range.to - range.from == 1 {
        current.path = vec![
            PathOperation::MoveTo(Point {
                x: first.x - bar_width / 2.0,
                y: first.y,
            }),
            PathOperation::LineTo(Point {
                x: first.x + bar_width / 2.0,
                y: first.y,
            }),
        ];
        return vec![current];
    }
    current.path.push(PathOperation::MoveTo(point(first)));
    let mut result = vec![];
    let mut distance = 0.0;
    let period: f64 = pattern.iter().sum();
    let mut style_first = range.from;
    for i in range.from + 1..range.to {
        let prev = point(&items[i - 1]);
        let end = point(&items[i]);
        match kind {
            LineType::Simple => {
                current.path.push(PathOperation::LineTo(end));
                distance += prev.distance(end);
            }
            LineType::Curved => {
                let (a, b) = control_points(items, i - 1, i);
                current.path.push(PathOperation::CubicTo {
                    control1: a,
                    control2: b,
                    end,
                });
                distance +=
                    (prev.distance(end) + prev.distance(a) + a.distance(b) + b.distance(end)) / 2.0;
            }
            LineType::WithSteps => {
                let corner = Point {
                    x: end.x,
                    y: prev.y,
                };
                current.path.push(PathOperation::LineTo(corner));
                distance += (end.x - prev.x).abs();
                if items[i].color != current.color {
                    result.push(current);
                    if period > 0.0 {
                        distance %= period;
                    }
                    current =
                        make_stroke(&items[i].color, if period > 0.0 { distance } else { 0.0 });
                    // Canvas lineTo on a new path starts it at the corner.
                    current.path.push(PathOperation::MoveTo(corner));
                    style_first = i;
                }
                current.path.push(PathOperation::LineTo(end));
                distance += (end.y - prev.y).abs();
            }
        }
        if kind != LineType::WithSteps && items[i].color != current.color {
            result.push(current);
            if period > 0.0 {
                distance %= period;
            }
            current = make_stroke(&items[i].color, if period > 0.0 { distance } else { 0.0 });
            current.path.push(PathOperation::MoveTo(end));
            style_first = i;
        }
    }
    if style_first != range.to - 1 || kind == LineType::WithSteps {
        result.push(current);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn items() -> Vec<LinePoint> {
        [(0., 0., "red"), (3., 4., "blue"), (6., 4., "blue")]
            .into_iter()
            .enumerate()
            .map(|(i, (x, y, color))| LinePoint {
                index: (i as f64).into(),
                x,
                y,
                color: color.into(),
            })
            .collect()
    }
    #[test]
    fn simple_changes_style_at_endpoint_with_fractional_phase() {
        let strokes = walk_line(
            &items(),
            SeriesItemsIndexesRange { from: 0, to: 3 },
            LineType::Simple,
            6.,
            2.,
            &[1.5, 1.5],
        );
        assert_eq!(strokes.len(), 2);
        assert_eq!(strokes[0].color, "red");
        assert_eq!(
            strokes[0].path[1],
            PathOperation::LineTo(Point { x: 3., y: 4. })
        );
        assert_eq!(strokes[1].dash_offset, 2.);
        assert_eq!(strokes[1].color, "blue");
    }
    #[test]
    fn steps_change_at_corner_before_vertical_leg() {
        let strokes = walk_line(
            &items(),
            SeriesItemsIndexesRange { from: 0, to: 2 },
            LineType::WithSteps,
            6.,
            2.,
            &[2., 2.],
        );
        assert_eq!(
            strokes[0].path[1],
            PathOperation::LineTo(Point { x: 3., y: 0. })
        );
        assert_eq!(
            strokes[1].path,
            vec![
                PathOperation::MoveTo(Point { x: 3., y: 0. }),
                PathOperation::LineTo(Point { x: 3., y: 4. })
            ]
        );
        assert_eq!(strokes[1].dash_offset, 3.);
    }
    #[test]
    fn curves_use_full_neighbors_and_source_distance_estimate() {
        let items = items();
        let (a, b) = control_points(&items, 0, 1);
        assert_eq!(a, Point { x: 0.5, y: 4. / 6. });
        assert_eq!(
            b,
            Point {
                x: 2.,
                y: 4. - 4. / 6.
            }
        );
        let strokes = walk_line(
            &items,
            SeriesItemsIndexesRange { from: 0, to: 3 },
            LineType::Curved,
            6.,
            1.,
            &[10., 10.],
        );
        let expected = (5.
            + Point { x: 0., y: 0. }.distance(a)
            + a.distance(b)
            + b.distance(Point { x: 3., y: 4. }))
            / 2.;
        assert!((strokes[1].dash_offset - expected).abs() < 1e-12);
    }
    #[test]
    fn single_item_and_empty_ranges() {
        let strokes = walk_line(
            &items(),
            SeriesItemsIndexesRange { from: 1, to: 2 },
            LineType::Curved,
            6.,
            1.,
            &[],
        );
        assert_eq!(
            strokes[0].path,
            vec![
                PathOperation::MoveTo(Point { x: 0., y: 4. }),
                PathOperation::LineTo(Point { x: 6., y: 4. })
            ]
        );
        assert!(
            walk_line(
                &[],
                SeriesItemsIndexesRange { from: 0, to: 0 },
                LineType::Simple,
                6.,
                1.,
                &[]
            )
            .is_empty()
        );
    }
}
