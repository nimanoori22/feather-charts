//! Constructor parser policy; application ink resolves before renderer freezing.
use super::*;
use feather_charts::model::{
    chart_model::ChartModelOptions,
    colors::Rgba,
    layout_options::{Background, ColorSpace},
};
use std::cell::Cell;
pub const DARK: &str = "color(display-p3 0.2 0.3 0.4 / 0.5)";
pub const LIGHT: &str = "color(display-p3 0.8 0.9 1 / 0.25)";
pub struct Panel {
    pub(super) calls: Rc<Cell<usize>>,
    light: bool,
    row: bool,
    dark_background: bool,
    line: Option<PriceLineId>,
    watermark: Option<PrimitiveId>,
    status: String,
}
impl Panel {
    pub fn row_color(&self) -> Option<String> {
        self.row
            .then(|| if self.light { DARK } else { LIGHT }.into())
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Palette,
    Background,
    Row,
    Extensions,
    Invalid,
    Target,
}
pub fn options() -> (ChartModelOptions, Panel) {
    let calls = Rc::new(Cell::new(0));
    let mut options = ChartModelOptions::default();
    options.layout.color_parsers.push(Box::new({
        let calls = calls.clone();
        move |token| {
            calls.set(calls.get() + 1);
            match token {
                DARK => Some(Rgba::new(20, 40, 60, 0.5)),
                LIGHT => Some(Rgba::new(220, 230, 240, 0.25)),
                _ => None,
            }
        }
    }));
    (
        options,
        Panel {
            calls,
            light: false,
            row: false,
            dark_background: false,
            line: None,
            watermark: None,
            status: "Configured strings resolve to sRGB; parser policy is fixed at creation".into(),
        },
    )
}
impl<C: Host> App<C> {
    pub fn new_colors(live: bool) -> Result<Self, ChartError> {
        let (options, panel) = options();
        let mut app = Self::with_chart(C::with_colors(options)?, live);
        app.colors = Some(panel);
        app.chart.color_series(app.line, &app.custom, DARK, live)?;
        Ok(app)
    }
    pub fn color_action(&mut self, action: Action) -> Result<(), ChartError> {
        let Some(panel) = &mut self.colors else {
            return Ok(());
        };
        match action {
            Action::Palette => {
                panel.light = !panel.light;
                self.chart.color_series(
                    self.line,
                    &self.custom,
                    if panel.light { LIGHT } else { DARK },
                    self.live,
                )?;
                panel.status =
                    "Series and native custom-renderer ink changed through the shared resolver"
                        .into();
            }
            Action::Background => {
                panel.dark_background = !panel.dark_background;
                let (bg, text) = if panel.dark_background {
                    (DARK, LIGHT)
                } else {
                    (LIGHT, DARK)
                };
                self.chart.configure(ChartOptionsPatch {
                    model: ChartModelOptionsPatch {
                        layout: Some(LayoutOptionsPatch {
                            background: Some(Background::Solid { color: bg.into() }),
                            text_color: Some(text.into()),
                            ..Default::default()
                        }),
                        grid: Some(GridOptionsPatch {
                            vert_lines: Some(GridLineOptionsPatch {
                                color: Some(text.into()),
                                ..Default::default()
                            }),
                            horz_lines: Some(GridLineOptionsPatch {
                                color: Some(text.into()),
                                ..Default::default()
                            }),
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                })?;
                panel.status="Configured layout/grid colors; automatic axis-label contrast remains source-defined".into();
            }
            Action::Row => {
                panel.row = !panel.row;
                let mut rows = self.chart.core().series_data(self.line)?;
                if let Some(BuiltInSeriesDataItem::Line(LineDataItem::Data(row))) = rows.last_mut()
                {
                    row.color = panel.row_color();
                }
                self.chart.raw_data(self.line, rows)?;
                panel.status =
                    "Latest row override toggled; live updates preserve the selected override"
                        .into();
            }
            Action::Extensions => {
                let mut line = PriceLineOptions::new(4.6);
                line.color = DARK.into();
                line.axis_label_color = LIGHT.into();
                line.axis_label_text_color = DARK.into();
                line.title = "Configured".into();
                if let Some(id) = panel.line {
                    self.chart.color_price_line(Some(id), self.line, line)?;
                } else {
                    panel.line = Some(self.chart.color_price_line(None, self.line, line)?);
                }
                let wm = TextWatermark::with_color_parser(
                    TextWatermarkOptions {
                        lines: vec![TextWatermarkLine {
                            text: "sRGB ink".into(),
                            color: DARK.into(),
                            font_size: 28.,
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                    self.chart.core().color_parser(),
                )
                .unwrap();
                panel.watermark = Some(self.chart.color_watermark(panel.watermark, wm)?);
                panel.status =
                    "Price-line/label contrast and text watermark use the configured parser".into();
            }
            Action::Invalid => {
                let before = self.chart.core().runtime_options();
                let result = self.chart.configure(ChartOptionsPatch {
                    model: ChartModelOptionsPatch {
                        layout: Some(LayoutOptionsPatch {
                            text_color: Some(DARK.into()),
                            background: Some(Background::Solid {
                                color: "invalid-color".into(),
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                });
                assert!(result.is_err());
                assert_eq!(self.chart.core().runtime_options(), before);
                panel.status =
                    "Invalid color rejected atomically; appearance and existing exports preserved"
                        .into();
            }
            Action::Target => {
                let before = self.chart.core().runtime_options();
                let result = self.chart.configure(ChartOptionsPatch {
                    model: ChartModelOptionsPatch {
                        layout: Some(LayoutOptionsPatch {
                            color_space: Some(ColorSpace::DisplayP3),
                            text_color: Some(DARK.into()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                });
                assert!(matches!(
                    result,
                    Err(ChartError::Model(
                        feather_charts::model::chart_model::ChartModelError::UnsupportedColorSpace(
                            ColorSpace::DisplayP3
                        )
                    ))
                ));
                assert_eq!(self.chart.core().runtime_options(), before);
                panel.status =
                    "Display-P3 output is unsupported; sRGB output and options preserved".into();
            }
        }
        Ok(())
    }
    pub fn color_controls(&self) -> Element<'_, Message> {
        let panel = self.colors.as_ref().unwrap();
        let b = |s: &'static str, a| button(s).on_press(Message::Color(a));
        iced::widget::column![
            iced::widget::scrollable(
                row![
                    b("Palette", Action::Palette),
                    b("Background", Action::Background),
                    b("Latest row", Action::Row),
                    b("Lines/watermark", Action::Extensions),
                    b("Invalid color", Action::Invalid),
                    b("P3 target", Action::Target)
                ]
                .spacing(6)
            )
            .direction(iced::widget::scrollable::Direction::Horizontal(
                Default::default()
            )),
            text(format!(
                "sRGB output | parser calls={} | {}",
                panel.calls.get(),
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
    fn run<C: Host>() {
        let mut a = App::<C>::new_colors(true).unwrap();
        for action in [
            Action::Palette,
            Action::Background,
            Action::Row,
            Action::Extensions,
            Action::Invalid,
            Action::Target,
        ] {
            a.color_action(action).unwrap();
            for size in [
                Size::new(800., 500.),
                Size::new(300., 300.),
                Size::new(300., 90.),
            ] {
                a.chart.prepare(size, 1.25, Instant::now()).unwrap();
            }
        }
        let before = a.chart.core().series_data(a.line).unwrap().len();
        a.action(Message::Tick(Instant::now())).unwrap();
        assert_eq!(a.chart.core().series_data(a.line).unwrap().len(), before);
        a.action(Message::Reset).unwrap();
        assert!(a.colors.is_some());
    }
    #[test]
    fn configured_color_controls_in_all_domains() {
        run::<Chart>();
        run::<OptionsChart>();
        run::<YieldCurveChart>();
        run::<Chart<session_behavior::Sessions>>();
    }
}
