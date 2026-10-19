//! An application modifier-click consumer using only owned public notifications.
use super::*;
pub struct Panel<T, M> {
    pub last: Option<PointerInput>,
    pub pinned: Option<PointerEvent<T, M>>,
    pub clicks: usize,
    pub doubles: usize,
}
impl<C: Host> App<C> {
    pub fn enable_pointer(&mut self) {
        self.pointer = Some(Panel {
            last: None,
            pinned: None,
            clicks: 0,
            doubles: 0,
        });
        let mut interests = self.chart.core().event_interests();
        interests.double_click = true;
        self.chart.interests(interests);
    }
    pub fn observe_pointer(&mut self, event: &ChartEvent<C::Item, C::Metadata>) {
        let Some(panel) = &mut self.pointer else {
            return;
        };
        let (point, click) = match event {
            ChartEvent::Click(p) => {
                panel.clicks += 1;
                (p, true)
            }
            ChartEvent::DoubleClick(p) => {
                panel.doubles += 1;
                (p, true)
            }
            ChartEvent::CrosshairMoved(p) => (p, false),
            _ => return,
        };
        panel.last = point.source_event;
        if click && let Some(modifiers) = point.source_event.and_then(|p| p.modifiers) {
            if modifiers.shift() {
                panel.pinned = None;
            } else if modifiers.control() {
                panel.pinned = Some(point.clone());
            }
        }
    }
    pub fn pointer_controls(&self) -> Element<'_, Message> {
        let panel = self.pointer.as_ref().unwrap();
        let pinned = panel.pinned.as_ref().map_or_else(
            || "none".into(),
            |p| {
                format!(
                    "pane {:?}, price {:?}, {} built-in / {} custom rows",
                    p.pane,
                    p.price,
                    p.series_data.len(),
                    p.custom_series_data.len()
                )
            },
        );
        let source = panel.last.map_or_else(
            || "Source: none (programmatic/replay)".into(),
            |p| {
                let point = |p: Option<iced::Point>| {
                    p.map_or_else(|| "?".into(), |p| format!("{:.1},{:.1}", p.x, p.y))
                };
                format!(
                    "{:?} {:?} {:?} | modifiers {:?} | chart {} | window {}",
                    p.device,
                    p.kind,
                    p.button,
                    p.modifiers,
                    point(p.chart_position),
                    point(p.window_position)
                )
            },
        );
        let controls = row![
            text(format!(
                "Pinned: {pinned} | clicks {} / doubles {}",
                panel.clicks, panel.doubles
            )),
            button("Clear pin").on_press(Message::PointerClear),
            button("Programmatic click").on_press(Message::PointerProgrammatic),
            text("Ctrl-click: pin | Shift-click: clear | unknown modifiers: None"),
        ]
        .spacing(6);
        iced::widget::column![
            iced::widget::scrollable(controls).direction(
                iced::widget::scrollable::Direction::Horizontal(Default::default())
            ),
            iced::widget::scrollable(text(source).size(12).wrapping(text::Wrapping::None))
                .direction(iced::widget::scrollable::Direction::Horizontal(
                    Default::default()
                )),
        ]
        .spacing(4)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::{
        Event, Point,
        keyboard::{Event as Key, Modifiers},
        mouse,
        widget::canvas::Program,
    };
    fn send<C: Host>(app: &mut App<C>, state: &mut ChartState, event: Event, point: Point) {
        let message = app
            .chart
            .core()
            .program(std::convert::identity)
            .update(
                state,
                &event,
                iced::Rectangle::with_size(app.size),
                mouse::Cursor::Available(point),
            )
            .and_then(|a| a.into_inner().0);
        if let Some(message) = message {
            let events = app.chart.update(message).unwrap();
            app.consume_events(events);
        }
    }
    fn click<C: Host>(app: &mut App<C>, state: &mut ChartState, point: Point) {
        for event in [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            send(app, state, Event::Mouse(event), point);
        }
    }
    fn exercise<C: Host<Metadata = Rc<metadata::Record>>>() {
        let mut app = App::<C>::new(false);
        app.enable_pointer();
        app.size = Size::new(800., 500.);
        app.prepare(Instant::now());
        app.action(Message::Swap).unwrap();
        app.action(Message::LineNew).unwrap();
        app.prepare(Instant::now());
        let pane = app.chart.core().series_pane(app.line).unwrap();
        let plot = app.chart.core().multi_frame().unwrap().layout.panes[pane].plot;
        let point = Point::new(
            plot.origin.x as f32 + plot.size.width as f32 * 0.75,
            plot.origin.y as f32 + plot.size.height as f32 * 0.3,
        );
        let mut state = ChartState::default();
        send(
            &mut app,
            &mut state,
            Event::Keyboard(Key::ModifiersChanged(Modifiers::CTRL)),
            point,
        );
        click(&mut app, &mut state, point);
        let pin = app.pointer.as_ref().unwrap().pinned.as_ref().unwrap();
        assert_eq!(pin.pane, Some(pane));
        assert_eq!(pin.source_event.unwrap().modifiers, Some(Modifiers::CTRL));
        assert!(pin.series_data.iter().any(|(id, _)| *id == app.line));
        // A programmatic click carries no modifiers and cannot clear or replace the pin.
        app.action(Message::PointerProgrammatic).unwrap();
        assert!(app.pointer.as_ref().unwrap().last.is_none());
        assert_eq!(
            app.pointer.as_ref().unwrap().pinned.as_ref().unwrap().pane,
            Some(pane)
        );
        send(
            &mut app,
            &mut state,
            Event::Keyboard(Key::ModifiersChanged(Modifiers::SHIFT)),
            point,
        );
        click(&mut app, &mut state, point);
        assert!(app.pointer.as_ref().unwrap().pinned.is_none());
        app.action(Message::Reset).unwrap();
        assert!(app.pointer.is_some());
    }
    #[test]
    fn modifier_consumer_works_after_transfers_in_all_four_domains() {
        exercise::<Chart<HorzScaleBehaviorTime, Rc<metadata::Record>>>();
        exercise::<Chart<HorzScaleBehaviorPrice, Rc<metadata::Record>>>();
        exercise::<YieldCurveChart<Rc<metadata::Record>>>();
        exercise::<Chart<session_behavior::Sessions, Rc<metadata::Record>>>();
    }
}
