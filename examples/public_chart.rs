//! A host application using the public owner/widget API, without demo support.
use feather_charts::{
    api::{Chart, ChartEvent, ChartMessage, PriceLineId, PriceLineOptions},
    model::{
        data_consumer::{
            AreaData, AreaDataItem, BuiltInSeriesDataItem, CandlestickData, CandlestickDataItem,
            HistogramData, HistogramDataItem, LineData, LineDataItem,
        },
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{
            AreaSeriesOptions, CandlestickSeriesOptions, HistogramSeriesOptions, LineSeriesOptions,
            PriceFormat, PriceFormatBuiltIn, PriceFormatBuiltInType, SeriesOptionsMap, SeriesType,
        },
    },
};
use iced::{
    Element, Size, Subscription, Task,
    widget::{button, column, row, text},
};
use std::time::Instant;

struct App {
    chart: Chart,
    size: Size,
    ratio: f32,
    status: String,
    price: feather_charts::model::data_layer::SeriesId,
    volume: feather_charts::model::data_layer::SeriesId,
    levels: Vec<PriceLineId>,
    latest: usize,
    show_last: bool,
    kind: SeriesType,
    style_step: u8,
}
#[derive(Clone, Debug)]
enum Message {
    Chart(ChartMessage),
    Window(iced::window::Id, iced::window::Event),
    Ratio(f32),
    Append,
    MoveStop,
    ToggleLevels,
    Cluster,
    ToggleLast,
    Style,
}
impl Default for App {
    fn default() -> Self {
        Self::new(SeriesType::Candlestick)
    }
}
impl App {
    fn new(kind: SeriesType) -> Self {
        let mut chart = Chart::default();
        let price = chart
            .add_series(sample_options(kind, true), 0, PriceScalePosition::Right)
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
                        sample_row(kind, time, close)
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
        let levels = add_levels(&mut chart, price);
        Self {
            chart,
            size: Size::ZERO,
            ratio: 1.,
            status: "Move/click; scroll into history to compare visible and global last prices"
                .into(),
            price,
            volume,
            levels,
            latest: 119,
            show_last: true,
            kind,
            style_step: 0,
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
            Message::Append => {
                self.latest += 1;
                let i = self.latest;
                let time = Time::from(UtcTimestamp::new(1_700_000_000. + i as f64 * 60.));
                let close = 100. + (i as f64 * 0.1).sin() * 8.;
                self.chart
                    .update_data(
                        self.price,
                        sample_row(self.kind, time.clone(), close),
                        false,
                    )
                    .unwrap();
                self.chart
                    .update_data(
                        self.volume,
                        BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
                            time,
                            value: 10_000. + (i % 11) as f64 * 2300.,
                            color: None,
                            custom_values: None,
                        })),
                        false,
                    )
                    .unwrap();
                self.prepare();
            }
            Message::MoveStop => {
                if let Some(line) = self.levels.get(1) {
                    let mut options = self.chart.price_line_options(*line).unwrap();
                    options.price += 0.5;
                    self.chart
                        .replace_price_line_options(*line, options)
                        .unwrap();
                }
                self.prepare();
            }
            Message::ToggleLevels => {
                if self.levels.is_empty() {
                    self.levels = add_levels(&mut self.chart, self.price);
                } else {
                    for line in self.levels.drain(..) {
                        self.chart.remove_price_line(line).unwrap();
                    }
                }
                self.prepare();
            }
            Message::Cluster => {
                for (i, line) in self.levels.iter().enumerate() {
                    let mut options = self.chart.price_line_options(*line).unwrap();
                    options.price = 100. + i as f64 * 0.02;
                    options.title = format!("Clustered level {}", i + 1);
                    self.chart
                        .replace_price_line_options(*line, options)
                        .unwrap();
                }
                self.prepare();
            }
            Message::Style => {
                self.style_step = (self.style_step + 1) % 6;
                let mut options = sample_options(self.kind, self.show_last);
                use feather_charts::renderers::draw_line::LineType;
                match &mut options {
                    SeriesOptionsMap::Area(o) => {
                        o.style.invert_filled_area = !self.style_step.is_multiple_of(2);
                        o.style.relative_gradient = !self.style_step.is_multiple_of(2);
                        o.style.line_type = match self.style_step % 3 {
                            1 => LineType::WithSteps,
                            2 => LineType::Curved,
                            _ => LineType::Simple,
                        };
                    }
                    SeriesOptionsMap::Line(o) => {
                        o.style.line_type = match self.style_step % 3 {
                            1 => LineType::WithSteps,
                            2 => LineType::Curved,
                            _ => LineType::Simple,
                        };
                    }
                    SeriesOptionsMap::Candlestick(o) => {
                        o.style.wick_visible = self.style_step.is_multiple_of(2)
                    }
                    SeriesOptionsMap::Histogram(o) => {
                        o.style.base = 10. * f64::from(self.style_step)
                    }
                    _ => unreachable!(),
                }
                self.chart
                    .replace_series_options(self.price, options)
                    .unwrap();
                self.prepare();
            }
            Message::ToggleLast => {
                self.show_last = !self.show_last;
                let options = sample_options(self.kind, self.show_last);
                self.chart
                    .replace_series_options(self.price, options)
                    .unwrap();
                self.prepare();
            }
        }
        Task::none()
    }
    fn prepare(&mut self) {
        if let Err(error) = self.chart.prepare(self.size, self.ratio, Instant::now()) {
            self.status = error.to_string();
        }
    }
    fn view(&self) -> Element<'_, Message> {
        column![
            text(&self.status),
            row![
                button("Append bar").on_press(Message::Append),
                button("Change style").on_press(Message::Style),
                button("Move stop").on_press(Message::MoveStop),
                button("Add/remove levels").on_press(Message::ToggleLevels),
                button("Cluster labels").on_press(Message::Cluster),
                button("Last-value label").on_press(Message::ToggleLast),
            ]
            .spacing(8),
            self.chart.element().map(Message::Chart)
        ]
        .spacing(4)
        .into()
    }
    fn subscription(&self) -> Subscription<Message> {
        iced::window::events().map(|(id, event)| Message::Window(id, event))
    }
}
fn sample_options(kind: SeriesType, last: bool) -> SeriesOptionsMap {
    match kind {
        SeriesType::Area => {
            let mut o = AreaSeriesOptions::default();
            o.common.last_value_visible = last;
            SeriesOptionsMap::Area(o)
        }
        SeriesType::Line => {
            let mut o = LineSeriesOptions::default();
            o.common.last_value_visible = last;
            SeriesOptionsMap::Line(o)
        }
        SeriesType::Histogram => {
            let mut o = HistogramSeriesOptions::default();
            o.common.last_value_visible = last;
            SeriesOptionsMap::Histogram(o)
        }
        SeriesType::Candlestick => {
            let mut o = CandlestickSeriesOptions::default();
            o.common.last_value_visible = last;
            SeriesOptionsMap::Candlestick(o)
        }
        _ => unreachable!("example accepts supported built-in types"),
    }
}
fn sample_row(kind: SeriesType, time: Time, close: f64) -> BuiltInSeriesDataItem<Time> {
    match kind {
        SeriesType::Area => BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
            time,
            value: close,
            line_color: None,
            top_color: None,
            bottom_color: None,
            custom_values: None,
        })),
        SeriesType::Line => BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time,
            value: close,
            color: None,
            custom_values: None,
        })),
        SeriesType::Histogram => {
            BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
                time,
                value: close,
                color: None,
                custom_values: None,
            }))
        }
        SeriesType::Candlestick => {
            BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(CandlestickData {
                time,
                open: close - 1.,
                high: close + 2.,
                low: close - 2.,
                close,
                color: None,
                border_color: None,
                wick_color: None,
                custom_values: None,
            }))
        }
        _ => unreachable!("example accepts supported built-in types"),
    }
}
fn add_levels(
    chart: &mut Chart,
    series: feather_charts::model::data_layer::SeriesId,
) -> Vec<PriceLineId> {
    [
        (100., "Entry", "#2962ff"),
        (94., "Stop", "#ef5350"),
        (106., "Target", "#26a69a"),
    ]
    .into_iter()
    .map(|(price, title, color)| {
        let mut options = PriceLineOptions::new(price);
        options.title = title.into();
        options.color = color.into();
        chart.add_price_line(series, options).unwrap()
    })
    .collect()
}
fn capture(directory: &std::path::Path, wgpu: bool, kind: SeriesType) {
    use iced::advanced::{
        graphics::geometry::Renderer as _,
        renderer::{Headless, Renderer as _},
    };
    let backend = if wgpu { "wgpu" } else { "tiny-skia" };
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some(backend),
    ))
    .unwrap();
    std::fs::create_dir_all(directory).unwrap();
    let mut scenarios = vec!["default", "cluster", "hidden", "live", "left", "empty"];
    if kind == SeriesType::Area {
        scenarios.extend([
            "inverted", "relative", "curved", "steps", "styles", "single",
        ]);
    }
    let count = scenarios.len() * 4;
    for scenario in scenarios {
        for ratio in [1., 1.25, 1.5, 2.] {
            let mut app = App {
                size: Size::new(800., 500.),
                ratio,
                ..App::new(kind)
            };
            match scenario {
                "cluster" => {
                    let _ = app.update(Message::Cluster);
                }
                "hidden" => {
                    let _ = app.update(Message::ToggleLevels);
                    let _ = app.update(Message::ToggleLast);
                }
                "live" => {
                    let _ = app.update(Message::Append);
                    let _ = app.update(Message::MoveStop);
                }
                "left" => {
                    use feather_charts::model::{
                        axis_snapshots::PriceAxisSide, plot_data::MutablePlotRow,
                        price_scale::PriceScaleOptionsPatch,
                    };
                    let left = app
                        .chart
                        .add_series(
                            SeriesOptionsMap::Candlestick(CandlestickSeriesOptions::default()),
                            0,
                            PriceScalePosition::Left,
                        )
                        .unwrap();
                    let rows = app
                        .chart
                        .model()
                        .series(app.price)
                        .unwrap()
                        .bars()
                        .rows()
                        .to_vec();
                    let data = rows
                        .into_iter()
                        .map(|row| {
                            let values = row.values().unwrap();
                            BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(
                                CandlestickData {
                                    time: row.original_time().clone(),
                                    open: values[0],
                                    high: values[1],
                                    low: values[2],
                                    close: values[3],
                                    color: None,
                                    border_color: None,
                                    wick_color: None,
                                    custom_values: None,
                                },
                            ))
                        })
                        .collect();
                    app.chart.set_data(left, data).unwrap();
                    app.chart
                        .apply_price_axis_options(
                            0,
                            PriceAxisSide::Left,
                            PriceScaleOptionsPatch {
                                visible: Some(true),
                                ..Default::default()
                            },
                        )
                        .unwrap();
                    let mut level = PriceLineOptions::new(100.);
                    level.title = "Long left-side entry title".into();
                    app.chart.add_price_line(left, level).unwrap();
                }
                "inverted" | "relative" | "curved" | "steps" => {
                    let mut options = AreaSeriesOptions::default();
                    options.style.invert_filled_area = scenario == "inverted";
                    options.style.relative_gradient = scenario == "relative";
                    use feather_charts::renderers::draw_line::LineType;
                    options.style.line_type = match scenario {
                        "curved" => LineType::Curved,
                        "steps" => LineType::WithSteps,
                        _ => LineType::Simple,
                    };
                    app.chart
                        .replace_series_options(app.price, SeriesOptionsMap::Area(options))
                        .unwrap();
                }
                "styles" => {
                    let data = (0..120)
                        .map(|i| {
                            let mut row = sample_row(
                                kind,
                                Time::from(UtcTimestamp::new(1_700_000_000. + i as f64 * 60.)),
                                100. + (i as f64 * 0.1).sin() * 8.,
                            );
                            if i % 10 > 4
                                && let BuiltInSeriesDataItem::Area(AreaDataItem::Data(data)) =
                                    &mut row
                            {
                                data.line_color = Some("yellow".into());
                                data.top_color = Some("rgba(0,0,0,0.6)".into());
                                data.bottom_color = Some("rgba(0,0,255,0.2)".into());
                            }
                            row
                        })
                        .collect();
                    app.chart.set_data(app.price, data).unwrap();
                }
                "single" => {
                    app.chart
                        .set_data(
                            app.price,
                            vec![sample_row(
                                kind,
                                Time::from(UtcTimestamp::new(1_700_000_000.)),
                                100.,
                            )],
                        )
                        .unwrap();
                    app.chart.fit_content();
                }
                "empty" => {
                    app.chart.set_data(app.price, vec![]).unwrap();
                    app.chart.set_data(app.volume, vec![]).unwrap();
                }
                _ => {}
            }
            app.prepare();
            renderer.reset(iced::Rectangle::with_size(app.size));
            let mut frame = iced::widget::canvas::Frame::new(&renderer, app.size);
            app.chart.multi_frame().unwrap().draw(&mut frame);
            renderer.draw_geometry(frame.into_geometry());
            let physical = iced::Size::new((800. * ratio) as u32, (500. * ratio) as u32);
            let rgba = renderer.screenshot(physical, ratio, iced::Color::from_rgb8(255, 0, 255));
            assert_eq!(
                rgba,
                renderer.screenshot(physical, ratio, iced::Color::from_rgb8(0, 255, 0)),
                "price-line frame must cover the clear color"
            );
            assert_eq!(
                rgba,
                renderer.screenshot(physical, ratio, iced::Color::BLACK),
                "drawing must remain repeatable"
            );
            let mut ppm = format!("P6\n{} {}\n255\n", physical.width, physical.height).into_bytes();
            for pixel in rgba.as_chunks::<4>().0 {
                ppm.extend_from_slice(&pixel[..3]);
            }
            std::fs::write(
                directory.join(format!("{scenario}-{backend}-{ratio}.ppm")),
                ppm,
            )
            .unwrap();
        }
    }
    println!("{count} {backend} {kind:?} captures passed");
}
fn main() -> iced::Result {
    let args = std::env::args().collect::<Vec<_>>();
    let kind = if args.iter().any(|a| a == "--area") {
        SeriesType::Area
    } else if args.iter().any(|a| a == "--line") {
        SeriesType::Line
    } else if args.iter().any(|a| a == "--histogram") {
        SeriesType::Histogram
    } else {
        SeriesType::Candlestick
    };
    if let Some(index) = args.iter().position(|a| a == "--capture-dir") {
        capture(
            std::path::Path::new(args.get(index + 1).expect("capture directory")),
            args.iter().any(|a| a == "--wgpu"),
            kind,
        );
        return Ok(());
    }
    if std::env::args().any(|a| a == "--smoke") {
        let mut app = App::new(kind);
        app.chart
            .prepare(Size::new(800., 500.), 1.25, Instant::now())
            .unwrap();
        let frame = app.chart.multi_frame().unwrap();
        assert_eq!(frame.panes.len(), 2);
        assert!(frame.panes.iter().all(|pane| !pane.plot.candles.is_empty()
            || !pane.plot.lines.is_empty()
            || !pane.plot.fills.is_empty()));
        app.size = Size::new(800., 500.);
        app.ratio = 1.25;
        for message in [
            Message::Append,
            Message::MoveStop,
            Message::Cluster,
            Message::ToggleLast,
            Message::ToggleLevels,
            Message::ToggleLevels,
        ] {
            let _ = app.update(message);
        }
        assert_eq!(app.levels.len(), 3);
        assert!(
            app.chart.multi_frame().unwrap().panes[0]
                .plot
                .price_lines
                .len()
                >= 3
        );
        println!("public chart API and price-line lifecycle smoke passed");
        return Ok(());
    }
    iced::application(move || App::new(kind), App::update, App::view)
        .title("Feather Charts — public API")
        .subscription(App::subscription)
        .run()
}
