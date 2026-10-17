//! Source bitmap geometry; preparation converts it to native logical coordinates.
use crate::api::{MarkerSign, PrimitiveRenderer};
use iced::{
    Color, Point, Size,
    widget::canvas::{Frame, Path, Stroke},
};
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MarkerGeometry {
    pub circle: [f64; 3],
    pub arrow: Option<[[f64; 2]; 3]>,
    pub width: f64,
}
pub(crate) fn geometry(x: f64, y: f64, sign: MarkerSign, rx: f64, ry: f64) -> MarkerGeometry {
    let correction = (rx.floor().max(1.) % 2.) / 2.;
    let sign = match sign {
        MarkerSign::Negative => -1.,
        MarkerSign::Neutral => 0.,
        MarkerSign::Positive => 1.,
    };
    MarkerGeometry {
        circle: [
            (x * rx + 0.5).floor() + correction,
            y * ry,
            4. * ry + correction,
        ],
        arrow: (sign != 0.).then_some([
            [(x - 4.7) * rx + correction, (y - 7. * sign) * ry],
            [x * rx + correction, (y - 10.5 * sign) * ry],
            [(x + 4.7) * rx + correction, (y - 7. * sign) * ry],
        ]),
        width: (2. * rx).floor(),
    }
}
#[derive(Debug)]
pub(crate) struct UpDownRenderer {
    pub points: Vec<(MarkerGeometry, Color)>,
    pub ratio: f64,
}
impl PrimitiveRenderer for UpDownRenderer {
    fn draw(&self, frame: &mut Frame) {
        let point = |x: f64, y: f64| Point::new((x / self.ratio) as f32, (y / self.ratio) as f32);
        for (shape, color) in &self.points {
            let [x, y, r] = shape.circle;
            frame.fill(&Path::circle(point(x, y), (r / self.ratio) as f32), *color);
            if let Some([a, b, c]) = shape.arrow {
                let path = Path::new(|p| {
                    p.move_to(point(a[0], a[1]));
                    p.line_to(point(b[0], b[1]));
                    p.line_to(point(c[0], c[1]));
                });
                if shape.width > 0. {
                    frame.stroke(
                        &path,
                        Stroke::default()
                            .with_color(*color)
                            .with_width((shape.width / self.ratio) as f32),
                    );
                }
            }
        }
    }
}
pub(crate) fn in_view(x: f64, y: f64, size: Size, ratio: f64) -> bool {
    let pad = 16. + 1. / ratio;
    x.is_finite()
        && y.is_finite()
        && x >= -pad
        && y >= -pad
        && x <= f64::from(size.width) + pad
        && y <= f64::from(size.height) + pad
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn up_down_markers_geometry_reference() {
        let mut count = 0;
        for (rx, ry) in [
            (1., 1.),
            (1.25, 1.25),
            (1.5, 1.5),
            (2., 2.),
            (1., 1.5),
            (2., 1.25),
        ] {
            for x in [-0.5, 20.25, 200.] {
                for (sign, n) in [
                    (MarkerSign::Negative, -1),
                    (MarkerSign::Neutral, 0),
                    (MarkerSign::Positive, 1),
                ] {
                    let y = 30.5;
                    let g = geometry(x, y, sign, rx, ry);
                    let mut actual = g.circle.to_vec();
                    if let Some(arrow) = g.arrow {
                        actual.extend(arrow.into_iter().flatten());
                        actual.push(g.width);
                    }
                    println!(
                        "UP_DOWN_GEOMETRY {{\"rx\":{rx},\"ry\":{ry},\"x\":{x},\"y\":{y},\"sign\":{n},\"actual\":{actual:?}}}"
                    );
                    count += 1;
                }
            }
        }
        assert_eq!(count, 54);
    }
}
