//! A host application using the public owner/widget API, without demo support.
use feather_charts::{
    api::{
        Chart, ChartEvent, ChartMessage, MarkerCollectionId, MarkerZOrder, PriceLineId,
        PriceLineOptions, SeriesMarkersOptions,
    },
    model::{
        data_consumer::{
            AreaData, AreaDataItem, BarData, BarDataItem, BaselineData, BaselineDataItem,
            BuiltInSeriesDataItem, CandlestickData, CandlestickDataItem, HistogramData,
            HistogramDataItem, LineData, LineDataItem,
        },
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{
            AreaSeriesOptions, BarSeriesOptions, BaseValuePrice, BaseValueType,
            BaselineSeriesOptions, CandlestickSeriesOptions, HistogramSeriesOptions,
            LastPriceAnimationMode, LineSeriesOptions, PriceFormat, PriceFormatBuiltIn,
            PriceFormatBuiltInType, SeriesOptionsMap, SeriesType,
        },
    },
};
use iced::{
    Element, Size, Subscription, Task,
    widget::{button, column, row, text},
};
use std::time::Instant;
#[path = "public_chart/hover.rs"]
mod hover_demo;
#[path = "public_chart/markers.rs"]
mod marker_demo;
#[path = "public_chart/primitives.rs"]
mod primitive_demo;
#[cfg(test)]
#[path = "public_chart/tests.rs"]
mod tests;
use marker_demo::sample_markers;

#[path = "public_chart/image_watermark.rs"]
mod image_watermark_demo;
#[path = "public_chart/software_drag_profile.rs"]
mod software_drag_profile;
#[path = "public_chart/up_down_markers.rs"]
mod up_down_demo;
#[path = "public_chart/watermark.rs"]
mod watermark_demo;

