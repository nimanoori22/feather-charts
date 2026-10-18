use super::*;
use feather_charts::model::{
    crosshair::CrosshairMode,
    layout_options::Background,
    price_scale::{PriceScaleMarginsPatch, PriceScaleMode, PriceScaleOptionsPatch},
};
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Pane,
    Side,
    Manual,
    Narrow,
    Widen,
    Auto,
    Freeze,
    Mode,
    Invert,
    Margins,
    Appearance,
    Grid,
    Crosshair,
    Mouse,
    Resize,
    Invalid,
}
impl<C: Host> App<C> {
    pub(super) fn with_configuration(live: bool, ordered: bool, data: bool, enabled: bool) -> Self {
        let mut app = Self::with_modes(live, ordered, data);
        app.config_mode = enabled;
        app.config_left = matches!(C::side(), PriceScalePosition::Left);
        app
    }
    fn config_side(&self) -> PriceScalePosition {
        if self.config_left {
            PriceScalePosition::Left
        } else {
            PriceScalePosition::Right
        }
    }
    pub(super) fn configuration_action(&mut self, action: Action) -> Result<(), ChartError> {
        let n = self.chart.core().model().panes().len();
        if n == 0 {
            return Err(ChartError::InvalidInteraction);
        }
        self.config_pane %= n;
        let pane = self.config_pane;
        let side = self.config_side();
        let opts = self.chart.core().price_scale_options(pane, &side)?;
        let mut patch = ChartOptionsPatch::default();
        match action {
            Action::Pane => self.config_pane = (pane + 1) % n,
            Action::Side => self.config_left = !self.config_left,
            Action::Manual => self.chart.set_range(
                pane,
                &side,
                if opts.mode == PriceScaleMode::Logarithmic {
                    PriceRange {
                        min_value: 4.5,
                        max_value: 4.8,
                    }
                } else {
                    PriceRange {
                        min_value: 3.5,
                        max_value: 6.5,
                    }
                },
            )?,
            Action::Narrow | Action::Widen => {
                let scale = self.chart.core().model().panes()[pane]
                    .price_scale_by_id(if self.config_left { "left" } else { "right" })
                    .unwrap();
                let r = scale.price_range().ok_or(ChartError::InvalidPriceRange)?;
                let center = r.min_value() + r.length() / 2.;
                let radius = r.length() / 2.
                    * if matches!(action, Action::Narrow) {
                        0.8
                    } else {
                        1.25
                    };
                self.chart.set_range(
                    pane,
                    &side,
                    PriceRange {
                        min_value: center - radius,
                        max_value: center + radius,
                    },
                )?;
            }
            Action::Auto | Action::Freeze => {
                self.chart
                    .set_auto(pane, &side, matches!(action, Action::Auto))?
            }
            Action::Mode => self.chart.scale_patch(
                pane,
                &side,
                PriceScaleOptionsPatch {
                    mode: Some(match opts.mode {
                        PriceScaleMode::Normal => PriceScaleMode::Logarithmic,
                        PriceScaleMode::Logarithmic => PriceScaleMode::Percentage,
                        PriceScaleMode::Percentage => PriceScaleMode::IndexedTo100,
                        PriceScaleMode::IndexedTo100 => PriceScaleMode::Normal,
                    }),
                    ..Default::default()
                },
            )?,
            Action::Invert => self.chart.scale_patch(
                pane,
                &side,
                PriceScaleOptionsPatch {
                    invert_scale: Some(!opts.invert_scale),
                    ..Default::default()
                },
            )?,
            Action::Margins => self.chart.scale_patch(
                pane,
                &side,
                PriceScaleOptionsPatch {
                    scale_margins: Some(PriceScaleMarginsPatch {
                        top: Some(if opts.scale_margins.top < 0.3 {
                            0.35
                        } else {
                            0.1
                        }),
                        bottom: Some(0.1),
                    }),
                    ..Default::default()
                },
            )?,
            Action::Appearance => {
                let o = self.chart.core().runtime_options();
                let dark = o.model.layout.font_size < 16.;
                patch.model.layout = Some(LayoutOptionsPatch {
                    background: Some(if dark {
                        Background::VerticalGradient {
                            top_color: "#122033".into(),
                            bottom_color: "#293c55".into(),
                        }
                    } else {
                        Background::Solid {
                            color: "white".into(),
                        }
                    }),
                    text_color: Some(if dark { "#eef4ff" } else { "#222222" }.into()),
                    font_size: Some(if dark { 17. } else { 12. }),
                    font_family: Some(if dark { "monospace" } else { "sans-serif" }.into()),
                    panes: Some(PaneLayoutOptionsPatch {
                        separator_color: Some(if dark { "#59718a" } else { "#cccccc" }.into()),
                        separator_hover_color: Some("#ff8800".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                });
                self.chart.configure(patch)?;
            }
            Action::Grid => {
                let visible = !self
                    .chart
                    .core()
                    .runtime_options()
                    .model
                    .grid
                    .vert_lines
                    .visible;
                patch.model.grid = Some(GridOptionsPatch {
                    vert_lines: Some(GridLineOptionsPatch {
                        visible: Some(visible),
                        ..Default::default()
                    }),
                    horz_lines: Some(GridLineOptionsPatch {
                        visible: Some(visible),
                        ..Default::default()
                    }),
                });
                self.chart.configure(patch)?;
            }
            Action::Crosshair => {
                let mut options = self.chart.core().runtime_options().model.crosshair;
                options.mode = if options.mode == CrosshairMode::Hidden {
                    CrosshairMode::Magnet
                } else {
                    CrosshairMode::Hidden
                };
                patch.model.crosshair = Some(options);
                self.chart.configure(patch)?;
            }
            Action::Mouse => {
                let enabled = !self.chart.core().mouse_navigation_options().wheel_zoom;
                patch.mouse_navigation = Some(MouseNavigationOptions {
                    wheel_zoom: enabled,
                    wheel_scroll: enabled,
                    pressed_mouse_move: enabled,
                });
                self.chart.configure(patch)?;
            }
            Action::Resize => {
                let enabled = !self
                    .chart
                    .core()
                    .runtime_options()
                    .model
                    .layout
                    .panes
                    .enable_resize;
                patch.model.layout = Some(LayoutOptionsPatch {
                    panes: Some(PaneLayoutOptionsPatch {
                        enable_resize: Some(enabled),
                        ..Default::default()
                    }),
                    ..Default::default()
                });
                self.chart.configure(patch)?;
            }
            Action::Invalid => {
                let result = self.chart.set_range(
                    pane,
                    &side,
                    PriceRange {
                        min_value: f64::NAN,
                        max_value: 1.,
                    },
                );
                assert!(result.is_err());
                self.status = "Invalid range rejected; chart unchanged".into();
            }
        }
        Ok(())
    }
    pub(super) fn configuration_controls(&self) -> Element<'_, Message> {
        let side = self.config_side();
        let pane = self.config_pane;
        let range = self.chart.core().visible_price_range(pane, &side);
        let opts = self.chart.core().price_scale_options(pane, &side);
        let label = format!(
            "Pane {pane} {side:?} | {:?} | {:?}",
            range,
            opts.map(|o| (o.mode, o.auto_scale))
        );
        let button_for =
            |label: &'static str, action| button(label).on_press(Message::Config(action));
        iced::widget::column![
            iced::widget::scrollable(
                row![
                    button_for("Next pane", Action::Pane),
                    button_for("Left / right", Action::Side),
                    button_for("Manual range", Action::Manual),
                    button_for("Narrow", Action::Narrow),
                    button_for("Widen", Action::Widen),
                    button_for("Autoscale on", Action::Auto),
                    button_for("Freeze", Action::Freeze),
                    button_for("Mode", Action::Mode),
                    button_for("Invert", Action::Invert),
                    button_for("Margins", Action::Margins),
                    button_for("Invalid input", Action::Invalid)
                ]
                .spacing(6)
            )
            .direction(iced::widget::scrollable::Direction::Horizontal(
                Default::default()
            )),
            iced::widget::scrollable(
                row![
                    button_for("Appearance", Action::Appearance),
                    button_for("Grid", Action::Grid),
                    button_for("Crosshair", Action::Crosshair),
                    button_for("Mouse on/off", Action::Mouse),
                    button_for("Pane resize on/off", Action::Resize)
                ]
                .spacing(6)
            )
            .direction(iced::widget::scrollable::Direction::Horizontal(
                Default::default()
            )),
            text(label).size(12).wrapping(text::Wrapping::None)
        ]
        .spacing(4)
        .into()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn controls<C: Host>() {
        let mut app = App::<C>::with_configuration(true, false, false, true);
        app.size = Size::new(800., 500.);
        app.chart.prepare(app.size, 1., Instant::now()).unwrap();
        let id = app.line;
        for action in [
            Action::Manual,
            Action::Narrow,
            Action::Widen,
            Action::Freeze,
            Action::Auto,
            Action::Mode,
            Action::Manual,
            Action::Auto,
            Action::Mode,
            Action::Mode,
            Action::Mode,
            Action::Invert,
            Action::Margins,
            Action::Appearance,
            Action::Grid,
            Action::Crosshair,
            Action::Mouse,
            Action::Resize,
            Action::Invalid,
            Action::Side,
            Action::Manual,
            Action::Auto,
            Action::Pane,
            Action::Manual,
            Action::Auto,
        ] {
            app.configuration_action(action).unwrap();
            app.chart
                .prepare(Size::new(300., 90.), 1.5, Instant::now())
                .unwrap();
            app.chart.prepare(app.size, 1., Instant::now()).unwrap();
            assert!(app.chart.core().series_ids().any(|v| v == id));
        }
        app.action(Message::Reset).unwrap();
        assert!(app.config_mode);
    }
    #[test]
    fn calendar_configuration() {
        controls::<Chart>();
    }
    #[test]
    fn strike_configuration() {
        controls::<OptionsChart>();
    }
    #[test]
    fn maturity_configuration() {
        controls::<YieldCurveChart>();
    }
}
