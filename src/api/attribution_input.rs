//! Logo gestures own no chart state and never enter plot recognition.
use super::PrimitivePaneId;
use crate::renderers::attribution_logo_renderer::contains;
use iced::{Point, Rectangle, keyboard, mouse, touch, widget::canvas::Event};
use std::time::{Duration, Instant};
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Target {
    pub owner: PrimitivePaneId,
    pub bounds: Rectangle,
    pub dark: bool,
}
#[derive(Clone, Copy, PartialEq)]
enum Device {
    Mouse,
    Touch(touch::Finger),
}
struct Press {
    target: Target,
    device: Device,
    start: Point,
    valid: bool,
}
#[derive(Default)]
pub(super) struct Input {
    press: Option<Press>,
    suppress_mouse_until: Option<Instant>,
}
pub(super) struct Context {
    pub eligible: bool,
    pub invalidated: bool,
    pub now: Instant,
}
pub(super) enum Action {
    Pass,
    Capture,
    Activate,
}
impl Input {
    pub fn update(
        &mut self,
        event: &Event,
        mouse_point: Option<Point>,
        origin: Point,
        target: Option<Target>,
        context: Context,
    ) -> Action {
        let Context {
            eligible,
            invalidated,
            now,
        } = context;
        let point = match event {
            Event::Touch(
                touch::Event::FingerPressed { position, .. }
                | touch::Event::FingerMoved { position, .. }
                | touch::Event::FingerLifted { position, .. }
                | touch::Event::FingerLost { position, .. },
            ) => Some(Point::new(position.x - origin.x, position.y - origin.y)),
            _ => mouse_point,
        };
        if matches!(event,Event::Mouse(e) if !matches!(e,mouse::Event::WheelScrolled{..}))
            && self.suppress_mouse_until.is_some_and(|t| now < t)
        {
            return Action::Capture;
        }
        if let Some(press) = &mut self.press {
            press.valid &= !invalidated && target == Some(press.target);
            if matches!(
                event,
                Event::Window(iced::window::Event::Unfocused)
                    | Event::Mouse(mouse::Event::CursorLeft)
                    | Event::Keyboard(keyboard::Event::KeyPressed {
                        key: keyboard::Key::Named(keyboard::key::Named::Escape),
                        ..
                    })
            ) {
                self.press = None;
                return Action::Capture;
            }
            let slop = if press.device == Device::Mouse {
                5.
            } else {
                8.
            };
            let moved = match (press.device, event) {
                (
                    Device::Mouse,
                    Event::Mouse(
                        mouse::Event::CursorMoved { .. } | mouse::Event::ButtonReleased(_),
                    ),
                ) => true,
                (
                    Device::Touch(id),
                    Event::Touch(
                        touch::Event::FingerMoved { id: current, .. }
                        | touch::Event::FingerLifted { id: current, .. },
                    ),
                ) => id == *current,
                _ => false,
            };
            if moved {
                press.valid &= point.is_some_and(|p| {
                    contains(press.target.bounds, p)
                        && (p.x - press.start.x).abs() + (p.y - press.start.y).abs() < slop
                });
            }
            let released = match (press.device, event) {
                (
                    Device::Mouse,
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ) => true,
                (
                    Device::Touch(id),
                    Event::Touch(
                        touch::Event::FingerLifted { id: up, .. }
                        | touch::Event::FingerLost { id: up, .. },
                    ),
                ) => id == *up,
                _ => false,
            };
            if matches!(event, Event::Touch(_)) {
                self.suppress_mouse_until = Some(now + Duration::from_millis(500));
            }
            if matches!(
                event,
                Event::Touch(touch::Event::FingerLost { .. } | touch::Event::FingerPressed { .. })
                    | Event::Mouse(
                        mouse::Event::WheelScrolled { .. } | mouse::Event::ButtonPressed(_)
                    )
            ) {
                press.valid = false;
            }
            if released {
                let valid = press.valid;
                self.press = None;
                return if valid {
                    Action::Activate
                } else {
                    Action::Capture
                };
            }
            if matches!(event, Event::Mouse(_) | Event::Touch(_)) {
                return Action::Capture;
            }
        }
        if eligible
            && let Some(target) = target
            && let Some(point) = point.filter(|p| contains(target.bounds, *p))
        {
            let device = match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                    Some(Device::Mouse)
                }
                Event::Touch(touch::Event::FingerPressed { id, .. }) => Some(Device::Touch(*id)),
                _ => None,
            };
            if let Some(device) = device {
                if matches!(device, Device::Touch(_)) {
                    self.suppress_mouse_until = Some(now + Duration::from_millis(500));
                }
                self.press = Some(Press {
                    target,
                    device,
                    start: point,
                    valid: true,
                });
                return Action::Capture;
            }
        }
        Action::Pass
    }
}
