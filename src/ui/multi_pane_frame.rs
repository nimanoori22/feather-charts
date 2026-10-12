//! One chart-wide invalidation/animation cycle for synchronous pane preparation.
use super::{
    chart_frame::{
        AxisMeasurer, ChartFrameError, ChartFrameSnapshot, IcedChartFrame, prepare_corner,
    },
    chart_layout::AxisSizeRequests,
    line_chart::{FrameError, FrameLayout, PaneView, prepare_series_plot},
    multi_pane_layout::{MultiPaneLayout, allocate_multi_pane_layout},
    price_axis::AxisFontResolver,
};
use crate::{
    model::{
        axis_snapshots::PriceAxisSide,
        chart_model::ChartModel,
        data_layer::SeriesId,
        ihorz_scale_behavior::HorzScaleBehavior,
        invalidate_mask::{InvalidateMask, InvalidationLevel},
        series::{
            candlesticks_pane_view::CandlesticksPaneView, histogram_pane_view::HistogramPaneView,
            line_pane_view::LinePaneView,
        },
    },
    renderers::{
        grid_renderer::PixelRatio,
        price_axis_renderer::{AxisBounds, prepare_price_axis},
        time_axis_renderer::prepare_time_axis,
    },
    views::time_scale_animation::TimeScaleAnimationController,
};
use std::{hash::Hash, time::Instant};