struct App {
    up_down: Option<up_down_demo::Scene>,
    image_watermark: Option<image_watermark_demo::Scene>,
    watermark: Option<watermark_demo::Scene>,
    primitive_scene: Option<primitive_demo::Scene>,
    annotations: Option<MarkerCollectionId>,
    marker_step: usize,
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
    live_since: Option<Instant>,
}
#[derive(Clone, Debug)]
enum Message {
    UpDownLatest,
    UpDownHistorical,
    UpDownManual,
    UpDownClear,
    UpDownLifetime,
    UpDownColors,
    UpDownPause,
    ImageOpacity,
    ImagePadding,
    ImageLimits,
    ImageReplace,
    ToggleWatermark,
    WatermarkAlignment,
    WatermarkText,
    WatermarkStyle,
    TogglePrimitive,
    MovePrimitive,
    PrimitiveLayer,
    ToggleHoverOrder,
    ToggleMarkers,
    MarkerLayer,
    Chart(ChartMessage),
    Window(iced::window::Id, iced::window::Event),
    Ratio(f32),
    Append,
    ToggleLive,
    FeedTick(Instant),
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
        let dense = std::env::args().any(|a| a == "--conflation");
        let count = if dense { 8192 } else { 120 };
        if dense {
            chart.apply_time_scale_options(
                feather_charts::model::time_scale_options::HorzScaleOptionsPatch {
                    enable_conflation: Some(true),
                    min_bar_spacing: Some(0.001),
                    precompute_conflation_on_init: Some(true),
                    ..Default::default()
                },
            );
        }
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
        let times = (0..count).map(|i| {
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
        if std::env::args().any(|a| a == "--hover-selection" || a == "--primitives") {
            hover_demo::add_overlays(&mut chart);
        }
        chart.fit_content();
        let levels = add_levels(&mut chart, price);
        let annotations = std::env::args()
            .any(|a| a == "--series-markers" || a == "--hover-selection" || a == "--primitives")
            .then(|| {
                chart
                    .add_series_markers(
                        volume,
                        sample_markers(true),
                        SeriesMarkersOptions::default(),
                    )
                    .unwrap();
                chart
                    .add_series_markers(
                        price,
                        sample_markers(false),
                        SeriesMarkersOptions::default(),
                    )
                    .unwrap()
            });
        let primitive_scene = std::env::args()
            .any(|a| a == "--primitives")
            .then(|| primitive_demo::Scene::new(&mut chart, price));
        let watermark = std::env::args()
            .any(|a| a == "--text-watermark")
            .then(|| watermark_demo::Scene::new(&mut chart));
        let image_watermark = std::env::args()
            .any(|a| a == "--image-watermark")
            .then(|| image_watermark_demo::Scene::new(&mut chart));
        let up_down = std::env::args()
            .any(|a| a == "--up-down-markers")
            .then(|| up_down_demo::Scene::new(&mut chart, price, kind, count));
        Self {
            up_down,
            image_watermark,
            watermark,
            primitive_scene,
            annotations,
            marker_step: 0,
            chart,
            size: Size::ZERO,
            ratio: 1.,
            status: "Move/click; scroll into history to compare visible and global last prices"
                .into(),
            price,
            volume,
            levels,
            latest: count - 1,
            show_last: true,
            kind,
            style_step: 0,
            live_since: None,
        }
    }
}
impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::UpDownLatest
            | Message::UpDownHistorical
            | Message::UpDownManual
            | Message::UpDownClear
            | Message::UpDownLifetime
            | Message::UpDownColors => {
                if let Some(scene) = &mut self.up_down
                    && let Err(error) =
                        scene.change(&mut self.chart, &message, self.latest, Instant::now())
                {
                    self.status = format!("Update rejected: {error:?}");
                }
                self.prepare();
            }
            Message::UpDownPause => {
                self.live_since = None;
                let mut options = sample_options(self.kind, self.show_last);
                match &mut options {
                    SeriesOptionsMap::Line(o) => {
                        o.style.last_price_animation = LastPriceAnimationMode::Disabled
                    }
                    SeriesOptionsMap::Area(o) => {
                        o.style.last_price_animation = LastPriceAnimationMode::Disabled
                    }
                    _ => unreachable!(),
                }
                self.chart
                    .replace_series_options(self.price, options)
                    .unwrap();
                self.prepare();
            }
            Message::ImageOpacity
            | Message::ImagePadding
            | Message::ImageLimits
            | Message::ImageReplace => {
                if let Some(scene) = &mut self.image_watermark {
                    scene.change(&mut self.chart, &message);
                }
                self.prepare();
            }
            Message::ToggleWatermark
            | Message::WatermarkAlignment
            | Message::WatermarkText
            | Message::WatermarkStyle => {
                if let Some(scene) = &mut self.watermark {
                    scene.change(&mut self.chart, &message);
                }
                self.prepare();
            }
            Message::ToggleLive => {
                self.live_since = self.live_since.is_none().then(Instant::now);
            }
            Message::FeedTick(at) => {
                if self.live_since.is_some_and(|since| at >= since) {
                    if let Some(scene) = &mut self.up_down {
                        if let Err(error) =
                            scene.change(&mut self.chart, &Message::UpDownLatest, self.latest, at)
                        {
                            self.status = format!("Update rejected: {error:?}");
                        }
                        self.prepare();
                        return Task::none();
                    }
                    return self.update(Message::Append);
                }
            }
            Message::TogglePrimitive | Message::MovePrimitive | Message::PrimitiveLayer => {
                if let Some(scene) = &mut self.primitive_scene {
                    match message {
                        Message::TogglePrimitive => scene.toggle(&mut self.chart),
                        Message::MovePrimitive => scene.shift(&mut self.chart),
                        _ => scene.layer(&mut self.chart),
                    }
                }
                self.prepare();
            }
            Message::ToggleHoverOrder => {
                self.chart
                    .set_hovered_series_on_top(!self.chart.hovered_series_on_top());
                self.prepare();
            }
            Message::ToggleMarkers => {
                if let Some(id) = self.annotations {
                    let markers = if self.chart.series_markers(id).unwrap().is_empty() {
                        sample_markers(false)
                    } else {
                        vec![]
                    };
                    self.chart.set_series_markers(id, markers).unwrap();
                } else {
                    self.annotations = Some(
                        self.chart
                            .add_series_markers(
                                self.price,
                                sample_markers(false),
                                SeriesMarkersOptions::default(),
                            )
                            .unwrap(),
                    );
                }
                self.prepare();
            }
            Message::MarkerLayer => {
                self.marker_step += 1;
                if let Some(id) = self.annotations {
                    self.chart
                        .replace_series_marker_options(
                            id,
                            SeriesMarkersOptions {
                                auto_scale: true,
                                z_order: match self.marker_step % 3 {
                                    1 => MarkerZOrder::AboveSeries,
                                    2 => MarkerZOrder::Top,
                                    _ => MarkerZOrder::Normal,
                                },
                            },
                        )
                        .unwrap();
                }
                self.prepare();
            }
            Message::Chart(message) => {
                if let ChartMessage::Bounds(size) = &message {
                    self.size = *size;
                }
                match self.chart.update(message) {
                    Ok(events) => {
                        for event in events {
                            let (kind, data) = match event {
                                ChartEvent::AttributionActivated { url } => {
                                    self.status = format!("Host link action: {url}");
                                    continue;
                                }
                                ChartEvent::DataChanged { .. }
                                | ChartEvent::DoubleClick(_)
                                | ChartEvent::VisibleLogicalRangeChanged(_)
                                | ChartEvent::VisibleHorizontalRangeChanged(_)
                                | ChartEvent::HorizontalAxisSizeChanged { .. } => continue,
                                ChartEvent::PrimitiveEdit(edit) => {
                                    let result = self
                                        .primitive_scene
                                        .as_mut()
                                        .map_or(Ok(()), |scene| scene.edit(&mut self.chart, &edit));
                                    self.prepare();
                                    self.status = result.err().map_or_else(
                                        || format!("Primitive edit: {:?}", edit.phase),
                                        |error| error.to_string(),
                                    );
                                    continue;
                                }
                                ChartEvent::Click(data) => {
                                    if let Some(scene) = &mut self.primitive_scene
                                        && data.primitive.as_ref().map(|h| h.primitive)
                                            != scene.region
                                    {
                                        scene.deselect(&mut self.chart);
                                    }
                                    self.prepare();
                                    ("Click", data)
                                }
                                ChartEvent::CrosshairMoved(data) => ("Crosshair", data),
                            };
                            self.status = format!(
                                "{kind}: time {:?}, price {:?}, {} series, marker {:?}, target {:?}, primitive {:?}",
                                data.time,
                                data.price,
                                data.series_data.len(),
                                data.marker,
                                data.hit,
                                data.primitive
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
                    SeriesOptionsMap::Baseline(o) => {
                        o.style.base_value = BaseValueType::Price(BaseValuePrice {
                            price: 100. + (f64::from(self.style_step) - 3.) * 3.,
                        });
                        o.style.relative_gradient = !self.style_step.is_multiple_of(2);
                        o.style.line_type = match self.style_step % 3 {
                            1 => LineType::WithSteps,
                            2 => LineType::Curved,
                            _ => LineType::Simple,
                        };
                    }
                    SeriesOptionsMap::Bar(o) => {
                        o.style.open_visible = self.style_step.is_multiple_of(2);
                        o.style.thin_bars = self.style_step.is_multiple_of(3);
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
        let content = column![
            // Hover details must not resize the chart when the window is narrow.
            text(&self.status)
                .width(iced::Length::Fill)
                .wrapping(text::Wrapping::None),
            row![
                button("Append bar").on_press(Message::Append),
                button("Change style").on_press(Message::Style),
                button("Move stop").on_press(Message::MoveStop),
                button("Add/remove levels").on_press(Message::ToggleLevels),
                button("Cluster labels").on_press(Message::Cluster),
                button("Last-value label").on_press(Message::ToggleLast),
            ]
            .spacing(8),
            row![
                button(if self.live_since.is_some() {
                    "Stop live feed"
                } else {
                    "Start live feed"
                })
                .width(130)
                .on_press(Message::ToggleLive),
                button("Add/clear markers").on_press(Message::ToggleMarkers),
                button("Marker layer").on_press(Message::MarkerLayer),
                button(if self.chart.hovered_series_on_top() {
                    "Raise hovered: on"
                } else {
                    "Raise hovered: off"
                })
                .on_press(Message::ToggleHoverOrder)
            ]
            .spacing(8),
        ]
        .spacing(4);
        let content = if self.primitive_scene.is_some() {
            content.push(
                row![
                    button("Add/remove region").on_press(Message::TogglePrimitive),
                    button("Move region").on_press(Message::MovePrimitive),
                    button("Primitive layer").on_press(Message::PrimitiveLayer)
                ]
                .spacing(8),
            )
        } else {
            content
        };
        let content = if self.watermark.is_some() {
            content.push(
                row![
                    button("Show/hide watermark").on_press(Message::ToggleWatermark),
                    button("Watermark alignment").on_press(Message::WatermarkAlignment),
                    button("Watermark text").on_press(Message::WatermarkText),
                    button("Watermark style").on_press(Message::WatermarkStyle)
                ]
                .spacing(8),
            )
        } else {
            content
        };
        let content = if self.image_watermark.is_some() {
            content.push(
                row![
                    button("Image opacity").on_press(Message::ImageOpacity),
                    button("Image padding").on_press(Message::ImagePadding),
                    button("Image limits").on_press(Message::ImageLimits),
                    button("Replace image").on_press(Message::ImageReplace)
                ]
                .spacing(8),
            )
        } else {
            content
        };
        let content = if let Some(scene) = &self.up_down {
            content
                .push(
                    row![
                        button("Latest change").on_press(Message::UpDownLatest),
                        button("History change").on_press(Message::UpDownHistorical),
                        button("Manual markers").on_press(Message::UpDownManual),
                        button("Clear updates").on_press(Message::UpDownClear)
                    ]
                    .spacing(8),
                )
                .push(
                    row![
                        button(text(scene.lifetime_label())).on_press(Message::UpDownLifetime),
                        button("Marker colors").on_press(Message::UpDownColors),
                        button("Pause feed/pulse").on_press(Message::UpDownPause)
                    ]
                    .spacing(8),
                )
        } else {
            content
        };
        content
            .push(self.chart.element().map(Message::Chart))
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
fn sample_options(kind: SeriesType, last: bool) -> SeriesOptionsMap {
    let animation = if std::env::args().any(|a| a == "--last-price-animation-continuous") {
        LastPriceAnimationMode::Continuous
    } else if std::env::args().any(|a| a == "--last-price-animation") {
        LastPriceAnimationMode::OnDataUpdate
    } else {
        LastPriceAnimationMode::Disabled
    };
    match kind {
        SeriesType::Baseline => {
            let mut o = BaselineSeriesOptions::default();
            o.style.last_price_animation = animation;
            o.common.last_value_visible = last;
            o.style.base_value = BaseValueType::Price(BaseValuePrice { price: 100. });
            SeriesOptionsMap::Baseline(o)
        }
        SeriesType::Bar => {
            let mut o = BarSeriesOptions::default();
            o.common.last_value_visible = last;
            SeriesOptionsMap::Bar(o)
        }
        SeriesType::Area => {
            let mut o = AreaSeriesOptions::default();
            o.style.last_price_animation = animation;
            o.common.last_value_visible = last;
            SeriesOptionsMap::Area(o)
        }
        SeriesType::Line => {
            let mut o = LineSeriesOptions::default();
            o.style.last_price_animation = animation;
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
        SeriesType::Baseline => {
            BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(BaselineData {
                time,
                value: close,
                top_fill_color1: None,
                top_fill_color2: None,
                bottom_fill_color1: None,
                bottom_fill_color2: None,
                top_line_color: None,
                bottom_line_color: None,
                custom_values: None,
            }))
        }
        SeriesType::Bar => BuiltInSeriesDataItem::Bar(BarDataItem::Data(BarData {
            time,
            open: close - 1.,
            high: close + 2.,
            low: close - 2.,
            close,
            color: None,
            custom_values: None,
        })),
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
    use feather_charts::model::{
        axis_snapshots::PriceAxisSide, price_scale::PriceScaleOptionsPatch,
    };
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
    let mut scenarios = vec![
        "default",
        "cluster",
        "hidden",
        "live",
        "live-narrow",
        "live-burst",
        "left",
        "empty",
    ];
    if std::env::args().any(|a| a == "--series-markers") {
        scenarios.extend([
            "annotation-stack",
            "annotation-price",
            "annotation-inverted",
            "annotation-layers",
            "annotation-navigation",
            "annotation-line-overlap",
        ]);
    }
    if std::env::args().any(|a| a == "--hover-selection") {
        scenarios.extend([
            "hover-line",
            "hover-area",
            "hover-area-disabled",
            "hover-baseline-disabled",
            "hover-fill-only-disabled",
            "hover-baseline",
            "hover-fill-only",
            "hover-bar",
            "hover-candle",
            "hover-histogram",
            "hover-disabled",
            "hover-crossover",
            "hover-removed",
            "hover-narrow",
        ]);
    }
    if kind == SeriesType::Area {
        scenarios.extend([
            "inverted", "relative", "curved", "steps", "styles", "single",
        ]);
    }
    if kind == SeriesType::Bar {
        scenarios.extend(["fat", "no-open", "compressed", "styles", "single"]);
    }
    if kind == SeriesType::Baseline {
        scenarios.extend([
            "inverted",
            "relative",
            "curved",
            "steps",
            "styles",
            "single",
            "base-above",
            "base-below",
            "markers",
        ]);
    }
    if std::env::args().any(|a| a == "--marker-captures-only") {
        scenarios.retain(|s| s.starts_with("annotation-"));
    }
    if std::env::args().any(|a| a == "--hover-captures-only") {
        scenarios.retain(|s| s.starts_with("hover-"));
    }
    if std::env::args().any(|a| a == "--primitives") {
        scenarios.extend([
            "primitive-normal",
            "primitive-top",
            "primitive-bottom",
            "primitive-hover",
            "primitive-updated",
            "primitive-removed",
            "primitive-narrow",
            "primitive-selected",
            "primitive-selected-narrow",
            "primitive-moving",
            "primitive-resized",
            "primitive-cancelled",
        ]);
    }
    if std::env::args().any(|a| a == "--primitive-captures-only") {
        scenarios.retain(|s| s.starts_with("primitive-"));
    }
    if std::env::args().any(|a| a == "--primitive-edit-captures-only") {
        scenarios.retain(|s| {
            matches!(
                *s,
                "primitive-selected"
                    | "primitive-selected-narrow"
                    | "primitive-moving"
                    | "primitive-resized"
                    | "primitive-cancelled"
            )
        });
    }
    if std::env::args().any(|a| a == "--append-captures-only") {
        scenarios.retain(|scenario| scenario.starts_with("live"));
    }
    if std::env::args().any(|a| a == "--conflation-captures-only") {
        scenarios.retain(|s| {
            matches!(
                *s,
                "default" | "live-narrow" | "inverted" | "curved" | "steps"
            )
        });
    }
    let count = scenarios.len() * 4;
    for scenario in scenarios {
        for ratio in [1., 1.25, 1.5, 2.] {
            let mut app = App {
                size: Size::new(800., 500.),
                ratio,
                ..App::new(kind)
            };
            if matches!(scenario, "compressed" | "fat") {
                app.prepare();
            }
            match scenario {
                scenario if scenario.starts_with("primitive-") => {
                    primitive_demo::setup_capture(&mut app, scenario)
                }
                scenario if scenario.starts_with("hover-") => {
                    hover_demo::setup_capture(&mut app, scenario)
                }
                scenario if scenario.starts_with("annotation-") => marker_demo::setup_capture(
                    &mut app.chart,
                    app.price,
                    app.annotations.unwrap(),
                    scenario,
                    app.size,
                    app.ratio,
                ),
                "cluster" => {
                    let _ = app.update(Message::Cluster);
                }
                "hidden" => {
                    let _ = app.update(Message::ToggleLevels);
                    let _ = app.update(Message::ToggleLast);
                }
                "live" | "live-narrow" | "live-burst" => {
                    if scenario == "live-narrow" {
                        app.size = Size::new(400., 300.);
                    }
                    for _ in 0..if scenario == "live-burst" { 32 } else { 1 } {
                        let _ = app.update(Message::Append);
                    }
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
                "inverted" | "relative" | "curved" | "steps" | "base-above" | "base-below"
                | "markers" => {
                    use feather_charts::renderers::draw_line::LineType;
                    let line_type = match scenario {
                        "curved" => LineType::Curved,
                        "steps" => LineType::WithSteps,
                        _ => LineType::Simple,
                    };
                    let mut options = sample_options(kind, true);
                    match &mut options {
                        SeriesOptionsMap::Area(o) => {
                            o.style.invert_filled_area = scenario == "inverted";
                            o.style.relative_gradient = scenario == "relative";
                            o.style.line_type = line_type;
                        }
                        SeriesOptionsMap::Baseline(o) => {
                            o.style.relative_gradient = scenario == "relative";
                            o.style.line_type = line_type;
                            if scenario == "base-above" || scenario == "base-below" {
                                o.style.base_value = BaseValueType::Price(BaseValuePrice {
                                    price: if scenario == "base-above" { 150. } else { 50. },
                                });
                            }
                            if scenario == "markers" {
                                o.style.line_visible = false;
                                o.style.point_markers_visible = true;
                            }
                            if scenario == "inverted" {
                                app.chart
                                    .apply_price_axis_options(
                                        0,
                                        PriceAxisSide::Right,
                                        PriceScaleOptionsPatch {
                                            invert_scale: Some(true),
                                            ..Default::default()
                                        },
                                    )
                                    .unwrap();
                            }
                        }
                        _ => unreachable!(),
                    }
                    app.chart
                        .replace_series_options(app.price, options)
                        .unwrap();
                }
                "fat" | "no-open" | "compressed" => {
                    let mut o = BarSeriesOptions::default();
                    o.style.thin_bars = scenario != "fat";
                    o.style.open_visible = scenario != "no-open";
                    app.chart
                        .replace_series_options(app.price, SeriesOptionsMap::Bar(o))
                        .unwrap();
                    if matches!(scenario, "compressed" | "fat") {
                        app.chart.apply_time_scale_options(
                            feather_charts::model::time_scale_options::HorzScaleOptionsPatch {
                                bar_spacing: Some(if scenario == "compressed" { 0.5 } else { 20. }),
                                ..Default::default()
                            },
                        );
                    }
                }
                "styles" => {
                    let data =
                        (0..120)
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
                                if i % 10 > 4
                                    && let BuiltInSeriesDataItem::Bar(BarDataItem::Data(data)) =
                                        &mut row
                                {
                                    data.color = Some("yellow".into());
                                }
                                if i % 10 > 4
                                    && let BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(
                                        data,
                                    )) = &mut row
                                {
                                    data.top_fill_color1 = Some("rgba(0,0,255,0.6)".into());
                                    data.bottom_fill_color2 = Some("rgba(255,255,0,0.6)".into());
                                    data.top_line_color = Some("blue".into());
                                    data.bottom_line_color = Some("yellow".into());
                                }
                                row
                            })
                            .collect();
                    app.chart.set_data(app.price, data).unwrap();
                }
                "single" => {
                    if kind == SeriesType::Baseline {
                        let mut o = BaselineSeriesOptions::default();
                        o.style.base_value = BaseValueType::Price(BaseValuePrice { price: 100. });
                        o.style.relative_gradient = true;
                        app.chart
                            .replace_series_options(app.price, SeriesOptionsMap::Baseline(o))
                            .unwrap();
                    }

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
            if scenario == "compressed" {
                assert!(
                    app.chart.model().time_scale().bar_spacing() < 1.,
                    "capture must actually use compressed spacing"
                );
            }
            if scenario == "fat" {
                assert!(
                    app.chart.frame().plot.candles[0].size.width > 1.5,
                    "capture must show thick stems"
                );
            }
            renderer.reset(iced::Rectangle::with_size(app.size));
            let mut frame = iced::widget::canvas::Frame::new(&renderer, app.size);
            app.chart.multi_frame().unwrap().draw(&mut frame);
            renderer.draw_geometry(frame.into_geometry());
            let physical = iced::Size::new(
                (app.size.width * ratio) as u32,
                (app.size.height * ratio) as u32,
            );
            let rgba = renderer.screenshot(physical, ratio, iced::Color::from_rgb8(255, 0, 255));
            hover_demo::verify_pixels(&app, scenario, &rgba, physical.width as usize);
            primitive_demo::verify_pixels(&app, scenario, &rgba, physical.width as usize);
            if scenario == "annotation-line-overlap" {
                let marker = &app.chart.frame().plot.annotations[0].items[0];
                let plot = app.chart.frame().layout.plot;
                let x = ((marker.center.x + plot.origin.x) * f64::from(ratio)).round() as usize;
                let y = ((marker.center.y + plot.origin.y) * f64::from(ratio)).round() as usize;
                let pixel = &rgba[(y * physical.width as usize + x) * 4..][..3];
                assert_eq!(
                    pixel,
                    [255, 0, 0],
                    "normal marker must cover its own price line"
                );
            }
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
    } else if args.iter().any(|a| a == "--baseline") {
        SeriesType::Baseline
    } else if args.iter().any(|a| a == "--bar") {
        SeriesType::Bar
    } else if args.iter().any(|a| a == "--line")
        || (args.iter().any(|a| a == "--up-down-markers")
            && !args
                .iter()
                .any(|a| a == "--histogram" || a == "--candlestick"))
    {
        SeriesType::Line
    } else if args.iter().any(|a| a == "--histogram") {
        SeriesType::Histogram
    } else {
        SeriesType::Candlestick
    };
    if args.iter().any(|a| a == "--up-down-markers")
        && !matches!(kind, SeriesType::Line | SeriesType::Area)
    {
        eprintln!("Up/down update markers support --line or --area.");
        return Ok(());
    }
    if let Some(index) = args.iter().position(|a| a == "--up-down-captures") {
        up_down_demo::capture(
            std::path::Path::new(args.get(index + 1).expect("capture directory")),
            args.iter().any(|a| a == "--wgpu"),
        );
        return Ok(());
    }
    if let Some(index) = args.iter().position(|a| a == "--image-watermark-captures") {
        image_watermark_demo::capture(
            std::path::Path::new(args.get(index + 1).expect("capture directory")),
            args.iter().any(|a| a == "--wgpu"),
        );
        return Ok(());
    }
    if let Some(index) = args.iter().position(|a| a == "--watermark-captures") {
        watermark_demo::capture(
            std::path::Path::new(args.get(index + 1).expect("capture directory")),
            args.iter().any(|a| a == "--wgpu"),
        );
        return Ok(());
    }
    if args.iter().any(|a| a == "--cached-geometry-check") {
        software_drag_profile::check_cached(args.iter().any(|a| a == "--wgpu"));
        return Ok(());
    }
    if args.iter().any(|a| a == "--dense-zoom-profile") {
        software_drag_profile::zoom();
        return Ok(());
    }
    if args.iter().any(|a| a == "--software-drag-profile") {
        software_drag_profile::run();
        return Ok(());
    }
    if args.iter().any(|a| a == "--primitive-edit-profile") {
        primitive_demo::profile();
        return Ok(());
    }
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
            Message::Style,
            Message::MoveStop,
            Message::Cluster,
            Message::ToggleLast,
            Message::ToggleLevels,
            Message::ToggleLevels,
        ] {
            let _ = app.update(message);
        }
        assert_eq!(app.levels.len(), 3);
        if let Some(id) = app.annotations {
            assert!(!app.chart.frame().plot.annotation_geometry.is_empty());
            let _ = app.update(Message::MarkerLayer);
            let _ = app.update(Message::ToggleMarkers);
            assert!(app.chart.series_markers(id).unwrap().is_empty());
            let _ = app.update(Message::ToggleMarkers);
            assert!(!app.chart.series_markers(id).unwrap().is_empty());
        }
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
    let live = args.iter().any(|a| a == "--live-feed");
    iced::application(
        move || {
            let mut app = App::new(kind);
            app.live_since = live.then(Instant::now);
            app
        },
        App::update,
        App::view,
    )
    .title("Feather Charts — public API")
    .subscription(App::subscription)
    .run()
}
