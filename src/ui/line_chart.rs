//! Iced adapter. Model/frame preparation is separate from immutable Canvas draw.
use crate::{
    model::{
        axis_snapshots::{AxisSnapshotError, AxisSnapshots},
        chart_model::{ChartModel, ChartModelError},
        data_layer::SeriesId,
        ihorz_scale_behavior::HorzScaleBehavior,
        invalidate_mask::InvalidateMask,
        series::line_pane_view::{LinePaneView, LinePreparationError},
    },
    renderers::{
        draw_line::dash_pattern,
        grid_renderer::{BitmapSize, GridDrawCommand, PixelRatio},
        line_renderer::{LineDrawCommands, PathOperation, Point, StyledStroke},
    },
    views::time_scale_animation::TimeScaleAnimationController,
};
use iced::{
    Color, Rectangle, Size,
    widget::canvas::{Frame, LineCap, LineJoin, Path, Stroke},
};
use std::{hash::Hash, time::Instant};

#[derive(Debug)]
pub enum FrameError {
    Axes(AxisSnapshotError),
    Model(ChartModelError),
    Line(LinePreparationError),
    InvalidPixelRatio,
    InvalidColor(String),
    SeriesPaneMismatch { series: SeriesId, pane: usize },
}
pub struct FrameLayout {
    pub pane: usize,
    pub size: Size,
    pub pixel_ratio: PixelRatio,
    pub background: String,
}
#[derive(Clone, Debug)]
pub struct CanvasStroke {
    pub path: Vec<PathOperation>,
    pub color: Color,
    pub width: f32,
}
#[derive(Clone, Debug)]
pub struct CanvasMarker {
    pub center: iced::Point,
    pub radius: f32,
    pub color: Color,
    pub color_key: String,
}
#[derive(Clone, Debug)]
pub struct PlotSnapshot {
    /// Prepared from the same settled viewport; intentionally not drawn yet.
    pub axes: Option<AxisSnapshots>,
    pub size: Size,
    pub background: Color,
    pub grid: Vec<CanvasStroke>,
    pub lines: Vec<CanvasStroke>,
    pub markers: Vec<CanvasMarker>,
}
impl Default for PlotSnapshot {
    fn default() -> Self {
        Self {
            axes: None,
            size: Size::ZERO,
            background: Color::WHITE,
            grid: vec![],
            lines: vec![],
            markers: vec![],
        }
    }
}
fn color(value: &str) -> Result<Color, FrameError> {
    let parsed = value
        .parse::<csscolorparser::Color>()
        .map_err(|_| FrameError::InvalidColor(value.into()))?;
    Ok(Color::from_rgba(parsed.r, parsed.g, parsed.b, parsed.a))
}
fn logical(p: Point, ratio: PixelRatio) -> Point {
    Point {
        x: p.x / f64::from(ratio.horizontal),
        y: p.y / f64::from(ratio.vertical),
    }
}
fn canvas_stroke(stroke: StyledStroke, ratio: PixelRatio) -> Result<CanvasStroke, FrameError> {
    let path =
        super::dashed_path::dash_path(&stroke.path, &stroke.dash_pattern, stroke.dash_offset)
            .into_iter()
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
            .collect();
    Ok(CanvasStroke {
        path,
        color: color(&stroke.color)?,
        width: (stroke.width / f64::from(ratio.vertical)) as f32,
    })
}
impl PlotSnapshot {
    pub fn from_commands(
        layout: &FrameLayout,
        grid: Vec<GridDrawCommand>,
        line: LineDrawCommands,
    ) -> Result<Self, FrameError> {
        let ratio = layout.pixel_ratio;
        if ratio.horizontal != ratio.vertical
            || !ratio.horizontal.is_finite()
            || !ratio.vertical.is_finite()
            || ratio.horizontal <= 0.
            || ratio.vertical <= 0.
        {
            return Err(FrameError::InvalidPixelRatio);
        }
        let mut grid_strokes = vec![];
        for cmd in grid {
            let (paths, color, style, width) = match cmd {
                GridDrawCommand::VerticalLines {
                    x_positions,
                    color,
                    style,
                    width,
                    y_start,
                    y_end,
                } => (
                    x_positions
                        .into_iter()
                        .map(|x| {
                            (
                                Point {
                                    x: f64::from(x),
                                    y: f64::from(y_start),
                                },
                                Point {
                                    x: f64::from(x),
                                    y: f64::from(y_end),
                                },
                            )
                        })
                        .collect::<Vec<_>>(),
                    color,
                    style,
                    width,
                ),
                GridDrawCommand::HorizontalLines {
                    y_positions,
                    color,
                    style,
                    width,
                    x_start,
                    x_end,
                } => (
                    y_positions
                        .into_iter()
                        .map(|y| {
                            (
                                Point {
                                    x: f64::from(x_start),
                                    y: f64::from(y),
                                },
                                Point {
                                    x: f64::from(x_end),
                                    y: f64::from(y),
                                },
                            )
                        })
                        .collect::<Vec<_>>(),
                    color,
                    style,
                    width,
                ),
            };
            // Existing grid strokes start each parallel line at the same phase.
            for (a, b) in paths {
                grid_strokes.push(canvas_stroke(
                    StyledStroke {
                        path: vec![PathOperation::MoveTo(a), PathOperation::LineTo(b)],
                        color: color.clone(),
                        width: f64::from(width),
                        dash_pattern: dash_pattern(style, width)
                            .into_iter()
                            .map(f64::from)
                            .collect(),
                        dash_offset: 0.,
                    },
                    ratio,
                )?);
            }
        }
        let lines = line
            .strokes
            .into_iter()
            .map(|s| canvas_stroke(s, ratio))
            .collect::<Result<_, _>>()?;
        let markers = line
            .markers
            .into_iter()
            .map(|m| {
                let p = logical(m.center, ratio);
                Ok(CanvasMarker {
                    center: iced::Point::new(p.x as f32, p.y as f32),
                    radius: (m.radius / f64::from(ratio.vertical)) as f32,
                    color: color(&m.color)?,
                    color_key: m.color,
                })
            })
            .collect::<Result<_, FrameError>>()?;
        Ok(Self {
            axes: None,
            size: layout.size,
            background: color(&layout.background)?,
            grid: grid_strokes,
            lines,
            markers,
        })
    }
    pub fn draw(&self, frame: &mut Frame) {
        let size = Size::new(
            self.size.width.min(frame.width()),
            self.size.height.min(frame.height()),
        );
        if size.width <= 0. || size.height <= 0. {
            return;
        }
        frame.with_clip(Rectangle::new(iced::Point::ORIGIN, size), |frame| {
            frame.fill_rectangle(iced::Point::ORIGIN, size, self.background);
            for stroke in self.grid.iter().chain(&self.lines) {
                let path = Path::new(|builder| {
                    for op in &stroke.path {
                        let to_iced = |p: Point| iced::Point::new(p.x as f32, p.y as f32);
                        match *op {
                            PathOperation::MoveTo(p) => builder.move_to(to_iced(p)),
                            PathOperation::LineTo(p) => builder.line_to(to_iced(p)),
                            PathOperation::CubicTo {
                                control1,
                                control2,
                                end,
                            } => builder.bezier_curve_to(
                                to_iced(control1),
                                to_iced(control2),
                                to_iced(end),
                            ),
                        }
                    }
                });
                frame.stroke(
                    &path,
                    Stroke::default()
                        .with_color(stroke.color)
                        .with_width(stroke.width)
                        .with_line_cap(LineCap::Butt)
                        .with_line_join(LineJoin::Round),
                );
            }
            // One fill per source style run avoids double alpha blending where
            // adjacent same-color markers overlap.
            for group in self.markers.chunk_by(|a, b| a.color_key == b.color_key) {
                let path = Path::new(|builder| {
                    for marker in group {
                        builder.circle(marker.center, marker.radius);
                    }
                });
                frame.fill(&path, group[0].color);
            }
        });
    }
}

