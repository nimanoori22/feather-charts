use super::*;
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Next,
    Small,
    Medium,
    Large,
    Minimum,
    Maximum,
    Save,
    Restore,
    Invalid,
}
#[derive(Default)]
pub struct Panel {
    pub pane: usize,
    pub saved: Vec<f64>,
    pub status: String,
}
impl<C: Host> App<C> {
    pub fn sizing_action(&mut self, action: Action) -> Result<(), ChartError> {
        let count = self.chart.core().model().panes().len();
        let panel = self.sizing.as_mut().unwrap();
        panel.pane = panel.pane.min(count - 1);
        match action {
            Action::Next => panel.pane = (panel.pane + 1) % count,
            Action::Save => {
                panel.saved = self
                    .chart
                    .core()
                    .model()
                    .panes()
                    .iter()
                    .map(|p| p.stretch_factor())
                    .collect();
                panel.status =
                    "Saved allocations by pane slot; resizing scales the saved layout.".into();
            }
            Action::Restore => {
                if panel.saved.len() != count {
                    panel.status =
                        "Restore needs the same pane count as Save; Reset and save again.".into();
                    return Ok(());
                }
                for (pane, factor) in panel.saved.iter().copied().enumerate() {
                    self.chart.pane_stretch(pane, factor)?;
                }
                panel.status = "Restored allocations by current pane slot.".into();
            }
            Action::Invalid => {
                assert!(self.chart.pane_height(panel.pane, f64::NAN).is_err());
                assert!(self.chart.pane_height(count, 150.).is_err());
                panel.status =
                    "Invalid pane and non-finite height rejected without layout changes.".into();
            }
            _ => {
                let height = match action {
                    Action::Small => 80.,
                    Action::Medium => 150.,
                    Action::Large => 300.,
                    Action::Minimum => -100.,
                    Action::Maximum => 1e6,
                    _ => unreachable!(),
                };
                self.chart.pane_height(panel.pane, height)?;
                panel.status = format!(
                    "Requested {height:.0}px; settled heights below include clamps and rounding."
                );
            }
        }
        Ok(())
    }
    pub fn sizing_controls(&self) -> Element<'_, Message> {
        let panel = self.sizing.as_ref().unwrap();
        let controls = [
            ("Next pane", Action::Next),
            ("80px", Action::Small),
            ("150px", Action::Medium),
            ("300px", Action::Large),
            ("Minimum", Action::Minimum),
            ("Maximum", Action::Maximum),
            ("Save layout", Action::Save),
            ("Restore layout", Action::Restore),
            ("Invalid height", Action::Invalid),
        ];
        let buttons = controls
            .into_iter()
            .fold(row![].spacing(6), |row, (label, action)| {
                row.push(button(label).on_press(Message::Sizing(action)))
            });
        let heights: Vec<_> = self
            .chart
            .core()
            .model()
            .panes()
            .iter()
            .enumerate()
            .map(|(i, p)| format!("{i}: {:.1}px", p.height()))
            .collect();
        iced::widget::column![
            iced::widget::scrollable(buttons).direction(
                iced::widget::scrollable::Direction::Horizontal(Default::default())
            ),
            text(format!(
                "Selected pane {} | {} | {}",
                panel.pane.min(heights.len() - 1),
                heights.join(", "),
                panel.status
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
        app.sizing = Some(Default::default());
        app.size = Size::new(800., 500.);
        app.prepare(Instant::now());
        app.sizing_action(Action::Save).unwrap();
        let saved = app.sizing.as_ref().unwrap().saved.clone();
        for action in [
            Action::Next,
            Action::Small,
            Action::Medium,
            Action::Large,
            Action::Minimum,
            Action::Maximum,
            Action::Invalid,
            Action::Restore,
        ] {
            app.sizing_action(action).unwrap();
            app.prepare(Instant::now());
        }
        assert_eq!(
            app.chart
                .core()
                .model()
                .panes()
                .iter()
                .map(|p| p.stretch_factor())
                .collect::<Vec<_>>(),
            saved
        );
        app.action(Message::Swap).unwrap();
        app.sizing_action(Action::Restore).unwrap();
        app.chart
            .prepare(Size::new(300., 30.), 1.25, Instant::now())
            .unwrap();
        app.sizing_action(Action::Large).unwrap();
        app.prepare(Instant::now());
        app.action(Message::Remove).unwrap();
        app.sizing_action(Action::Restore).unwrap();
        assert!(
            app.sizing
                .as_ref()
                .unwrap()
                .status
                .contains("same pane count")
        );
        app.action(Message::Reset).unwrap();
        assert!(app.sizing.is_some());
    }
    #[test]
    fn saved_layout_controls_in_all_domains() {
        controls::<Chart>();
        controls::<OptionsChart>();
        controls::<YieldCurveChart>();
        controls::<Chart<session_behavior::Sessions>>();
    }
}
