//! Synchronous measured frame owner. Only final settled geometry is published.
use crate::{
    model::{
        axis_snapshots::{AxisSnapshots, PriceAxisSide},
        chart_model::{ChartModel, ChartModelError},
        data_layer::SeriesId,
        ihorz_scale_behavior::HorzScaleBehavior,
        invalidate_mask::{InvalidateMask, InvalidationLevel},
        series::line_pane_view::{LinePaneView, LinePreparationError},
    },
    renderers::{
        grid_renderer::PixelRatio,
        line_renderer::Point,
        price_axis_renderer::{
            AxisBounds, AxisRect, PreparedPriceAxis, PriceAxisMeasurement, PriceAxisRenderError,
            measure_price_axis, prepare_price_axis,
        },
        time_axis_renderer::{
            PreparedTimeAxis, TimeAxisCommand, TimeAxisMeasurement, TimeAxisRenderError,
            measure_time_axis, prepare_time_axis,
        },
    },
    ui::{
        chart_layout::{AxisSizeRequests, ChartLayout, ChartLayoutError, allocate_chart_layout},
        line_chart::{FrameError, FrameLayout, PlotSnapshot, prepare_settled_plot},
        price_axis::{
            AxisFontResolver, IcedAxisTextMeasurer, IcedPriceAxes, IcedPriceAxis, PriceAxisUiError,
        },
        time_axis::{IcedTimeAxis, TimeAxisUiError, axis_font},
    },
    views::time_scale_animation::TimeScaleAnimationController,
};
use std::{hash::Hash, time::Instant};

#[derive(Debug)]
pub enum ChartFrameError {
    Layout(ChartLayoutError),
    Plot(FrameError),
    Model(ChartModelError),
    Price(PriceAxisRenderError),
    Time(TimeAxisRenderError),
    PriceUi(PriceAxisUiError),
    TimeUi(TimeAxisUiError),
    UnsupportedPaneCount(usize),
    InvalidPassLimit,
    LayoutDidNotConverge { passes: usize },
}

pub struct AxisMeasurements {
    pub left: PriceAxisMeasurement,
    pub right: PriceAxisMeasurement,
    pub time: TimeAxisMeasurement,
}
impl AxisMeasurements {
    pub fn requests(&self) -> AxisSizeRequests {
        AxisSizeRequests {
            left_width: self.left.required_width(),
            right_width: self.right.required_width(),
            time_height: self.time.required_height(),
        }
    }
}
/// The backend supplies matching-font measurements, not tick selection.
pub trait AxisMeasurer {
    fn measure(&mut self, snapshots: &AxisSnapshots) -> Result<AxisMeasurements, ChartFrameError>;
}
pub struct IcedAxisMeasurer<'a> {
    pub fonts: &'a AxisFontResolver,
}
impl AxisMeasurer for IcedAxisMeasurer<'_> {
    fn measure(&mut self, snapshots: &AxisSnapshots) -> Result<AxisMeasurements, ChartFrameError> {
        let price = |s: &crate::model::axis_snapshots::PriceAxisSnapshot| {
            let mut m = IcedAxisTextMeasurer::new(&s.renderer_options, self.fonts)
                .map_err(ChartFrameError::PriceUi)?;
            measure_price_axis(s, &mut m).map_err(ChartFrameError::Price)
        };
        let options = &snapshots.time.renderer_options;
        let mut normal = IcedAxisTextMeasurer::with_font(
            axis_font(
                self.fonts,
                &options.font_family,
                crate::model::axis_snapshots::TimeLabelEmphasis::Normal,
            )
            .map_err(ChartFrameError::TimeUi)?,
            options.font_size,
        )
        .map_err(ChartFrameError::PriceUi)?;
        let mut bold = IcedAxisTextMeasurer::with_font(
            axis_font(
                self.fonts,
                &options.font_family,
                crate::model::axis_snapshots::TimeLabelEmphasis::Emphasized,
            )
            .map_err(ChartFrameError::TimeUi)?,
            options.font_size,
        )
        .map_err(ChartFrameError::PriceUi)?;
        Ok(AxisMeasurements {
            left: price(&snapshots.left)?,
            right: price(&snapshots.right)?,
            time: measure_time_axis(&snapshots.time, &mut normal, &mut bold)
                .map_err(ChartFrameError::Time)?,
        })
    }
}

