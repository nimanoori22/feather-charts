//! Fractional-distance dashing independent of Iced's backend-specific offset.
use crate::renderers::line_renderer::{PathOperation, Point};

fn flatten(a: Point, b: Point, c: Point, d: Point, depth: u8, out: &mut Vec<PathOperation>) {
    // Distance approximation bounds excess arc length in bitmap pixels.
    if depth == 16 || a.distance(b) + b.distance(c) + c.distance(d) - a.distance(d) <= 0.05 {
        out.push(PathOperation::LineTo(d));
        return;
    }
    let ab = a.lerp(b, 0.5);
    let bc = b.lerp(c, 0.5);
    let cd = c.lerp(d, 0.5);
    let abc = ab.lerp(bc, 0.5);
    let bcd = bc.lerp(cd, 0.5);
    let mid = abc.lerp(bcd, 0.5);
    flatten(a, ab, abc, mid, depth + 1, out);
    flatten(mid, bcd, cd, d, depth + 1, out);
}
pub(super) fn dash_path(
    path: &[PathOperation],
    pattern: &[f64],
    offset: f64,
) -> Vec<PathOperation> {
    if pattern.is_empty() {
        return path.to_vec();
    }
    if pattern.iter().any(|v| !v.is_finite() || *v <= 0.0) || !offset.is_finite() {
        return vec![];
    }
    let mut flat = vec![];
    let mut current = Point { x: 0., y: 0. };
    for op in path {
        match *op {
            PathOperation::MoveTo(p) => {
                flat.push(op.clone());
                current = p;
            }
            PathOperation::LineTo(p) => {
                flat.push(op.clone());
                current = p;
            }
            PathOperation::CubicTo {
                control1,
                control2,
                end,
            } => {
                flatten(current, control1, control2, end, 0, &mut flat);
                current = end;
            }
        }
    }
    let pattern = if pattern.len() % 2 == 1 {
        [pattern, pattern].concat()
    } else {
        pattern.to_vec()
    };
    let period: f64 = pattern.iter().sum();
    let phase = offset.rem_euclid(period);
    let mut out = vec![];
    let mut index = 0;
    let mut remaining = 0.;
    let mut drawing = false;
    for op in flat {
        match op {
            PathOperation::MoveTo(p) => {
                current = p;
                index = 0;
                let mut phase = phase;
                while phase >= pattern[index] {
                    phase -= pattern[index];
                    index = (index + 1) % pattern.len();
                }
                remaining = pattern[index] - phase;
                drawing = false;
            }
            PathOperation::LineTo(end) => {
                let length = current.distance(end);
                let start = current;
                let mut travelled = 0.;
                while travelled + 1e-10 < length {
                    let take = remaining.min(length - travelled);
                    let a = start.lerp(end, travelled / length);
                    let b = start.lerp(end, (travelled + take) / length);
                    if index % 2 == 0 {
                        if !drawing {
                            out.push(PathOperation::MoveTo(a));
                        }
                        out.push(PathOperation::LineTo(b));
                        drawing = true;
                    } else {
                        drawing = false;
                    }
                    travelled += take;
                    remaining -= take;
                    if remaining <= 1e-10 {
                        index = (index + 1) % pattern.len();
                        remaining = pattern[index];
                    }
                }
                current = end;
            }
            PathOperation::CubicTo { .. } => unreachable!(),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractional_offset_and_polyline_corners_are_preserved() {
        let path = vec![
            PathOperation::MoveTo(Point { x: 0., y: 0. }),
            PathOperation::LineTo(Point { x: 1., y: 0. }),
            PathOperation::LineTo(Point { x: 1., y: 4. }),
        ];
        let dashed = dash_path(&path, &[2., 2.], 0.5);
        assert_eq!(
            dashed[..3],
            [
                PathOperation::MoveTo(Point { x: 0., y: 0. }),
                PathOperation::LineTo(Point { x: 1., y: 0. }),
                PathOperation::LineTo(Point { x: 1., y: 0.5 })
            ]
        );
        assert_eq!(dashed[3], PathOperation::MoveTo(Point { x: 1., y: 2.5 }));
    }
    #[test]
    fn solid_cubic_remains_cubic_and_dashed_cubic_is_finite() {
        let path = vec![
            PathOperation::MoveTo(Point { x: 0., y: 0. }),
            PathOperation::CubicTo {
                control1: Point { x: 0., y: 10. },
                control2: Point { x: 10., y: 10. },
                end: Point { x: 10., y: 0. },
            },
        ];
        assert_eq!(dash_path(&path, &[], 0.), path);
        let dashed = dash_path(&path, &[2., 2.], 0.25);
        assert!(dashed.len() > 4);
        assert!(dashed.iter().all(|op|matches!(op,PathOperation::MoveTo(p)|PathOperation::LineTo(p) if p.x.is_finite() && p.y.is_finite())));
    }
}
