//! Existing series retain identity while changing their owning price scale.
use super::*;
#[derive(Default)]
pub struct Panel {
    via_options: bool,
}
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Select,
    Method,
    Left,
    Right,
    Shared,
    Other,
    Empty,
    Freeze,
    Invalid,
}
impl<C: Host> App<C> {
    pub fn enable_assignment(&mut self) -> Result<(), ChartError> {
        self.assignment = Some(Default::default());
        self.show_assignment_axes()
    }
    fn show_assignment_axes(&mut self) -> Result<(), ChartError> {
        for pane in 0..self.chart.core().model().panes().len() {
            for side in [PriceScalePosition::Left, PriceScalePosition::Right] {
                self.chart.scale_patch(
                    pane,
                    &side,
                    feather_charts::model::price_scale::PriceScaleOptionsPatch {
                        visible: Some(true),
                        ..Default::default()
                    },
                )?;
            }
        }
        Ok(())
    }
    pub fn assignment_action(&mut self, action: Action) -> Result<(), ChartError> {
        let Some(panel) = &mut self.assignment else {
            return Ok(());
        };
        match action {
            Action::Select => {
                self.selected_custom = !self.selected_custom;
                return Ok(());
            }
            Action::Method => {
                panel.via_options = !panel.via_options;
                return Ok(());
            }
            Action::Invalid => {
                let id = self.selected();
                let old = self.chart.core().series_price_scale(id)?;
                assert!(
                    self.chart
                        .assign_scale(id, PriceScalePosition::Overlay("left".into()))
                        .is_err()
                );
                assert_eq!(self.chart.core().series_price_scale(id)?, old);
                self.status = "Reserved overlay alias rejected; assignment unchanged".into();
                return Ok(());
            }
            _ => {}
        }
        let id = self.selected();
        let current = self.chart.core().series_price_scale(id)?;
        if matches!(action, Action::Freeze) {
            let pane = self
                .chart
                .core()
                .model()
                .pane_for_series(id)
                .ok_or(ChartError::UnknownSeries(id))?;
            if self
                .chart
                .core()
                .price_scale_options(pane, &current)?
                .auto_scale
            {
                self.chart.set_range(
                    pane,
                    &current,
                    PriceRange {
                        min_value: 0.,
                        max_value: 10.,
                    },
                )?;
            } else {
                self.chart.set_auto(pane, &current, true)?;
            }
            return Ok(());
        }
        let target = match action {
            Action::Left => PriceScalePosition::Left,
            Action::Right => PriceScalePosition::Right,
            Action::Shared => PriceScalePosition::Overlay("shared-A".into()),
            Action::Other => PriceScalePosition::Overlay("other-B".into()),
            Action::Empty => PriceScalePosition::Overlay(String::new()),
            _ => return Ok(()),
        };
        if self.assignment.as_ref().unwrap().via_options {
            self.chart.assign_options(
                self.line,
                &self.custom,
                self.selected_custom,
                target.id(),
                self.live,
            )?;
        } else {
            self.chart.assign_scale(id, target)?;
        }
        self.show_assignment_axes()?;
        Ok(())
    }
    pub fn assignment_controls(&self) -> Element<'_, Message> {
        let id = self.selected();
        let position = self.chart.core().series_price_scale(id);
        let pane = self.chart.core().model().pane_for_series(id);
        let stored = if self.selected_custom {
            self.custom.price_scale_id()
        } else {
            self.chart
                .core()
                .model()
                .series(id)
                .and_then(|s| s.common_options().price_scale_id.clone())
        };
        let button_for =
            |label: &'static str, action| button(label).on_press(Message::Assign(action));
        iced::widget::column![iced::widget::scrollable(row![
            button(if self.selected_custom{"Selected: custom"}else{"Selected: blue line"}).on_press(Message::Assign(Action::Select)),
            button(if self.assignment.as_ref().unwrap().via_options{"Via options"}else{"Via setter"}).on_press(Message::Assign(Action::Method)),
            button_for("Left",Action::Left),button_for("Right",Action::Right),button_for("Shared A",Action::Shared),button_for("Other B",Action::Other),button_for("Empty-name overlay",Action::Empty),button_for("Freeze / auto",Action::Freeze),button_for("Invalid alias",Action::Invalid)
        ].spacing(6)).direction(iced::widget::scrollable::Direction::Horizontal(Default::default())),
        text(format!("Pane {pane:?} | actual {position:?} | stored {stored:?} | order {:?}. Plot drag follows the default scale; axis gestures affect that axis.",self.chart.core().series_order(id).ok())).size(12).wrapping(text::Wrapping::None)].spacing(4).into()
    }
}
#[path = "scale_assignment_captures.rs"]
mod capture;
pub fn captures(dir: &std::path::Path, wgpu: bool) {
    capture::run(dir, wgpu);
}
#[cfg(test)]
mod tests {
    use super::*;
    fn controls<C: Host>() {
        let mut app = App::<C>::new(true);
        app.enable_assignment().unwrap();
        app.size = Size::new(800., 500.);
        app.prepare(Instant::now());
        for action in [
            Action::Left,
            Action::Right,
            Action::Shared,
            Action::Freeze,
            Action::Other,
            Action::Empty,
            Action::Right,
            Action::Invalid,
            Action::Select,
            Action::Method,
            Action::Left,
            Action::Right,
            Action::Shared,
            Action::Freeze,
            Action::Other,
            Action::Empty,
            Action::Right,
        ] {
            let order = app.chart.core().series_order(app.selected()).unwrap();
            app.assignment_action(action).unwrap();
            app.chart
                .prepare(Size::new(300., 90.), 1.5, Instant::now())
                .unwrap();
            app.chart.prepare(app.size, 1., Instant::now()).unwrap();
            if !matches!(action, Action::Select) {
                assert_eq!(
                    app.chart.core().series_order(app.selected()).unwrap(),
                    order
                );
            }
            let actual = app.chart.core().series_price_scale(app.selected()).unwrap();
            let stored = if app.selected_custom {
                app.custom.price_scale_id()
            } else {
                app.chart
                    .core()
                    .model()
                    .series(app.line)
                    .unwrap()
                    .common_options()
                    .price_scale_id
                    .clone()
            };
            assert_eq!(stored.as_deref(), Some(actual.id()));
        }
        app.action(Message::Reset).unwrap();
        assert!(app.assignment.is_some());
    }
    #[test]
    fn assignment_in_all_domains() {
        controls::<Chart>();
        controls::<OptionsChart>();
        controls::<YieldCurveChart>();
        controls::<Chart<session_behavior::Sessions>>();
    }
}