pub enum SeriesPaneView {
    Line(LinePaneView),
    Candlesticks(CandlesticksPaneView),
    Histogram(HistogramPaneView),
}
impl SeriesPaneView {
    fn borrowed(&mut self) -> PaneView<'_> {
        match self {
            Self::Line(v) => PaneView::Line(v),
            Self::Candlesticks(v) => PaneView::Candlesticks(v),
            Self::Histogram(v) => PaneView::Histogram(v),
        }
    }
}
pub struct MultiPaneChartFrameInput {
    pub series: Vec<Option<SeriesId>>,
    pub bounds: AxisBounds,
    pub pixel_ratio: PixelRatio,
    pub background: String,
}
#[derive(Clone, Debug)]
pub struct MultiPaneChartFrameSnapshot {
    pub layout: MultiPaneLayout,
    pub series: Vec<Option<SeriesId>>,
    pub panes: Vec<ChartFrameSnapshot>,
}
pub struct MultiPaneChartFrameOwner {
    allocated: AxisSizeRequests,
    last_bounds: Option<AxisBounds>,
    last_ratio: Option<PixelRatio>,
    last_count: usize,
    retained: Option<InvalidateMask>,
    pass_limit: usize,
}
impl Default for MultiPaneChartFrameOwner {
    fn default() -> Self {
        Self {
            allocated: AxisSizeRequests::default(),
            last_bounds: None,
            last_ratio: None,
            last_count: 0,
            retained: None,
            pass_limit: 8,
        }
    }
}
impl MultiPaneChartFrameOwner {
    pub fn retained_mask(&self) -> Option<&InvalidateMask> {
        self.retained.as_ref()
    }
    pub fn with_pass_limit(pass_limit: usize) -> Result<Self, ChartFrameError> {
        if pass_limit == 0 {
            return Err(ChartFrameError::InvalidPassLimit);
        }
        Ok(Self {
            pass_limit,
            ..Self::default()
        })
    }
    pub fn prepare<B, M, A>(
        &mut self,
        model: &mut ChartModel<B, M>,
        controller: &mut TimeScaleAnimationController,
        views: &mut [SeriesPaneView],
        input: &MultiPaneChartFrameInput,
        measurer: &mut A,
        now: Instant,
    ) -> Result<MultiPaneChartFrameSnapshot, ChartFrameError>
    where
        B: HorzScaleBehavior,
        B::Item: 'static,
        B::InternalItem: 'static,
        B::Key: PartialOrd,
        B::CacheKey: Eq + Hash,
        M: Clone + 'static,
        A: AxisMeasurer,
    {
        let result = (|| {
            for view in views.iter_mut() {
                view.borrowed().clear();
            }
            let count = model.panes().len();
            if count == 0 || views.len() != count || input.series.len() != count {
                return Err(ChartFrameError::UnsupportedPaneCount(count));
            }
            let stretches = model
                .panes()
                .iter()
                .map(|p| p.stretch_factor())
                .collect::<Vec<_>>();
            allocate_multi_pane_layout(
                input.bounds,
                AxisSizeRequests::default(),
                input.pixel_ratio,
                &stretches,
            )
            .map_err(ChartFrameError::Layout)?;
            if input.pixel_ratio.horizontal != input.pixel_ratio.vertical {
                return Err(ChartFrameError::Plot(FrameError::InvalidPixelRatio));
            }
            for (pane, id) in input.series.iter().enumerate() {
                if let Some(id) = id {
                    match model.pane_for_series(*id) {
                        None => {
                            return Err(ChartFrameError::Plot(
                                views[pane].borrowed().unknown_series(*id),
                            ));
                        }
                        Some(index) if index != pane => {
                            return Err(ChartFrameError::Plot(FrameError::SeriesPaneMismatch {
                                series: *id,
                                pane,
                            }));
                        }
                        _ => {}
                    }
                }
            }
            let mut measure_all = |model: &mut ChartModel<B, M>| {
                let mut axes = Vec::with_capacity(count);
                let mut measurements = Vec::with_capacity(count);
                let mut requests = AxisSizeRequests::default();
                for pane in 0..count {
                    let snapshot = model
                        .prepare_axis_snapshots(pane)
                        .map_err(|e| ChartFrameError::Plot(FrameError::Axes(e)))?;
                    let measured = measurer.measure(&snapshot)?;
                    let q = measured.requests();
                    requests.left_width = requests.left_width.max(q.left_width);
                    requests.right_width = requests.right_width.max(q.right_width);
                    requests.time_height = requests.time_height.max(q.time_height);
                    axes.push(snapshot);
                    measurements.push(measured);
                }
                Ok::<_, ChartFrameError>((axes, measurements, requests))
            };
            let (_, _, measured) = measure_all(model)?;
            self.retained = controller.begin_frame(model, now);
            let full = self.last_bounds != Some(input.bounds)
                || self.last_ratio != Some(input.pixel_ratio)
                || self.last_count != count
                || self
                    .retained
                    .as_ref()
                    .is_some_and(|m| m.global_level() == InvalidationLevel::Full);
            let mut requests = if full {
                measured
            } else {
                self.allocated.grow_to(measured)
            };
            let mut sampled = None;
            for pass in 1..=self.pass_limit {
                let layout = allocate_multi_pane_layout(
                    input.bounds,
                    requests,
                    input.pixel_ratio,
                    &stretches,
                )
                .map_err(ChartFrameError::Layout)?;
                let width = layout.panes[0].plot.size.width;
                if model.time_scale().width() != width {
                    model.set_width(width).map_err(ChartFrameError::Model)?;
                }
                for (i, pane) in layout.panes.iter().enumerate() {
                    if model.panes()[i].height() != pane.plot.size.height {
                        model
                            .set_pane_height(i, pane.plot.size.height)
                            .map_err(ChartFrameError::Model)?;
                    }
                }
                if let Some(mask) = &self.retained {
                    if pass == 1 {
                        sampled = controller.apply_frame_sampled(model, mask, now);
                    } else {
                        model.apply_invalidation(mask);
                        if let Some(offset) = sampled {
                            model.apply_animation_offset(offset);
                        }
                    }
                }
                let (axes, measurements, measured) = measure_all(model)?;
                let next = requests.grow_to(measured);
                if next != requests {
                    requests = next;
                    continue;
                }
                let mut panes = Vec::with_capacity(count);
                for (i, ((axes, measured), pane_layout)) in axes
                    .iter()
                    .zip(&measurements)
                    .zip(&layout.panes)
                    .enumerate()
                {
                    let plot_layout = FrameLayout {
                        pane: i,
                        size: iced::Size::new(width as f32, pane_layout.plot.size.height as f32),
                        pixel_ratio: input.pixel_ratio,
                        background: input.background.clone(),
                    };
                    let plot = prepare_series_plot(
                        model,
                        &mut views[i].borrowed(),
                        input.series[i],
                        &plot_layout,
                    )
                    .map_err(ChartFrameError::Plot)?;
                    panes.push(ChartFrameSnapshot {
                        layout: *pane_layout,
                        requests: measured.requests(),
                        passes: pass,
                        plot,
                        left: prepare_price_axis(
                            &measured.left,
                            pane_layout.left_axis.size,
                            input.pixel_ratio,
                        )
                        .map_err(ChartFrameError::Price)?,
                        right: prepare_price_axis(
                            &measured.right,
                            pane_layout.right_axis.size,
                            input.pixel_ratio,
                        )
                        .map_err(ChartFrameError::Price)?,
                        time: prepare_time_axis(
                            &measured.time,
                            layout.chart.time_axis.size,
                            input.pixel_ratio,
                        )
                        .map_err(ChartFrameError::Time)?,
                        left_corner: prepare_corner(
                            axes,
                            PriceAxisSide::Left,
                            layout.chart.left_corner.size,
                            input.pixel_ratio,
                        ),
                        right_corner: prepare_corner(
                            axes,
                            PriceAxisSide::Right,
                            layout.chart.right_corner.size,
                            input.pixel_ratio,
                        ),
                    });
                }
                self.allocated = requests;
                self.last_bounds = Some(input.bounds);
                self.last_ratio = Some(input.pixel_ratio);
                self.last_count = count;
                return Ok(MultiPaneChartFrameSnapshot {
                    layout,
                    series: input.series.clone(),
                    panes,
                });
            }
            Err(ChartFrameError::LayoutDidNotConverge {
                passes: self.pass_limit,
            })
        })();
        if result.is_err() {
            for view in views.iter_mut() {
                view.borrowed().clear();
            }
        }
        result
    }
}

