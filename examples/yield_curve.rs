//! Numeric maturities on the shared synchronous chart/widget path.
use feather_charts::{
    api::*,
    model::{
        data_consumer::{AreaData, AreaDataItem, BuiltInSeriesDataItem, LineData, LineDataItem},
        data_layer::SeriesId,
        pane::PriceScalePosition,
        series_options::{
            AreaSeriesOptionsPatch, AreaStyleOptionsPatch, LineSeriesOptionsPatch,
            LineStyleOptionsPatch,
        },
        yield_curve_horz_scale_behavior::YieldCurveHorzScaleBehavior,
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
#[path = "yield_curve/captures.rs"]
mod captures;
#[path = "yield_curve/region.rs"]
mod region;
const TENORS: [f64; 12] = [1., 2., 3., 6., 12., 24., 36., 60., 84., 120., 240., 360.];
const VALUES: [f64; 12] = [
    5.378, 5.372, 5.271, 5.094, 4.739, 4.237, 4.036, 3.887, 3.921, 4.007, 4.366, 4.29,
];
fn maturity(months: f64) -> Maturity {
    Maturity::from_months(months).unwrap()
}
fn item(area: bool, months: f64, value: f64) -> BuiltInSeriesDataItem<Maturity> {
    if area {
        BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
            time: maturity(months),
            value,
            line_color: None,
            top_color: None,
            bottom_color: None,
            custom_values: None,
        }))
    } else {
        BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time: maturity(months),
            value,
            color: None,
            custom_values: None,
        }))
    }
}
fn data(area: bool) -> Vec<BuiltInSeriesDataItem<Maturity>> {
    TENORS
        .into_iter()
        .zip(VALUES)
        .map(|(t, v)| item(area, t, v + if area { 0.15 } else { 0. }))
        .collect()
}
struct App {
    chart: YieldCurveChart,
    line: SeriesId,
    area: SeriesId,
    markers: UpDownMarkersId,
    size: Size,
    ratio: f32,
    step: usize,
    live: bool,
    custom: bool,
    coarse: bool,
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
        let mut chart = YieldCurveChart::new(Default::default()).unwrap();
        let line = chart
            .add_line_series(
                LineSeriesOptionsPatch {
                    style: LineStyleOptionsPatch {
                        line_type: Some(LineType::Curved),
                        point_markers_visible: Some(true),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                0,
                PriceScalePosition::Left,
            )
            .unwrap();
        let area = chart
            .add_area_series(
                AreaSeriesOptionsPatch {
                    style: AreaStyleOptionsPatch {
                        line_color: Some("#9c27b0".into()),
                        top_color: Some("rgba(156,39,176,0.15)".into()),
                        bottom_color: Some("rgba(156,39,176,0.02)".into()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                0,
                PriceScalePosition::Left,
            )
            .unwrap();
        let markers = chart.add_up_down_markers(line, Default::default()).unwrap();
        chart.set_up_down_data(markers, data(false)).unwrap();
        chart.set_data(area, data(true)).unwrap();
        chart
            .add_pane_primitive(
                0,
                TextWatermark::new(TextWatermarkOptions {
                    lines: vec![TextWatermarkLine {
                        text: "Yield curves".into(),
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
        let region = region::Scene::new(&mut chart, line);
        chart.fit_content();
        Self {
            chart,
            line,
            area,
            markers,
            size: Size::ZERO,
            ratio: 1.,
            step: 0,
            live,
            custom: false,
            coarse: false,
            status: "Numeric maturities in months; drag the box or its right edge".into(),
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
            Message::Change | Message::Tick(_) => {
                let i = [3, 4, 6][self.step % 3];
                let delta = [0.1, 0., -0.1][(self.step / 3) % 3];
                self.chart.update_up_down_data(
                    self.markers,
                    item(false, TENORS[i], VALUES[i] + delta),
                    true,
                    now,
                )?;
                self.step += 1;
            }
            Message::Extend => self
                .chart
                .update_data(self.area, item(true, 420., 4.3), false)?,
            Message::Reset => {
                self.chart.set_up_down_data(self.markers, data(false))?;
                self.chart.set_data(self.area, data(true))?;
                self.chart.fit_content();
            }
            Message::Empty => {
                self.chart.set_data(self.line, vec![])?;
                self.chart.set_data(self.area, vec![])?;
                self.chart.clear_up_down_markers(self.markers)?;
            }
            Message::Domain => {
                self.coarse = !self.coarse;
                self.chart.replace_yield_curve_options(YieldCurveOptions {
                    base_resolution: if self.coarse { 12. } else { 1. },
                    start_time_range: if self.coarse { 3. } else { 0. },
                    ..Default::default()
                })?;
                self.chart.fit_content();
            }
            Message::Formatter => {
                self.custom = !self.custom;
                self.chart.set_maturity_formatter(
                    self.custom
                        .then(|| Rc::new(|v: &Maturity| format!("{} mo", v.months())) as _),
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
                        time: maturity(TENORS[i]),
                        value: VALUES[i],
                        sign,
                    })
                    .collect(),
            )?,
            Message::Clear => self.chart.clear_up_down_markers(self.markers)?,
            Message::Pause => self.live = !self.live,
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
                                ChartEvent::DataChanged { .. }
                                | ChartEvent::DoubleClick(_)
                                | ChartEvent::VisibleLogicalRangeChanged(_)
                                | ChartEvent::VisibleHorizontalRangeChanged(_)
                                | ChartEvent::HorizontalAxisSizeChanged { .. } => {}
                                ChartEvent::PrimitiveEdit(e) => {
                                    if let Err(error) = self.region.edit(&mut self.chart, &e) {
                                        self.status = error.to_string();
                                    }
                                    self.prepare(Instant::now());
                                }
                                ChartEvent::Click(p) | ChartEvent::CrosshairMoved(p) => {
                                    self.status = format!(
                                        "Maturity: {} | {} curves",
                                        p.time.map_or_else(|| "—".into(), Maturity::label),
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
            row![
                button("Change yield").on_press(Message::Change),
                button("Extend 35Y").on_press(Message::Extend),
                button("Reset curves").on_press(Message::Reset),
                button("Empty").on_press(Message::Empty)
            ]
            .spacing(6),
            row![
                button("Domain step").on_press(Message::Domain),
                button("Labels").on_press(Message::Formatter),
                button("Manual markers").on_press(Message::Manual),
                button("Clear markers").on_press(Message::Clear),
                button(if self.live {
                    "Pause feed"
                } else {
                    "Resume feed"
                })
                .on_press(Message::Pause)
            ]
            .spacing(6),
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
        .title("Feather Charts — yield curves")
        .subscription(App::subscription)
        .run()
}
