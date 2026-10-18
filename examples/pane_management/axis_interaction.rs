use super::*;
#[derive(Clone, Copy, Debug)]
pub enum Action {
    TimeDrag,
    TimeReset,
    PriceDrag,
    PriceReset,
    TouchX,
    TouchY,
    Defaults,
    AllOn,
    AllOff,
}
impl<C: Host> App<C> {
    pub fn enable_axis(&mut self) -> Result<(), ChartError> {
        self.axis_mode = true;
        self.axis_action(Action::Defaults)
    }
    pub fn axis_action(&mut self, action: Action) -> Result<(), ChartError> {
        let mut axes = self.chart.core().axis_interaction_options();
        let mut touch = self.chart.core().touch_navigation_options();
        match action {
            Action::TimeDrag => axes.time_drag = !axes.time_drag,
            Action::TimeReset => axes.time_reset = !axes.time_reset,
            Action::PriceDrag => axes.price_drag = !axes.price_drag,
            Action::PriceReset => axes.price_reset = !axes.price_reset,
            Action::TouchX => touch.horizontal_drag = !touch.horizontal_drag,
            Action::TouchY => touch.vertical_drag = !touch.vertical_drag,
            Action::AllOff => {
                return self.chart.configure(ChartOptionsPatch {
                    axis_interaction: Some(AxisInteractionOptions {
                        time_drag: false,
                        time_reset: false,
                        price_drag: false,
                        price_reset: false,
                    }),
                    mouse_navigation: Some(MouseNavigationOptions {
                        wheel_zoom: false,
                        wheel_scroll: false,
                        pressed_mouse_move: false,
                    }),
                    touch_navigation: Some(TouchNavigationOptions {
                        horizontal_drag: false,
                        vertical_drag: false,
                        pinch: false,
                        ..Default::default()
                    }),
                    ..Default::default()
                });
            }
            Action::AllOn => {
                axes = Default::default();
                touch = Default::default();
            }
            Action::Defaults => {
                return self.chart.configure(ChartOptionsPatch {
                    model: ChartModelOptionsPatch {
                        time_scale: Some(
                            feather_charts::model::time_scale_options::HorzScaleOptionsPatch {
                                bar_spacing: Some(9.),
                                right_offset: Some(3.),
                                right_offset_pixels: Some(None),
                                ..Default::default()
                            },
                        ),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            }
        }
        self.chart.configure(ChartOptionsPatch {
            mouse_navigation: matches!(action, Action::AllOn).then(MouseNavigationOptions::default),
            axis_interaction: Some(axes),
            touch_navigation: Some(touch),
            ..Default::default()
        })
    }
    pub fn axis_controls(&self) -> Element<'_, Message> {
        let a = self.chart.core().axis_interaction_options();
        let t = self.chart.core().touch_navigation_options();
        let toggle = |label: &str, on: bool, action| {
            button(text(format!("{label}: {}", if on { "on" } else { "off" })))
                .on_press(Message::Axis(action))
        };
        iced::widget::column![iced::widget::scrollable(row![toggle("Time drag",a.time_drag,Action::TimeDrag),toggle("Time reset",a.time_reset,Action::TimeReset),toggle("Price drag",a.price_drag,Action::PriceDrag),toggle("Price reset",a.price_reset,Action::PriceReset),toggle("Touch X",t.horizontal_drag,Action::TouchX),toggle("Touch Y",t.vertical_drag,Action::TouchY),button("Defaults 9 / +3").on_press(Message::Axis(Action::Defaults)),button("All off").on_press(Message::Axis(Action::AllOff)),button("All on").on_press(Message::Axis(Action::AllOn))].spacing(6)).direction(iced::widget::scrollable::Direction::Horizontal(Default::default())),text(format!("Drag the bottom labels horizontally; double-click to restore configured defaults. Spacing {:.3}, offset {:.3}. Wheel/pinch remain separate.",self.chart.core().model().time_scale().bar_spacing(),self.chart.core().model().time_scale().right_offset())).size(12)].spacing(4).into()
    }
}
#[path = "axis_interaction_captures.rs"]
mod capture;
pub fn captures(dir: &std::path::Path, wgpu: bool) {
    capture::run(dir, wgpu);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn controls<C: Host>() {
        let mut app = App::<C>::new(true);
        app.enable_axis().unwrap();
        app.size = Size::new(800., 500.);
        app.prepare(Instant::now());
        for action in [
            Action::TimeDrag,
            Action::TimeReset,
            Action::PriceDrag,
            Action::PriceReset,
            Action::TouchX,
            Action::TouchY,
            Action::AllOff,
            Action::AllOn,
            Action::Defaults,
        ] {
            app.axis_action(action).unwrap();
            app.chart
                .prepare(Size::new(300., 90.), 1.5, Instant::now())
                .unwrap();
            app.chart.prepare(app.size, 1., Instant::now()).unwrap();
        }
        app.action(Message::Reset).unwrap();
        assert!(app.axis_mode);
        assert_eq!(
            app.chart.core().axis_interaction_options(),
            Default::default()
        );
        assert_eq!(
            app.chart
                .core()
                .runtime_options()
                .model
                .time_scale
                .bar_spacing,
            9.
        );
    }
    #[test]
    fn controls_in_all_domains() {
        controls::<Chart>();
        controls::<OptionsChart>();
        controls::<YieldCurveChart>();
        controls::<Chart<session_behavior::Sessions>>();
    }
}
