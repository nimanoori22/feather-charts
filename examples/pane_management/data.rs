use super::*;
use feather_charts::model::{
    icustom_series::CustomSeriesWhitespaceData, plot_list::MismatchDirection,
    time_data::LogicalRange,
};
fn custom_row<C: Host>(i: usize) -> CustomSeriesDataItem<interval::Datum<C::Item>, C::Item> {
    CustomSeriesDataItem::Data(interval::Datum {
        time: C::key(i),
        value: 4.5,
        low: 4.3,
        high: 4.75,
    })
}
impl<C: Host> App<C> {
    pub(super) fn with_modes(live: bool, ordered: bool, data: bool) -> Self {
        let mut app = Self::with_order(live, ordered || data);
        app.data_mode = data;
        if data {
            app.chart
                .data(
                    app.markers,
                    (0..24)
                        .filter(|i| i % 2 == 0)
                        .map(|i| line_row::<C>(i, 0.))
                        .collect(),
                )
                .unwrap();
            let mut rows: Vec<_> = (0..23)
                .filter(|i| i % 3 == 0)
                .map(custom_row::<C>)
                .collect();
            rows.push(CustomSeriesDataItem::Whitespace(
                CustomSeriesWhitespaceData {
                    time: C::key(23),
                    custom_values: None,
                },
            ));
            app.chart.custom_data(&app.custom, rows).unwrap();
            app.chart.fit();
            app.inspect_data().unwrap();
        }
        app
    }
    pub(super) fn inspect_data(&mut self) -> Result<(), ChartError> {
        let range = Some(LogicalRange {
            from: 0.0.into(),
            to: 10.0.into(),
        });
        let (count, first, range) = if self.selected_custom {
            if !self
                .chart
                .core()
                .series_ids()
                .any(|id| id == self.custom.id())
            {
                return Err(ChartError::UnknownSeries(self.custom.id()));
            }
            (
                self.custom.data().len(),
                self.custom
                    .data_by_index(0.0.into(), MismatchDirection::NearestRight)?
                    .map(|d| d.value),
                self.custom.bars_in_logical_range(range)?,
            )
        } else {
            let rows = self.chart.core().series_data(self.line)?;
            let first = self
                .chart
                .core()
                .data_by_index(self.line, 0.0.into(), MismatchDirection::NearestRight)?
                .and_then(|d| match d {
                    BuiltInSeriesDataItem::Line(LineDataItem::Data(d)) => Some(d.value),
                    _ => None,
                });
            (
                rows.len(),
                first,
                self.chart.core().bars_in_logical_range(self.line, range)?,
            )
        };
        self.data_status = format!("{count} bars | nearest index 0={first:?} | range={range:?}");
        Ok(())
    }
    pub(super) fn pop_selected(&mut self, count: usize) -> Result<(), ChartError> {
        let values: Vec<_> = if self.selected_custom {
            self.chart
                .pop_custom_data(&self.custom, count)?
                .into_iter()
                .map(|d| d.value)
                .collect()
        } else {
            self.chart
                .pop_data(self.line, count)?
                .into_iter()
                .filter_map(|d| match d {
                    BuiltInSeriesDataItem::Line(LineDataItem::Data(d)) => Some(d.value),
                    _ => None,
                })
                .collect()
        };
        self.inspect_data()?;
        self.data_status = format!("Removed newest first {values:?} | {}", self.data_status);
        Ok(())
    }
    pub(super) fn append_selected(&mut self) -> Result<(), ChartError> {
        if self.selected_custom {
            self.chart
                .update_custom(&self.custom, custom_row::<C>(self.data_next))?;
        } else {
            self.chart.update_data(
                self.markers,
                line_row::<C>(self.data_next, 0.),
                Instant::now(),
            )?;
        }
        self.data_next += 1;
        self.inspect_data()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn controls<C: Host>() {
        let mut app = App::<C>::with_modes(true, false, true);
        app.size = Size::new(800., 500.);
        for custom in [false, true] {
            app.selected_custom = custom;
            for count in [0, 1, 3, usize::MAX] {
                app.action(Message::Pop(count)).unwrap();
                app.action(Message::Inspect).unwrap();
                app.chart
                    .prepare(Size::new(300., 90.), 1.5, Instant::now())
                    .unwrap();
                app.chart.prepare(app.size, 1., Instant::now()).unwrap();
            }
            app.action(Message::Tick(Instant::now())).unwrap();
            let count = if custom {
                app.custom.data().len()
            } else {
                app.chart.core().series_data(app.line).unwrap().len()
            };
            assert_eq!(count, 0, "feed must not resurrect an emptied owner");
            app.action(Message::Append).unwrap();
            app.action(Message::Tick(Instant::now())).unwrap();
            app.action(Message::LineNew).unwrap();
            app.action(Message::CustomToLine).unwrap();
            app.action(Message::ToFront).unwrap();
            app.chart.prepare(app.size, 1., Instant::now()).unwrap();
        }
        app.action(Message::Reset).unwrap();
        assert!(app.data_mode && app.order_mode);
        assert_eq!(app.size, Size::new(800., 500.));
        assert!(!app.custom.data().is_empty());
    }
    #[test]
    fn calendar_data_controls() {
        controls::<Chart>();
    }
    #[test]
    fn strike_data_controls() {
        controls::<OptionsChart>();
    }
    #[test]
    fn maturity_data_controls() {
        controls::<YieldCurveChart>();
    }
}
