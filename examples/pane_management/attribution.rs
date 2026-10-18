//! Host-controlled attribution actions; this demo never launches external URLs.
use super::*;
use feather_charts::model::layout_options::Background;
#[derive(Default)]
pub struct Panel {
    dark: bool,
    bright: bool,
    pub activations: usize,
    pub status: String,
}
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Visible,
    Background,
    Text,
    Clear,
}
impl<C: Host> App<C> {
    pub fn enable_attribution(&mut self) -> Result<(), ChartError> {
        self.attribution = Some(Panel::default());
        self.chart.configure(ChartOptionsPatch {
            model: ChartModelOptionsPatch {
                layout: Some(LayoutOptionsPatch {
                    attribution_logo: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }
    pub fn attribution_action(&mut self, action: Action) -> Result<(), ChartError> {
        let Some(panel) = &mut self.attribution else {
            return Ok(());
        };
        let mut patch = LayoutOptionsPatch::default();
        match action {
            Action::Visible => {
                patch.attribution_logo = Some(
                    !self
                        .chart
                        .core()
                        .runtime_options()
                        .model
                        .layout
                        .attribution_logo,
                )
            }
            Action::Background => {
                panel.dark = !panel.dark;
                patch.background = Some(Background::Solid {
                    color: if panel.dark { "#122033" } else { "white" }.into(),
                });
            }
            Action::Text => {
                panel.bright = !panel.bright;
                patch.text_color = Some(if panel.bright { "#d1d4dc" } else { "#131722" }.into());
            }
            Action::Clear => {
                panel.activations = 0;
                panel.status.clear();
            }
        }
        self.chart.configure(ChartOptionsPatch {
            model: ChartModelOptionsPatch {
                layout: Some(patch),
                ..Default::default()
            },
            ..Default::default()
        })
    }
    pub fn attribution_controls(&self) -> Element<'_, Message> {
        let panel = self.attribution.as_ref().unwrap();
        iced::widget::scrollable(
            row![
                button("Logo on/off").on_press(Message::Attribution(Action::Visible)),
                button("Background").on_press(Message::Attribution(Action::Background)),
                button("Text / logo theme").on_press(Message::Attribution(Action::Text)),
                button("Clear link count").on_press(Message::Attribution(Action::Clear)),
                text(format!(
                    "{} activations | {}",
                    panel.activations, panel.status
                ))
                .size(12)
                .wrapping(text::Wrapping::None),
            ]
            .spacing(6),
        )
        .direction(iced::widget::scrollable::Direction::Horizontal(
            Default::default(),
        ))
        .into()
    }
}
#[path = "attribution_captures.rs"]
mod capture;
pub fn captures(dir: &std::path::Path, wgpu: bool) {
    capture::run(dir, wgpu);
}
#[cfg(test)]
mod tests {
    use super::*;
    fn controls<C: Host>() {
        let mut app = App::<C>::new(false);
        app.enable_attribution().unwrap();
        app.size = Size::new(800., 500.);
        app.prepare(Instant::now());
        assert!(
            app.chart
                .core()
                .runtime_options()
                .model
                .layout
                .attribution_logo
        );
        for action in [
            Action::Text,
            Action::Background,
            Action::Visible,
            Action::Visible,
            Action::Clear,
        ] {
            app.attribution_action(action).unwrap();
            app.prepare(Instant::now());
        }
        app.action(Message::Swap).unwrap();
        app.action(Message::Remove).unwrap();
        app.prepare(Instant::now());
        app.action(Message::Reset).unwrap();
        app.prepare(Instant::now());
        assert!(
            app.chart
                .core()
                .runtime_options()
                .model
                .layout
                .attribution_logo
        );
        app.consume_events(vec![ChartEvent::AttributionActivated {
            url: "https://www.tradingview.com/",
        }]);
        assert_eq!(app.attribution.as_ref().unwrap().activations, 1);
    }
    #[test]
    fn all_domain_controls() {
        controls::<Chart>();
        controls::<OptionsChart>();
        controls::<YieldCurveChart>();
        controls::<Chart<session_behavior::Sessions>>();
    }
}
