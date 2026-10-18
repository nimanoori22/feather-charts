//! External application consumer: uses public helpers, never private projections.
use super::*;
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Range,
    Reset,
    Live,
    Inspect,
    Crosshair,
    Clear,
    Lines,
    Invalid,
    Sync,
}
pub struct Panel<C: Host> {
    pub peer: C,
    pub peer_line: SeriesId,
    pub size: Size,
    pub sync: bool,
    pub deliveries: usize,
    pub status: String,
}
impl<C: Host> App<C> {
    pub fn enable_helpers(&mut self) -> Result<(), ChartError> {
        let mut peer = C::new();
        let peer_line = peer.line(0, false);
        peer.raw_data(peer_line, (0..24).map(|i| line_row::<C>(i, 0.)).collect())?;
        peer.fit();
        self.helpers = Some(Panel {
            peer,
            peer_line,
            size: Size::ZERO,
            sync: true,
            deliveries: 0,
            status: "Inspect reports current ownership and conversions. Readouts update on demand."
                .into(),
        });
        Ok(())
    }
    pub fn helper_action(&mut self, action: Action) -> Result<(), ChartError> {
        if matches!(action, Action::Sync) {
            if let Some(panel) = &mut self.helpers {
                panel.sync = !panel.sync;
            }
            return Ok(());
        }
        let id = self.selected();
        let mut status = self.chart.helper_action(action, id)?;
        if self.selected_custom {
            let visible = self.custom.with_options(|options| options.common.visible);
            let color = self
                .custom
                .with_view_options(self.chart.core(), |options| options.color)?;
            status.push_str(&format!(" custom visible={visible} view color={color:?}"));
        } else {
            let visible = self
                .chart
                .core()
                .with_series_options(id, |options| match options {
                    SeriesOptionsMap::Line(options) => options.common.visible,
                    _ => false,
                })?;
            status.push_str(&format!(" visible={visible}"));
        }
        if let Some(panel) = &mut self.helpers {
            panel.status = status;
        }
        Ok(())
    }
    pub fn sync_helper_crosshair(&mut self, event: &ChartEvent<C::Item>) {
        let Some(panel) = &mut self.helpers else {
            return;
        };
        if !panel.sync {
            return;
        }
        if let ChartEvent::CrosshairMoved(pointer) = event {
            let result = panel.peer.synthetic(
                panel.peer_line,
                pointer.time.as_ref(),
                pointer.price.unwrap_or(4.5),
            );
            match result {
                Ok(()) => panel.deliveries += 1,
                Err(error) => panel.status = error.to_string(),
            }
            if let Err(error) = panel.peer.prepare(panel.size, self.ratio, Instant::now()) {
                panel.status = error.to_string();
            }
        }
    }
    pub fn update_peer(&mut self, message: ChartMessage) {
        let Some(panel) = &mut self.helpers else {
            return;
        };
        if let ChartMessage::Bounds(size) = message {
            panel.size = size;
        }
        match panel.peer.update(message) {
            Err(error) => panel.status = error.to_string(),
            Ok(events) => {
                if panel.sync {
                    for event in events {
                        if let ChartEvent::CrosshairMoved(pointer) = event {
                            if let Err(error) = self.chart.synthetic(
                                self.line,
                                pointer.time.as_ref(),
                                pointer.price.unwrap_or(4.5),
                            ) {
                                panel.status = error.to_string();
                            }
                            panel.deliveries += 1;
                            if let Err(error) =
                                self.chart.prepare(self.size, self.ratio, Instant::now())
                            {
                                panel.status = error.to_string();
                            }
                        }
                    }
                }
            }
        }
    }
    pub fn helper_controls(&self) -> Element<'_, Message> {
        let panel = self.helpers.as_ref().unwrap();
        let controls = [
            ("Items 4..18", Action::Range),
            ("Reset axis", Action::Reset),
            ("Go live", Action::Live),
            ("Inspect", Action::Inspect),
            ("Place crosshair", Action::Crosshair),
            ("Clear crosshair", Action::Clear),
            ("Add/remove line", Action::Lines),
            ("Invalid range", Action::Invalid),
            (
                if panel.sync { "Sync: on" } else { "Sync: off" },
                Action::Sync,
            ),
        ];
        let buttons = controls
            .into_iter()
            .fold(row![].spacing(6), |row, (label, action)| {
                row.push(button(label).on_press(Message::Helper(action)))
            });
        iced::widget::column![
            iced::widget::scrollable(buttons).direction(
                iced::widget::scrollable::Direction::Horizontal(Default::default())
            ),
            text(format!(
                "{} | sync deliveries={}",
                panel.status, panel.deliveries
            ))
            .size(12)
            .wrapping(text::Wrapping::None)
        ]
        .spacing(4)
        .into()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn consumer<C: Host>() {
        let mut app = App::<C>::new(false);
        app.enable_helpers().unwrap();
        app.size = Size::new(800., 500.);
        app.helpers.as_mut().unwrap().size = Size::new(800., 250.);
        app.prepare(Instant::now());
        for custom in [false, true] {
            app.selected_custom = custom;
            for action in [
                Action::Range,
                Action::Inspect,
                Action::Lines,
                Action::Invalid,
                Action::Crosshair,
                Action::Clear,
                Action::Reset,
                Action::Live,
            ] {
                app.helper_action(action).unwrap();
                app.prepare(Instant::now());
            }
            let id = app.selected();
            assert_eq!(app.chart.core().price_lines(id).unwrap().len(), 1);
            app.action(Message::LineNew).unwrap();
            app.prepare(Instant::now());
            app.helper_action(Action::Inspect).unwrap();
        }
        let point = feather_charts::ui::chart_input::ChartInteraction {
            position: Some(iced::Point::new(
                app.chart
                    .core()
                    .horizontal_to_coordinate(&C::key(12))
                    .unwrap()
                    .unwrap() as f32,
                30.,
            )),
            navigation: None,
            capture: false,
        };
        let events = app.chart.update(ChartMessage::Interaction(point)).unwrap();
        app.consume_events(events);
        assert!(app.helpers.as_ref().unwrap().deliveries > 0);
        assert!(
            app.helpers
                .as_ref()
                .unwrap()
                .peer
                .core()
                .model()
                .crosshair()
                .origin()
                .is_some()
        );
        app.helper_action(Action::Sync).unwrap();
        let before = app.helpers.as_ref().unwrap().deliveries;
        app.prepare(Instant::now());
        assert_eq!(app.helpers.as_ref().unwrap().deliveries, before);
    }
    #[test]
    fn calendar_consumer() {
        consumer::<Chart>();
    }
    #[test]
    fn strike_consumer() {
        consumer::<OptionsChart>();
    }
    #[test]
    fn maturity_consumer() {
        consumer::<YieldCurveChart>();
    }
    #[test]
    fn sessions_consumer() {
        consumer::<Chart<session_behavior::Sessions>>();
    }
}
