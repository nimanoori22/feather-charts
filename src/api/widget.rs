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
    widget::canvas::{self, Geometry},
};
use std::time::{Duration, Instant};
impl<B: super::ChartBehavior> Chart<B> {
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
            marker_deadline: self.next_up_down_expiration(),
            enabled: !self.model.time_scale().is_empty(),
            resize_panes: self.resize_panes,
            hover_color: self.hover_color,
            revision: self.revision,
            membership: self.series_ids().collect(),
            primitive_edit: self.primitive_edit_id(),
            touch_options: self.touch_options,
            touch_epoch: self.touch_epoch,
            applied_crosshair: self.model.crosshair().position().and_then(|p| {
                p.y.map(|y| (p.pane, iced::Point::new(p.x as f32, y as f32)))
            }),
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
    marker_deadline: Option<Instant>,
    enabled: bool,
    resize_panes: bool,
    hover_color: iced::Color,
    revision: u64,
    membership: Vec<crate::model::data_layer::SeriesId>,
    primitive_edit: Option<super::PrimitiveEditId>,
    touch_options: super::TouchNavigationOptions,
    touch_epoch: u64,
    applied_crosshair: Option<(usize, iced::Point)>,
    on_message: fn(ChartMessage) -> Message,
}
#[derive(Default)]
pub struct ChartState {
    geometry: canvas::Cache,
    touch_input: super::touch_input::TouchInput,
    touch_epoch: Option<u64>,
    edit_input: super::primitive_edit_input::EditInput,
    unfocused: bool,
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
        match event {
            canvas::Event::Window(iced::window::Event::Unfocused) => state.unfocused = true,
            canvas::Event::Window(iced::window::Event::Focused) => state.unfocused = false,
            _ => {}
        }
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
        let now = match event {
            canvas::Event::Window(iced::window::Event::RedrawRequested(now)) => *now,
            _ => Instant::now(),
        };
        if matches!(
            event,
            canvas::Event::Window(iced::window::Event::RedrawRequested(_))
        ) && bounds.size()
            != Size::new(
                self.snapshot.layout.available.width as f32,
                self.snapshot.layout.available.height as f32,
            )
        {
            state.touch_input.cancel(now);
            return Some(self.publish(ChartMessage::Bounds(bounds.size())));
        }
        let touch_changed = state
            .touch_epoch
            .replace(self.touch_epoch)
            .is_some_and(|old| old != self.touch_epoch);
        if state.touch_input.is_active()
            && (touch_changed
                || bounds_changed
                || membership_changed
                || state.unfocused
                || !self.enabled
                || self.primitive_edit.is_some()
                || !state.touch_input.owner_matches(self.axis_layouts()))
            && let Some(mut output) = state.touch_input.cancel(now)
        {
            // A removed pane cannot receive an input message. Model-side cancellation
            // already ended its interaction; clear tracking through a surviving pane.
            if output.input.pane >= self.axis_layouts().len() {
                output.input.pane = 0;
            }
            return Some(self.publish(ChartMessage::Touch(output.input)));
        }
        if matches!(event, canvas::Event::Mouse(event) if !matches!(event, iced::mouse::Event::WheelScrolled { .. }))
            && state.touch_input.suppresses_mouse(now)
        {
            return None;
        }
        if self.enabled
            && !state.unfocused
            && self.primitive_edit.is_none()
            && !state.axis_input.is_dragging()
            && !state.input.is_dragging()
            && !state.multi_input.is_dragging()
            && let Some(output) = state.touch_input.update(
                event,
                bounds,
                self.axis_layouts(),
                self.touch_options,
                self.applied_crosshair,
                now,
            )
        {
            let action = self.publish(ChartMessage::Touch(output.input));
            return Some(if output.capture {
                action.and_capture()
            } else {
                action
            });
        }
        let local = cursor
            .position()
            .map(|p| iced::Point::new(p.x - bounds.x, p.y - bounds.y));
        let target = if self.enabled
            && !state.unfocused
            && !state.axis_input.is_dragging()
            && !state.input.is_dragging()
            && !state.multi_input.is_dragging()
            && matches!(
                event,
                canvas::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left))
            ) {
            local.and_then(|p| {
                self.axis_layouts()
                    .iter()
                    .enumerate()
                    .find_map(|(pane, layout)| {
                        let position = iced::Point::new(
                            p.x - layout.plot.origin.x as f32,
                            p.y - layout.plot.origin.y as f32,
                        );
                        let frame = self
                            .multi
                            .and_then(|m| m.panes.get(pane))
                            .unwrap_or(self.snapshot);
                        if frame.plot.primitives.is_empty() {
                            return None;
                        }
                        frame
                            .plot
                            .primitive_hit(position)
                            .map(|hit| (pane, layout.plot, position, hit))
                    })
            })
        } else {
            None
        };
        match state.edit_input.update(
            event,
            local,
            target,
            self.primitive_edit,
            revision_changed || bounds_changed || membership_changed,
        ) {
            super::primitive_edit_input::EditAction::Capture(message) => {
                state.touch_input.cancel(now);
                state.click = None;
                state.input = ChartInput::default();
                state.multi_input = MultiPaneInput::default();
                return Some(message.map_or_else(
                    || canvas::Action::capture(),
                    |e| self.publish(ChartMessage::PrimitiveEdit(e)).and_capture(),
                ));
            }
            super::primitive_edit_input::EditAction::Pass => {}
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
                state.touch_input.cancel(now);
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
                    state.touch_input.cancel(now);
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
                    state.touch_input.cancel(now);
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
            if self
                .marker_deadline
                .is_some_and(|deadline| deadline <= *now)
            {
                return Some(self.publish(ChartMessage::Tick(*now)));
            }
            let deadline = state
                .touch_input
                .deadline()
                .into_iter()
                .chain(self.marker_deadline)
                .min();
            if self.animate {
                let next = state
                    .last_redraw
                    .map(|last| last + Duration::from_millis(16))
                    .unwrap_or(*now);
                if *now < next {
                    return Some(canvas::Action::request_redraw_at(
                        deadline.map_or(next, |deadline| deadline.min(next)),
                    ));
                }
                state.last_redraw = Some(*now);
                return Some(self.publish(ChartMessage::Tick(*now)));
            }
            state.last_redraw = None;
            if let Some(deadline) = deadline {
                return Some(canvas::Action::request_redraw_at(deadline));
            }
        }
        None
    }
    fn mouse_interaction(
        &self,
        state: &ChartState,
        bounds: iced::Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> iced::mouse::Interaction {
        if state.unfocused {
            return iced::mouse::Interaction::default();
        }
        if let Some(cursor) = state.edit_input.cursor(self.primitive_edit) {
            return cursor;
        }
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
            local_primitive_cursor(
                self.axis_layouts(),
                self.multi,
                self.snapshot,
                cursor
                    .position()
                    .map(|p| iced::Point::new(p.x - bounds.x, p.y - bounds.y)),
            )
            .unwrap_or_default()
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
        // Cached geometry gives the compositor one damage boundary for the chart.
        _state.geometry.clear();
        let geometry = _state.geometry.draw(renderer, bounds.size(), |frame| {
            if let Some(multi) = self.multi {
                multi.draw(frame);
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
                self.snapshot.draw(frame);
            }
        });
        vec![geometry]
    }
}

