//! A host application using the public owner/widget API, without demo support.
use feather_charts::{
    api::{Chart, ChartEvent, ChartMessage},
    model::{
        data_consumer::{
            BuiltInSeriesDataItem, CandlestickData, CandlestickDataItem, HistogramData,
            HistogramDataItem,
        },
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{
            CandlestickSeriesOptions, HistogramSeriesOptions, PriceFormat, PriceFormatBuiltIn,
            PriceFormatBuiltInType, SeriesOptionsMap,
        },
    },
};
use iced::{
    Element, Size, Subscription, Task,
    widget::{column, text},
};
use std::time::Instant;

struct App {
    chart: Chart,
    size: Size,
    ratio: f32,
    status: String,
}
#[derive(Clone, Debug)]
enum Message {
    Chart(ChartMessage),
    Window(iced::window::Id, iced::window::Event),
    Ratio(f32),
}
impl Default for App {
    fn default() -> Self {
        let mut chart = Chart::default();
        let price = chart
            .add_series(
                SeriesOptionsMap::Candlestick(CandlestickSeriesOptions::default()),
                0,
                PriceScalePosition::Right,
            )
            .unwrap();
        let pane = chart.add_pane();
        let mut volume_options = HistogramSeriesOptions::default();
        volume_options.common.price_format = PriceFormat::BuiltIn(PriceFormatBuiltIn {
            kind: PriceFormatBuiltInType::Volume,
            precision: 0,
            min_move: 1.,
            base: None,
        });
        let volume = chart
            .add_series(
                SeriesOptionsMap::Histogram(volume_options),
                pane,
                PriceScalePosition::Right,
            )
            .unwrap();
        chart.set_pane_stretch_factor(0, 2.).unwrap();
        let times = (0..120).map(|i| {
            (
                i,
                Time::from(UtcTimestamp::new(1_700_000_000. + i as f64 * 60.)),
            )
        });
        chart
            .set_data(
                price,
                times
                    .clone()
                    .map(|(i, time)| {
                        let close = 100. + (i as f64 * 0.1).sin() * 8.;
                        BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(
                            CandlestickData {
                                time,
                                open: close - 1.,
                                high: close + 2.,
                                low: close - 2.,
                                close,
                                color: None,
                                border_color: None,
                                wick_color: None,
                                custom_values: None,
                            },
                        ))
                    })
                    .collect(),
            )
            .unwrap();
        chart
            .set_data(
                volume,
                times
                    .map(|(i, time)| {
                        BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
                            time,
                            value: 10_000. + (i % 11) as f64 * 2300.,
                            color: None,
                            custom_values: None,
                        }))
                    })
                    .collect(),
            )
            .unwrap();
        chart.fit_content();
        Self {
            chart,
            size: Size::ZERO,
            ratio: 1.,
            status: "Move or click inside either pane".into(),
        }
    }
}
impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Chart(message) => {
                if let ChartMessage::Bounds(size) = &message {
                    self.size = *size;
                }
                match self.chart.update(message) {
                    Ok(events) => {
                        for event in events {
                            let (kind, data) = match event {
                                ChartEvent::Click(data) => ("Click", data),
                                ChartEvent::CrosshairMoved(data) => ("Crosshair", data),
                            };
                            self.status = format!(
                                "{kind}: time {:?}, price {:?}, {} series",
                                data.time,
                                data.price,
                                data.series_data.len()
                            );
                        }
                    }
                    Err(error) => self.status = error.to_string(),
                }
            }
            Message::Window(id, iced::window::Event::Opened { .. }) => {
                return iced::window::scale_factor(id).map(Message::Ratio);
            }
            Message::Window(_, iced::window::Event::Rescaled(ratio)) | Message::Ratio(ratio) => {
                self.ratio = ratio;
                if let Err(error) = self.chart.prepare(self.size, ratio, Instant::now()) {
                    self.status = error.to_string();
                }
            }
            Message::Window(_, _) => {}
        }
        Task::none()
    }
    fn view(&self) -> Element<'_, Message> {
        column![text(&self.status), self.chart.element().map(Message::Chart)].into()
    }
    fn subscription(&self) -> Subscription<Message> {
        iced::window::events().map(|(id, event)| Message::Window(id, event))
    }
}
fn main() -> iced::Result {
    if std::env::args().any(|a| a == "--smoke") {
        let mut app = App::default();
        app.chart
            .prepare(Size::new(800., 500.), 1.25, Instant::now())
            .unwrap();
        let frame = app.chart.multi_frame().unwrap();
        assert_eq!(frame.panes.len(), 2);
        assert!(frame.panes.iter().all(|pane| !pane.plot.candles.is_empty()));
        println!("public chart API smoke passed");
        return Ok(());
    }
    iced::application(App::default, App::update, App::view)
        .title("Feather Charts — public API")
        .subscription(App::subscription)
        .run()
}
