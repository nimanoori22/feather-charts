//! Iced adapter. Model/frame preparation is separate from immutable Canvas draw.
use crate::{
    model::{
        axis_snapshots::{AxisSnapshotError, AxisSnapshots},
        chart_model::{ChartModel, ChartModelError},
        data_layer::SeriesId,
        ihorz_scale_behavior::HorzScaleBehavior,
        invalidate_mask::InvalidateMask,
        series::{
            candlesticks_pane_view::{CandlestickPreparationError, CandlesticksPaneView},
            histogram_pane_view::{HistogramPaneView, HistogramPreparationError},
            line_pane_view::{LinePaneView, LinePreparationError},
        },
    },
    renderers::{
        candlesticks_renderer::{CandleRectangle, CandlestickRenderError},
        draw_line::dash_pattern,
        grid_renderer::{BitmapSize, GridDrawCommand, PixelRatio},
        histogram_renderer::HistogramRenderError,
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
    Candlesticks(CandlestickPreparationError),
    CandleGeometry(CandlestickRenderError),
    Histogram(HistogramPreparationError),
    HistogramGeometry(HistogramRenderError),
    InvalidPixelRatio,
    InvalidColor(String),
    SeriesPaneMismatch { series: SeriesId, pane: usize },
}
pub(crate) enum PaneView<'a> {
    Line(&'a mut LinePaneView),
    Candlesticks(&'a mut CandlesticksPaneView),
    Histogram(&'a mut HistogramPaneView),
}
impl PaneView<'_> {
    pub(crate) fn clear(&mut self) {
        match self {
            Self::Line(v) => v.clear(),
            Self::Candlesticks(v) => v.clear(),
            Self::Histogram(v) => v.clear(),
        }
    }
    pub(crate) fn unknown_series(&self, id: SeriesId) -> FrameError {
        match self {
            Self::Histogram(_) => {
                FrameError::Histogram(HistogramPreparationError::UnknownSeries(id))
            }
            Self::Line(_) => FrameError::Line(LinePreparationError::UnknownSeries(id)),
            Self::Candlesticks(_) => {
                FrameError::Candlesticks(CandlestickPreparationError::UnknownSeries(id))
            }
        }
    }
}
#[derive(Clone, Debug)]
pub struct CanvasRectangle {
    pub origin: iced::Point,
    pub size: Size,
    pub color: Color,
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
    /// Prepared from the same settled viewport as the grid and line.
    pub axes: Option<AxisSnapshots>,
    pub size: Size,
    pub background: Color,
    pub background_gradient: Option<(Color, Color)>,
    pub grid: Vec<CanvasStroke>,
    pub lines: Vec<CanvasStroke>,
    pub markers: Vec<CanvasMarker>,
    pub candles: Vec<CanvasRectangle>,
    pub crosshair: Vec<CanvasStroke>,
    pub crosshair_markers: Vec<CanvasMarker>,
}
impl Default for PlotSnapshot {
    fn default() -> Self {
        Self {
            axes: None,
            size: Size::ZERO,
            background: Color::WHITE,
            background_gradient: None,
            grid: vec![],
            lines: vec![],
            markers: vec![],
            candles: vec![],
            crosshair: vec![],
            crosshair_markers: vec![],
        }
    }
}
pub(crate) fn color(value: &str) -> Result<Color, FrameError> {
    let parsed = value
        .parse::<csscolorparser::Color>()
        .map_err(|_| FrameError::InvalidColor(value.into()))?;
    Ok(Color::from_rgba(parsed.r, parsed.g, parsed.b, parsed.a))
}
fn canvas_rectangle(
    rect: CandleRectangle,
    ratio: PixelRatio,
) -> Result<CanvasRectangle, FrameError> {
    let x = rect.x / f64::from(ratio.horizontal);
    let y = rect.y / f64::from(ratio.vertical);
    let width = rect.width / f64::from(ratio.horizontal);
    let height = rect.height / f64::from(ratio.vertical);
    // Canvas2D fills negative extents in the opposite direction.
    let rectangle = CanvasRectangle {
        origin: iced::Point::new(x.min(x + width) as f32, y.min(y + height) as f32),
        size: Size::new(width.abs() as f32, height.abs() as f32),
        color: color(&rect.color)?,
    };
    let coordinates = [
        rectangle.origin.x,
        rectangle.origin.y,
        rectangle.size.width,
        rectangle.size.height,
    ];
    if coordinates.iter().any(|v| !v.is_finite()) {
        return Err(FrameError::CandleGeometry(
            CandlestickRenderError::InvalidData,
        ));
    }
    Ok(rectangle)
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
            background_gradient: None,
            grid: grid_strokes,
            lines,
            markers,
            candles: vec![],
            crosshair: vec![],
            crosshair_markers: vec![],
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
            self.draw_contents(frame, size);
        });
    }
    pub fn draw_at(
        &self,
        frame: &mut Frame,
        region: crate::renderers::price_axis_renderer::AxisRect,
    ) {
        if let Some(clip) = crate::ui::price_axis::axis_region_clip(region, frame.size()) {
            frame.with_clip(clip, |frame| {
                frame.translate(iced::Vector::new(clip.x, clip.y));
                self.draw_contents(frame, clip.size());
            });
        }
    }
    fn draw_contents(&self, frame: &mut Frame, size: Size) {
        if let Some((top, bottom)) = self.background_gradient {
            frame.fill_rectangle(
                iced::Point::ORIGIN,
                size,
                iced::widget::canvas::gradient::Linear::new(
                    iced::Point::ORIGIN,
                    iced::Point::new(0., self.size.height),
                )
                .add_stop(0., top)
                .add_stop(1., bottom),
            );
        } else {
            frame.fill_rectangle(iced::Point::ORIGIN, size, self.background);
        }
        self.draw_strokes(frame, self.grid.iter().chain(&self.lines));
        for rect in &self.candles {
            frame.fill_rectangle(rect.origin, rect.size, rect.color);
        }
        self.draw_strokes(frame, self.crosshair.iter());
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
        for marker in &self.crosshair_markers {
            frame.fill(&Path::circle(marker.center, marker.radius), marker.color);
        }
    }
    fn draw_strokes<'a>(&self, frame: &mut Frame, strokes: impl Iterator<Item = &'a CanvasStroke>) {
        for stroke in strokes {
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
    let snapshot = prepare_settled_plot(model, view, id, layout)?;
    Ok((snapshot, mask))
}

/// Geometry only: dimensions, deferred commands and timing have already settled.
pub fn prepare_settled_plot<B, M>(
    model: &mut ChartModel<B, M>,
    view: &mut LinePaneView,
    id: Option<SeriesId>,
    layout: &FrameLayout,
) -> Result<PlotSnapshot, FrameError>
where
    B: HorzScaleBehavior,
    B::Item: 'static,
    B::InternalItem: 'static,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone + 'static,
{
    prepare_series_plot(model, &mut PaneView::Line(view), id, layout)
}

pub(crate) fn prepare_series_plot<B, M>(
    model: &mut ChartModel<B, M>,
    view: &mut PaneView<'_>,
    id: Option<SeriesId>,
    layout: &FrameLayout,
) -> Result<PlotSnapshot, FrameError>
where
    B: HorzScaleBehavior,
    B::Item: 'static,
    B::InternalItem: 'static,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone + 'static,
{
    view.clear();
    let pane = model
        .panes()
        .get(layout.pane)
        .ok_or(FrameError::Model(ChartModelError::InvalidPane(layout.pane)))?;
    if !layout.size.width.is_finite()
        || !layout.size.height.is_finite()
        || layout.size.width < 0.
        || layout.size.height < 0.
        || model.time_scale().width() != f64::from(layout.size.width)
        || pane.height() != f64::from(layout.size.height)
    {
        return Err(FrameError::Model(ChartModelError::InvalidDimensions));
    }
    if layout.pixel_ratio.horizontal != layout.pixel_ratio.vertical
        || !layout.pixel_ratio.horizontal.is_finite()
        || layout.pixel_ratio.horizontal <= 0.
    {
        return Err(FrameError::InvalidPixelRatio);
    }
    if let Some(id) = id {
        match model.pane_for_series(id) {
            None => return Err(view.unknown_series(id)),
            Some(pane) if pane != layout.pane => {
                return Err(FrameError::SeriesPaneMismatch {
                    series: id,
                    pane: layout.pane,
                });
            }
            _ => {}
        }
    }
    let axes = model
        .prepare_axis_snapshots(layout.pane)
        .map_err(FrameError::Axes)?;
    if let Some(id) = id {
        match view {
            PaneView::Histogram(view) => view.refresh(model, id).map_err(FrameError::Histogram)?,
            PaneView::Line(view) => view.refresh(model, id).map_err(FrameError::Line)?,
            PaneView::Candlesticks(view) => {
                view.refresh(model, id).map_err(FrameError::Candlesticks)?
            }
        }
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
    let line = match view {
        PaneView::Line(view) => view
            .data()
            .map(|data| data.draw_commands(ratio))
            .unwrap_or_default(),
        PaneView::Candlesticks(_) | PaneView::Histogram(_) => LineDrawCommands::default(),
    };
    let mut snapshot = PlotSnapshot::from_commands(layout, grid, line)?;
    if let PaneView::Candlesticks(view) = view
        && let Some(data) = view.data()
    {
        snapshot.candles = data
            .draw_commands(ratio)
            .map_err(FrameError::CandleGeometry)?
            .into_iter()
            .map(|rectangle| canvas_rectangle(rectangle, ratio))
            .collect::<Result<_, FrameError>>()?;
    }
    if let PaneView::Histogram(view) = view
        && let Some(data) = view.data()
    {
        snapshot.candles = data
            .draw_commands(ratio)
            .map_err(FrameError::HistogramGeometry)?
            .into_iter()
            .map(|r| canvas_rectangle(r, ratio))
            .collect::<Result<_, _>>()?;
    }
    if let Some(position) = model
        .crosshair()
        .position()
        .filter(|p| p.pane == layout.pane)
    {
        snapshot.crosshair = crate::renderers::crosshair_renderer::draw_commands(
            position,
            model.crosshair().options(),
            crate::renderers::price_axis_renderer::AxisBounds {
                width: f64::from(layout.size.width),
                height: f64::from(layout.size.height),
            },
            ratio,
        )
        .into_iter()
        .map(|stroke| canvas_stroke(stroke, ratio))
        .collect::<Result<_, _>>()?;
    }
    if let crate::model::layout_options::Background::VerticalGradient {
        top_color,
        bottom_color,
    } = &axes.right.background
    {
        snapshot.background_gradient = Some((color(top_color)?, color(bottom_color)?));
    }
    snapshot.axes = Some(axes);
    let correction = (ratio.horizontal.floor().max(1.) as u32 % 2) as f64 / 2.;
    for marker in model.crosshair_markers(layout.pane) {
        let center = iced::Point::new(
            ((marker.point.x * f64::from(ratio.horizontal) + 0.5).floor() + correction) as f32
                / ratio.horizontal,
            marker.point.y as f32,
        );
        if marker.border_width > 0. {
            snapshot.crosshair_markers.push(CanvasMarker {
                center,
                radius: (marker.radius
                    + marker.border_width
                    + correction / f64::from(ratio.vertical)) as f32,
                color: color(&marker.border_color)?,
                color_key: marker.border_color,
            });
        }
        snapshot.crosshair_markers.push(CanvasMarker {
            center,
            radius: (marker.radius + correction / f64::from(ratio.vertical)) as f32,
            color: color(&marker.color)?,
            color_key: marker.color,
        });
    }
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderers::{draw_line::LineStyle, line_renderer::PointMarker};
    #[test]
    fn candle_rectangles_preserve_signed_extents_and_reject_invalid_canvas_values() {
        let rectangle = CandleRectangle {
            x: 10.,
            y: 20.,
            width: -4.,
            height: -8.,
            color: "rgba(255,0,0,0.5)".into(),
        };
        let ratio = PixelRatio {
            horizontal: 2.,
            vertical: 2.,
        };
        let canvas = canvas_rectangle(rectangle.clone(), ratio).unwrap();
        assert_eq!(canvas.origin, iced::Point::new(3., 6.));
        assert_eq!(canvas.size, Size::new(2., 4.));
        assert_eq!(canvas.color.a, 0.5);
        let mut overflowing = rectangle.clone();
        overflowing.x = f64::MAX;
        assert!(matches!(
            canvas_rectangle(overflowing, ratio),
            Err(FrameError::CandleGeometry(
                CandlestickRenderError::InvalidData
            ))
        ));
        let mut invalid_color = rectangle;
        invalid_color.color = "invalid".into();
        assert!(matches!(
            canvas_rectangle(invalid_color, ratio),
            Err(FrameError::InvalidColor(_))
        ));
    }
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
