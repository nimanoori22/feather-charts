use super::{
    PrimitiveEditCancelReason as Cancel, PrimitiveEditEvent, PrimitiveEditId,
    PrimitiveEditPhase as Phase, PrimitiveHit,
};
use crate::ui::chart_layout::LayoutRect;
use iced::{Point, keyboard, mouse, widget::canvas::Event};

#[derive(Default)]
pub(super) struct EditInput {
    capture: Option<Capture>,
}
struct Capture {
    event: PrimitiveEditEvent,
    plot: LayoutRect,
    dragged: bool,
}
pub(super) enum EditAction {
    Pass,
    Capture(Option<PrimitiveEditEvent>),
}
impl EditInput {
    pub fn cursor(&self) -> Option<mouse::Interaction> {
        self.capture.as_ref().map(|c| match c.event.hit.cursor {
            Some(mouse::Interaction::Grab) => mouse::Interaction::Grabbing,
            cursor => cursor.unwrap_or(mouse::Interaction::Grabbing),
        })
    }
    pub fn update(
        &mut self,
        event: &Event,
        local: Option<Point>,
        target: Option<(usize, LayoutRect, Point, PrimitiveHit)>,
        active: Option<PrimitiveEditId>,
        invalidated: bool,
    ) -> EditAction {
        if let Some(capture) = &mut self.capture {
            if active != Some(capture.event.id) {
                self.capture = None;
                return if matches!(event, Event::Mouse(_)) {
                    EditAction::Capture(None)
                } else {
                    EditAction::Pass
                };
            }
            let cancellation = if invalidated {
                Some(Cancel::ViewportChanged)
            } else {
                match event {
                    Event::Keyboard(keyboard::Event::KeyPressed {
                        key: keyboard::Key::Named(keyboard::key::Named::Escape),
                        ..
                    }) => Some(Cancel::Escape),
                    Event::Window(iced::window::Event::Unfocused) => Some(Cancel::FocusLost),
                    Event::Mouse(mouse::Event::CursorLeft) => Some(Cancel::WindowLeft),
                    _ => None,
                }
            };
            if let Some(reason) = cancellation {
                let mut c = self.capture.take().unwrap();
                c.event.phase = Phase::Cancel(reason);
                return EditAction::Capture(Some(c.event));
            }
            let position = local
                .filter(|p| p.x.is_finite() && p.y.is_finite())
                .map(|p| {
                    Point::new(
                        p.x - capture.plot.origin.x as f32,
                        p.y - capture.plot.origin.y as f32,
                    )
                });
            match event {
                Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                    if let Some(p) = position {
                        capture.dragged |= (p.x - capture.event.start.x).abs()
                            + (p.y - capture.event.start.y).abs()
                            >= 5.;
                        if capture.dragged && p != capture.event.position {
                            capture.event.position = p;
                            capture.event.phase = Phase::Update;
                            return EditAction::Capture(Some(capture.event.clone()));
                        }
                    }
                    return EditAction::Capture(None);
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    let mut c = self.capture.take().unwrap();
                    if c.dragged
                        && let Some(p) = position
                    {
                        c.event.position = p;
                    }
                    c.event.phase = Phase::Commit;
                    return EditAction::Capture(Some(c.event));
                }
                Event::Mouse(_) | Event::Keyboard(_) => return EditAction::Capture(None),
                _ => return EditAction::Pass,
            }
        }
        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        ) && let Some((pane, plot, position, hit)) = target.filter(|(_, _, _, h)| h.editable)
        {
            let event = PrimitiveEditEvent {
                id: PrimitiveEditId::new(),
                hit,
                pane,
                start: position,
                position,
                phase: Phase::Begin,
            };
            self.capture = Some(Capture {
                event: event.clone(),
                plot,
                dragged: false,
            });
            return EditAction::Capture(Some(event));
        }
        EditAction::Pass
    }
}
