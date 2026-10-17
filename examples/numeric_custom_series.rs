//! Custom intervals over strikes or maturities, using the same renderer and host UI.
#[path = "numeric_custom_series/captures.rs"]
mod captures;
#[path = "numeric_custom_series/host.rs"]
mod host;
#[path = "numeric_custom_series/view.rs"]
mod interval;
use feather_charts::{
    api::*,
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        data_layer::SeriesId,
        icustom_series::{CustomSeriesDataItem, CustomSeriesWhitespaceData},
        pane::PriceScalePosition,
        time_scale_options::HorzScaleOptionsPatch,
    },
};
use host::Host;
use iced::{
    Element, Size, Subscription, Task,
    widget::{button, column, row, text},
};
use interval::{Datum, Options, View};
use std::time::{Duration, Instant};
struct App<C: Host> {
    chart: C,
    custom: Option<CustomSeriesHandle<View<C::Item>>>,
    peer: SeriesId,
    size: Size,
    ratio: f32,
    count: usize,
    dense: bool,
    live: bool,
    styled: bool,
    labels: bool,
    step: usize,
    status: String,
}
#[derive(Clone, Debug)]
enum Message {
    Chart(ChartMessage),
    Window(iced::window::Id, iced::window::Event),
    Ratio(f32),
    Tick(Instant),
    Latest,
    History,
    Whitespace,
    Reset,
    Empty,
    Remove,
    Dense,
    Style,
    Labels,
    Pause,
}
fn has_custom(frame: &feather_charts::ui::chart_frame::IcedChartFrame) -> bool {
    frame.plot.custom.is_some() || frame.plot.series_layers.iter().any(|s| s.custom.is_some())
}
fn value(i: usize) -> f64 {
    4.5 + (i as f64 * 0.14).sin() * 0.4
}
impl<C: Host> App<C> {
    fn new(live: bool, dense: bool) -> Self {
        let mut chart = C::new();
        let peer = chart.peer(live);
        chart.decorate();
        chart.scale(HorzScaleOptionsPatch {
            enable_conflation: Some(true),
            min_bar_spacing: Some(0.001),
            ..Default::default()
        });
        let mut app = Self {
            chart,
            custom: None,
            peer,
            size: Size::ZERO,
            ratio: 1.,
            count: if dense { 8192 } else { 24 },
            dense,
            live,
            styled: false,
            labels: false,
            step: 0,
            status: "Custom intervals; hover for original data and renderer hit data".into(),
        };
        app.reset().unwrap();
        app
    }
    fn datum(&self, i: usize, delta: f64) -> CustomSeriesDataItem<Datum<C::Item>, C::Item> {
        let v = value(i) + delta;
        CustomSeriesDataItem::Data(Datum {
            time: C::key(i, self.dense),
            low: v - 0.2,
            high: v + 0.25,
            value: v,
        })
    }
    fn reset(&mut self) -> Result<(), ChartError> {
        if self.custom.is_none() {
            self.custom = Some(self.chart.add()?);
        }
        let rows = (0..self.count).map(|i| self.datum(i, 0.)).collect();
        self.chart.data(self.custom.as_ref().unwrap(), rows)?;
        self.chart.peer_data(
            self.peer,
            (0..self.count)
                .map(|i| {
                    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                        time: C::key(i, self.dense),
                        value: value(i),
                        color: None,
                        custom_values: None,
                    }))
                })
                .collect(),
        )?;
        self.chart.fit(C::key(self.count - 1, self.dense));
        Ok(())
    }
    fn action(&mut self, m: &Message) -> Result<(), ChartError> {
        match m {
            Message::Reset => self.reset()?,
            Message::Dense => {
                self.dense = !self.dense;
                self.count = if self.dense { 8192 } else { 24 };
                self.reset()?;
            }
            Message::Empty => {
                if let Some(h) = &self.custom {
                    self.chart.data(h, vec![])?;
                }
                self.chart.peer_data(self.peer, vec![])?;
                self.live = false;
            }
            Message::Remove => {
                if let Some(h) = self.custom.take() {
                    self.chart.remove(h.id())?;
                }
                self.live = false;
            }
            Message::Latest | Message::History | Message::Tick(_) => {
                if let Some(h) = &self.custom {
                    let i = if matches!(m, Message::History) {
                        self.count / 2
                    } else {
                        self.count - 1
                    };
                    let row = self.datum(i, [-0.12, 0., 0.12][self.step % 3]);
                    self.chart.change(h, row, matches!(m, Message::History))?;
                    self.step += 1;
                }
            }
            Message::Whitespace => {
                if let Some(h) = &self.custom {
                    self.chart.change(
                        h,
                        CustomSeriesDataItem::Whitespace(CustomSeriesWhitespaceData {
                            time: C::key(self.count - 1, self.dense),
                            custom_values: None,
                        }),
                        false,
                    )?;
                }
            }
            Message::Style => {
                self.styled = !self.styled;
                if let Some(h) = &self.custom {
                    self.chart.style(
                        h,
                        Options {
                            color: if self.styled {
                                iced::Color::from_rgba(0.9, 0.3, 0.15, 0.55)
                            } else {
                                Options::default().color
                            },
                        },
                    )?;
                }
            }
            Message::Labels => {
                self.labels = !self.labels;
                self.chart.labels(self.labels)?;
            }
            Message::Pause => self.live = !self.live,
            _ => {}
        }
        Ok(())
    }
    fn prepare(&mut self, at: Instant) {
        if let Err(e) = self.chart.prepare(self.size, self.ratio, at) {
            self.status = e.to_string();
        }
    }
    fn update(&mut self, m: Message) -> Task<Message> {
        match m {
            Message::Chart(message) => {
                if let ChartMessage::Bounds(size) = message {
                    self.size = size;
                }
                match self.chart.update(message) {
                    Ok(events) => {
                        for e in events {
                            if let ChartEvent::Click(p) | ChartEvent::CrosshairMoved(p) = e {
                                let payload = self
                                    .custom
                                    .as_ref()
                                    .and_then(|h| p.custom_data(h))
                                    .map(|d| format!("{:.3}", d.value));
                                let hit = self
                                    .custom
                                    .as_ref()
                                    .and_then(|h| p.custom_hit_data(h))
                                    .copied();
                                self.status = format!(
                                    "{} | original={payload:?} hit={hit:?}",
                                    p.time.as_ref().map(C::label).unwrap_or_else(|| "—".into())
                                );
                            }
                        }
                    }
                    Err(e) => self.status = e.to_string(),
                }
            }
            Message::Window(id, iced::window::Event::Opened { .. }) => {
                return iced::window::scale_factor(id).map(Message::Ratio);
            }
            Message::Window(_, iced::window::Event::Rescaled(r)) | Message::Ratio(r) => {
                self.ratio = r;
                self.prepare(Instant::now());
            }
            Message::Window(_, _) => {}
            Message::Tick(at) => {
                if self.live {
                    if let Err(e) = self.action(&Message::Tick(at)) {
                        self.status = e.to_string();
                    }
                    self.prepare(at);
                }
            }
            _ => {
                if let Err(e) = self.action(&m) {
                    self.status = e.to_string();
                }
                self.prepare(Instant::now());
            }
        }
        Task::none()
    }
    fn view(&self) -> Element<'_, Message> {
        column![
            iced::widget::scrollable(
                row![
                    button("Latest").on_press(Message::Latest),
                    button("History").on_press(Message::History),
                    button("Whitespace").on_press(Message::Whitespace),
                    button("Reset / re-add").on_press(Message::Reset),
                    button("Empty").on_press(Message::Empty),
                    button("Remove").on_press(Message::Remove),
                    button("Dense / sparse").on_press(Message::Dense),
                    button("Style").on_press(Message::Style),
                    button("Labels").on_press(Message::Labels),
                    button(if self.live {
                        "Pause feed"
                    } else {
                        "Resume feed"
                    })
                    .on_press(Message::Pause)
                ]
                .spacing(6)
            )
            .direction(iced::widget::scrollable::Direction::Horizontal(
                Default::default()
            )),
            text(&self.status).size(12).wrapping(text::Wrapping::None),
            self.chart.core().element().map(Message::Chart)
        ]
        .spacing(6)
        .into()
    }
    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::window::events().map(|(id, e)| Message::Window(id, e)),
            if self.live {
                iced::time::every(Duration::from_millis(800)).map(Message::Tick)
            } else {
                Subscription::none()
            },
        ])
    }
}
fn run<C: Host>(live: bool, dense: bool, title: &'static str) -> iced::Result {
    iced::application(move || App::<C>::new(live, dense), App::update, App::view)
        .title(title)
        .subscription(App::subscription)
        .run()
}
fn main() -> iced::Result {
    let args = std::env::args().collect::<Vec<_>>();
    if let Some(i) = args.iter().position(|a| a == "--captures") {
        captures::run(
            std::path::Path::new(&args[i + 1]),
            args.iter().any(|a| a == "--wgpu"),
        );
        return Ok(());
    }
    let live = args.iter().any(|a| a == "--live-feed");
    let dense = args.iter().any(|a| a == "--dense");
    if args.iter().any(|a| a == "--yield-curve") {
        run::<YieldCurveChart>(live, dense, "Feather Charts — maturity custom intervals")
    } else {
        run::<OptionsChart>(live, dense, "Feather Charts — strike custom intervals")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn controls<C: Host>() {
        let now = Instant::now();
        let mut app = App::<C>::new(false, false);
        app.size = Size::new(800., 500.);
        app.prepare(now);
        assert!(has_custom(app.chart.core().frame()));
        let key = C::key(4, false);
        let index = app
            .chart
            .core()
            .model()
            .time_scale()
            .time_to_index(&key, false)
            .unwrap();
        let x = app
            .chart
            .core()
            .model()
            .time_scale()
            .index_to_coordinate(index)
            .value() as f32;
        let pane = &app.chart.core().model().panes()[0];
        let scale = if C::side() == PriceScalePosition::Left {
            pane.left_price_scale()
        } else {
            pane.right_price_scale()
        };
        let y = scale.price_to_coordinate(value(4) + 0.15, value(0)).value() as f32;
        let _ = app.update(Message::Chart(ChartMessage::Interaction(
            feather_charts::ui::chart_input::ChartInteraction {
                position: Some(iced::Point::new(x, y)),
                navigation: None,
                capture: false,
            },
        )));
        assert!(
            app.status.contains("original=Some") && app.status.contains("hit=Some"),
            "{}",
            app.status
        );
        for message in [
            Message::Latest,
            Message::History,
            Message::Whitespace,
            Message::Style,
            Message::Labels,
        ] {
            app.action(&message).unwrap();
            app.prepare(now);
        }
        app.action(&Message::Empty).unwrap();
        app.prepare(now);
        assert!(!has_custom(app.chart.core().frame()));
        app.action(&Message::Reset).unwrap();
        app.prepare(now);
        assert!(has_custom(app.chart.core().frame()));
        let old = app.custom.as_ref().unwrap().id();
        app.action(&Message::Remove).unwrap();
        app.prepare(now);
        app.action(&Message::Reset).unwrap();
        assert_ne!(app.custom.as_ref().unwrap().id(), old);
        app.action(&Message::Dense).unwrap();
        app.prepare(now);
        assert_eq!(app.count, 8192);
        app.action(&Message::Dense).unwrap();
        for height in [90., 35., 500.] {
            app.size.height = height;
            app.chart.prepare(app.size, 1.25, now).unwrap();
        }
        assert!(has_custom(app.chart.core().frame()));
        assert!(
            app.chart.core().model().time_scale().bar_spacing() > 1.,
            "sparse reset must focus current data"
        );
    }
    #[test]
    fn price_demo_controls_and_custom_hit_payloads() {
        controls::<OptionsChart>();
    }
    #[test]
    fn maturity_demo_controls_and_custom_hit_payloads() {
        controls::<YieldCurveChart>();
    }
}