impl ChartState {
    pub(super) fn axis_dragging(&self) -> bool {
        self.axis_input.is_dragging()
    }
}

fn local_primitive_cursor(
    layouts: &[crate::ui::chart_layout::ChartLayout],
    multi: Option<&IcedMultiPaneChartFrame>,
    snapshot: &IcedChartFrame,
    point: Option<iced::Point>,
) -> Option<iced::mouse::Interaction> {
    let point = point?;
    layouts.iter().enumerate().find_map(|(index, l)| {
        let local = iced::Point::new(
            point.x - l.plot.origin.x as f32,
            point.y - l.plot.origin.y as f32,
        );
        let plot = multi.and_then(|m| m.panes.get(index)).unwrap_or(snapshot);
        match plot.plot.selection(local)? {
            crate::ui::primitive_hit_testing::PaneHit::Primitive(hit) => hit.cursor,
            crate::ui::primitive_hit_testing::PaneHit::BuiltIn(hit, _) => match hit.target {
                super::HitTarget::CustomSeries {
                    cursor_style: Some(style),
                    ..
                } => match style.as_str() {
                    "pointer" => Some(iced::mouse::Interaction::Pointer),
                    "crosshair" => Some(iced::mouse::Interaction::Crosshair),
                    "grab" => Some(iced::mouse::Interaction::Grab),
                    "grabbing" => Some(iced::mouse::Interaction::Grabbing),
                    "ns-resize" => Some(iced::mouse::Interaction::ResizingVertically),
                    "ew-resize" => Some(iced::mouse::Interaction::ResizingHorizontally),
                    _ => None,
                },
                _ => None,
            },
        }
    })
}
