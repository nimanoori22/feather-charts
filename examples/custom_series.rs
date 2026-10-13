//! Typed custom-series host. Run with --capture-dir PATH [--wgpu] for automated captures.
#[path = "custom_series/box_whisker.rs"]
mod box_whisker;
#[path = "custom_series/capture.rs"]
mod capture;
use box_whisker::{Options, View, sample};
use feather_charts::{
    api::{Chart, ChartMessage, CustomSeriesHandle, PriceLineOptions},
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        icustom_series::{CustomSeriesDataItem, CustomSeriesWhitespaceData},
        pane::PriceScalePosition,
        series_options::{CustomSeriesOptions, LineSeriesOptions, SeriesOptionsMap},
    },
};
use iced::{
    Element, Size, Subscription, Task,
    widget::{button, column, row, text},
};
use std::time::Instant;

struct App {
    chart: Chart,
    custom: Option<CustomSeriesHandle<View>>,
    latest: usize,
    style: bool,
    size: Size,
    ratio: f32,
    status: String,
}
#[derive(Clone, Debug)]
enum Message {
    Chart(ChartMessage),
    Window(iced::window::Id, iced::window::Event),
    Ratio(f32),
    Append,
    Replace,
    Historical,
    Whitespace,
    Reload,
    Clear,
    Remove,
    Style,
}
impl Default for App {
    fn default() -> Self {
        let mut chart = Chart::default();
        let line = chart
            .add_series(
                SeriesOptionsMap::Line(LineSeriesOptions::default()),
                0,
                PriceScalePosition::Right,
            )
            .unwrap();
        chart
            .set_data(
                line,
                (0..120)
                    .map(|index| {
                        let CustomSeriesDataItem::Data(data) = sample(index) else {
                            unreachable!()
                        };
                        BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                            time: data.time,
                            value: data.quartiles[2],
                            color: None,
                            custom_values: None,
                        }))
                    })
                    .collect(),
            )
            .unwrap();
        let mut app = Self {
            chart,
            custom: None,
            latest: 119,
            style: false,
            size: Size::ZERO,
            ratio: 1.,
            status: "Box-and-whisker custom data beside a built-in median line".into(),
        };
        app.reload();
        app
    }
}
impl App {
    fn reload(&mut self) {
        if self.custom.is_none() {
            let handle = self
                .chart
                .add_custom_series(
                    View::default(),
                    CustomSeriesOptions::default(),
                    0,
                    PriceScalePosition::Right,
                )
                .unwrap();
            self.chart
                .add_price_line(handle.id(), PriceLineOptions::new(100.))
                .unwrap();
            self.custom = Some(handle);
        }
        self.latest = 119;
        self.chart
            .set_custom_data(
                self.custom.as_ref().unwrap(),
                (0..120).map(sample).collect(),
            )
            .unwrap();
        self.chart.fit_content();
    }
    fn apply(&mut self, message: Message) {
        match message {
            Message::Reload => self.reload(),
            Message::Remove => {
                if let Some(handle) = self.custom.take() {
                    self.chart.remove_series(handle.id()).unwrap();
                }
            }
            Message::Style => {
                if let Some(handle) = &self.custom {
                    self.style = !self.style;
                    let mut options = Options::default();
                    if self.style {
                        options.colors.swap(1, 2);
                    }
                    self.chart
                        .replace_custom_view_options(handle, options)
                        .unwrap();
                }
            }
            _ => {
                let Some(handle) = &self.custom else {
                    self.status = "Reload to add the custom series".into();
                    return;
                };
                match message {
                    Message::Clear => self.chart.set_custom_data(handle, vec![]).unwrap(),
                    Message::Append => {
                        self.latest += 1;
                        self.chart
                            .update_custom_data(handle, sample(self.latest), false)
                            .unwrap();
                    }
                    Message::Replace | Message::Historical => {
                        let historical = matches!(message, Message::Historical);
                        let mut row = sample(if historical { 40 } else { self.latest });
                        if let CustomSeriesDataItem::Data(data) = &mut row {
                            for value in &mut data.quartiles {
                                *value += 3.;
                            }
                        }
                        if let Err(error) = self.chart.update_custom_data(handle, row, historical) {
                            self.status = error.to_string();
                        }
                    }
                    Message::Whitespace => {
                        self.latest += 1;
                        let CustomSeriesDataItem::Data(data) = sample(self.latest) else {
                            unreachable!()
                        };
                        self.chart
                            .update_custom_data(
                                handle,
                                CustomSeriesDataItem::Whitespace(CustomSeriesWhitespaceData {
                                    time: data.time,
                                    custom_values: None,
                                }),
                                false,
                            )
                            .unwrap();
                    }
                    _ => {}
                }
            }
        }
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Chart(message) => {
                if let ChartMessage::Bounds(size) = &message {
                    self.size = *size;
                }
                match self.chart.update(message) {
                    Ok(events) => {
                        for event in events {
                            self.status = format!("{event:?}");
                        }
                    }
                    Err(error) => self.status = error.to_string(),
                }
            }
            Message::Window(id, iced::window::Event::Opened { .. }) => {
                return iced::window::scale_factor(id).map(Message::Ratio);
            }
            Message::Window(_, iced::window::Event::Rescaled(ratio)) | Message::Ratio(ratio) => {
                self.ratio = ratio
            }
            Message::Window(_, _) => {}
            action => self.apply(action),
        }
        if self.size.width > 0. {
            self.chart
                .prepare(self.size, self.ratio, Instant::now())
                .unwrap();
        }
        Task::none()
    }
    fn view(&self) -> Element<'_, Message> {
        column![
            text(&self.status)
                .width(iced::Length::Fill)
                .wrapping(text::Wrapping::None),
            row![
                button("Append").on_press(Message::Append),
                button("Replace").on_press(Message::Replace),
                button("Historical").on_press(Message::Historical),
                button("Whitespace").on_press(Message::Whitespace)
            ]
            .spacing(4),
            row![
                button("Style").on_press(Message::Style),
                button("Clear").on_press(Message::Clear),
                button("Reload").on_press(Message::Reload),
                button("Remove").on_press(Message::Remove)
            ]
            .spacing(4),
            self.chart.element().map(Message::Chart),
        ]
        .spacing(4)
        .into()
    }
    fn subscription(&self) -> Subscription<Message> {
        iced::window::events().map(|(id, event)| Message::Window(id, event))
    }
}
fn main() -> iced::Result {
    let arguments: Vec<_> = std::env::args().collect();
    if let Some(index) = arguments
        .iter()
        .position(|argument| argument == "--capture-dir")
    {
        capture::run(
            &arguments[index + 1],
            arguments.iter().any(|argument| argument == "--wgpu"),
        );
        return Ok(());
    }
    iced::application(App::default, App::update, App::view)
        .title("Feather Charts — typed custom series")
        .subscription(App::subscription)
        .window_size((1100., 700.))
        .run()
}
