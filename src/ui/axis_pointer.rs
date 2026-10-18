//! Release pairing and native pointer ownership shared by axis controllers.
use crate::helpers::click_sequence::ClickSequence;
use iced::{Point, mouse, touch, widget::canvas::Event};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AxisGestureOptions {
    pub drag: bool,
    pub reset: bool,
    pub touch_drag: bool,
}
impl Default for AxisGestureOptions {
    fn default() -> Self {
        Self {
            drag: true,
            reset: true,
            touch_drag: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Pointer {
    Mouse,
    Touch(touch::Finger),
}
struct Press {
    pointer: Pointer,
    start: Point,
    last: Point,
    now: Instant,
    moved: bool,
}
pub(crate) enum Action {
    Down(Point),
    Move(Point),
    Up { reset: bool },
    Cancel,
}
#[derive(Default)]
pub(crate) struct AxisPointer {
    press: Option<Press>,
    mouse_pair: ClickSequence,
    touch_pair: ClickSequence,
    suppress_until: Option<Instant>,
}
impl AxisPointer {
    pub fn active(&self) -> bool {
        self.press.is_some()
    }
    pub fn suppresses_mouse(&self, now: Instant) -> bool {
        self.press
            .as_ref()
            .is_some_and(|p| matches!(p.pointer, Pointer::Touch(_)))
            || self.suppress_until.is_some_and(|until| now < until)
    }
    pub fn clear_pairs(&mut self) {
        self.mouse_pair.cancel();
        self.touch_pair.cancel();
    }
    pub fn cancel(&mut self) -> bool {
        self.clear_pairs();
        self.press.take().is_some()
    }
    pub fn update(
        &mut self,
        event: &Event,
        point: Option<Point>,
        eligible: bool,
        options: AxisGestureOptions,
        horizontal: bool,
        now: Instant,
    ) -> Option<Action> {
        if !options.reset {
            self.clear_pairs();
        }
        if matches!(
            event,
            Event::Window(iced::window::Event::Unfocused) | Event::Mouse(mouse::Event::CursorLeft)
        ) {
            return self.cancel().then_some(Action::Cancel);
        }
        let (pointer, kind, point) = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                (Pointer::Mouse, 0, point)
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => (Pointer::Mouse, 1, point),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                (Pointer::Mouse, 2, point)
            }
            Event::Touch(touch::Event::FingerPressed { id, .. }) => (Pointer::Touch(*id), 0, point),
            Event::Touch(touch::Event::FingerMoved { id, .. }) => (Pointer::Touch(*id), 1, point),
            Event::Touch(touch::Event::FingerLifted { id, .. }) => (Pointer::Touch(*id), 2, point),
            Event::Touch(touch::Event::FingerLost { id, .. }) => (Pointer::Touch(*id), 3, point),
            _ => return None,
        };
        if pointer == Pointer::Mouse && self.suppresses_mouse(now) {
            return None;
        }
        if let Some(press) = &self.press {
            if press.pointer != pointer {
                return None;
            }
        } else if kind != 0 || !eligible {
            if kind == 0 {
                self.clear_pairs();
            }
            return None;
        }
        let point = point.filter(|p| p.x.is_finite() && p.y.is_finite());
        if matches!(pointer, Pointer::Touch(_)) {
            self.suppress_until = Some(now + Duration::from_millis(500));
        }
        if kind == 3 {
            return self.cancel().then_some(Action::Cancel);
        }
        if kind == 0 {
            if self.press.is_some() {
                return None;
            }
            let point = point?;
            self.press = Some(Press {
                pointer,
                start: point,
                last: point,
                now,
                moved: false,
            });
            return Some(Action::Down(point));
        }
        if kind == 2 {
            let press = self.press.take()?;
            let pair = if pointer == Pointer::Mouse {
                &mut self.mouse_pair
            } else {
                &mut self.touch_pair
            };
            let double = if let Some(point) =
                point.filter(|p| !press.moved && distance(*p, press.start) < 5.)
            {
                options.reset
                    && pair.release(
                        0,
                        press.start,
                        press.now,
                        point,
                        now,
                        if pointer == Pointer::Mouse { 5. } else { 30. },
                    )
            } else {
                pair.cancel();
                false
            };
            return Some(Action::Up { reset: double });
        }
        let Some(point) = point else {
            return self.cancel().then_some(Action::Cancel);
        };
        let press = self.press.as_mut()?;
        if press.last == point {
            return None;
        }
        press.last = point;
        if !press.moved {
            if distance(point, press.start) < 5. {
                return None;
            }
            press.moved = true;
            self.mouse_pair.cancel();
            self.touch_pair.cancel();
            if matches!(pointer, Pointer::Touch(_)) {
                let vertical =
                    (point.y - press.start.y).abs() >= (point.x - press.start.x).abs() * 0.5;
                if !options.touch_drag || vertical == horizontal {
                    return self.cancel().then_some(Action::Cancel);
                }
            }
        }
        Some(Action::Move(point))
    }
}
fn distance(a: Point, b: Point) -> f32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
