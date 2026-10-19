use super::{Chart, ChartError};
use crate::{
    model::{
        axis_snapshots::PriceAxisSide, data_consumer::BuiltInSeriesDataItem,
        horz_scale_behavior_time::types::Time,
    },
    ui::{
        chart_input::{ChartInteraction, MultiPaneInteraction, Navigation},
        price_axis_input::{PriceAxisAction, PriceAxisInteraction},
    },
};
use std::time::Instant;
#[cfg(test)]
#[path = "interaction/tests.rs"]
mod tests;
#[derive(Clone, Debug)]
pub enum ChartMessage {
    AttributionActivated,
    Touch(super::TouchInteraction),
    PrimitiveEdit(super::PrimitiveEditEvent),
    Interaction(ChartInteraction),
    PaneInteraction(MultiPaneInteraction),
    PriceAxis(PriceAxisInteraction),
    TimeAxis(crate::ui::time_axis_input::TimeAxisAction),
    Bounds(iced::Size),
    Tick(Instant),
    Click { pane: usize, position: iced::Point },
    DoubleClick { pane: usize, position: iced::Point },
}
#[derive(Clone, Debug)]
pub struct PointerEvent<T = Time, M = ()> {
    pub custom_series_data: Vec<super::CustomSeriesEventData>,
    pub custom_hit: Option<super::CustomHitData>,
    pub primitive: Option<super::PrimitiveHit>,
    pub hit: Option<super::ChartHit>,
    pub marker: Option<super::MarkerHit>,
    pub pane: Option<usize>,
    pub point: Option<iced::Point>,
    pub logical: Option<crate::model::time_data::TimePointIndex>,
    pub time: Option<T>,
    pub price: Option<f64>,
    pub series_data: Vec<(
        crate::model::data_layer::SeriesId,
        BuiltInSeriesDataItem<T, M>,
    )>,
}
#[derive(Clone, Debug)]
pub enum ChartEvent<T = Time, M = ()> {
    /// Host-controlled navigation; the engine never opens this destination.
    AttributionActivated {
        url: &'static str,
    },
    DataChanged {
        series: crate::model::data_layer::SeriesId,
        scope: super::DataChangeScope,
    },
    DoubleClick(PointerEvent<T, M>),
    VisibleLogicalRangeChanged(Option<crate::model::time_data::LogicalRange>),
    VisibleHorizontalRangeChanged(Option<super::HorizontalRange<T>>),
    HorizontalAxisSizeChanged {
        width: f64,
        height: f64,
    },
    PrimitiveEdit(super::PrimitiveEditEvent),
    Click(PointerEvent<T, M>),
    CrosshairMoved(PointerEvent<T, M>),
}
pub(crate) struct PlotPan {
    pub pane: usize,
    scale: crate::model::pane::PriceScalePosition,
    anchor_y: f32,
    price_started: bool,
}
impl<B: super::ChartBehavior, M: Clone + 'static> Chart<B, M> {
    fn begin_kinetic(&mut self, now: Instant, touch: bool) {
        self.kinetic = None;
        if if touch {
            self.kinetic_options.touch
        } else {
            self.kinetic_options.mouse
        } {
            let spacing = self.model.time_scale().bar_spacing();
            self.kinetic = crate::model::kinetic_animation::KineticSampler::new(
                0.2 / spacing,
                7. / spacing,
                0.997,
                15. / spacing,
            );
            self.sample_kinetic(now);
        }
    }
    fn sample_kinetic(&mut self, now: Instant) {
        if let Some(sampler) = &mut self.kinetic {
            sampler.add_position(self.model.time_scale().right_offset(), now);
        }
    }
    fn finish_pan(&mut self, now: Instant, animate: bool) {
        let animation = self
            .kinetic
            .take()
            .filter(|_| animate)
            .and_then(|sampler| sampler.start(self.model.time_scale().right_offset(), now));
        self.touch_pending_pan = None;
        self.cancel_plot_pan();
        if let Some(animation) = animation {
            self.model.set_time_scale_animation(
                crate::model::invalidate_mask::AnimationRequest::new(animation),
            );
        }
    }
    fn start_plot_pan(&mut self, pane: usize, start: iced::Point) {
        self.cancel_plot_pan();
        let scale = crate::model::pane::PriceScalePosition::from_id(
            self.model.panes()[pane].default_price_scale().id(),
        );
        self.plot_pan = Some(PlotPan {
            pane,
            scale,
            anchor_y: start.y,
            price_started: false,
        });
        self.model.start_scroll_time(f64::from(start.x).into());
    }
    fn move_plot_pan(&mut self, position: iced::Point) -> Result<(), ChartError> {
        if let Some(pan) = &mut self.plot_pan {
            if !pan.price_started && position.y != pan.anchor_y {
                self.model.start_scroll_price_on_scale(
                    pan.pane,
                    &pan.scale,
                    f64::from(pan.anchor_y),
                )?;
                pan.price_started = true;
            }
            if pan.price_started {
                self.model
                    .scroll_price_to_on_scale(pan.pane, &pan.scale, f64::from(position.y))?;
            }
            self.model.scroll_time_to(f64::from(position.x).into());
        }
        Ok(())
    }
    fn apply_touch(
        &mut self,
        input: super::TouchInteraction,
    ) -> Result<Option<(usize, iced::Point)>, ChartError> {
        use super::TouchAction;
        match input.action {
            TouchAction::Press => {
                self.stop_time_scale_animation();
                self.cancel_plot_pan();
                self.cancel_price_axis();
                self.touch_pending_pan = None;
                if self.touch_tracking
                    && self
                        .model
                        .crosshair()
                        .position()
                        .is_some_and(|point| point.pane != input.pane)
                {
                    self.model.set_crosshair_position(input.pane, None)?;
                    self.touch_tracking = false;
                }
            }
            TouchAction::Pan { start, position } => {
                if !self.touch_options.horizontal_drag && !self.touch_options.vertical_drag {
                    return Ok(None);
                }
                if let Some(start) = start {
                    self.stop_time_scale_animation();
                    self.touch_pending_pan = Some((start, input.now));
                } else if let Some((point, started)) = self.touch_pending_pan
                    && point != position
                {
                    self.touch_pending_pan = None;
                    self.start_plot_pan(input.pane, position);
                    self.begin_kinetic(started, true);
                }
                self.move_plot_pan(position)?;
                self.sample_kinetic(input.now);
            }
            TouchAction::Pinch { anchor, zoom } => {
                self.stop_time_scale_animation();
                self.cancel_plot_pan();
                self.touch_pending_pan = None;
                if self.touch_options.pinch {
                    self.model.zoom_time(f64::from(anchor.x).into(), zoom);
                }
            }
            TouchAction::Track { position } => {
                self.stop_time_scale_animation();
                self.cancel_plot_pan();
                self.touch_pending_pan = None;
                self.touch_tracking = true;
                self.model.set_crosshair_position(
                    input.pane,
                    Some(crate::renderers::line_renderer::Point {
                        x: f64::from(position.x),
                        y: f64::from(position.y),
                    }),
                )?;
            }
            TouchAction::Tap { position } | TouchAction::DoubleTap { position } => {
                self.finish_pan(input.now, false);
                return Ok(Some((input.pane, position)));
            }
            TouchAction::End => self.finish_pan(input.now, true),
            TouchAction::ExitTracking | TouchAction::Cancel => {
                self.stop_time_scale_animation();
                self.finish_pan(input.now, false);
                if self.touch_tracking {
                    self.model.set_crosshair_position(input.pane, None)?;
                }
                self.touch_tracking = false;
            }
        }
        Ok(None)
    }
    fn cancel_plot_pan(&mut self) {
        self.model.end_scroll_time();
        if let Some(pan) = self.plot_pan.take() {
            let _ = self.model.end_scroll_price_on_scale(pan.pane, &pan.scale);
        }
    }
    pub(crate) fn cancel_time_axis(&mut self) {
        if std::mem::take(&mut self.active_time_axis) {
            self.model.end_scale_time();
        }
    }
    pub(super) fn cancel_price_axis(&mut self) {
        if let Some((pane, side)) = self.active_price_axis.take() {
            let _ = self.model.end_scale_price(pane, side);
        }
    }
    pub fn axis_scaling(&self) -> Vec<[bool; 2]> {
        self.model
            .panes()
            .iter()
            .map(|pane| {
                [pane.left_price_scale(), pane.right_price_scale()].map(|scale| {
                    scale.options().visible
                        && !scale.is_empty()
                        && !scale.is_percentage()
                        && !scale.is_indexed_to_100()
                })
            })
            .collect()
    }
    fn apply_interaction(
        &mut self,
        pane: usize,
        input: ChartInteraction,
        now: Instant,
    ) -> Result<(), ChartError> {
        if pane >= self.model.panes().len() {
            return Err(crate::model::chart_model::ChartModelError::InvalidPane(pane).into());
        }
        use crate::model::coordinate::Coordinate;
        self.touch_tracking = false;
        if input.navigation.is_some() {
            self.cancel_time_axis();
        }
        match input.navigation {
            Some(Navigation::StopAnimation) => {
                self.stop_time_scale_animation();
                self.touch_pending_pan = None;
            }
            Some(Navigation::Wheel { x, zoom, scroll }) => {
                self.stop_time_scale_animation();
                self.touch_pending_pan = None;
                self.cancel_plot_pan();
                self.model.zoom_time(Coordinate::new(x), zoom);
                if scroll != 0. {
                    self.model.start_scroll_time(Coordinate::new(0.));
                    self.model.scroll_time_to(Coordinate::new(scroll));
                    self.model.end_scroll_time();
                }
            }
            Some(Navigation::Pan { start, position }) => {
                if let Some(start) = start {
                    self.stop_time_scale_animation();
                    self.touch_pending_pan = None;
                    self.start_plot_pan(pane, start);
                    self.begin_kinetic(now, false);
                }
                self.move_plot_pan(position)?;
                self.sample_kinetic(now);
            }
            Some(Navigation::Scroll { start, x }) => {
                if let Some(start) = start {
                    self.stop_time_scale_animation();
                    self.touch_pending_pan = None;
                    self.cancel_plot_pan();
                    self.model.start_scroll_time(Coordinate::new(start));
                    self.begin_kinetic(now, false);
                }
                self.model.scroll_time_to(Coordinate::new(x));
                self.sample_kinetic(now);
            }
            Some(Navigation::EndScroll) => {
                if !input.capture {
                    self.stop_time_scale_animation();
                }
                self.finish_pan(now, input.capture);
            }
            None => {}
        }
        self.model.set_crosshair_position(
            pane,
            input
                .position
                .map(|point| crate::renderers::line_renderer::Point {
                    x: f64::from(point.x),
                    y: f64::from(point.y),
                }),
        )?;
        Ok(())
    }
    /// Cancel both model gestures before viewport or membership changes.
    pub fn cancel_interaction(&mut self) {
        self.cancel_primitive_edit(super::PrimitiveEditCancelReason::ViewportChanged);
        self.cancel_plot_pan();
        self.cancel_price_axis();
        self.cancel_time_axis();
        self.stop_time_scale_animation();
        self.touch_pending_pan = None;
        self.touch_epoch = self.touch_epoch.wrapping_add(1);
        if self.touch_tracking {
            let _ = self.model.set_crosshair_position(0, None);
        }
        self.touch_tracking = false;
        self.revision += 1;
    }
    pub(crate) fn cancel_viewport_interaction(&mut self) {
        self.cancel_primitive_edit(super::PrimitiveEditCancelReason::ViewportChanged);
        if self.plot_pan.is_some() || self.active_price_axis.is_some() || self.active_time_axis {
            self.revision += 1;
        }
        self.cancel_plot_pan();
        self.cancel_price_axis();
        self.cancel_time_axis();
        self.stop_time_scale_animation();
        self.touch_pending_pan = None;
        self.touch_epoch = self.touch_epoch.wrapping_add(1);
        if self.touch_tracking {
            let _ = self.model.set_crosshair_position(0, None);
        }
        self.touch_tracking = false;
    }
    pub fn is_panning(&self) -> bool {
        self.plot_pan.is_some()
    }
    pub fn pan_pane(&self) -> Option<usize> {
        self.plot_pan.as_ref().map(|p| p.pane)
    }
    pub fn update(
        &mut self,
        message: ChartMessage,
    ) -> Result<Vec<ChartEvent<B::Item, M>>, ChartError> {
        self.validate_message(&message)?;
        if let ChartMessage::Tick(now) = &message {
            self.expire_up_down_markers(*now);
        }
        self.check_primitive_edit(self.size, self.ratio);
        if let ChartMessage::Tick(now) = message
            && self.sample_price_animation_tick(now)
        {
            return Ok(self.take_events());
        }
        let edit_message = matches!(message, ChartMessage::PrimitiveEdit(_));
        let previous = (
            self.model.crosshair().position(),
            self.model.crosshair().origin(),
        );
        let mut clicked = None;
        let mut double_click = false;
        let mut now = Instant::now();
        match message {
            ChartMessage::AttributionActivated => {
                if self.model.attribution_options().0 && self.frame.attribution.is_some() {
                    self.push_event(ChartEvent::AttributionActivated {
                        url: "https://www.tradingview.com/",
                    });
                }
                return Ok(self.take_events());
            }
            ChartMessage::Touch(input) if self.primitive_edit.is_none() => {
                now = input.now;
                double_click = matches!(input.action, super::TouchAction::DoubleTap { .. });
                clicked = self.apply_touch(input)?;
            }
            ChartMessage::Touch(_) => {}
            ChartMessage::PrimitiveEdit(event) => {
                if let Some(event) = self.apply_primitive_edit(event) {
                    self.push_event(ChartEvent::PrimitiveEdit(event));
                }
            }
            ChartMessage::Interaction(_)
            | ChartMessage::PaneInteraction(_)
            | ChartMessage::PriceAxis(_)
            | ChartMessage::TimeAxis(_)
                if self.primitive_edit.is_some() => {}
            ChartMessage::TimeAxis(
                crate::ui::time_axis_input::TimeAxisAction::Start { .. }
                | crate::ui::time_axis_input::TimeAxisAction::Move { .. },
            ) if !self.axis_options.time_drag => {}
            ChartMessage::TimeAxis(action) => {
                use crate::ui::time_axis_input::TimeAxisAction;
                match action {
                    TimeAxisAction::Start { x } => {
                        self.stop_time_scale_animation();
                        self.cancel_plot_pan();
                        self.cancel_price_axis();
                        self.cancel_time_axis();
                        self.touch_pending_pan = None;
                        self.touch_tracking = false;
                        self.model.set_crosshair_position(0, None)?;
                        if self.axis_options.time_drag && !self.model.time_scale().is_empty() {
                            self.model.start_scale_time(x);
                            self.active_time_axis = true;
                        }
                    }
                    TimeAxisAction::Move { x }
                        if self.active_time_axis && self.axis_options.time_drag =>
                    {
                        self.model.scale_time_to(x)
                    }
                    TimeAxisAction::Move { .. } => {}
                    TimeAxisAction::End => self.cancel_time_axis(),
                    TimeAxisAction::Reset if self.axis_options.time_reset => {
                        self.cancel_viewport_interaction();
                        self.model.reset_time_scale();
                    }
                    TimeAxisAction::Reset => {}
                }
            }
            ChartMessage::PriceAxis(input)
                if matches!(
                    input.action,
                    PriceAxisAction::Start { .. } | PriceAxisAction::Move { .. }
                ) && !self.axis_options.price_drag => {}
            ChartMessage::PriceAxis(input)
                if matches!(input.action, PriceAxisAction::Reset)
                    && !self.axis_options.price_reset => {}
            ChartMessage::PriceAxis(input) => {
                self.cancel_time_axis();
                self.stop_time_scale_animation();
                self.touch_pending_pan = None;
                self.touch_epoch = self.touch_epoch.wrapping_add(1);
                self.touch_tracking = false;
                self.cancel_plot_pan();
                self.model.set_crosshair_position(0, None)?;
                let result = match input.action {
                    PriceAxisAction::Start { y } => {
                        self.cancel_price_axis();
                        self.active_price_axis = Some((input.pane, input.side));
                        self.model.start_scale_price(input.pane, input.side, y)
                    }
                    PriceAxisAction::Move { y } => {
                        if self.active_price_axis == Some((input.pane, input.side)) {
                            self.model.scale_price_to(input.pane, input.side, y)
                        } else {
                            Ok(())
                        }
                    }
                    PriceAxisAction::End => {
                        self.cancel_price_axis();
                        Ok(())
                    }
                    PriceAxisAction::Wheel { delta } => {
                        self.model.zoom_price_scale(input.pane, input.side, delta)
                    }
                    PriceAxisAction::Reset => {
                        self.cancel_price_axis();
                        self.model.reset_price_scale(input.pane, input.side)
                    }
                };
                if let Err(error) = result {
                    return Err(error.into());
                }
            }

            ChartMessage::Interaction(input) => self.apply_interaction(0, input, now)?,
            ChartMessage::PaneInteraction(action) => match action {
                MultiPaneInteraction::Plot { pane, interaction } => {
                    self.apply_interaction(pane, interaction, now)?
                }
                MultiPaneInteraction::Resize {
                    top,
                    upper_stretch,
                    lower_stretch,
                } => {
                    self.stop_time_scale_animation();
                    self.touch_pending_pan = None;
                    self.touch_epoch = self.touch_epoch.wrapping_add(1);
                    self.cancel_plot_pan();
                    self.model.set_crosshair_position(0, None)?;
                    self.model.set_pane_stretch_factor(top, upper_stretch)?;
                    self.model.set_pane_stretch_factor(top + 1, lower_stretch)?;
                }
                MultiPaneInteraction::Cancel => self.apply_interaction(
                    0,
                    ChartInteraction {
                        position: None,
                        navigation: Some(Navigation::EndScroll),
                        capture: false,
                    },
                    now,
                )?,
            },

            ChartMessage::Bounds(size) => {
                self.cancel_interaction();
                let startup = self.size.width == 0. && size.width > 0.;
                self.size = size;
                if startup {
                    self.model.fit_content();
                }
            }
            ChartMessage::Tick(time) => now = time,
            ChartMessage::Click { pane, position } => {
                self.cancel_plot_pan();
                self.apply_interaction(
                    pane,
                    ChartInteraction {
                        position: Some(position),
                        navigation: None,
                        capture: true,
                    },
                    now,
                )?;
                clicked = Some((pane, position));
            }
            ChartMessage::DoubleClick { pane, position } => {
                if self.primitive_edit.is_none() && self.event_interests.double_click {
                    clicked = Some((pane, position));
                    double_click = true;
                }
            }
        }
        if edit_message {
            // The host applies geometry before preparing once; avoid a discarded preview frame.
            return Ok(self.take_events());
        }
        self.prepare(self.size, self.ratio, now)?;
        if let Some((pane, position)) = clicked {
            let pointer = self.pointer_event_at(
                Some(pane),
                Some(position),
                Some(
                    self.model
                        .time_scale()
                        .coordinate_to_index(f64::from(position.x).into(), true),
                ),
                Some(f64::from(position.y)),
            );
            if double_click {
                if self.event_interests.double_click {
                    self.push_event(ChartEvent::DoubleClick(pointer));
                }
            } else {
                self.push_event(ChartEvent::Click(pointer));
            }
        }
        let pointer = self.pointer_event();
        if self.last_primitive != pointer.primitive
            || self.last_hit != pointer.hit
            || previous
                != (
                    self.model.crosshair().position(),
                    self.model.crosshair().origin(),
                )
        {
            self.last_hit = pointer.hit.clone();
            self.last_primitive = pointer.primitive.clone();
            self.push_event(ChartEvent::CrosshairMoved(pointer));
        }
        Ok(self.take_events())
    }
}
impl<B: super::ChartBehavior, M: Clone + 'static> Chart<B, M> {
    fn pointer_event(&self) -> PointerEvent<B::Item, M> {
        let position = self.model.crosshair().position();
        let origin = self.model.crosshair().origin();
        self.pointer_event_at(
            origin.map(|o| o.pane),
            origin.map(|o| iced::Point::new(o.point.x as f32, o.point.y as f32)),
            position.map(|p| p.index),
            position.and_then(|p| p.y),
        )
    }
    fn pointer_event_at(
        &self,
        pane: Option<usize>,
        point: Option<iced::Point>,
        logical: Option<crate::model::time_data::TimePointIndex>,
        y: Option<f64>,
    ) -> PointerEvent<B::Item, M> {
        let time = logical
            .and_then(|index| self.model.time_scale().index_to_original_time(index))
            .cloned();
        let mut series_data = Vec::new();
        let custom_series_data = logical
            .map(|index| {
                self.views
                    .iter()
                    .flatten()
                    .filter_map(|(_, view)| {
                        if let crate::ui::multi_pane_frame::SeriesPaneView::Custom(view) = view {
                            view.adapter.data_at(index)
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        if let Some(index) = logical {
            for id in self.kinds.keys() {
                let Some(series) = self.model.series(*id) else {
                    continue;
                };
                let Some(row) = series
                    .bars()
                    .search(index, crate::model::plot_list::MismatchDirection::None)
                else {
                    continue;
                };
                let Some(data) = super::series_data::public_data(row) else {
                    continue;
                };
                series_data.push((*id, data));
            }
        }
        let price = pane.and_then(|index| {
            let pane = self.model.panes().get(index)?;
            let scale = pane.default_price_scale();
            let first = scale.first_value()?;
            Some(scale.coordinate_to_price(y?.into(), first))
        });
        let selection = pane
            .zip(point)
            .and_then(|(pane, point)| self.frame_for_pane(pane)?.plot.selection(point));
        let (hit, primitive, custom_hit) = match selection {
            Some(crate::ui::primitive_hit_testing::PaneHit::BuiltIn(h, data)) => {
                (Some(h), None, data)
            }
            Some(crate::ui::primitive_hit_testing::PaneHit::Primitive(h)) => (None, Some(h), None),
            None => (None, None, None),
        };
        let marker = hit.as_ref().and_then(|h| match &h.target {
            super::HitTarget::Marker(m) => Some(m.clone()),
            _ => None,
        });
        PointerEvent {
            custom_series_data,
            custom_hit,
            primitive,
            hit,
            marker,
            pane,
            point,
            logical,
            time,
            price,
            series_data,
        }
    }
}
impl<B: super::ChartBehavior, M: Clone + 'static> Chart<B, M> {
    pub fn active_price_axis(&self) -> Option<(usize, PriceAxisSide)> {
        self.active_price_axis
    }
    pub fn reset_price_scale(
        &mut self,
        pane: usize,
        side: PriceAxisSide,
    ) -> Result<(), ChartError> {
        self.cancel_viewport_interaction();
        self.model.reset_price_scale(pane, side).map_err(Into::into)
    }
    pub fn zoom_time(&mut self, point: crate::model::coordinate::Coordinate, factor: f64) {
        self.cancel_viewport_interaction();
        self.model.zoom_time(point, factor);
    }
    pub fn set_crosshair_position(
        &mut self,
        pane: usize,
        point: Option<crate::renderers::line_renderer::Point>,
    ) -> Result<(), ChartError> {
        self.model
            .set_crosshair_position(pane, point)
            .map_err(Into::into)
    }
}
impl super::ChartState {
    pub fn is_axis_dragging(&self) -> bool {
        self.axis_dragging()
    }
}

impl<B: super::ChartBehavior, M: Clone + 'static> Chart<B, M> {
    fn validate_message(&self, message: &ChartMessage) -> Result<(), ChartError> {
        let point_valid = |point: iced::Point| point.x.is_finite() && point.y.is_finite();
        let input_valid = |input: &ChartInteraction| {
            input.position.is_none_or(point_valid)
                && match input.navigation {
                    Some(Navigation::Wheel { x, zoom, scroll }) => {
                        x.is_finite() && zoom.is_finite() && scroll.is_finite()
                    }
                    Some(Navigation::Pan { start, position }) => {
                        start.is_none_or(point_valid) && point_valid(position)
                    }
                    Some(Navigation::Scroll { start, x }) => {
                        start.is_none_or(f64::is_finite) && x.is_finite()
                    }
                    _ => true,
                }
        };
        let panes = self.model.panes().len();
        let valid = match message {
            ChartMessage::Touch(input) => {
                input.pane < panes
                    && match input.action {
                        super::TouchAction::Pan { start, position } => {
                            start.is_none_or(point_valid) && point_valid(position)
                        }
                        super::TouchAction::Pinch { anchor, zoom } => {
                            point_valid(anchor) && zoom.is_finite()
                        }
                        super::TouchAction::Track { position }
                        | super::TouchAction::Tap { position }
                        | super::TouchAction::DoubleTap { position } => point_valid(position),
                        _ => true,
                    }
            }
            ChartMessage::PrimitiveEdit(event) => {
                point_valid(event.start) && point_valid(event.position)
            }
            ChartMessage::Interaction(input) => panes > 0 && input_valid(input),
            ChartMessage::PaneInteraction(MultiPaneInteraction::Plot { pane, interaction }) => {
                *pane < panes && input_valid(interaction)
            }
            ChartMessage::PaneInteraction(MultiPaneInteraction::Resize {
                top,
                upper_stretch,
                lower_stretch,
            }) => {
                *top < panes.saturating_sub(1)
                    && upper_stretch.is_finite()
                    && *upper_stretch > 0.
                    && lower_stretch.is_finite()
                    && *lower_stretch > 0.
            }
            ChartMessage::PaneInteraction(MultiPaneInteraction::Cancel) => panes > 0,
            ChartMessage::TimeAxis(action) => match action {
                crate::ui::time_axis_input::TimeAxisAction::Start { x }
                | crate::ui::time_axis_input::TimeAxisAction::Move { x } => x.is_finite(),
                _ => true,
            },
            ChartMessage::PriceAxis(input) => {
                input.pane < panes
                    && match input.action {
                        PriceAxisAction::Start { y } | PriceAxisAction::Move { y } => y.is_finite(),
                        PriceAxisAction::Wheel { delta } => delta.is_finite(),
                        _ => true,
                    }
            }
            ChartMessage::Click { pane, position }
            | ChartMessage::DoubleClick { pane, position } => {
                *pane < panes && point_valid(*position)
            }
            ChartMessage::Bounds(size) => {
                size.width.is_finite()
                    && size.height.is_finite()
                    && size.width >= 0.
                    && size.height >= 0.
            }
            ChartMessage::Tick(_) | ChartMessage::AttributionActivated => true,
        };
        if valid {
            Ok(())
        } else {
            Err(ChartError::InvalidInteraction)
        }
    }
}