pub struct ChartFrameInput {
    pub series: Option<SeriesId>,
    pub bounds: AxisBounds,
    pub pixel_ratio: PixelRatio,
    pub background: String,
}

#[derive(Clone, Debug)]
pub struct ChartFrameSnapshot {
    pub layout: ChartLayout,
    /// Actual final measurements, distinct from retained growth-only allocation.
    pub requests: AxisSizeRequests,
    pub passes: usize,
    pub plot: PlotSnapshot,
    pub left: PreparedPriceAxis,
    pub right: PreparedPriceAxis,
    pub time: PreparedTimeAxis,
    pub left_corner: PreparedTimeAxis,
    pub right_corner: PreparedTimeAxis,
}

pub struct ChartFrameOwner {
    allocated: AxisSizeRequests,
    last_bounds: Option<AxisBounds>,
    last_ratio: Option<PixelRatio>,
    pass_limit: usize,
    retained: Option<InvalidateMask>,
}
impl Default for ChartFrameOwner {
    fn default() -> Self {
        Self {
            allocated: AxisSizeRequests::default(),
            last_bounds: None,
            last_ratio: None,
            pass_limit: 8,
            retained: None,
        }
    }
}
impl ChartFrameOwner {
    pub fn with_pass_limit(pass_limit: usize) -> Result<Self, ChartFrameError> {
        if pass_limit == 0 {
            return Err(ChartFrameError::InvalidPassLimit);
        }
        Ok(Self {
            pass_limit,
            ..Self::default()
        })
    }
    /// Retained only for this frame's layout replay, never a competing queue.
    pub fn retained_mask(&self) -> Option<&InvalidateMask> {
        self.retained.as_ref()
    }

