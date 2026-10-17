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
    live_since: Option<Instant>,
}
#[derive(Clone, Debug)]
enum Message {
    Chart(ChartMessage),
    Window(iced::window::Id, iced::window::Event),
    Ratio(f32),
    Append,
    ToggleLive,
    FeedTick(Instant),
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
        if std::env::args().any(|a| a == "--conflation") {
            chart.apply_time_scale_options(
                feather_charts::model::time_scale_options::HorzScaleOptionsPatch {
                    enable_conflation: Some(true),
                    min_bar_spacing: Some(0.001),
                    ..Default::default()
                },
            );
        }
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
                (0..sample_count())
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
            latest: sample_count() - 1,
            style: false,
            size: Size::ZERO,
            ratio: 1.,
            status: "Box-and-whisker custom data beside a built-in median line".into(),
            live_since: None,
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
        self.latest = sample_count() - 1;
        self.chart
            .set_custom_data(
                self.custom.as_ref().unwrap(),
                (0..sample_count()).map(sample).collect(),
            )
            .unwrap();
        self.chart.fit_content();
    }
    fn apply(&mut self, message: Message) {
        match message {
            Message::Reload => self.reload(),
            Message::Remove => {
                self.live_since = None;
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
            Message::ToggleLive => {
                self.live_since =
                    (self.live_since.is_none() && self.custom.is_some()).then(Instant::now);
            }
            Message::FeedTick(at) => {
                if self.live_since.is_some_and(|since| at >= since) {
                    self.apply(Message::Append);
                } else {
                    return Task::none();
                }
            }
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
                button(if self.live_since.is_some() {
                    "Stop live feed"
                } else {
                    "Start live feed"
                })
                .width(130)
                .on_press_maybe(self.custom.as_ref().map(|_| Message::ToggleLive)),
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
        Subscription::batch([
            iced::window::events().map(|(id, event)| Message::Window(id, event)),
            if self.live_since.is_some() {
                iced::time::every(std::time::Duration::from_millis(250)).map(Message::FeedTick)
            } else {
                Subscription::none()
            },
        ])
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
    let live = arguments.iter().any(|a| a == "--live-feed");
    iced::application(
        move || App {
            live_since: live.then(Instant::now),
            ..App::default()
        },
        App::update,
        App::view,
    )
    .title("Feather Charts — typed custom series")
    .subscription(App::subscription)
    .window_size((1100., 700.))
    .run()
}

#[cfg(test)]
mod live_tests {
    use super::*;

    #[test]
    fn custom_live_feed_pauses_and_stops_when_owner_is_removed() {
        let mut app = App::default();
        app.size = Size::new(400., 300.);
        let _ = app.update(Message::FeedTick(Instant::now()));
        assert_eq!(app.latest, sample_count() - 1);
        let _ = app.update(Message::ToggleLive);
        let _ = app.update(Message::FeedTick(Instant::now()));
        assert_eq!(app.latest, sample_count());
        let _ = app.update(Message::ToggleLive);
        let _ = app.update(Message::FeedTick(Instant::now()));
        assert_eq!(app.latest, sample_count());
        let _ = app.update(Message::ToggleLive);
        let stale = app.live_since.unwrap() - std::time::Duration::from_secs(1);
        let _ = app.update(Message::FeedTick(stale));
        assert_eq!(app.latest, sample_count());
        let _ = app.update(Message::Remove);
        assert!(app.live_since.is_none());
        let _ = app.update(Message::FeedTick(Instant::now()));
        assert_eq!(app.latest, sample_count());
        assert!(app.custom.is_none());
        let _ = app.update(Message::ToggleLive);
        assert!(app.live_since.is_none());
        let _ = app.update(Message::Reload);
        let _ = app.update(Message::ToggleLive);
        let _ = app.update(Message::FeedTick(Instant::now()));
        assert_eq!(app.latest, sample_count());
    }
}

fn sample_count() -> usize {
    if std::env::args().any(|a| a == "--conflation") {
        8192
    } else {
        120
    }
}
