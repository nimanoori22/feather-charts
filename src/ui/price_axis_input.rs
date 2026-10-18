//! Owned price-axis gestures shared by single- and multi-pane Canvas callers.
pub use super::time_axis_input::AxisGestureOptions;
use crate::{
    model::axis_snapshots::PriceAxisSide,
    ui::{
        axis_pointer::{Action, AxisPointer},
        chart_layout::{ChartLayout, LayoutRect},
    },
};
use iced::{Point, advanced::mouse, widget::canvas::Event};
use std::time::Instant;
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
struct Target {
    pane: usize,
    side: PriceAxisSide,
    layout: ChartLayout,
    count: usize,
}
#[derive(Default)]
pub struct PriceAxisInput {
    pub options: AxisGestureOptions,
    owner: Option<Target>,
    pointer: AxisPointer,
    scaling: bool,
    last_target: Option<(usize, PriceAxisSide)>,
}
impl PriceAxisInput {
    pub fn target(&self) -> Option<(usize, PriceAxisSide)> {
        self.owner.as_ref().map(|o| (o.pane, o.side))
    }
    pub fn is_dragging(&self) -> bool {
        self.pointer.active()
    }
    pub fn is_scaling(&self) -> bool {
        self.scaling
    }
    pub fn suppresses_mouse(&self, now: Instant) -> bool {
        self.pointer.suppresses_mouse(now)
    }
    pub fn cancel(&mut self) -> Option<PriceAxisInteraction> {
        self.pointer.cancel();
        self.scaling = false;
        self.last_target = None;
        self.owner.take().map(|o| PriceAxisInteraction {
            pane: o.pane,
            side: o.side,
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
        layouts.iter().enumerate().find_map(|(pane, l)| {
            [PriceAxisSide::Left, PriceAxisSide::Right]
                .into_iter()
                .enumerate()
                .find_map(|(index, side)| {
                    let r = axis(l, side);
                    (enabled.get(pane).is_some_and(|f| f[index])
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
        if let Some(o) = &self.owner {
            let old = axis(&o.layout, o.side);
            let valid = layouts.get(o.pane).is_some_and(|l| {
                let r = axis(l, o.side);
                l.available == o.layout.available
                    && layouts.len() == o.count
                    && r.origin.y == old.origin.y
                    && r.size.height == old.size.height
                    && r.size.width > 0.
                    && enabled.get(o.pane).is_some_and(|f| f[side_index(o.side)])
            });
            if !valid
                || self.scaling && !self.options.drag
                || (!self.options.drag && !self.options.reset)
            {
                return self.cancel();
            }
        }
        let hit = cursor.and_then(|p| Self::hit_test(p, layouts, enabled));
        if !self.pointer.active()
            && let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event
        {
            let (pane, side) = hit?;
            let y = match delta {
                mouse::ScrollDelta::Lines { y, .. } => f64::from(*y) * 32.,
                mouse::ScrollDelta::Pixels { y, .. } => f64::from(*y),
            };
            if !y.is_finite() || y == 0. {
                return None;
            }
            self.pointer.clear_pairs();
            return Some(PriceAxisInteraction {
                pane,
                side,
                action: PriceAxisAction::Wheel {
                    delta: (y / 100.).clamp(-1., 1.),
                },
            });
        }
        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                | Event::Touch(iced::touch::Event::FingerPressed { .. })
        ) && !self.pointer.active()
            && hit != self.last_target
        {
            self.pointer.clear_pairs();
        }
        let eligible = hit.is_some() && (self.options.drag || self.options.reset);
        let action =
            self.pointer
                .update(event, cursor, eligible, self.options, false, Instant::now())?;
        let action = match action {
            Action::Down(p) => {
                let (pane, side) = hit?;
                self.last_target = Some((pane, side));
                let layout = layouts[pane];
                self.owner = Some(Target {
                    pane,
                    side,
                    layout,
                    count: layouts.len(),
                });
                self.scaling = self.options.drag;
                if !self.options.drag {
                    return None;
                }
                PriceAxisAction::Start {
                    y: f64::from(p.y) - axis(&layout, side).origin.y,
                }
            }
            Action::Move(p) => {
                if !self.options.drag {
                    return None;
                }
                let o = self.owner.as_ref()?;
                PriceAxisAction::Move {
                    y: f64::from(p.y) - axis(&o.layout, o.side).origin.y,
                }
            }
            Action::Up { reset } => {
                if reset {
                    PriceAxisAction::Reset
                } else {
                    PriceAxisAction::End
                }
            }
            Action::Cancel => PriceAxisAction::End,
        };
        let o = self.owner.as_ref()?;
        let result = PriceAxisInteraction {
            pane: o.pane,
            side: o.side,
            action,
        };
        if matches!(action, PriceAxisAction::End | PriceAxisAction::Reset) {
            self.owner = None;
            self.scaling = false;
        }
        Some(result)
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
        assert!(matches!(
            input
                .update(&press, Some(point), &[layout], &[[true, true]])
                .unwrap()
                .action,
            PriceAxisAction::Start { .. }
        ));
        assert_eq!(
            input
                .update(&release, Some(point), &[layout], &[[true, true]])
                .unwrap()
                .action,
            PriceAxisAction::Reset
        );
        assert!(!input.is_dragging());
    }
}
