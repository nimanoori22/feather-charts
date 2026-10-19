//! Application records belong to the example, not the chart engine.
use super::*;
use std::fmt::Debug;
pub fn record_index<C: Host>(time: &C::Item, next: usize, fallback: usize) -> usize {
    (0..next.max(24))
        .find(|i| C::key(*i) == *time)
        .unwrap_or(fallback)
}
pub trait DemoMetadata: Clone + Debug + 'static {
    fn for_bar(index: usize, revision: usize) -> Option<Self>;
    fn id(&self) -> usize;
    fn describe(&self) -> String;
}
impl DemoMetadata for () {
    fn for_bar(_: usize, _: usize) -> Option<Self> {
        None
    }
    fn id(&self) -> usize {
        0
    }
    fn describe(&self) -> String {
        "unit".into()
    }
}
#[derive(Debug)]
pub struct Record {
    pub id: usize,
    pub revision: usize,
    pub label: String,
}
impl DemoMetadata for Rc<Record> {
    fn for_bar(index: usize, revision: usize) -> Option<Self> {
        Some(Rc::new(Record {
            id: index,
            revision,
            label: format!("Trade {index}"),
        }))
    }
    fn id(&self) -> usize {
        self.id
    }
    fn describe(&self) -> String {
        format!(
            "{} / id={} / revision={}",
            self.label, self.id, self.revision
        )
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Inspect,
    Latest,
    History,
    ClearMetadata,
    Retain,
    InspectRetained,
    Release,
    Restore,
    ClearData,
    Pop,
    Invalid,
}
pub struct Panel<T, M> {
    pub retained: Vec<BuiltInSeriesDataItem<T, M>>,
    pub revision: usize,
    pub status: String,
    pub hover: String,
}
impl<T, M> Default for Panel<T, M> {
    fn default() -> Self {
        Self{retained:vec![],revision:1000,status:"Blue-line records; hover shows the captured record. Retain before Clear data to inspect or restore it.".into(),hover:String::new()}
    }
}
fn description<T, M: DemoMetadata>(row: &BuiltInSeriesDataItem<T, M>) -> String {
    if let BuiltInSeriesDataItem::Line(LineDataItem::Data(row)) = row {
        row.custom_values
            .as_ref()
            .map_or_else(|| "no record".into(), DemoMetadata::describe)
    } else {
        "no record".into()
    }
}
impl<C: Host> App<C> {
    pub fn metadata_action(&mut self, action: Action) -> Result<(), ChartError> {
        let panel = self.metadata.as_mut().unwrap();
        match action {
            Action::Retain => {
                panel.retained = self.chart.core().series_data(self.line)?;
                panel.status = format!(
                    "Retained {} owned rows; later changes leave these records readable.",
                    panel.retained.len()
                );
            }
            Action::InspectRetained => {
                panel.status = format!(
                    "Retained {} rows: {}",
                    panel.retained.len(),
                    panel
                        .retained
                        .last()
                        .map_or_else(|| "none".into(), description)
                )
            }
            Action::Release => {
                panel.retained.clear();
                panel.status = "Released retained data.".into();
            }
            Action::Restore => {
                if panel.retained.is_empty() {
                    panel.status = "Retain data first.".into();
                } else {
                    self.chart.raw_data(self.line, panel.retained.clone())?;
                    panel.status = "Restored retained data and its original records.".into();
                }
            }
            Action::ClearData => {
                self.chart.raw_data(self.line, vec![])?;
                panel.status = "Blue data cleared; retained records remain readable.".into();
            }
            Action::Pop => {
                let removed = self.chart.pop_data(self.line, 1)?;
                panel.status = format!(
                    "Popped: {}",
                    removed.first().map_or_else(|| "empty".into(), description)
                );
            }
            Action::Inspect => {
                let rows = self.chart.core().series_data(self.line)?;
                panel.status = format!(
                    "{} current blue rows; latest: {}",
                    rows.len(),
                    rows.last().map_or_else(|| "empty".into(), description)
                );
            }
            _ => {
                let rows = self.chart.core().series_data(self.line)?;
                if rows.is_empty() {
                    panel.status = "No blue data; Restore data or Reset first.".into();
                    return Ok(());
                }
                let index = if matches!(action, Action::History) {
                    0
                } else {
                    rows.len() - 1
                };
                let mut row = rows[index].clone();
                let record_index = record_index::<C>(row.time(), self.data_next, index);
                if let BuiltInSeriesDataItem::Line(LineDataItem::Data(value)) = &mut row {
                    panel.revision += 1;
                    let id = value
                        .custom_values
                        .as_ref()
                        .map_or(record_index, DemoMetadata::id);
                    value.custom_values = if matches!(action, Action::ClearMetadata) {
                        None
                    } else {
                        C::Metadata::for_bar(id, panel.revision)
                    };
                    if matches!(action, Action::Invalid) {
                        value.value = f64::NAN;
                    }
                }
                let detail = description(&row);
                let result =
                    self.chart
                        .record_update(self.line, row, matches!(action, Action::History));
                if matches!(action, Action::Invalid) {
                    assert!(result.is_err());
                    panel.status =
                        "Invalid replacement rejected; existing data/record preserved.".into();
                } else {
                    result?;
                    panel.status = format!("Replaced record without changing price: {detail}");
                }
            }
        }
        Ok(())
    }
    pub fn observe_metadata(&mut self, event: &ChartEvent<C::Item, C::Metadata>) {
        let Some(panel) = &mut self.metadata else {
            return;
        };
        let pointer = match event {
            ChartEvent::Click(p) | ChartEvent::CrosshairMoved(p) | ChartEvent::DoubleClick(p) => p,
            _ => return,
        };
        panel.hover = pointer
            .series_data
            .iter()
            .find(|(id, _)| *id == self.line)
            .map_or_else(
                || "No blue row under pointer".into(),
                |(_, row)| description(row),
            );
    }
    pub fn metadata_controls(&self) -> Element<'_, Message> {
        let panel = self.metadata.as_ref().unwrap();
        let controls = [
            ("Inspect records", Action::Inspect),
            ("Latest record", Action::Latest),
            ("History record", Action::History),
            ("Clear record", Action::ClearMetadata),
            ("Retain data", Action::Retain),
            ("Inspect retained", Action::InspectRetained),
            ("Release retained", Action::Release),
            ("Restore data", Action::Restore),
            ("Clear blue data", Action::ClearData),
            ("Pop record", Action::Pop),
            ("Invalid record", Action::Invalid),
        ];
        let buttons = controls.into_iter().fold(
            row![text("Blue records")].spacing(6),
            |row, (label, action)| row.push(button(label).on_press(Message::Metadata(action))),
        );
        iced::widget::column![
            iced::widget::scrollable(buttons).direction(
                iced::widget::scrollable::Direction::Horizontal(Default::default())
            ),
            text(format!("{} | Hover: {}", panel.status, panel.hover))
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
    fn consumer<C: Host<Metadata = Rc<Record>>>() {
        let mut app = App::<C>::with_modes(true, true, true);
        app.metadata = Some(Default::default());
        app.size = Size::new(800., 500.);
        app.prepare(Instant::now());
        let before = app.chart.core().series_data(app.line).unwrap();
        let expected = description(before.last().unwrap());
        app.action(Message::Tick(Instant::now())).unwrap();
        let after = app.chart.core().series_data(app.line).unwrap();
        let BuiltInSeriesDataItem::Line(LineDataItem::Data(a)) = after.last().unwrap() else {
            panic!()
        };
        let BuiltInSeriesDataItem::Line(LineDataItem::Data(b)) = before.last().unwrap() else {
            panic!()
        };
        assert_eq!(
            a.custom_values.as_ref().unwrap().id,
            b.custom_values.as_ref().unwrap().id,
            "feed changed record ID for {expected}"
        );
        app.metadata_action(Action::Retain).unwrap();
        let held = app.metadata.as_ref().unwrap().retained.len();
        assert!(held > 0);
        for action in [
            Action::Inspect,
            Action::Latest,
            Action::History,
            Action::ClearMetadata,
            Action::Invalid,
            Action::Pop,
            Action::ClearData,
            Action::InspectRetained,
        ] {
            app.metadata_action(action).unwrap();
            app.prepare(Instant::now());
        }
        assert!(app.chart.core().series_data(app.line).unwrap().is_empty());
        assert_eq!(app.metadata.as_ref().unwrap().retained.len(), held);
        app.metadata_action(Action::Restore).unwrap();
        assert_eq!(app.chart.core().series_data(app.line).unwrap().len(), held);
        app.action(Message::Swap).unwrap();
        app.action(Message::LineNew).unwrap();
        app.prepare(Instant::now());
        app.metadata_action(Action::Latest).unwrap();
        app.chart
            .prepare(Size::new(300., 20.), 1.5, Instant::now())
            .unwrap();
        app.prepare(Instant::now());
        app.metadata_action(Action::Release).unwrap();
        assert!(app.metadata.as_ref().unwrap().retained.is_empty());
        app.action(Message::Reset).unwrap();
        assert!(app.metadata.is_some());
    }
    #[test]
    fn records_in_all_domains() {
        consumer::<Chart<HorzScaleBehaviorTime, Rc<Record>>>();
        consumer::<Chart<HorzScaleBehaviorPrice, Rc<Record>>>();
        consumer::<YieldCurveChart<Rc<Record>>>();
        consumer::<Chart<session_behavior::Sessions, Rc<Record>>>();
    }
}