/// Returns a retained mask for optional same-layout replay. New work stays pending.
pub fn prepare_frame<B, M>(
    model: &mut ChartModel<B, M>,
    controller: &mut TimeScaleAnimationController,
    view: &mut LinePaneView,
    id: Option<SeriesId>,
    layout: &FrameLayout,
    now: Instant,
) -> Result<(PlotSnapshot, Option<InvalidateMask>), FrameError>
where
    B: HorzScaleBehavior,
    B::Item: 'static,
    B::InternalItem: 'static,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone + 'static,
{
    view.clear();
    if layout.pixel_ratio.horizontal != layout.pixel_ratio.vertical
        || !layout.pixel_ratio.horizontal.is_finite()
        || !layout.pixel_ratio.vertical.is_finite()
        || layout.pixel_ratio.horizontal <= 0.0
        || layout.pixel_ratio.vertical <= 0.0
    {
        return Err(FrameError::InvalidPixelRatio);
    }
    if !layout.size.width.is_finite()
        || !layout.size.height.is_finite()
        || layout.size.width < 0.0
        || layout.size.height < 0.0
    {
        return Err(FrameError::Model(ChartModelError::InvalidDimensions));
    }
    if model.panes().get(layout.pane).is_none() {
        return Err(FrameError::Model(ChartModelError::InvalidPane(layout.pane)));
    }
    if let Some(id) = id {
        match model.pane_for_series(id) {
            None => return Err(FrameError::Line(LinePreparationError::UnknownSeries(id))),
            Some(pane) if pane != layout.pane => {
                return Err(FrameError::SeriesPaneMismatch {
                    series: id,
                    pane: layout.pane,
                });
            }
            _ => {}
        }
    }
    if model.time_scale().width() != f64::from(layout.size.width) {
        model
            .set_width(f64::from(layout.size.width))
            .map_err(FrameError::Model)?;
    }
    let pane = model
        .panes()
        .get(layout.pane)
        .ok_or(FrameError::Model(ChartModelError::InvalidPane(layout.pane)))?;
    if pane.height() != f64::from(layout.size.height) {
        model
            .set_pane_height(layout.pane, f64::from(layout.size.height))
            .map_err(FrameError::Model)?;
    }
    let mask = controller.draw_frame(model, now);
    let axes = model
        .prepare_axis_snapshots(layout.pane)
        .map_err(FrameError::Axes)?;
    if let Some(id) = id {
        view.refresh(model, id).map_err(FrameError::Line)?;
    } else {
        view.clear();
    }
    let ratio = layout.pixel_ratio;
    let grid = model.panes()[layout.pane]
        .grid()
        .pane_view()
        .renderer()
        .draw_commands(
            BitmapSize {
                width: layout.size.width * ratio.horizontal,
                height: layout.size.height * ratio.vertical,
            },
            ratio,
        );
    let line = view
        .data()
        .map(|data| data.draw_commands(ratio))
        .unwrap_or_default();
    let mut snapshot = PlotSnapshot::from_commands(layout, grid, line)?;
    snapshot.axes = Some(axes);
    Ok((snapshot, mask))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderers::{draw_line::LineStyle, line_renderer::PointMarker};
    #[test]
    fn adapter_converts_once_and_preserves_fractional_phase_at_multiple_ratios() {
        for r in [1., 1.25, 1.5, 2.] {
            let layout = FrameLayout {
                pane: 0,
                size: Size::new(100., 80.),
                pixel_ratio: PixelRatio {
                    horizontal: r,
                    vertical: r,
                },
                background: "#fff".into(),
            };
            let line = LineDrawCommands {
                strokes: vec![StyledStroke {
                    path: vec![
                        PathOperation::MoveTo(Point {
                            x: 10. * f64::from(r),
                            y: 5. * f64::from(r),
                        }),
                        PathOperation::LineTo(Point {
                            x: 30. * f64::from(r),
                            y: 5. * f64::from(r),
                        }),
                    ],
                    color: "#f00".into(),
                    width: 2. * f64::from(r),
                    dash_pattern: vec![2. * f64::from(r), 2. * f64::from(r)],
                    dash_offset: 0.5 * f64::from(r),
                }],
                markers: vec![PointMarker {
                    center: Point {
                        x: 20. * f64::from(r),
                        y: 10. * f64::from(r),
                    },
                    radius: 3. * f64::from(r),
                    color: "#00f".into(),
                }],
            };
            let grid = vec![GridDrawCommand::VerticalLines {
                x_positions: vec![10. * r],
                color: "#000".into(),
                style: LineStyle::Solid,
                width: r,
                y_start: 0.,
                y_end: 80. * r,
            }];
            let snapshot = PlotSnapshot::from_commands(&layout, grid, line).unwrap();
            assert_eq!(snapshot.lines[0].width, 2.);
            assert_eq!(
                snapshot.lines[0].path[1],
                PathOperation::LineTo(Point { x: 11.5, y: 5. })
            );
            assert_eq!(
                snapshot.grid[0].path[0],
                PathOperation::MoveTo(Point { x: 10., y: 0. })
            );
            assert_eq!(snapshot.markers[0].center, iced::Point::new(20., 10.));
            assert_eq!(snapshot.markers[0].radius, 3.);
        }
    }

    #[test]
    fn marker_fill_runs_preserve_style_identity_and_translucent_overlap() {
        let layout = FrameLayout {
            pane: 0,
            size: Size::new(100., 80.),
            pixel_ratio: PixelRatio {
                horizontal: 1.,
                vertical: 1.,
            },
            background: "white".into(),
        };
        let markers = ["rgba(255,0,0,0.5)", "rgba(255,0,0,0.5)", "#ff000080"]
            .map(|color| PointMarker {
                center: Point { x: 10., y: 10. },
                radius: 5.,
                color: color.into(),
            })
            .to_vec();
        let snapshot = PlotSnapshot::from_commands(
            &layout,
            vec![],
            LineDrawCommands {
                strokes: vec![],
                markers,
            },
        )
        .unwrap();
        let lengths = snapshot
            .markers
            .chunk_by(|a, b| a.color_key == b.color_key)
            .map(|group| group.len())
            .collect::<Vec<_>>();
        assert_eq!(lengths, vec![2, 1]);
    }
}