#[derive(Clone, Debug, Default)]
pub struct IcedMultiPaneChartFrame {
    pub layout: MultiPaneLayout,
    pub panes: Vec<IcedChartFrame>,
    pub series: Vec<Option<SeriesId>>,
    pub separator_color: iced::Color,
}
impl IcedMultiPaneChartFrame {
    pub fn from_snapshot(
        snapshot: MultiPaneChartFrameSnapshot,
        fonts: &AxisFontResolver,
        separator_color: &str,
    ) -> Result<Self, ChartFrameError> {
        Ok(Self {
            layout: snapshot.layout,
            series: snapshot.series,
            panes: snapshot
                .panes
                .into_iter()
                .map(|p| IcedChartFrame::from_snapshot(p, fonts))
                .collect::<Result<_, _>>()?,
            separator_color: super::line_chart::color(separator_color)
                .map_err(ChartFrameError::Plot)?,
        })
    }
    pub fn draw(&self, frame: &mut iced::widget::canvas::Frame) {
        if let Some(first) = self.panes.first() {
            first.draw(frame);
        }
        for pane in self.panes.iter().skip(1) {
            pane.plot.draw_at(frame, pane.layout.plot);
            pane.prices.left.draw_at(frame, pane.layout.left_axis);
            pane.prices.right.draw_at(frame, pane.layout.right_axis);
        }
        for region in &self.layout.separators {
            if let Some(clip) = super::price_axis::axis_region_clip(*region, frame.size()) {
                frame.with_clip(clip, |frame| {
                    frame.fill_rectangle(
                        iced::Point::new(clip.x, clip.y),
                        clip.size(),
                        self.separator_color,
                    )
                });
            }
        }
    }
}
