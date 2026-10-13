//! Owned bitmap fill commands share the source line walker, with independent styles.
use super::{
    draw_line::LineType,
    grid_renderer::PixelRatio,
    line_renderer::{LineDrawCommands, LineRendererData, PathOperation, Point},
};

#[derive(Clone, Debug, PartialEq)]
pub struct VerticalGradient {
    pub top: f64,
    pub bottom: f64,
    pub stops: Vec<(f64, String)>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FilledPath {
    pub path: Vec<PathOperation>,
    pub gradient: VerticalGradient,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AreaRendererData {
    pub line: LineRendererData,
    pub fill_colors: Vec<(String, String)>,
    pub fill_type: LineType,
    pub height: f64,
    pub top: f64,
    pub inverted: bool,
}
impl AreaRendererData {
    pub fn draw_commands(&self, ratio: PixelRatio) -> (Vec<FilledPath>, LineDrawCommands) {
        (self.fill_commands(ratio), self.line.draw_commands(ratio))
    }
    pub fn fill_commands(&self, ratio: PixelRatio) -> Vec<FilledPath> {
        let range = self.line.visible_range;
        if !ratio.horizontal.is_finite()
            || !ratio.vertical.is_finite()
            || ratio.horizontal <= 0.
            || ratio.vertical <= 0.
            || self.fill_colors.len() != self.line.items.len()
            || range.from >= range.to
            || range.to > self.line.items.len()
            || !self.top.is_finite()
            || !self.height.is_finite()
        {
            return vec![];
        }
        let items = self
            .line
            .items
            .iter()
            .map(|p| super::line_renderer::LinePoint {
                x: p.x * f64::from(ratio.horizontal),
                y: p.y * f64::from(ratio.vertical),
                ..p.clone()
            })
            .collect::<Vec<_>>();
        if items.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
            return vec![];
        }
        let base = if self.inverted {
            0.
        } else {
            self.height * f64::from(ratio.vertical)
        };
        let top = if self.top > 0. {
            self.top * f64::from(ratio.vertical)
        } else {
            self.top
        };
        super::walk_line::walk_segments(
            &items,
            range,
            self.fill_type,
            self.line.bar_width * f64::from(ratio.horizontal),
            1.,
            &[],
            |a, b| self.fill_colors[a] == self.fill_colors[b],
        )
        .into_iter()
        .map(|(i, stroke)| {
            let mut path = stroke.path;
            let first = match path.first().unwrap() {
                PathOperation::MoveTo(p) => *p,
                _ => unreachable!(),
            };
            let last = match path.last().unwrap() {
                PathOperation::MoveTo(p) | PathOperation::LineTo(p) => *p,
                PathOperation::CubicTo { end, .. } => *end,
            };
            path.push(PathOperation::LineTo(Point { x: last.x, y: base }));
            path.push(PathOperation::LineTo(Point {
                x: first.x,
                y: base,
            }));
            FilledPath {
                path,
                gradient: VerticalGradient {
                    top,
                    bottom: self.height * f64::from(ratio.vertical),
                    stops: vec![
                        (0., self.fill_colors[i].0.clone()),
                        (1., self.fill_colors[i].1.clone()),
                    ],
                },
            }
        })
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::time_data::SeriesItemsIndexesRange,
        renderers::{
            draw_line::{LineStyle, LineWidth},
            line_renderer::LinePoint,
        },
    };
    #[test]
    fn fill_changes_independently_and_closes_to_inverted_edge() {
        let mut data = AreaRendererData {
            line: LineRendererData {
                items: (0..3)
                    .map(|i| LinePoint {
                        index: (i as f64).into(),
                        x: i as f64 * 10.,
                        y: 10.,
                        color: "black".into(),
                    })
                    .collect(),
                visible_range: SeriesItemsIndexesRange { from: 0, to: 3 },
                bar_width: 6.,
                line_type: Some(LineType::Simple),
                line_width: LineWidth::Two,
                line_style: LineStyle::Solid,
                point_markers_radius: None,
            },
            fill_colors: vec![
                ("red".into(), "white".into()),
                ("blue".into(), "white".into()),
                ("blue".into(), "white".into()),
            ],
            fill_type: LineType::Simple,
            height: 80.,
            top: 10.,
            inverted: true,
        };
        let (fills, lines) = data.draw_commands(PixelRatio {
            horizontal: 1.5,
            vertical: 2.,
        });
        assert_eq!(lines.strokes.len(), 1);
        assert_eq!(fills.len(), 2);
        assert_eq!(fills[0].gradient.top, 20.);
        assert_eq!(
            fills[0].path.last(),
            Some(&PathOperation::LineTo(Point { x: 0., y: 0. }))
        );
        data.line.line_type = None;
        assert_eq!(
            data.draw_commands(PixelRatio {
                horizontal: 1.,
                vertical: 1.
            })
            .0
            .len(),
            2
        );
    }
}
