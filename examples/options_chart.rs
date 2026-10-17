//! Strike prices on the shared chart: ordinary defaults, decimal labels and live volume.
use feather_charts::{
    api::*,
    model::{
        data_consumer::{
            BuiltInSeriesDataItem, HistogramData, HistogramDataItem, LineData, LineDataItem,
        },
        data_layer::SeriesId,
        pane::PriceScalePosition,
        series_options::{
            HistogramSeriesOptions, LastPriceAnimationMode, LineSeriesOptions, SeriesOptionsMap,
        },
    },
    renderers::draw_line::LineType,
};
use iced::{
    Element, Size, Subscription, Task,
    widget::{button, column, row, text},
};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
#[path = "options_chart/captures.rs"]
mod captures;
#[path = "options_chart/region.rs"]
mod region;
const STRIKES: [f64; 12] = [
    90., 92.5, 95., 97.5, 100., 105., 110., 115., 125., 140., 160., 200.,
];
const VALUES: [f64; 12] = [
    5.378, 5.372, 5.271, 5.094, 4.739, 4.237, 4.036, 3.887, 3.921, 4.007, 4.366, 4.29,
];
fn price(value: f64) -> HorizontalPrice {
    HorizontalPrice::new(value).unwrap()
}
fn item(volume: bool, strike: f64, value: f64) -> BuiltInSeriesDataItem<HorizontalPrice> {
    if volume {
        BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
            time: price(strike),
            value: value * 80.,
            color: None,
            custom_values: None,
        }))
    } else {
        BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time: price(strike),
            value,
            color: None,
            custom_values: None,
        }))
    }
}
fn data(volume: bool) -> Vec<BuiltInSeriesDataItem<HorizontalPrice>> {
    STRIKES
        .into_iter()
        .zip(VALUES)
        .map(|(t, v)| item(volume, t, v + if volume { 0.15 } else { 0. }))
        .collect()
}
fn line_options(pulse: bool) -> SeriesOptionsMap {
    let mut options = LineSeriesOptions::default();
    options.style.line_type = LineType::Curved;
    options.style.point_markers_visible = true;
    options.style.last_price_animation = if pulse {
        LastPriceAnimationMode::Continuous
    } else {
        LastPriceAnimationMode::Disabled
    };
    SeriesOptionsMap::Line(options)
}
struct App {
    chart: OptionsChart,
    line: SeriesId,
    volume: SeriesId,
    markers: UpDownMarkersId,
    size: Size,
    ratio: f32,
    step: usize,
    live: bool,
    custom: bool,
    status: String,
    region: region::Scene,
}
#[derive(Clone, Debug)]
enum Message {
    Chart(ChartMessage),
    Window(iced::window::Id, iced::window::Event),
    Ratio(f32),
    Tick(Instant),
    Change,
    History,
    Extend,
    Reset,
    Empty,
    Domain,
    Formatter,
    Manual,
    Clear,
    Pause,
}
impl App {
    fn new(live: bool) -> Self {
        let mut chart_options = feather_charts::model::chart_model::ChartModelOptions::default();
        chart_options.pane.left_price_scale.visible = true;
        chart_options.pane.left_price_scale.scale_margins.top = 0.7;
        chart_options.pane.left_price_scale.scale_margins.bottom = 0.;
        let mut chart = OptionsChart::with_options_chart_options(
            OptionsChartOptions { precision: 2 },
            chart_options,
            Default::default(),
            feather_charts::model::localization_options::LocalizationOptions::new("en-US", ""),
        )
        .unwrap();
        let line = chart
            .add_series(line_options(live), 0, PriceScalePosition::Right)
            .unwrap();
        let volume = chart
            .add_series(
                SeriesOptionsMap::Histogram(HistogramSeriesOptions::default()),
                0,
                PriceScalePosition::Left,
            )
            .unwrap();
        let markers = chart.add_up_down_markers(line, Default::default()).unwrap();
        chart.set_up_down_data(markers, data(false)).unwrap();
        chart.set_data(volume, data(true)).unwrap();
        chart
            .add_pane_primitive(
                0,
                TextWatermark::new(TextWatermarkOptions {
                    lines: vec![TextWatermarkLine {
                        text: "Strike prices".into(),
                        ..Default::default()
                    }],
                    ..Default::default()
                })
                .unwrap(),
            )
            .unwrap();
        chart
            .add_pane_primitive(
                0,
                ImageWatermark::from_rgba(
                    2,
                    2,
                    vec![
                        50, 180, 180, 25, 100, 50, 180, 25, 100, 50, 180, 25, 50, 180, 180, 25,
                    ],
                    ImageWatermarkOptions {
                        max_width: Some(200.),
                        max_height: Some(120.),
                        ..Default::default()
                    },
                )
                .unwrap(),
            )
            .unwrap();
        let region = region::Scene::new(&mut chart, line, 3.0..7.0);
        chart.fit_content();
        Self {
            chart,
            line,
            volume,
            markers,
            size: Size::ZERO,
            ratio: 1.,
            step: 0,
            live,
            custom: false,
            status: "Strike prices are index-spaced; right axis = line, left axis = volume".into(),
            region,
        }
    }
    fn prepare(&mut self, now: Instant) {
        if let Err(e) = self.chart.prepare(self.size, self.ratio, now) {
            self.status = e.to_string();
        }
    }
    fn action(&mut self, message: &Message, now: Instant) -> Result<(), ChartError> {
        match message {
            Message::Change | Message::History | Message::Tick(_) => {
                let i = if matches!(message, Message::History) {
                    [3, 4, 6][self.step % 3]
                } else {
                    11
                };
                let delta = [0.1, 0., -0.1][(self.step / 3) % 3];
                self.chart.update_up_down_data(
                    self.markers,
                    item(false, STRIKES[i], VALUES[i] + delta),
                    true,
                    now,
                )?;
                self.step += 1;
            }
            Message::Extend => self
                .chart
                .update_data(self.volume, item(true, 225., 4.3), false)?,
            Message::Reset => {
                self.chart.set_up_down_data(self.markers, data(false))?;
                self.chart.set_data(self.volume, data(true))?;
                self.chart.fit_content();
            }
            Message::Empty => {
                self.chart.set_data(self.line, vec![])?;
                self.chart.set_data(self.volume, vec![])?;
                self.chart.clear_up_down_markers(self.markers)?;
            }
            Message::Domain => {
                let precision = match self.chart.options_chart_options().precision {
                    0 => 1,
                    1 => 2,
                    2 => 4,
                    _ => 0,
                };
                self.chart
                    .replace_options_chart_options(OptionsChartOptions { precision })?;
            }
            Message::Formatter => {
                self.custom = !self.custom;
                self.chart.set_horizontal_price_formatter(
                    self.custom.then(|| {
                        Rc::new(|v: &HorizontalPrice| format!("Strike {}", v.value())) as _
                    }),
                );
            }
            Message::Manual => self.chart.set_up_down_markers(
                self.markers,
                [3, 4, 6]
                    .into_iter()
                    .zip([
                        MarkerSign::Positive,
                        MarkerSign::Neutral,
                        MarkerSign::Negative,
                    ])
                    .map(|(i, sign)| UpDownMarker {
                        time: price(STRIKES[i]),
                        value: VALUES[i],
                        sign,
                    })
                    .collect(),
            )?,
            Message::Clear => self.chart.clear_up_down_markers(self.markers)?,
            Message::Pause => {
                self.live = !self.live;
                self.chart
                    .replace_series_options(self.line, line_options(self.live))?;
            }
            _ => {}
        }
        Ok(())
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Chart(m) => {
                if let ChartMessage::Bounds(size) = m {
                    self.size = size;
                }
                match self.chart.update(m) {
                    Ok(events) => {
                        for event in events {
                            match event {
                                ChartEvent::PrimitiveEdit(e) => {
                                    if let Err(error) = self.region.edit(&mut self.chart, &e) {
                                        self.status = error.to_string();
                                    }
                                    self.prepare(Instant::now());
                                }
                                ChartEvent::Click(p) | ChartEvent::CrosshairMoved(p) => {
                                    self.status = format!(
                                        "Strike: {} | {} series",
                                        p.time
                                            .map_or_else(|| "—".into(), |p| p.value().to_string()),
                                        p.series_data.len()
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => self.status = e.to_string(),
                }
            }
            Message::Window(id, iced::window::Event::Opened { .. }) => {
                return iced::window::scale_factor(id).map(Message::Ratio);
            }
            Message::Window(_, iced::window::Event::Rescaled(ratio)) | Message::Ratio(ratio) => {
                self.ratio = ratio;
                self.prepare(Instant::now());
            }
            Message::Window(_, _) => {}
            Message::Tick(at) => {
                if self.live {
                    if let Err(e) = self.action(&Message::Tick(at), at) {
                        self.status = e.to_string();
                    }
                    self.prepare(at);
                }
            }
            _ => {
                let now = Instant::now();
                if let Err(e) = self.action(&message, now) {
                    self.status = e.to_string();
                }
                self.prepare(now);
            }
        }
        Task::none()
    }
    fn view(&self) -> Element<'_, Message> {
        column![
            iced::widget::scrollable(
                row![
                    button("Latest change").on_press(Message::Change),
                    button("History change").on_press(Message::History),
                    button("Extend 225").on_press(Message::Extend),
                    button("Reset").on_press(Message::Reset),
                    button("Empty").on_press(Message::Empty),
                    button("Precision 0/1/2/4").on_press(Message::Domain),
                    button("Labels").on_press(Message::Formatter),
                    button("Manual markers").on_press(Message::Manual),
                    button("Clear markers").on_press(Message::Clear),
                    button(if self.live {
                        "Pause feed/pulse"
                    } else {
                        "Resume feed/pulse"
                    })
                    .on_press(Message::Pause)
                ]
                .spacing(6)
            )
            .direction(iced::widget::scrollable::Direction::Horizontal(
                Default::default()
            )),
            text(&self.status)
                .size(12)
                .width(iced::Length::Fill)
                .wrapping(text::Wrapping::None),
            self.chart.element().map(Message::Chart)
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
fn main() -> iced::Result {
    let args = std::env::args().collect::<Vec<_>>();
    if let Some(i) = args.iter().position(|v| v == "--captures") {
        captures::run(
            std::path::Path::new(&args[i + 1]),
            args.iter().any(|v| v == "--wgpu"),
        );
        return Ok(());
    }
    let live = args.iter().any(|v| v == "--live-feed");
    iced::application(move || App::new(live), App::update, App::view)
        .title("Feather Charts — strike prices")
        .subscription(App::subscription)
        .run()
}
