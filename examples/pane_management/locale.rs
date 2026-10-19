//! Calendar locale defaults; other horizontal domains keep their own labels.
use super::*;
use feather_charts::formatters::time_locale::time_locale_support;

#[derive(Default)]
pub struct Panel {
    callback: bool,
    long_date: bool,
}
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Set(&'static str),
    Callback,
    Date,
}
impl<C: Host> App<C> {
    pub fn enable_locale(&mut self) -> Result<(), ChartError> {
        self.locale = Some(Default::default());
        let ids: Vec<_> = self.chart.core().series_ids().collect();
        for id in ids {
            if id == self.custom.id() {
                continue;
            }
            let mut rows = self.chart.core().series_data(id)?;
            for (i, row) in rows.iter_mut().enumerate() {
                *row.time_mut() = C::locale_key(i);
            }
            self.chart.raw_data(id, rows)?;
        }
        self.chart.custom_data(
            &self.custom,
            (0..24)
                .map(|i| {
                    let value = 4.5 + (i as f64 * 0.3).sin() * 0.4;
                    CustomSeriesDataItem::Data(interval::Datum {
                        time: C::locale_key(i),
                        low: value - 0.2,
                        high: value + 0.25,
                        value,
                    })
                })
                .collect(),
        )?;
        self.chart.fit();
        self.locale_action(Action::Set("en-US"));
        Ok(())
    }
    pub fn locale_action(&mut self, action: Action) {
        let Some(panel) = &mut self.locale else {
            return;
        };
        let mut patch = LocalizationOptionsPatch::default();
        match action {
            Action::Set(locale) => patch.locale = Some(locale.into()),
            Action::Date => {
                panel.long_date = !panel.long_date;
                patch.date_format = Some(
                    if panel.long_date {
                        "yyyy-MM-dd MMMM"
                    } else {
                        "dd MMM 'yy"
                    }
                    .into(),
                );
            }
            Action::Callback => {
                panel.callback = !panel.callback;
                patch.time_formatter = if panel.callback {
                    FieldPatch::Set(Rc::new(|item| format!("Custom {item:?}")))
                } else {
                    FieldPatch::Clear
                };
            }
        }
        self.chart.localize(patch);
    }
    pub fn locale_controls(&self) -> Element<'_, Message> {
        let opts = self.chart.core().localization_options();
        let locales = [
            "en-US",
            "fr-FR",
            "de-DE",
            "ar-EG",
            "ar",
            "ar-EG-u-nu-latn",
            "ar-EG-u-nu-arabext",
            "de-DE-u-nu-arab",
            "fa-IR",
            "bad_locale",
        ];
        let mut buttons = row![];
        for locale in locales {
            buttons = buttons.push(button(locale).on_press(Message::Locale(Action::Set(locale))));
        }
        buttons = buttons
            .push(button("Item override").on_press(Message::Locale(Action::Callback)))
            .push(button("Date tokens").on_press(Message::Locale(Action::Date)));
        iced::widget::column![
            iced::widget::scrollable(buttons.spacing(6)).direction(
                iced::widget::scrollable::Direction::Horizontal(Default::default())
            ),
            text(format!(
                "{} | calendar {:?} | item override={} | non-calendar axes retain domain labels",
                opts.base.locale,
                time_locale_support(&opts.base.locale),
                self.locale.as_ref().unwrap().callback
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
    fn controls<C: Host>() {
        let mut app = App::<C>::new(true);
        app.enable_locale().unwrap();
        let keys: Vec<_> = app
            .chart
            .core()
            .series_data(app.line)
            .unwrap()
            .iter()
            .map(|r| r.time().clone())
            .collect();
        for action in [
            Action::Set("ar-EG"),
            Action::Callback,
            Action::Set("de-DE-u-nu-arab"),
            Action::Date,
            Action::Callback,
            Action::Set("bad_locale"),
        ] {
            app.locale_action(action);
            for size in [
                Size::new(800., 500.),
                Size::new(300., 300.),
                Size::new(300., 90.),
            ] {
                app.chart.prepare(size, 1.25, Instant::now()).unwrap();
            }
        }
        app.action(Message::Tick(Instant::now())).unwrap();
        assert_eq!(
            app.chart
                .core()
                .series_data(app.line)
                .unwrap()
                .iter()
                .map(|r| r.time().clone())
                .collect::<Vec<_>>(),
            keys
        );
        app.action(Message::Reset).unwrap();
        assert!(app.locale.is_some());
    }
    #[test]
    fn locale_controls_preserve_each_domain_and_live_update_keys() {
        controls::<Chart>();
        controls::<OptionsChart>();
        controls::<YieldCurveChart>();
        controls::<Chart<session_behavior::Sessions>>();
    }
}
