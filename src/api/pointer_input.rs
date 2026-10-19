//! Owned native input snapshots; coordinates use Iced logical pixels.
use super::{ChartMessage, TouchInteraction};
use crate::ui::{
    chart_input::{ChartInteraction, MultiPaneInteraction},
    price_axis_input::PriceAxisInteraction,
    time_axis_input::TimeAxisAction,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerDevice {
    Mouse,
    Touch(iced::touch::Finger),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerInputKind {
    Enter,
    Move,
    Press,
    Release,
    Leave,
    Wheel,
    Cancel,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerInput {
    pub device: PointerDevice,
    pub kind: PointerInputKind,
    /// The changed mouse button; not a set of held buttons. None for touch/move.
    pub button: Option<iced::mouse::Button>,
    /// Canvas-local, including axes and pane offsets; not pane-plot-local.
    pub chart_position: Option<iced::Point>,
    /// Window-content-relative; no desktop-screen/page coordinates are available.
    pub window_position: Option<iced::Point>,
    /// Last keyboard state observed by this Canvas; unknown after focus changes.
    pub modifiers: Option<iced::keyboard::Modifiers>,
}
impl PointerInput {
    pub(super) fn is_finite(self) -> bool {
        [self.chart_position, self.window_position]
            .into_iter()
            .flatten()
            .all(|p| p.x.is_finite() && p.y.is_finite())
    }
}
/// Recognized pointer actions, stored inline without recursive message allocation.
#[derive(Clone, Copy, Debug)]
pub enum PointerAction {
    Touch(TouchInteraction),
    Interaction(ChartInteraction),
    PaneInteraction(MultiPaneInteraction),
    PriceAxis(PriceAxisInteraction),
    TimeAxis(TimeAxisAction),
    Click { pane: usize, position: iced::Point },
    DoubleClick { pane: usize, position: iced::Point },
}
impl ChartMessage {
    /// Attach an owned source to pointer actions. Replaces an existing source;
    /// non-pointer actions (including animation ticks) deliberately remain unsourced.
    pub fn with_pointer_input(self, input: PointerInput) -> Self {
        let action = match self {
            Self::PointerInput { action, .. } => action,
            Self::Touch(a) => PointerAction::Touch(a),
            Self::Interaction(a) => PointerAction::Interaction(a),
            Self::PaneInteraction(a) => PointerAction::PaneInteraction(a),
            Self::PriceAxis(a) => PointerAction::PriceAxis(a),
            Self::TimeAxis(a) => PointerAction::TimeAxis(a),
            Self::Click { pane, position } => PointerAction::Click { pane, position },
            Self::DoubleClick { pane, position } => PointerAction::DoubleClick { pane, position },
            other => return other,
        };
        Self::PointerInput { action, input }
    }
    /// Recover a legacy action and its optional snapshot, e.g. in host adapters.
    pub fn into_pointer_parts(self) -> (Self, Option<PointerInput>) {
        let Self::PointerInput { action, input } = self else {
            return (self, None);
        };
        let message = match action {
            PointerAction::Touch(a) => Self::Touch(a),
            PointerAction::Interaction(a) => Self::Interaction(a),
            PointerAction::PaneInteraction(a) => Self::PaneInteraction(a),
            PointerAction::PriceAxis(a) => Self::PriceAxis(a),
            PointerAction::TimeAxis(a) => Self::TimeAxis(a),
            PointerAction::Click { pane, position } => Self::Click { pane, position },
            PointerAction::DoubleClick { pane, position } => Self::DoubleClick { pane, position },
        };
        (message, Some(input))
    }
}

#[derive(Default)]
pub(super) struct InputState {
    modifiers: Option<iced::keyboard::Modifiers>,
    touch_press: Option<PointerInput>,
}
impl InputState {
    pub fn observe(
        &mut self,
        event: &iced::Event,
        bounds: iced::Rectangle,
        cursor: iced::mouse::Cursor,
        unfocused: bool,
    ) -> Option<PointerInput> {
        use iced::{Event, keyboard, mouse, touch, window};
        match event {
            Event::Window(window::Event::Focused | window::Event::Unfocused) => {
                *self = Self::default();
                return None;
            }
            Event::Keyboard(
                keyboard::Event::ModifiersChanged(modifiers)
                | keyboard::Event::KeyPressed { modifiers, .. }
                | keyboard::Event::KeyReleased { modifiers, .. },
            ) if !unfocused => {
                self.modifiers = Some(*modifiers);
                return None;
            }
            _ => {}
        }
        if unfocused {
            return None;
        }
        let (device, kind, button, position) = match *event {
            Event::Mouse(event) => {
                let (kind, button, position) = match event {
                    mouse::Event::CursorMoved { position } => {
                        (PointerInputKind::Move, None, Some(position))
                    }
                    mouse::Event::ButtonPressed(button) => {
                        (PointerInputKind::Press, Some(button), cursor.position())
                    }
                    mouse::Event::ButtonReleased(button) => {
                        (PointerInputKind::Release, Some(button), cursor.position())
                    }
                    mouse::Event::WheelScrolled { .. } => {
                        (PointerInputKind::Wheel, None, cursor.position())
                    }
                    mouse::Event::CursorLeft => (PointerInputKind::Leave, None, None),
                    mouse::Event::CursorEntered => {
                        (PointerInputKind::Enter, None, cursor.position())
                    }
                };
                (PointerDevice::Mouse, kind, button, position)
            }
            Event::Touch(event) => {
                let (id, position, kind) = match event {
                    touch::Event::FingerPressed { id, position } => {
                        (id, position, PointerInputKind::Press)
                    }
                    touch::Event::FingerMoved { id, position } => {
                        (id, position, PointerInputKind::Move)
                    }
                    touch::Event::FingerLifted { id, position } => {
                        (id, position, PointerInputKind::Release)
                    }
                    touch::Event::FingerLost { id, position } => {
                        (id, position, PointerInputKind::Cancel)
                    }
                };
                (PointerDevice::Touch(id), kind, None, Some(position))
            }
            _ => return None,
        };
        let input = PointerInput {
            device,
            kind,
            button,
            chart_position: position.map(|p| iced::Point::new(p.x - bounds.x, p.y - bounds.y)),
            window_position: position,
            modifiers: self.modifiers,
        };
        input.is_finite().then_some(input)
    }
    pub fn touch_source(
        &mut self,
        action: super::TouchAction,
        current: Option<PointerInput>,
    ) -> Option<PointerInput> {
        match action {
            super::TouchAction::Press => self.touch_press = current,
            super::TouchAction::Track { .. } if current.is_none() => return self.touch_press,
            super::TouchAction::End
            | super::TouchAction::Cancel
            | super::TouchAction::ExitTracking => self.touch_press = None,
            _ => {}
        }
        current
    }
}
