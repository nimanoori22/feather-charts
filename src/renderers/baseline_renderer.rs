use super::{
    area_renderer::{FilledPath, VerticalGradient},
    draw_line::LineType,
    grid_renderer::PixelRatio,
    line_renderer::{LineRendererData, PointMarker, StyledStroke},
};
#[derive(Clone, Debug, PartialEq)]
pub struct GradientStroke {
    pub stroke: StyledStroke,
    pub gradient: VerticalGradient,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GradientMarkers {
    pub markers: Vec<PointMarker>,
    pub gradient: VerticalGradient,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BaselineDrawCommands {
    pub fills: Vec<FilledPath>,
    pub strokes: Vec<GradientStroke>,
    pub markers: Vec<GradientMarkers>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BaselineRendererData {
    pub line: LineRendererData,
    pub fill_colors: Vec<[String; 4]>,
    pub stroke_colors: Vec<[String; 2]>,
    pub fill_type: LineType,
    pub base: f64,
    pub top: Option<f64>,
    pub bottom: Option<f64>,
    pub height: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaselineRenderError {
    InvalidData,
    InvalidPixelRatio,
}
impl BaselineRendererData {
    pub fn draw_commands(
        &self,
        ratio: PixelRatio,
    ) -> Result<BaselineDrawCommands, BaselineRenderError> {
        let h = f64::from(ratio.horizontal);
        let v = f64::from(ratio.vertical);
        let range = self.line.visible_range;
        if !h.is_finite() || !v.is_finite() || h <= 0. || v <= 0. {
            return Err(BaselineRenderError::InvalidPixelRatio);
        }
        if self.fill_colors.len() != self.line.items.len()
            || self.stroke_colors.len() != self.line.items.len()
            || range.from > range.to
            || range.to > self.line.items.len()
            || !self.base.is_finite()
            || !self.height.is_finite()
            || self.top.is_some_and(|x| !x.is_finite())
            || self.bottom.is_some_and(|x| !x.is_finite())
            || !self.line.bar_width.is_finite()
            || self.line.bar_width <= 0.
        {
            return Err(BaselineRenderError::InvalidData);
        }
        if range.from == range.to {
            return Ok(BaselineDrawCommands::default());
        }
        let items = self
            .line
            .items
            .iter()
            .map(|p| super::line_renderer::LinePoint {
                x: p.x * h,
                y: p.y * v,
                ..p.clone()
            })
            .collect::<Vec<_>>();
        if items.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
            return Err(BaselineRenderError::InvalidData);
        }
        let raw_top = self.top.unwrap_or(0.);
        let raw_bottom = self.bottom.unwrap_or(self.height * v);
        let multiplier = if self.base != 0. || raw_top > 0. {
            v
        } else {
            1.
        };
        let top = raw_top * multiplier;
        let bottom = if raw_bottom == self.height * v {
            raw_bottom
        } else {
            raw_bottom * multiplier
        };
        let base = self.base * multiplier;
        if [top, bottom, base, self.base * v]
            .iter()
            .any(|x| !x.is_finite())
        {
            return Err(BaselineRenderError::InvalidData);
        }
        // A zero-length Canvas gradient has no painted pixels; keep its finite stops.
        let offset = if bottom == top {
            if base > top { 1. } else { 0. }
        } else {
            ((base - top) / (bottom - top)).clamp(0., 1.)
        };
        let gradient = |colors: &[String; 4]| VerticalGradient {
            top,
            bottom,
            stops: vec![
                (0., colors[0].clone()),
                (offset, colors[1].clone()),
                (offset, colors[2].clone()),
                (1., colors[3].clone()),
            ],
        };
        let line_gradient = |i: usize| {
            let colors = &self.stroke_colors[i];
            gradient(&[
                colors[0].clone(),
                colors[0].clone(),
                colors[1].clone(),
                colors[1].clone(),
            ])
        };
        let fills = super::walk_line::walk_segments(
            &items,
            range,
            self.fill_type,
            self.line.bar_width * h,
            1.,
            &[],
            |a, b| self.fill_colors[a] == self.fill_colors[b],
        )
        .into_iter()
        .map(|(i, stroke)| {
            use super::line_renderer::{PathOperation, Point};
            let mut path = stroke.path;
            let first = match path.first().unwrap() {
                PathOperation::MoveTo(p) => *p,
                _ => unreachable!(),
            };
            let last = match path.last().unwrap() {
                PathOperation::MoveTo(p) | PathOperation::LineTo(p) => *p,
                PathOperation::CubicTo { end, .. } => *end,
            };
            path.push(PathOperation::LineTo(Point {
                x: last.x,
                y: self.base * v,
            }));
            path.push(PathOperation::LineTo(Point {
                x: first.x,
                y: self.base * v,
            }));
            FilledPath {
                path,
                gradient: gradient(&self.fill_colors[i]),
            }
        })
        .collect();
        let mut commands = BaselineDrawCommands {
            fills,
            ..Default::default()
        };
        if let Some(kind) = self.line.line_type {
            let width = f64::from(self.line.line_width.pixels()) * v;
            let pattern = super::draw_line::dash_pattern(self.line.line_style, width as f32)
                .into_iter()
                .map(f64::from)
                .collect::<Vec<_>>();
            commands.strokes = super::walk_line::walk_segments(
                &items,
                range,
                kind,
                self.line.bar_width * h,
                width,
                &pattern,
                |a, b| self.stroke_colors[a] == self.stroke_colors[b],
            )
            .into_iter()
            .map(|(i, stroke)| GradientStroke {
                stroke,
                gradient: line_gradient(i),
            })
            .collect();
        }
        if let Some(radius) = self
            .line
            .point_markers_radius
            .filter(|r| r.is_finite() && *r > 0.)
        {
            let correction = (h.floor().max(1.) % 2.) / 2.;
            for i in (range.from..range.to).rev() {
                let gradient = line_gradient(i);
                if commands
                    .markers
                    .last()
                    .is_none_or(|g| g.gradient != gradient)
                {
                    commands.markers.push(GradientMarkers {
                        markers: vec![],
                        gradient,
                    });
                }
                commands
                    .markers
                    .last_mut()
                    .unwrap()
                    .markers
                    .push(PointMarker {
                        center: super::line_renderer::Point {
                            x: (items[i].x + 0.5).floor() + correction,
                            y: items[i].y,
                        },
                        radius: radius * v + correction,
                        color: items[i].color.clone(),
                    });
            }
        }
        Ok(commands)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::time_data::ValueRange,
        renderers::{
            draw_line::{LineStyle, LineWidth},
            line_renderer::{LinePoint, PathOperation, Point},
        },
    };
    pub(super) fn fixture() -> BaselineRendererData {
        BaselineRendererData {
            line: LineRendererData {
                items: vec![
                    LinePoint {
                        index: 0.0.into(),
                        x: 10.,
                        y: 20.,
                        color: "black".into(),
                    },
                    LinePoint {
                        index: 1.0.into(),
                        x: 30.,
                        y: 60.,
                        color: "black".into(),
                    },
                ],
                visible_range: ValueRange { from: 0, to: 2 },
                bar_width: 6.,
                line_type: Some(LineType::Simple),
                line_width: LineWidth::Three,
                line_style: LineStyle::Dashed,
                point_markers_radius: Some(3.),
            },
            fill_colors: vec![["green".into(), "white".into(), "white".into(), "red".into()]; 2],
            stroke_colors: vec![["green".into(), "red".into()]; 2],
            fill_type: LineType::Simple,
            base: 40.,
            top: None,
            bottom: None,
            height: 80.,
        }
    }
    #[test]
    fn crossings_share_ordered_gradient_stops_and_close_at_base() {
        let d = fixture();
        let c = d
            .draw_commands(PixelRatio {
                horizontal: 1.5,
                vertical: 2.,
            })
            .unwrap();
        assert_eq!(c.fills.len(), 1);
        assert_eq!(c.strokes.len(), 1);
        assert_eq!(c.markers.len(), 1);
        assert_eq!(
            c.fills[0]
                .gradient
                .stops
                .iter()
                .map(|(o, _)| *o)
                .collect::<Vec<_>>(),
            vec![0., 0.5, 0.5, 1.]
        );
        assert_eq!(
            c.fills[0].path.last(),
            Some(&PathOperation::LineTo(Point { x: 15., y: 80. }))
        );
        assert_eq!(c.markers[0].markers[0].center.x, 45.5);
        let mut d = d;
        d.line.line_type = None;
        let c = d
            .draw_commands(PixelRatio {
                horizontal: 1.,
                vertical: 1.,
            })
            .unwrap();
        assert_eq!(c.fills.len(), 1);
        assert!(c.strokes.is_empty());
        assert_eq!(c.markers.len(), 1);
    }
    #[test]
    fn bad_inputs_empty_and_degenerate_bounds_are_safe() {
        let mut d = fixture();
        let r = PixelRatio {
            horizontal: 1.,
            vertical: 1.,
        };
        d.line.visible_range.to = 0;
        assert_eq!(d.draw_commands(r).unwrap(), BaselineDrawCommands::default());
        d.line.visible_range.to = 2;
        d.top = Some(20.);
        d.bottom = Some(20.);
        assert!(d.draw_commands(r).is_ok());
        d.base = f64::NAN;
        assert_eq!(d.draw_commands(r), Err(BaselineRenderError::InvalidData));
    }
}
