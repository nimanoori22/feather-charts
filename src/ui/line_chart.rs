//! Iced adapter. Model/frame preparation is separate from immutable Canvas draw.
#[cfg(test)]
mod rectangle_clipping_tests;
mod series_drawing;
mod series_paint;
use crate::{
    model::{
        axis_snapshots::{AxisSnapshotError, AxisSnapshots},
        chart_model::{ChartModel, ChartModelError},
        data_layer::SeriesId,
        ihorz_scale_behavior::HorzScaleBehavior,
        invalidate_mask::InvalidateMask,
        series::{
            area_pane_view::{AreaPaneView, AreaPreparationError},
            bars_pane_view::{BarPreparationError, BarsPaneView},
            baseline_pane_view::{BaselinePaneView, BaselinePreparationError},
            candlesticks_pane_view::{CandlestickPreparationError, CandlesticksPaneView},
            histogram_pane_view::{HistogramPaneView, HistogramPreparationError},
            line_pane_view::{LinePaneView, LinePreparationError},
        },
    },
    renderers::{
        bars_renderer::BarRenderError,
        baseline_renderer::BaselineRenderError,
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
pub use series_drawing::{SeriesDrawPass, SeriesPassKind};
pub use series_paint::{CanvasGradientMarkers, CanvasGradientStroke};
use series_paint::{canvas_fill_with_colors, canvas_gradient_with_colors};
use std::{hash::Hash, time::Instant};

#[derive(Debug)]
pub enum FrameError {
    Axes(AxisSnapshotError),
    Model(ChartModelError),
    Line(LinePreparationError),
    Area(AreaPreparationError),
    Bars(BarPreparationError),
    Baseline(BaselinePreparationError),
    BaselineGeometry(BaselineRenderError),
    BarGeometry(BarRenderError),
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
    Area(&'a mut AreaPaneView),
    Bars(&'a mut BarsPaneView),
    Baseline(&'a mut BaselinePaneView),
    Candlesticks(&'a mut CandlesticksPaneView),
    Histogram(&'a mut HistogramPaneView),
}
impl PaneView<'_> {
    pub(crate) fn clear(&mut self) {
        match self {
            Self::Line(v) => v.clear(),
            Self::Area(v) => v.clear(),
            Self::Bars(v) => v.clear(),
            Self::Baseline(v) => v.clear(),
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
            Self::Area(_) => FrameError::Area(AreaPreparationError::UnknownSeries(id)),
            Self::Bars(_) => FrameError::Bars(BarPreparationError::UnknownSeries(id)),
            Self::Baseline(_) => FrameError::Baseline(BaselinePreparationError::UnknownSeries(id)),
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
impl CanvasRectangle {
    fn clipped_to(&self, bounds: Rectangle) -> Option<Self> {
        let clipped = Rectangle::new(self.origin, self.size).intersection(&bounds)?;
        (clipped.width > 0. && clipped.height > 0.).then_some(Self {
            origin: clipped.position(),
            size: clipped.size(),
            color: self.color,
        })
    }
    fn covered_by(&self, later: &Self, bounds: Rectangle) -> bool {
        later.color.a == 1.
            && self.clipped_to(bounds).is_some_and(|visible| {
                Rectangle::new(visible.origin, visible.size)
                    .is_within(&Rectangle::new(later.origin, later.size))
            })
    }
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
pub struct CanvasFill {
    pub path: Vec<PathOperation>,
    pub gradient: iced::widget::canvas::gradient::Linear,
}
#[derive(Clone, Debug, Default)]
pub struct CanvasSeriesGeometry {
    pub custom: Option<super::custom_series::PreparedCustomSeries>,
    pub series_id: Option<SeriesId>,
    pub fills: Vec<CanvasFill>,
    pub gradient_lines: Vec<CanvasGradientStroke>,
    pub gradient_markers: Vec<CanvasGradientMarkers>,
    pub lines: Vec<CanvasStroke>,
    pub candles: Vec<CanvasRectangle>,
    pub markers: Vec<CanvasMarker>,
}
#[derive(Clone, Debug)]
pub struct PlotSnapshot {
    pub last_price_animations:
        Vec<crate::renderers::series_last_price_animation_renderer::LastPriceAnimation>,
    pub hovered_custom_data: Option<crate::api::CustomHitData>,
    pub custom: Option<super::custom_series::PreparedCustomSeries>,
    pub marker_orders:
        std::collections::BTreeMap<crate::model::series_markers::MarkerCollectionId, u64>,
    pub primitives: Vec<crate::api::primitives::PreparedAttachment>,
    pub time_primitives: Vec<crate::api::primitives::PreparedAttachment>,
    pub hovered_series: Option<SeriesId>,
    pub hovered_series_on_top: bool,
    /// Visible hit geometry in stable source order, independent of raised drawing.
    pub hit_series: Vec<super::hit_testing::PreparedSeriesHit>,
    pub series_id: Option<SeriesId>,
    pub price_line_series: Vec<SeriesId>,
    /// Sorted by SeriesId; ties retain the source pane-view order.
    pub hit_price_lines: Vec<super::hit_testing::PreparedPriceLineHit>,
    pub price_line_kinds: Vec<crate::model::hit_test::PriceLineKind>,
    pub annotation_ratio: f64,
    pub annotations: Vec<crate::renderers::series_markers_renderer::PreparedMarkerCollection>,
    pub annotation_geometry: Vec<super::series_annotations::CanvasAnnotations>,
    pub fills: Vec<CanvasFill>,
    pub gradient_lines: Vec<CanvasGradientStroke>,
    pub gradient_markers: Vec<CanvasGradientMarkers>,
    /// Prepared from the same settled viewport as the grid and line.
    pub axes: Option<AxisSnapshots>,
    pub size: Size,
    pub background: Color,
    pub background_gradient: Option<(Color, Color)>,
    pub grid: Vec<CanvasStroke>,
    pub lines: Vec<CanvasStroke>,
    pub markers: Vec<CanvasMarker>,
    pub candles: Vec<CanvasRectangle>,
    pub series_layers: Vec<CanvasSeriesGeometry>,
    pub price_lines: Vec<CanvasStroke>,
    pub crosshair: Vec<CanvasStroke>,
    pub crosshair_markers: Vec<CanvasMarker>,
}
impl Default for PlotSnapshot {
    fn default() -> Self {
        Self {
            last_price_animations: vec![],
            hovered_custom_data: None,
            custom: None,
            marker_orders: Default::default(),
            primitives: vec![],
            time_primitives: vec![],
            hovered_series: None,
            hovered_series_on_top: true,
            series_id: None,
            price_line_series: vec![],
            hit_price_lines: vec![],
            price_line_kinds: vec![],
            hit_series: vec![],
            annotation_ratio: 1.,
            annotations: vec![],
            annotation_geometry: vec![],
            axes: None,
            size: Size::ZERO,
            background: Color::WHITE,
            background_gradient: None,
            grid: vec![],
            lines: vec![],
            markers: vec![],
            candles: vec![],
            series_layers: vec![],
            fills: vec![],
            gradient_lines: vec![],
            gradient_markers: vec![],
            price_lines: vec![],
            crosshair: vec![],
            crosshair_markers: vec![],
        }
    }
}
pub(crate) fn color_with_parser(
    value: &str,
    colors: &crate::model::colors::ColorParser,
) -> Result<Color, FrameError> {
    let [r, g, b, a] = colors
        .clone()
        .rendering_rgba(value)
        .map_err(|_| FrameError::InvalidColor(value.into()))?;
    Ok(Color::from_rgba(r, g, b, a))
}
#[cfg(test)]
fn canvas_rectangle(
    rect: CandleRectangle,
    ratio: PixelRatio,
) -> Result<CanvasRectangle, FrameError> {
    canvas_rectangle_with_colors(rect, ratio, &Default::default())
}
fn canvas_rectangle_with_colors(
    rect: CandleRectangle,
    ratio: PixelRatio,
    colors: &crate::model::colors::ColorParser,
) -> Result<CanvasRectangle, FrameError> {
    let x = rect.x / f64::from(ratio.horizontal);
    let y = rect.y / f64::from(ratio.vertical);
    let width = rect.width / f64::from(ratio.horizontal);
    let height = rect.height / f64::from(ratio.vertical);
    // Canvas2D fills negative extents in the opposite direction.
    let rectangle = CanvasRectangle {
        origin: iced::Point::new(x.min(x + width) as f32, y.min(y + height) as f32),
        size: Size::new(width.abs() as f32, height.abs() as f32),
        color: color_with_parser(&rect.color, colors)?,
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
fn canvas_stroke_with_colors(
    stroke: StyledStroke,
    ratio: PixelRatio,
    colors: &crate::model::colors::ColorParser,
) -> Result<CanvasStroke, FrameError> {
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
        color: color_with_parser(&stroke.color, colors)?,
        width: (stroke.width / f64::from(ratio.vertical)) as f32,
    })
}
fn canvas_path(ops: &[PathOperation], closed: bool) -> Path {
    Path::new(|builder| {
        for op in ops {
            let to_iced = |p: Point| iced::Point::new(p.x as f32, p.y as f32);
            match *op {
                PathOperation::MoveTo(p) => builder.move_to(to_iced(p)),
                PathOperation::LineTo(p) => builder.line_to(to_iced(p)),
                PathOperation::CubicTo {
                    control1,
                    control2,
                    end,
                } => builder.bezier_curve_to(to_iced(control1), to_iced(control2), to_iced(end)),
            }
        }
        if closed {
            builder.close();
        }
    })
}
impl PlotSnapshot {
    pub fn from_commands(
        layout: &FrameLayout,
        grid: Vec<GridDrawCommand>,
        line: LineDrawCommands,
    ) -> Result<Self, FrameError> {
        Self::from_commands_with_colors(layout, grid, line, &Default::default())
    }
    /// Resolves configured colors during conversion; returned geometry owns its ink.
    pub fn from_commands_with_colors(
        layout: &FrameLayout,
        grid: Vec<GridDrawCommand>,
        line: LineDrawCommands,
        colors: &crate::model::colors::ColorParser,
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
                grid_strokes.push(canvas_stroke_with_colors(
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
                    colors,
                )?);
            }
        }
        let lines = line
            .strokes
            .into_iter()
            .map(|s| canvas_stroke_with_colors(s, ratio, colors))
            .collect::<Result<_, _>>()?;
        let markers = line
            .markers
            .into_iter()
            .map(|m| {
                let p = logical(m.center, ratio);
                Ok(CanvasMarker {
                    center: iced::Point::new(p.x as f32, p.y as f32),
                    radius: (m.radius / f64::from(ratio.vertical)) as f32,
                    color: color_with_parser(&m.color, colors)?,
                    color_key: m.color,
                })
            })
            .collect::<Result<_, FrameError>>()?;
        Ok(Self {
            last_price_animations: vec![],
            hovered_custom_data: None,
            custom: None,
            marker_orders: Default::default(),
            primitives: vec![],
            time_primitives: vec![],
            hovered_series: None,
            hovered_series_on_top: true,
            series_id: None,
            price_line_series: vec![],
            hit_price_lines: vec![],
            price_line_kinds: vec![],
            hit_series: vec![],
            annotation_ratio: f64::from(ratio.horizontal),
            annotations: vec![],
            annotation_geometry: vec![],
            axes: None,
            size: layout.size,
            background: color_with_parser(&layout.background, colors)?,
            background_gradient: None,
            grid: grid_strokes,
            lines,
            markers,
            candles: vec![],
            series_layers: vec![],
            fills: vec![],
            gradient_lines: vec![],
            gradient_markers: vec![],
            price_lines: vec![],
            crosshair: vec![],
            crosshair_markers: vec![],
        })
    }
    pub(crate) fn sample_last_price_animations(&mut self, now: Instant) {
        for animation in &mut self.last_price_animations {
            animation.sample(now);
        }
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
            self.draw_contents(frame, size, true);
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
                self.draw_contents(frame, clip.size(), true);
            });
        }
    }
    pub(crate) fn draw_base_at(
        &self,
        frame: &mut Frame,
        region: crate::renderers::price_axis_renderer::AxisRect,
    ) {
        if let Some(clip) = crate::ui::price_axis::axis_region_clip(region, frame.size()) {
            frame.with_clip(clip, |frame| {
                frame.translate(iced::Vector::new(clip.x, clip.y));
                self.draw_contents(frame, clip.size(), false);
            });
        }
    }
    pub(crate) fn draw_crosshair_at(
        &self,
        frame: &mut Frame,
        region: crate::renderers::price_axis_renderer::AxisRect,
    ) {
        if let Some(clip) = crate::ui::price_axis::axis_region_clip(region, frame.size()) {
            frame.with_clip(clip, |frame| {
                frame.translate(iced::Vector::new(clip.x, clip.y));
                self.draw_crosshair(frame);
                self.draw_primitive_layer(frame, crate::api::PrimitiveLayer::Top);
            });
        }
    }
    fn draw_contents(&self, frame: &mut Frame, size: Size, crosshair: bool) {
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
        self.draw_primitive_layer(frame, crate::api::PrimitiveLayer::Bottom);
        self.draw_strokes(frame, self.grid.iter());
        self.draw_series(frame);
        // Low-level hosts can append strokes without an owning series.
        self.draw_strokes(
            frame,
            self.price_lines.iter().skip(self.price_line_series.len()),
        );
        if crosshair {
            self.draw_crosshair(frame);
            self.draw_primitive_layer(frame, crate::api::PrimitiveLayer::Top);
        }
    }
    fn draw_crosshair(&self, frame: &mut Frame) {
        self.draw_strokes(frame, self.crosshair.iter());
        for marker in &self.crosshair_markers {
            frame.fill(&Path::circle(marker.center, marker.radius), marker.color);
        }
        self.draw_annotations(
            frame,
            crate::model::series_markers::MarkerZOrder::AboveSeries,
            None,
        );
    }
    fn draw_series_price_lines(&self, frame: &mut Frame, series: Option<SeriesId>, base: bool) {
        self.draw_strokes(
            frame,
            self.price_lines
                .iter()
                .zip(&self.price_line_series)
                .zip(&self.price_line_kinds)
                .filter_map(|((line, id), kind)| {
                    (Some(*id) == series
                        && (*kind == crate::model::hit_test::PriceLineKind::RelativeBase) == base)
                        .then_some(line)
                }),
        );
    }
    fn draw_markers(&self, frame: &mut Frame, markers: &[CanvasMarker]) {
        // One fill per source style run avoids double alpha blending where
        // adjacent same-color markers overlap.
        for group in markers.chunk_by(|a, b| a.color_key == b.color_key) {
            let path = Path::new(|builder| {
                for marker in group {
                    builder.circle(marker.center, marker.radius);
                }
            });
            frame.fill(&path, group[0].color);
        }
    }
    fn draw_fills(&self, frame: &mut Frame, fills: &[CanvasFill]) {
        for fill in fills {
            frame.fill(&canvas_path(&fill.path, true), fill.gradient);
        }
    }
    fn draw_strokes<'a>(&self, frame: &mut Frame, strokes: impl Iterator<Item = &'a CanvasStroke>) {
        for stroke in strokes {
            let path = canvas_path(&stroke.path, false);
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
    let mut snapshot = prepare_settled_plot(model, view, id, layout)?;
    snapshot.sample_last_price_animations(now);
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
    prepare_series_plot(
        model,
        &mut PaneView::Line(view),
        id,
        layout,
        PlotContent::Pane,
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlotContent {
    Pane,
    Series,
}

pub(crate) fn prepare_series_plot<B, M>(
    model: &mut ChartModel<B, M>,
    view: &mut PaneView<'_>,
    id: Option<SeriesId>,
    layout: &FrameLayout,
    content: PlotContent,
) -> Result<PlotSnapshot, FrameError>
where
    B: HorzScaleBehavior,
    B::Item: 'static,
    B::InternalItem: 'static,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone + 'static,
{
    let parser = model.color_parser();
    let colors = &parser;
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
    let axes = (content == PlotContent::Pane)
        .then(|| {
            model
                .prepare_axis_snapshots(layout.pane)
                .map_err(FrameError::Axes)
        })
        .transpose()?;
    if let Some(id) = id {
        match view {
            PaneView::Histogram(view) => view.refresh(model, id).map_err(FrameError::Histogram)?,
            PaneView::Line(view) => view.refresh(model, id).map_err(FrameError::Line)?,
            PaneView::Area(view) => view.refresh(model, id).map_err(FrameError::Area)?,
            PaneView::Bars(view) => view.refresh(model, id).map_err(FrameError::Bars)?,
            PaneView::Baseline(view) => view.refresh(model, id).map_err(FrameError::Baseline)?,
            PaneView::Candlesticks(view) => {
                view.refresh(model, id).map_err(FrameError::Candlesticks)?
            }
        }
    } else {
        view.clear();
    }
    let ratio = layout.pixel_ratio;
    let grid = if content == PlotContent::Pane {
        model.panes()[layout.pane]
            .grid()
            .pane_view()
            .renderer()
            .draw_commands(
                BitmapSize {
                    width: layout.size.width * ratio.horizontal,
                    height: layout.size.height * ratio.vertical,
                },
                ratio,
            )
    } else {
        Vec::new()
    };
    let line = match view {
        PaneView::Line(view) => view
            .data()
            .map(|data| data.draw_commands(ratio))
            .unwrap_or_default(),
        PaneView::Area(view) => view
            .data()
            .map(|data| data.line.draw_commands(ratio))
            .unwrap_or_default(),
        PaneView::Baseline(_)
        | PaneView::Bars(_)
        | PaneView::Candlesticks(_)
        | PaneView::Histogram(_) => LineDrawCommands::default(),
    };
    let mut snapshot = PlotSnapshot::from_commands_with_colors(layout, grid, line, colors)?;
    snapshot.series_id = id;
    if let Some(id) = id {
        snapshot.last_price_animations.extend(
            crate::views::pane::series_last_price_animation_pane_view::prepare(
                model,
                id,
                ratio.horizontal,
                Instant::now(),
            )?,
        );
    }
    if let Some(id) = id
        && let Some(hit) = super::hit_testing::prepare(model, id, view)
    {
        snapshot.hit_series.push(hit);
    }

    if let Some(id) = id {
        snapshot.annotations = model.prepare_series_markers(id);
    }
    if let PaneView::Area(view) = view
        && let Some(data) = view.data()
    {
        snapshot.fills = data
            .fill_commands(ratio)
            .into_iter()
            .map(|f| canvas_fill_with_colors(f, ratio, colors))
            .collect::<Result<_, _>>()?;
    }
    if let PaneView::Baseline(view) = view
        && let Some(data) = view.data()
    {
        let commands = data
            .draw_commands(ratio)
            .map_err(FrameError::BaselineGeometry)?;
        snapshot.fills = commands
            .fills
            .into_iter()
            .map(|f| canvas_fill_with_colors(f, ratio, colors))
            .collect::<Result<_, _>>()?;
        snapshot.gradient_lines = commands
            .strokes
            .into_iter()
            .map(|c| {
                Ok(CanvasGradientStroke {
                    stroke: canvas_stroke_with_colors(c.stroke, ratio, colors)?,
                    gradient: canvas_gradient_with_colors(c.gradient, ratio, colors)?,
                })
            })
            .collect::<Result<_, FrameError>>()?;
        snapshot.gradient_markers = commands
            .markers
            .into_iter()
            .map(|c| {
                let markers = c
                    .markers
                    .into_iter()
                    .map(|m| {
                        let p = logical(m.center, ratio);
                        Ok(CanvasMarker {
                            center: iced::Point::new(p.x as f32, p.y as f32),
                            radius: (m.radius / f64::from(ratio.vertical)) as f32,
                            color: color_with_parser(&m.color, colors)?,
                            color_key: m.color,
                        })
                    })
                    .collect::<Result<_, FrameError>>()?;
                Ok(CanvasGradientMarkers {
                    markers,
                    gradient: canvas_gradient_with_colors(c.gradient, ratio, colors)?,
                })
            })
            .collect::<Result<_, FrameError>>()?;
    }
    if let PaneView::Bars(view) = view
        && let Some(data) = view.data()
    {
        snapshot.candles = data
            .draw_commands(ratio)
            .map_err(FrameError::BarGeometry)?
            .into_iter()
            .map(|rectangle| canvas_rectangle_with_colors(rectangle, ratio, colors))
            .collect::<Result<_, FrameError>>()?;
    }
    if let PaneView::Candlesticks(view) = view
        && let Some(data) = view.data()
    {
        snapshot.candles = data
            .draw_commands(ratio)
            .map_err(FrameError::CandleGeometry)?
            .into_iter()
            .map(|rectangle| canvas_rectangle_with_colors(rectangle, ratio, colors))
            .collect::<Result<_, FrameError>>()?;
    }
    if let PaneView::Histogram(view) = view
        && let Some(data) = view.data()
    {
        snapshot.candles = data
            .draw_commands(ratio)
            .map_err(FrameError::HistogramGeometry)?
            .into_iter()
            .map(|r| canvas_rectangle_with_colors(r, ratio, colors))
            .collect::<Result<_, _>>()?;
    }
    // Additional sources contribute series geometry; their pane-wide copies are discarded.
    let Some(axes) = axes else {
        return Ok(snapshot);
    };
    let decorations = model
        .price_decorations(layout.pane)
        .map_err(FrameError::Axes)?;
    for decoration in &decorations.lines {
        let series = decoration.series;
        let line = &decoration.line;
        snapshot
            .hit_price_lines
            .push(super::hit_testing::PreparedPriceLineHit {
                series,
                y: line.coordinate,
                width: f64::from(line.width.pixels()),
                kind: decoration.kind,
                external_id: decoration.external_id.clone(),
            });
        if let Some(stroke) = crate::renderers::horizontal_line_renderer::draw_commands(
            line,
            crate::renderers::price_axis_renderer::AxisBounds {
                width: f64::from(layout.size.width),
                height: f64::from(layout.size.height),
            },
            ratio,
        ) {
            snapshot
                .price_lines
                .push(canvas_stroke_with_colors(stroke, ratio, colors)?);
            snapshot.price_line_series.push(series);
            snapshot.price_line_kinds.push(decoration.kind);
        }
    }
    if let Some(mut position) = model.crosshair().position() {
        if position.pane != layout.pane {
            position.y = None;
        }
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
        .map(|stroke| canvas_stroke_with_colors(stroke, ratio, colors))
        .collect::<Result<_, _>>()?;
    }
    if let crate::model::layout_options::Background::VerticalGradient {
        top_color,
        bottom_color,
    } = &axes.right.background
    {
        snapshot.background_gradient = Some((
            color_with_parser(top_color, colors)?,
            color_with_parser(bottom_color, colors)?,
        ));
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
                color: color_with_parser(&marker.border_color, colors)?,
                color_key: marker.border_color,
            });
        }
        snapshot.crosshair_markers.push(CanvasMarker {
            center,
            radius: (marker.radius + correction / f64::from(ratio.vertical)) as f32,
            color: color_with_parser(&marker.color, colors)?,
            color_key: marker.color,
        });
    }
    snapshot.hit_price_lines.sort_by_key(|p| p.series);
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