    pub fn prepare<B, M, A>(
        &mut self,
        model: &mut ChartModel<B, M>,
        controller: &mut TimeScaleAnimationController,
        view: &mut LinePaneView,
        input: &ChartFrameInput,
        measurer: &mut A,
        now: Instant,
    ) -> Result<ChartFrameSnapshot, ChartFrameError>
    where
        B: HorzScaleBehavior,
        B::Item: 'static,
        B::InternalItem: 'static,
        B::Key: PartialOrd,
        B::CacheKey: Eq + Hash,
        M: Clone + 'static,
        A: AxisMeasurer,
    {
        let id = input.series;
        // Validate before detaching commands or modifying dimensions.
        allocate_chart_layout(input.bounds, AxisSizeRequests::default(), input.pixel_ratio)
            .map_err(ChartFrameError::Layout)?;
        if input.pixel_ratio.horizontal != input.pixel_ratio.vertical {
            return Err(ChartFrameError::Plot(FrameError::InvalidPixelRatio));
        }
        if model.panes().len() != 1 {
            return Err(ChartFrameError::UnsupportedPaneCount(model.panes().len()));
        }
        if let Some(id) = id
            && model.pane_for_series(id) != Some(0)
        {
            return Err(ChartFrameError::Plot(FrameError::Line(
                LinePreparationError::UnknownSeries(id),
            )));
        }
        view.clear();
        // Old dimensions provide a provisional request, not final geometry.
        let axes = model
            .prepare_axis_snapshots(0)
            .map_err(|e| ChartFrameError::Plot(FrameError::Axes(e)))?;
        let measured = measurer.measure(&axes)?;
        self.retained = controller.begin_frame(model, now);
        let full = self.last_bounds != Some(input.bounds)
            || self.last_ratio != Some(input.pixel_ratio)
            || self
                .retained
                .as_ref()
                .is_some_and(|m| m.global_level() == InvalidationLevel::Full);
        let mut requests = if full {
            measured.requests()
        } else {
            self.allocated.grow_to(measured.requests())
        };
        let mut sampled = None;
        for pass in 1..=self.pass_limit {
            let mut layout = allocate_chart_layout(input.bounds, requests, input.pixel_ratio)
                .map_err(ChartFrameError::Layout)?;
            // Iced geometry has f32 dimensions. Make this conversion explicit
            // once so the model and every snapshot use those exact dimensions.
            let height = f64::from(layout.plot.size.height as f32);
            for region in [
                &mut layout.plot,
                &mut layout.left_axis,
                &mut layout.right_axis,
            ] {
                region.size.height = height;
            }
            for region in [
                &mut layout.time_axis,
                &mut layout.left_corner,
                &mut layout.right_corner,
            ] {
                region.origin.y = height;
            }
            let width = f64::from(layout.plot.size.width as f32);
            layout.plot.size.width = width;
            layout.time_axis.size.width = width;
            layout.right_axis.origin.x = layout.plot.origin.x + width;
            layout.right_corner.origin.x = layout.right_axis.origin.x;
            if model.time_scale().width() != width {
                model.set_width(width).map_err(ChartFrameError::Model)?;
            }
            if model.panes()[0].height() != height {
                model
                    .set_pane_height(0, height)
                    .map_err(ChartFrameError::Model)?;
            }
            if let Some(mask) = &self.retained {
                if pass == 1 {
                    sampled = controller.apply_frame_sampled(model, mask, now);
                } else {
                    // New work remains pending; only the retained frame's
                    // original command sequence is replayed at changed bounds.
                    model.apply_invalidation(mask);
                    if let Some(offset) = sampled {
                        model.apply_animation_offset(offset);
                    }
                }
            }
            let axes = model
                .prepare_axis_snapshots(0)
                .map_err(|e| ChartFrameError::Plot(FrameError::Axes(e)))?;
            let measured = measurer.measure(&axes)?;
            let next = requests.grow_to(measured.requests());
            if next != requests {
                requests = next;
                continue;
            }
            let plot_layout = FrameLayout {
                pane: 0,
                size: iced::Size::new(width as f32, height as f32),
                pixel_ratio: input.pixel_ratio,
                background: input.background.clone(),
            };
            let plot = prepare_settled_plot(model, view, id, &plot_layout)
                .map_err(ChartFrameError::Plot)?;
            let left = prepare_price_axis(&measured.left, layout.left_axis.size, input.pixel_ratio)
                .map_err(ChartFrameError::Price)?;
            let right =
                prepare_price_axis(&measured.right, layout.right_axis.size, input.pixel_ratio)
                    .map_err(ChartFrameError::Price)?;
            let time = prepare_time_axis(&measured.time, layout.time_axis.size, input.pixel_ratio)
                .map_err(ChartFrameError::Time)?;
            let left_corner = prepare_corner(
                &axes,
                PriceAxisSide::Left,
                layout.left_corner.size,
                input.pixel_ratio,
            );
            let right_corner = prepare_corner(
                &axes,
                PriceAxisSide::Right,
                layout.right_corner.size,
                input.pixel_ratio,
            );
            self.allocated = requests;
            self.last_bounds = Some(input.bounds);
            self.last_ratio = Some(input.pixel_ratio);
            return Ok(ChartFrameSnapshot {
                layout,
                requests: measured.requests(),
                passes: pass,
                plot,
                left,
                right,
                time,
                left_corner,
                right_corner,
            });
        }
        // No intermediate/mismatched frame is published. Original mask remains
        // inspectable on the owner, and newly generated model work stays pending.
        Err(ChartFrameError::LayoutDidNotConverge {
            passes: self.pass_limit,
        })
    }
}

