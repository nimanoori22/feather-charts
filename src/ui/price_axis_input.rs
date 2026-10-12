//! Owned price-axis gestures shared by single- and multi-pane Canvas callers.
use crate::{
    model::axis_snapshots::PriceAxisSide,
    ui::chart_layout::{ChartLayout, LayoutRect},
};
use iced::{Point, advanced::mouse, widget::canvas::Event};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PriceAxisAction {
    Start { y: f64 },
    Move { y: f64 },
    End,
    Wheel { delta: f64 },
    Reset,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PriceAxisInteraction {
    pub pane: usize,
    pub side: PriceAxisSide,
    pub action: PriceAxisAction,
}
struct Drag {
    pane: usize,
    side: PriceAxisSide,
    origin_y: f64,
    height: f64,
    last_y: f32,
    available: crate::renderers::price_axis_renderer::AxisBounds,
    count: usize,
}
#[derive(Default)]
pub struct PriceAxisInput {
    drag: Option<Drag>,
    last_click: Option<(usize, PriceAxisSide, mouse::Click)>,
}
impl PriceAxisInput {
    pub fn target(&self) -> Option<(usize, PriceAxisSide)> {
        self.drag.as_ref().map(|drag| (drag.pane, drag.side))
    }
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }
    pub fn cancel(&mut self) -> Option<PriceAxisInteraction> {
        self.last_click = None;
        self.drag.take().map(|d| PriceAxisInteraction {
            pane: d.pane,
            side: d.side,
            action: PriceAxisAction::End,
        })
    }
    pub fn hit_test(
        cursor: Point,
        layouts: &[ChartLayout],
        enabled: &[[bool; 2]],
    ) -> Option<(usize, PriceAxisSide)> {
        if !cursor.x.is_finite() || !cursor.y.is_finite() {
            return None;
        }
        layouts.iter().enumerate().find_map(|(pane, layout)| {
            [PriceAxisSide::Left, PriceAxisSide::Right]
                .into_iter()
                .enumerate()
                .find_map(|(index, side)| {
                    let r = axis(layout, side);
                    (enabled.get(pane).is_some_and(|flags| flags[index])
                        && f64::from(cursor.x) >= r.origin.x
                        && f64::from(cursor.x) < r.origin.x + r.size.width
                        && f64::from(cursor.y) >= r.origin.y
                        && f64::from(cursor.y) < r.origin.y + r.size.height)
                        .then_some((pane, side))
                })
        })
    }
    pub fn update(
        &mut self,
        event: &Event,
        cursor: Option<Point>,
        layouts: &[ChartLayout],
        enabled: &[[bool; 2]],
    ) -> Option<PriceAxisInteraction> {
        let cursor = cursor.filter(|p| p.x.is_finite() && p.y.is_finite());
        let leaving = matches!(
            event,
            Event::Mouse(mouse::Event::CursorLeft) | Event::Window(iced::window::Event::Unfocused)
        );
        if let Some(drag) = &self.drag {
            let valid = layouts.get(drag.pane).is_some_and(|layout| {
                let r = axis(layout, drag.side);
                layout.available == drag.available
                    && layouts.len() == drag.count
                    && r.origin.y == drag.origin_y
                    && r.size.height == drag.height
                    && r.size.width > 0.
                    && enabled
                        .get(drag.pane)
                        .is_some_and(|flags| flags[side_index(drag.side)])
            });
            if leaving || !valid {
                return self.cancel();
            }
        } else if leaving {
            self.last_click = None;
            return None;
        }
        if let Some(drag) = &mut self.drag {
            let action = match event {
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    PriceAxisAction::End
                }
                Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                    let p = cursor?;
                    if drag.last_y == p.y {
                        return None;
                    }
                    drag.last_y = p.y;
                    if self
                        .last_click
                        .is_some_and(|(_, _, c)| c.position().distance(p) >= 6.)
                    {
                        self.last_click = None;
                    }
                    PriceAxisAction::Move {
                        y: f64::from(p.y) - drag.origin_y,
                    }
                }
                _ => return None,
            };
            let result = PriceAxisInteraction {
                pane: drag.pane,
                side: drag.side,
                action,
            };
            if action == PriceAxisAction::End {
                self.drag = None;
            }
            return Some(result);
        }
        let p = cursor?;
        let (pane, side) = Self::hit_test(p, layouts, enabled)?;
        let layout = &layouts[pane];
        let r = axis(layout, side);
        let action = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let previous = self
                    .last_click
                    .filter(|(old_pane, old_side, _)| *old_pane == pane && *old_side == side)
                    .map(|(_, _, c)| c);
                let click = mouse::Click::new(p, mouse::Button::Left, previous);
                self.last_click = Some((pane, side, click));
                if click.kind() == mouse::click::Kind::Double {
                    PriceAxisAction::Reset
                } else {
                    self.drag = Some(Drag {
                        pane,
                        side,
                        origin_y: r.origin.y,
                        height: r.size.height,
                        last_y: p.y,
                        available: layout.available,
                        count: layouts.len(),
                    });
                    PriceAxisAction::Start {
                        y: f64::from(p.y) - r.origin.y,
                    }
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => f64::from(*y) * 32.,
                    mouse::ScrollDelta::Pixels { y, .. } => f64::from(*y),
                };
                if !y.is_finite() || y == 0. {
                    return None;
                }
                self.last_click = None;
                PriceAxisAction::Wheel {
                    delta: (y / 100.).clamp(-1., 1.),
                }
            }
            _ => return None,
        };
        Some(PriceAxisInteraction { pane, side, action })
    }
}
fn side_index(side: PriceAxisSide) -> usize {
    match side {
        PriceAxisSide::Left => 0,
        PriceAxisSide::Right => 1,
    }
}
fn axis(layout: &ChartLayout, side: PriceAxisSide) -> LayoutRect {
    match side {
        PriceAxisSide::Left => layout.left_axis,
        PriceAxisSide::Right => layout.right_axis,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        renderers::{grid_renderer::PixelRatio, price_axis_renderer::AxisBounds},
        ui::chart_layout::{AxisSizeRequests, allocate_chart_layout},
    };
    fn layout() -> ChartLayout {
        allocate_chart_layout(
            AxisBounds {
                width: 800.,
                height: 500.,
            },
            AxisSizeRequests {
                left_width: 60.,
                right_width: 60.,
                time_height: 30.,
            },
            PixelRatio {
                horizontal: 1.25,
                vertical: 1.25,
            },
        )
        .unwrap()
    }
    fn mouse(e: mouse::Event) -> Event {
        Event::Mouse(e)
    }
    #[test]
    fn axis_drag_owns_pointer_outside_scale_and_suppresses_replay() {
        let layout = layout();
        let mut input = PriceAxisInput::default();
        let point = Point::new(770., 150.);
        let enabled = [[true, true]];
        let press = mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        assert_eq!(
            input
                .update(&press, Some(point), &[layout], &enabled)
                .unwrap()
                .action,
            PriceAxisAction::Start { y: 150. }
        );
        let outside = Point::new(200., 110.);
        let movement = mouse(mouse::Event::CursorMoved { position: outside });
        let action = input
            .update(&movement, Some(outside), &[layout], &enabled)
            .unwrap();
        assert_eq!(action.side, PriceAxisSide::Right);
        assert_eq!(action.action, PriceAxisAction::Move { y: 110. });
        assert!(
            input
                .update(&movement, Some(outside), &[layout], &enabled)
                .is_none()
        );
        assert_eq!(
            input
                .update(
                    &mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                    None,
                    &[layout],
                    &enabled
                )
                .unwrap()
                .action,
            PriceAxisAction::End
        );
        assert!(!input.is_dragging());
    }
    #[test]
    fn own_label_width_changes_keep_drag_but_resize_focus_and_disable_cancel() {
        for reason in 0..3 {
            let layout = layout();
            let mut input = PriceAxisInput::default();
            let point = Point::new(770., 150.);
            input
                .update(
                    &mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    Some(point),
                    &[layout],
                    &[[true, true]],
                )
                .unwrap();
            let mut wider = layout;
            wider.right_axis.origin.x -= 20.;
            wider.right_axis.size.width += 20.;
            let point = Point::new(770., 140.);
            let movement = mouse(mouse::Event::CursorMoved { position: point });
            assert_eq!(
                input
                    .update(&movement, Some(point), &[wider], &[[true, true]])
                    .unwrap()
                    .action,
                PriceAxisAction::Move { y: 140. }
            );
            let mut enabled = [[true, true]];
            let event = match reason {
                0 => {
                    wider.available.height += 10.;
                    movement
                }
                1 => Event::Window(iced::window::Event::Unfocused),
                _ => {
                    enabled[0][1] = false;
                    movement
                }
            };
            assert_eq!(
                input
                    .update(&event, Some(point), &[wider], &enabled)
                    .unwrap()
                    .action,
                PriceAxisAction::End
            );
            assert!(!input.is_dragging());
        }
    }
    #[test]
    fn wheels_normalize_pixels_and_lines_and_ignore_invalid_and_hidden_input() {
        let layout = layout();
        let point = Point::new(20., 150.);
        let mut input = PriceAxisInput::default();
        for delta in [
            mouse::ScrollDelta::Lines { x: 0., y: 1. },
            mouse::ScrollDelta::Pixels { x: 0., y: 32. },
        ] {
            let action = input
                .update(
                    &mouse(mouse::Event::WheelScrolled { delta }),
                    Some(point),
                    &[layout],
                    &[[true, true]],
                )
                .unwrap();
            assert_eq!(action.side, PriceAxisSide::Left);
            assert_eq!(action.action, PriceAxisAction::Wheel { delta: 0.32 });
        }
        for y in [0., f32::NAN, f32::INFINITY] {
            assert!(
                input
                    .update(
                        &mouse(mouse::Event::WheelScrolled {
                            delta: mouse::ScrollDelta::Pixels { x: 0., y }
                        }),
                        Some(point),
                        &[layout],
                        &[[true, true]]
                    )
                    .is_none()
            );
        }
        assert!(
            input
                .update(
                    &mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    Some(point),
                    &[layout],
                    &[[false, true]]
                )
                .is_none()
        );
        assert!(
            PriceAxisInput::hit_test(Point::new(400., 150.), &[layout], &[[true, true]]).is_none()
        );
        assert!(
            PriceAxisInput::hit_test(Point::new(20., 490.), &[layout], &[[true, true]]).is_none()
        );
    }
    #[test]
    fn double_click_resets_only_same_axis() {
        let layout = layout();
        let mut input = PriceAxisInput::default();
        let point = Point::new(770., 150.);
        let press = mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let release = mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        input
            .update(&press, Some(point), &[layout], &[[true, true]])
            .unwrap();
        input
            .update(&release, Some(point), &[layout], &[[true, true]])
            .unwrap();
        assert_eq!(
            input
                .update(&press, Some(point), &[layout], &[[true, true]])
                .unwrap()
                .action,
            PriceAxisAction::Reset
        );
        assert!(!input.is_dragging());
    }
}
