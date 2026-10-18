//! Public formatting controls shared by all four domain demos.
use super::*;
use feather_charts::model::price_scale::{PriceScaleMode, PriceScaleOptionsPatch};
#[derive(Default)]
pub struct Panel {
    series: bool,
}
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Price,
    PriceTicks,
    Percent,
    PercentTicks,
    Horizontal,
    Locale,
    SeriesBatch,
    Mode,
    Clear,
}
pub fn series_format() -> PriceFormat {
    PriceFormat::Custom(PriceFormatCustom {
        formatter: Rc::new(|v| format!("S {:.5}", v.value())),
        tickmarks_formatter: Some(Rc::new(|v| {
            let spread = v
                .iter()
                .map(|p| p.value())
                .fold(f64::NEG_INFINITY, f64::max)
                - v.iter().map(|p| p.value()).fold(f64::INFINITY, f64::min);
            let precision = if spread > 1. { 1 } else { 3 };
            v.iter()
                .map(|p| format!("B {:.*}", precision, p.value()))
                .collect()
        })),
        min_move: 0.01,
        base: None,
    })
}
impl<C: Host> App<C> {
    pub fn formatting_action(&mut self, action: Action) -> Result<(), ChartError> {
        let Some(panel) = &mut self.formatting else {
            return Ok(());
        };
        let current = self.chart.core().localization_options();
        let mut patch = LocalizationOptionsPatch::default();
        match action {
            Action::Price => {
                patch.price_formatter = if current.base.price_formatter.is_some() {
                    FieldPatch::Clear
                } else {
                    FieldPatch::Set(Rc::new(|v| format!("P {:.3}", v.value())))
                }
            }
            Action::PriceTicks => {
                patch.tickmarks_price_formatter =
                    if current.base.tickmarks_price_formatter.is_some() {
                        FieldPatch::Clear
                    } else {
                        FieldPatch::Set(Rc::new(|v| {
                            v.iter().map(|p| format!("T {:.2}", p.value())).collect()
                        }))
                    }
            }
            Action::Percent => {
                patch.percentage_formatter = if current.base.percentage_formatter.is_some() {
                    FieldPatch::Clear
                } else {
                    FieldPatch::Set(Rc::new(|v| format!("{v:.3} pct")))
                }
            }
            Action::PercentTicks => {
                patch.tickmarks_percentage_formatter =
                    if current.base.tickmarks_percentage_formatter.is_some() {
                        FieldPatch::Clear
                    } else {
                        FieldPatch::Set(Rc::new(|v| {
                            v.iter().map(|p| format!("{p:.2} pctB")).collect()
                        }))
                    }
            }
            Action::Horizontal => {
                patch.time_formatter = if current.time_formatter.is_some() {
                    FieldPatch::Clear
                } else {
                    FieldPatch::Set(Rc::new(|v| format!("H {v:?}")))
                }
            }
            Action::Locale => {
                patch.locale = Some(
                    if current.base.locale == "en-US" {
                        "de-DE"
                    } else {
                        "en-US"
                    }
                    .into(),
                );
                patch.date_format = Some("yyyy-MM-dd".into());
            }
            Action::SeriesBatch => {
                panel.series = !panel.series;
                self.chart
                    .format_series(self.line, &self.custom, panel.series, self.live)?;
            }
            Action::Mode => {
                let side = C::side();
                let mode = match self.chart.core().price_scale_options(0, &side)?.mode {
                    PriceScaleMode::Normal => PriceScaleMode::Logarithmic,
                    PriceScaleMode::Logarithmic => PriceScaleMode::Percentage,
                    PriceScaleMode::Percentage => PriceScaleMode::IndexedTo100,
                    PriceScaleMode::IndexedTo100 => PriceScaleMode::Normal,
                };
                for pane in 0..self.chart.core().model().panes().len() {
                    self.chart.scale_patch(
                        pane,
                        &side,
                        PriceScaleOptionsPatch {
                            mode: Some(mode),
                            ..Default::default()
                        },
                    )?;
                }
            }
            Action::Clear => {
                patch = LocalizationOptionsPatch {
                    price_formatter: FieldPatch::Clear,
                    tickmarks_price_formatter: FieldPatch::Clear,
                    percentage_formatter: FieldPatch::Clear,
                    tickmarks_percentage_formatter: FieldPatch::Clear,
                    time_formatter: FieldPatch::Clear,
                    ..Default::default()
                }
            }
        }
        self.chart.localize(patch);
        Ok(())
    }
    pub fn formatting_controls(&self) -> Element<'_, Message> {
        let opts = self.chart.core().localization_options();
        let button_for =
            |label: &'static str, action| button(label).on_press(Message::Formatting(action));
        iced::widget::column![
            iced::widget::scrollable(
                row![
                    button_for("Price", Action::Price),
                    button_for("Price ticks", Action::PriceTicks),
                    button_for("Percent", Action::Percent),
                    button_for("Percent ticks", Action::PercentTicks),
                    button_for("Horizontal", Action::Horizontal),
                    button_for("Locale", Action::Locale),
                    button_for("Series batch", Action::SeriesBatch),
                    button_for("Scale mode", Action::Mode),
                    button_for("Clear formatters", Action::Clear)
                ]
                .spacing(6)
            )
            .direction(iced::widget::scrollable::Direction::Horizontal(
                Default::default()
            )),
            text(format!(
                "{} | global price={} batch={} percent={} batch={} horizontal={} | series batch={}",
                opts.base.locale,
                opts.base.price_formatter.is_some(),
                opts.base.tickmarks_price_formatter.is_some(),
                opts.base.percentage_formatter.is_some(),
                opts.base.tickmarks_percentage_formatter.is_some(),
                opts.time_formatter.is_some(),
                self.formatting.as_ref().unwrap().series
            ))
            .size(12)
            .wrapping(text::Wrapping::None)
        ]
        .spacing(4)
        .into()
    }
}
#[path = "formatting_captures.rs"]
mod capture;
pub fn captures(dir: &std::path::Path, wgpu: bool) {
    capture::run(dir, wgpu);
}
#[cfg(test)]
mod tests {
    use super::*;
    fn controls<C: Host>() {
        let mut app = App::<C>::new(true);
        app.formatting = Some(Default::default());
        app.size = Size::new(800., 500.);
        app.prepare(Instant::now());
        for action in [
            Action::SeriesBatch,
            Action::Price,
            Action::PriceTicks,
            Action::Percent,
            Action::PercentTicks,
            Action::Horizontal,
            Action::Locale,
            Action::Mode,
            Action::Mode,
            Action::Mode,
            Action::Mode,
            Action::Clear,
            Action::SeriesBatch,
        ] {
            app.formatting_action(action).unwrap();
            app.chart
                .prepare(Size::new(300., 90.), 1.5, Instant::now())
                .unwrap();
            app.chart.prepare(app.size, 1., Instant::now()).unwrap();
        }
        assert_eq!(app.chart.core().series_data(app.line).unwrap().len(), 24);
        assert!(
            app.chart
                .core()
                .localization_options()
                .base
                .price_formatter
                .is_none()
        );
        app.action(Message::Reset).unwrap();
        assert!(app.formatting.is_some());
    }
    #[test]
    fn formatting_in_all_domains() {
        controls::<Chart>();
        controls::<OptionsChart>();
        controls::<YieldCurveChart>();
        controls::<Chart<session_behavior::Sessions>>();
    }
}