pub fn prepare_corner(
    axes: &AxisSnapshots,
    side: PriceAxisSide,
    bounds: AxisBounds,
    ratio: PixelRatio,
) -> PreparedTimeAxis {
    let price = match side {
        PriceAxisSide::Left => &axes.left,
        PriceAxisSide::Right => &axes.right,
    };
    let mut result = PreparedTimeAxis {
        required_height: 0.,
        bounds,
        commands: vec![],
    };
    if !axes.time.visible || !price.visible || bounds.width == 0. || bounds.height == 0. {
        return result;
    }
    result.commands.push(TimeAxisCommand::Background(
        axes.time.background_fill_color().into(),
    ));
    // The source TimeAxisWidget passes the same left-scale visibility getter to
    // both stubs. Preserve that shared policy, including asymmetric borders.
    if axes.left.border_visible && axes.time.border_visible {
        let width = (f64::from(price.renderer_options.border_size) * f64::from(ratio.horizontal))
            .floor()
            / f64::from(ratio.horizontal);
        let height = (f64::from(price.renderer_options.border_size) * f64::from(ratio.vertical))
            .floor()
            / f64::from(ratio.vertical);
        result.commands.push(TimeAxisCommand::Rectangle {
            rect: AxisRect {
                origin: Point {
                    x: if side == PriceAxisSide::Left {
                        bounds.width - width
                    } else {
                        0.
                    },
                    y: 0.,
                },
                size: AxisBounds { width, height },
            },
            color: axes.time.border_color.clone(),
        });
    }
    result
}

#[derive(Clone, Debug, Default)]
pub struct IcedChartFrame {
    pub layout: ChartLayout,
    pub requests: AxisSizeRequests,
    pub passes: usize,
    pub plot: PlotSnapshot,
    pub prices: IcedPriceAxes,
    pub time: IcedTimeAxis,
    pub left_corner: IcedTimeAxis,
    pub right_corner: IcedTimeAxis,
}
impl IcedChartFrame {
    pub fn from_snapshot(
        snapshot: ChartFrameSnapshot,
        fonts: &AxisFontResolver,
    ) -> Result<Self, ChartFrameError> {
        Ok(Self {
            layout: snapshot.layout,
            requests: snapshot.requests,
            passes: snapshot.passes,
            plot: snapshot.plot,
            prices: IcedPriceAxes {
                left: IcedPriceAxis::from_commands(&snapshot.left, fonts)
                    .map_err(ChartFrameError::PriceUi)?,
                right: IcedPriceAxis::from_commands(&snapshot.right, fonts)
                    .map_err(ChartFrameError::PriceUi)?,
            },
            time: IcedTimeAxis::from_commands(&snapshot.time, fonts)
                .map_err(ChartFrameError::TimeUi)?,
            left_corner: IcedTimeAxis::from_commands(&snapshot.left_corner, fonts)
                .map_err(ChartFrameError::TimeUi)?,
            right_corner: IcedTimeAxis::from_commands(&snapshot.right_corner, fonts)
                .map_err(ChartFrameError::TimeUi)?,
        })
    }
    pub fn draw(&self, frame: &mut iced::widget::canvas::Frame) {
        // Fractional clip edges can leave partially covered pixels on WGPU.
        // Paint a continuous prepared backdrop before the independently clipped
        // regions, so their antialiasing never exposes the parent's clear color.
        let size = iced::Size::new(
            self.layout.chart_size.width as f32,
            self.layout.chart_size.height as f32,
        );
        if size.width > 0. && size.height > 0. {
            // Keep the backdrop in its own draft: WGPU flushes a frame's
            // ungrouped meshes after pasted clips, otherwise obscuring them.
            frame.with_clip(iced::Rectangle::with_size(size), |frame| {
                if let Some((top, bottom)) = self.plot.background_gradient {
                    frame.fill_rectangle(
                        iced::Point::ORIGIN,
                        size,
                        iced::widget::canvas::gradient::Linear::new(
                            iced::Point::ORIGIN,
                            iced::Point::new(0., self.plot.size.height),
                        )
                        .add_stop(0., top)
                        .add_stop(1., bottom),
                    );
                } else {
                    frame.fill_rectangle(iced::Point::ORIGIN, size, self.plot.background);
                }
            });
        }
        self.plot.draw_at(frame, self.layout.plot);
        self.prices.left.draw_at(frame, self.layout.left_axis);
        self.prices.right.draw_at(frame, self.layout.right_axis);
        self.time.draw_at(frame, self.layout.time_axis);
        self.left_corner.draw_at(frame, self.layout.left_corner);
        self.right_corner.draw_at(frame, self.layout.right_corner);
    }
}
