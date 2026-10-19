//! Ordered gradient paint adaptation shared by Area and Baseline.
use super::*;
#[derive(Clone, Debug)]
pub struct CanvasGradientStroke {
    pub stroke: CanvasStroke,
    pub gradient: iced::widget::canvas::gradient::Linear,
}
#[derive(Clone, Debug)]
pub struct CanvasGradientMarkers {
    pub markers: Vec<CanvasMarker>,
    pub gradient: iced::widget::canvas::gradient::Linear,
}
fn logical_path(path: Vec<PathOperation>, ratio: PixelRatio) -> Vec<PathOperation> {
    path.into_iter()
        .map(|op| match op {
            PathOperation::MoveTo(p) => PathOperation::MoveTo(logical(p, ratio)),
            PathOperation::LineTo(p) => PathOperation::LineTo(logical(p, ratio)),
            PathOperation::CubicTo {
                control1,
                control2,
                end,
            } => PathOperation::CubicTo {
                control1: logical(control1, ratio),
                control2: logical(control2, ratio),
                end: logical(end, ratio),
            },
        })
        .collect()
}
#[cfg(test)]
fn canvas_gradient(
    g: crate::renderers::area_renderer::VerticalGradient,
    ratio: PixelRatio,
) -> Result<iced::widget::canvas::gradient::Linear, FrameError> {
    canvas_gradient_with_colors(g, ratio, &Default::default())
}
pub(super) fn canvas_gradient_with_colors(
    g: crate::renderers::area_renderer::VerticalGradient,
    ratio: PixelRatio,
    colors: &crate::model::colors::ColorParser,
) -> Result<iced::widget::canvas::gradient::Linear, FrameError> {
    let top = (g.top / f64::from(ratio.vertical)) as f32;
    let bottom = (g.bottom / f64::from(ratio.vertical)) as f32;
    if !top.is_finite() || !bottom.is_finite() || g.stops.len() > 8 {
        return Err(FrameError::BaselineGeometry(
            BaselineRenderError::InvalidData,
        ));
    }
    let mut gradient = iced::widget::canvas::gradient::Linear::new(
        iced::Point::new(0., top),
        iced::Point::new(0., bottom),
    );
    if top == bottom {
        gradient.start = iced::Point::ORIGIN;
        gradient.end = iced::Point::new(0., 1.);
        gradient.stops[0] = Some(iced::gradient::ColorStop {
            offset: 0.,
            color: Color::TRANSPARENT,
        });
        gradient.stops[1] = Some(iced::gradient::ColorStop {
            offset: 1.,
            color: Color::TRANSPARENT,
        });
    } else {
        for (i, (offset, c)) in g.stops.into_iter().enumerate() {
            if !offset.is_finite() || !(0. ..=1.).contains(&offset) {
                return Err(FrameError::BaselineGeometry(
                    BaselineRenderError::InvalidData,
                ));
            }
            gradient.stops[i] = Some(iced::gradient::ColorStop {
                offset: offset as f32,
                color: color_with_parser(&c, colors)?,
            });
        }
    }
    Ok(gradient)
}
#[cfg(test)]
fn canvas_fill(
    fill: crate::renderers::area_renderer::FilledPath,
    ratio: PixelRatio,
) -> Result<CanvasFill, FrameError> {
    canvas_fill_with_colors(fill, ratio, &Default::default())
}
pub(super) fn canvas_fill_with_colors(
    fill: crate::renderers::area_renderer::FilledPath,
    ratio: PixelRatio,
    colors: &crate::model::colors::ColorParser,
) -> Result<CanvasFill, FrameError> {
    let path = logical_path(fill.path, ratio);
    if path.iter().any(|op| {
        let finite = |p: Point| (p.x as f32).is_finite() && (p.y as f32).is_finite();
        match *op {
            PathOperation::MoveTo(p) | PathOperation::LineTo(p) => !finite(p),
            PathOperation::CubicTo {
                control1,
                control2,
                end,
            } => !finite(control1) || !finite(control2) || !finite(end),
        }
    }) {
        return Err(FrameError::BaselineGeometry(
            BaselineRenderError::InvalidData,
        ));
    }
    Ok(CanvasFill {
        path,
        gradient: canvas_gradient_with_colors(fill.gradient, ratio, colors)?,
    })
}
impl PlotSnapshot {
    pub(super) fn draw_gradient_strokes(
        &self,
        frame: &mut Frame,
        strokes: &[CanvasGradientStroke],
    ) {
        for s in strokes {
            frame.stroke(
                &canvas_path(&s.stroke.path, false),
                Stroke {
                    style: iced::widget::canvas::Style::Gradient(s.gradient.into()),
                    width: s.stroke.width,
                    line_cap: LineCap::Butt,
                    line_join: LineJoin::Round,
                    ..Stroke::default()
                },
            );
        }
    }
    pub(super) fn draw_gradient_markers(
        &self,
        frame: &mut Frame,
        groups: &[CanvasGradientMarkers],
    ) {
        for g in groups {
            let path = Path::new(|b| {
                for m in &g.markers {
                    b.circle(m.center, m.radius);
                }
            });
            frame.fill(&path, g.gradient);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderers::area_renderer::{FilledPath, VerticalGradient};
    fn ratio() -> PixelRatio {
        PixelRatio {
            horizontal: 1.5,
            vertical: 1.5,
        }
    }
    #[test]
    fn duplicate_stops_survive_conversion_and_degenerate_paint_is_transparent() {
        let gradient = canvas_gradient(
            VerticalGradient {
                top: 0.,
                bottom: 150.,
                stops: vec![
                    (0., "green".into()),
                    (0.5, "green".into()),
                    (0.5, "red".into()),
                    (1., "red".into()),
                ],
            },
            ratio(),
        )
        .unwrap();
        assert_eq!(gradient.end.y, 100.);
        assert_eq!(
            gradient.stops[1].unwrap().offset,
            gradient.stops[2].unwrap().offset
        );
        assert_ne!(
            gradient.stops[1].unwrap().color,
            gradient.stops[2].unwrap().color
        );
        let gradient = canvas_gradient(
            VerticalGradient {
                top: 1e20,
                bottom: 1e20,
                stops: vec![(0., "red".into()), (1., "green".into())],
            },
            ratio(),
        )
        .unwrap();
        assert_ne!(gradient.start, gradient.end);
        assert!(
            gradient
                .stops
                .into_iter()
                .flatten()
                .all(|s| s.color == Color::TRANSPARENT)
        );
    }
    #[test]
    fn nonfinite_backend_geometry_and_gradient_offsets_are_rejected() {
        assert!(
            canvas_gradient(
                VerticalGradient {
                    top: 0.,
                    bottom: 100.,
                    stops: vec![(f64::NAN, "red".into())]
                },
                ratio()
            )
            .is_err()
        );
        assert!(
            canvas_fill(
                FilledPath {
                    path: vec![PathOperation::MoveTo(Point { x: f64::MAX, y: 0. })],
                    gradient: VerticalGradient {
                        top: 0.,
                        bottom: 100.,
                        stops: vec![(0., "red".into()), (1., "green".into())]
                    }
                },
                ratio()
            )
            .is_err()
        );
    }
}
