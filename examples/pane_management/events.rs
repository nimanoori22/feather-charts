use super::*;
use iced::widget::column;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Toggle(usize),
    Clear,
    Axis,
    Fractional,
    Replace,
    Change,
}
pub struct Monitor {
    interests: EventInterests,
    counts: [usize; 5],
    latest: [String; 5],
    history: VecDeque<String>,
}
impl Default for Monitor {
    fn default() -> Self {
        Self {
            interests: EventInterests::ALL,
            counts: [0; 5],
            latest: Default::default(),
            history: Default::default(),
        }
    }
}
impl Monitor {
    pub fn observe<T: std::fmt::Debug>(&mut self, event: &ChartEvent<T>) {
        let index = match event {
            ChartEvent::DataChanged { .. } => 0,
            ChartEvent::DoubleClick(_) => 1,
            ChartEvent::VisibleLogicalRangeChanged(_) => 2,
            ChartEvent::VisibleHorizontalRangeChanged(_) => 3,
            ChartEvent::HorizontalAxisSizeChanged { .. } => 4,
            _ => return,
        };
        let summary = match event {
            ChartEvent::DoubleClick(p) => format!(
                "Double: pane {:?}, item {:?}, logical {:?}",
                p.pane, p.time, p.logical
            ),
            _ => format!("{event:?}"),
        };
        self.counts[index] += 1;
        self.latest[index] = summary.clone();
        self.history.push_back(summary);
        if self.history.len() > 12 {
            self.history.pop_front();
        }
    }
}
impl<C: Host> App<C> {
    pub(super) fn with_public_events(
        live: bool,
        ordered: bool,
        data: bool,
        config: bool,
        enabled: bool,
    ) -> Self {
        let mut app = Self::with_configuration(live, ordered || enabled, data || enabled, config);
        if enabled {
            app.events = Some(Monitor::default());
            app.chart.interests(EventInterests::ALL);
        }
        app
    }
    pub(super) fn event_action(&mut self, action: Action) -> Result<(), ChartError> {
        match action {
            Action::Toggle(index) => {
                let Some(m) = self.events.as_mut() else {
                    return Ok(());
                };
                let i = &mut m.interests;
                let flags = [
                    &mut i.data_changed,
                    &mut i.double_click,
                    &mut i.visible_logical_range,
                    &mut i.visible_horizontal_range,
                    &mut i.horizontal_axis_size,
                ];
                if let Some(flag) = flags.into_iter().nth(index) {
                    *flag = !*flag;
                }
                self.chart.interests(*i);
            }
            Action::Clear => {
                if let Some(m) = &mut self.events {
                    let interests = m.interests;
                    *m = Monitor::default();
                    m.interests = interests;
                }
            }
            Action::Axis => self
                .chart
                .axis_visible(!self.chart.core().model().time_scale().options().visible),
            Action::Fractional => {
                let offset = self.chart.core().model().time_scale().right_offset();
                self.chart.offset(offset + 0.1);
            }
            Action::Replace => {
                if self.selected_custom {
                    self.chart.custom_data(
                        &self.custom,
                        self.custom
                            .data()
                            .into_iter()
                            .map(CustomSeriesDataItem::Data)
                            .collect(),
                    )?;
                } else {
                    let rows = self.chart.core().series_data(self.line)?;
                    self.chart.raw_data(self.line, rows)?;
                }
            }
            Action::Change => {
                if self.selected_custom {
                    if let Some(mut row) = self.custom.data().last().cloned() {
                        row.value += 0.01;
                        row.high += 0.01;
                        self.chart
                            .update_custom(&self.custom, CustomSeriesDataItem::Data(row))?;
                    }
                } else if let Some(mut row) =
                    self.chart.core().series_data(self.line)?.last().cloned()
                {
                    if let BuiltInSeriesDataItem::Line(LineDataItem::Data(d)) = &mut row {
                        d.value += 0.01;
                    }
                    self.chart.update_data(self.markers, row, Instant::now())?;
                }
            }
        }
        Ok(())
    }
    pub(super) fn event_controls(&self) -> Element<'_, Message> {
        let Some(m) = &self.events else {
            return row![].into();
        };
        let i = m.interests;
        let mut controls = row![];
        for (index, (name, enabled)) in [
            ("Data", i.data_changed),
            ("Double", i.double_click),
            ("Logical", i.visible_logical_range),
            ("Horizontal", i.visible_horizontal_range),
            ("Axis size", i.horizontal_axis_size),
        ]
        .into_iter()
        .enumerate()
        {
            controls = controls.push(
                button(text(format!(
                    "{name}: {} ({})",
                    if enabled { "on" } else { "off" },
                    m.counts[index]
                )))
                .on_press(Message::Events(Action::Toggle(index))),
            );
        }
        for (name, action) in [
            ("Clear events", Action::Clear),
            ("Axis visible", Action::Axis),
            ("Fractional +0.1", Action::Fractional),
            ("Replace equal", Action::Replace),
            ("Change latest", Action::Change),
        ] {
            controls = controls.push(button(name).on_press(Message::Events(action)));
        }
        controls = controls.push(iced::widget::tooltip(
            text("Recent events").size(12),
            text(m.history.iter().cloned().collect::<Vec<_>>().join("\n")).size(12),
            iced::widget::tooltip::Position::Bottom,
        ));
        column![
            iced::widget::scrollable(controls.spacing(6)).direction(
                iced::widget::scrollable::Direction::Horizontal(Default::default())
            ),
            iced::widget::scrollable(
                text(
                    m.latest
                        .iter()
                        .filter(|s| !s.is_empty())
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(" | ")
                )
                .size(12)
                .wrapping(text::Wrapping::None)
            )
            .direction(iced::widget::scrollable::Direction::Horizontal(
                Default::default()
            ))
        ]
        .spacing(4)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn exercise<C: Host>() {
        let mut app = App::<C>::with_public_events(false, false, false, false, true);
        app.size = Size::new(800., 500.);
        app.prepare(Instant::now());
        assert!(app.events.as_ref().unwrap().counts[4] > 0);
        assert!(app.chart.take_events().is_empty());
        app.event_action(Action::Clear).unwrap();
        app.event_action(Action::Replace).unwrap();
        app.prepare(Instant::now());
        assert_eq!(app.events.as_ref().unwrap().counts[0], 1);
        app.event_action(Action::Change).unwrap();
        let batch = app
            .chart
            .update(ChartMessage::Tick(Instant::now()))
            .unwrap();
        app.consume_events(batch);
        assert_eq!(app.events.as_ref().unwrap().counts[0], 2);
        assert!(app.chart.take_events().is_empty());
        app.event_action(Action::Toggle(0)).unwrap();
        app.event_action(Action::Change).unwrap();
        app.prepare(Instant::now());
        assert_eq!(app.events.as_ref().unwrap().counts[0], 2);
        app.event_action(Action::Clear).unwrap();
        assert_eq!(app.events.as_ref().unwrap().counts, [0; 5]);
        app.event_action(Action::Toggle(0)).unwrap();
        for _ in 0..20 {
            app.event_action(Action::Change).unwrap();
            app.prepare(Instant::now());
        }
        assert_eq!(app.events.as_ref().unwrap().counts[0], 20);
        assert_eq!(app.events.as_ref().unwrap().history.len(), 12);
        app.action(Message::Reset).unwrap();
        app.prepare(Instant::now());
        assert!(app.events.is_some());
        assert_eq!(app.chart.core().event_interests(), EventInterests::ALL);
    }
    #[test]
    fn all_hosts_consume_setters_and_input_once_with_bounded_display_history() {
        exercise::<Chart>();
        exercise::<OptionsChart>();
        exercise::<YieldCurveChart>();
    }
}
