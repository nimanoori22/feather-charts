//! Borrowing Canvas adapter. Native input is recognized here; model mutation
//! happens synchronously in Chart::update on the host application's update path.
use super::{Chart, ChartMessage};
use crate::{
    renderers::price_axis_renderer::AxisBounds,
    ui::{
        chart_frame::IcedChartFrame,
        chart_input::{ChartInput, MultiPaneInput, MultiPaneInteraction},
        multi_pane_frame::IcedMultiPaneChartFrame,
        price_axis_input::PriceAxisInput,
    },
};
use iced::{
    Size, Theme,
    widget::canvas::{self, Frame, Geometry},
};
use std::time::{Duration, Instant};
impl Chart {
    pub fn program<Message>(
        &self,
        on_message: fn(ChartMessage) -> Message,
    ) -> ChartProgram<'_, Message> {
        ChartProgram {
            snapshot: self.frame(),
            multi: self.multi_frame(),
            stretches: self
                .model
                .panes()
                .iter()
                .map(|p| p.stretch_factor())
                .collect(),
            axis_scaling: self.axis_scaling(),
            active_price_axis: self.active_price_axis,
            animate: self.is_animating(),
            enabled: !self.model.time_scale().is_empty(),
            resize_panes: self.resize_panes,
            hover_color: self.hover_color,
            revision: self.revision,
            membership: self.series_ids().collect(),
            on_message,
        }
    }
    pub fn element(&self) -> iced::Element<'_, ChartMessage> {
        iced::widget::canvas::Canvas::new(self.program(std::convert::identity))
            .width(iced::Fill)
            .height(iced::Fill)
            .into()
    }
}
pub struct ChartProgram<'a, Message> {
    snapshot: &'a IcedChartFrame,
    multi: Option<&'a IcedMultiPaneChartFrame>,
    stretches: Vec<f64>,
    axis_scaling: Vec<[bool; 2]>,
    active_price_axis: Option<(usize, crate::model::axis_snapshots::PriceAxisSide)>,
    animate: bool,
    enabled: bool,
    resize_panes: bool,
    hover_color: iced::Color,
    revision: u64,
    membership: Vec<crate::model::data_layer::SeriesId>,
    on_message: fn(ChartMessage) -> Message,
}
#[derive(Default)]
pub struct ChartState {
    input: ChartInput,
    multi_input: MultiPaneInput,
    axis_input: PriceAxisInput,
    last_redraw: Option<Instant>,
    available: Option<AxisBounds>,
    revision: Option<u64>,
    membership: Vec<crate::model::data_layer::SeriesId>,
    click: Option<(usize, iced::Point)>,
}
impl<Message> ChartProgram<'_, Message> {
    fn publish(&self, message: ChartMessage) -> canvas::Action<Message> {
        canvas::Action::publish((self.on_message)(message))
    }
    fn axis_layouts(&self) -> &[crate::ui::chart_layout::ChartLayout] {
        self.multi
            .map_or(std::slice::from_ref(&self.snapshot.layout), |multi| {
                multi.layout.panes.as_slice()
            })
    }
}
impl<Message> canvas::Program<Message> for ChartProgram<'_, Message> {
    type State = ChartState;
    fn update(
        &self,
        state: &mut ChartState,
        event: &canvas::Event,
        bounds: iced::Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let revision_changed = state
            .revision
            .replace(self.revision)
            .is_some_and(|old| old != self.revision);
        let bounds_changed = state
            .available
            .replace(self.snapshot.layout.available)
            .is_some_and(|old| old != self.snapshot.layout.available);
        let membership_changed = state.revision.is_some() && state.membership != self.membership;
        if membership_changed {
            state.membership.clone_from(&self.membership);
        }
        if revision_changed || bounds_changed || membership_changed {
            let was_dragging = state.input.is_dragging()
                || state.multi_input.is_dragging()
                || state.axis_input.is_dragging();
            state.input = ChartInput::default();
            state.multi_input = MultiPaneInput::default();
            state.click = None;
            if let Some(end) = state.axis_input.cancel() {
                return Some(self.publish(ChartMessage::PriceAxis(end)));
            }
            if was_dragging || bounds_changed {
                return Some(
                    self.publish(ChartMessage::PaneInteraction(MultiPaneInteraction::Cancel)),
                );
            }
        }
        if state.axis_input.is_dragging()
            && state.axis_input.target() != self.active_price_axis
            && let Some(end) = state.axis_input.cancel()
        {
            return Some(self.publish(ChartMessage::PriceAxis(end)));
        }
        let local = cursor
            .position()
            .map(|p| iced::Point::new(p.x - bounds.x, p.y - bounds.y));
        match event {
            canvas::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
                state.click = local.and_then(|point| {
                    self.axis_layouts()
                        .iter()
                        .enumerate()
                        .find_map(|(pane, layout)| {
                            let plot = layout.plot;
                            (f64::from(point.x) >= plot.origin.x
                                && f64::from(point.x) < plot.origin.x + plot.size.width
                                && f64::from(point.y) >= plot.origin.y
                                && f64::from(point.y) < plot.origin.y + plot.size.height)
                                .then_some((pane, point))
                        })
                });
            }
            canvas::Event::Mouse(iced::mouse::Event::CursorMoved { .. }) => {
                if state.click.is_some_and(|(_, start)| {
                    local.is_none_or(|p| (p.x - start.x).abs() + (p.y - start.y).abs() >= 5.)
                }) {
                    state.click = None;
                }
            }
            canvas::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
            }
            canvas::Event::Mouse(iced::mouse::Event::WheelScrolled { .. })
            | canvas::Event::Window(iced::window::Event::Unfocused) => state.click = None,
            _ => {}
        }
        let clicked = if matches!(
            event,
            canvas::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left
            ))
        ) {
            state.click.take().and_then(|(pane, start)| {
                local.and_then(|point| {
                    let plot = self.axis_layouts().get(pane)?.plot;
                    ((point.x - start.x).abs() + (point.y - start.y).abs() < 5.
                        && f64::from(point.x) >= plot.origin.x
                        && f64::from(point.x) < plot.origin.x + plot.size.width
                        && f64::from(point.y) >= plot.origin.y
                        && f64::from(point.y) < plot.origin.y + plot.size.height)
                        .then_some(ChartMessage::Click {
                            pane,
                            position: iced::Point::new(
                                point.x - plot.origin.x as f32,
                                point.y - plot.origin.y as f32,
                            ),
                        })
                })
            })
        } else {
            None
        };
        let over_divider = self.multi.is_some_and(|multi| {
            local.is_some_and(|p| {
                multi.layout.separators.iter().any(|r| {
                    f64::from(p.y) >= r.origin.y - 4.
                        && f64::from(p.y) < r.origin.y + 5.
                        && p.x >= 0.
                        && f64::from(p.x) < r.size.width
                })
            })
        });
        let plot_dragging = if self.multi.is_some() {
            state.multi_input.is_dragging()
        } else {
            state.input.is_dragging()
        };
        if !plot_dragging && (!over_divider || state.axis_input.is_dragging()) {
            if let Some(action) =
                state
                    .axis_input
                    .update(event, local, self.axis_layouts(), &self.axis_scaling)
            {
                return Some(self.publish(ChartMessage::PriceAxis(action)).and_capture());
            }
            if state.axis_input.is_dragging() && matches!(event, canvas::Event::Mouse(_)) {
                return None;
            }
        }
        if !state.axis_input.is_dragging() {
            if let Some(multi) = self.multi {
                if let Some(action) = state.multi_input.update(
                    event,
                    cursor
                        .position()
                        .map(|p| iced::Point::new(p.x - bounds.x, p.y - bounds.y)),
                    &multi.layout,
                    &self.stretches,
                    self.enabled,
                    self.resize_panes,
                ) {
                    if let Some(clicked) = clicked {
                        return Some(self.publish(clicked).and_capture());
                    }
                    let capture = action.capture();
                    let publish = self.publish(ChartMessage::PaneInteraction(action));
                    return Some(if capture {
                        publish.and_capture()
                    } else {
                        publish
                    });
                }
            } else {
                if let Some(action) = state.input.update(
                    event,
                    cursor
                        .position()
                        .map(|p| iced::Point::new(p.x - bounds.x, p.y - bounds.y)),
                    self.snapshot.layout.plot,
                    self.enabled,
                ) {
                    if let Some(clicked) = clicked {
                        return Some(self.publish(clicked).and_capture());
                    }
                    let publish = self.publish(ChartMessage::Interaction(action));
                    return Some(if action.capture {
                        publish.and_capture()
                    } else {
                        publish
                    });
                }
            }
        }
        if let Some(clicked) = clicked {
            return Some(self.publish(clicked).and_capture());
        }
        if let canvas::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            if bounds.size()
                != Size::new(
                    self.snapshot.layout.available.width as f32,
                    self.snapshot.layout.available.height as f32,
                )
            {
                return Some(self.publish(ChartMessage::Bounds(bounds.size())));
            }
            if self.animate {
                let next = state
                    .last_redraw
                    .map(|last| last + Duration::from_millis(16))
                    .unwrap_or(*now);
                if *now < next {
                    return Some(canvas::Action::request_redraw_at(next));
                }
                state.last_redraw = Some(*now);
                return Some(self.publish(ChartMessage::Tick(*now)));
            }
            state.last_redraw = None;
        }
        None
    }
    fn mouse_interaction(
        &self,
        state: &ChartState,
        bounds: iced::Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> iced::mouse::Interaction {
        if state.input.is_dragging() || state.multi_input.is_dragging() {
            return iced::mouse::Interaction::Grabbing;
        }
        if state.axis_input.is_dragging()
            || cursor.position().is_some_and(|p| {
                PriceAxisInput::hit_test(
                    iced::Point::new(p.x - bounds.x, p.y - bounds.y),
                    self.axis_layouts(),
                    &self.axis_scaling,
                )
                .is_some()
            })
        {
            iced::mouse::Interaction::ResizingVertically
        } else {
            iced::mouse::Interaction::default()
        }
    }
    fn draw(
        &self,
        _state: &ChartState,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        if let Some(multi) = self.multi {
            multi.draw(&mut frame);
            if let Some(top) = _state.multi_input.hovered_divider
                && let Some(region) = multi.layout.separators.get(top)
            {
                let hover = iced::Rectangle::new(
                    iced::Point::new(0., region.origin.y as f32 - 4.),
                    Size::new(region.size.width as f32, 9.),
                );
                frame.with_clip(hover, |frame| {
                    frame.fill_rectangle(hover.position(), hover.size(), self.hover_color)
                });
            }
        } else {
            self.snapshot.draw(&mut frame);
        }
        vec![frame.into_geometry()]
    }
}

impl ChartState {
    pub(super) fn axis_dragging(&self) -> bool {
        self.axis_input.is_dragging()
    }
}
