//! An external consumer: session/phase items, intervals, annotations and export.
#[path = "custom_horizontal/behavior.rs"]
mod behavior;
#[path = "custom_horizontal/captures.rs"]
mod captures;
#[path = "numeric_custom_series/view.rs"]
mod interval;
#[path = "support/editable_box.rs"]
mod region;
use behavior::{Options, SessionKey, SessionPoint, Sessions};
use feather_charts::{
    api::*,
    model::{
        data_consumer::*, data_layer::SeriesId, icustom_series::CustomSeriesDataItem,
        localization_options::LocalizationOptions, pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
    },
};
use iced::{
    Element, Size, Subscription, Task,
    widget::{button, column, row, text},
};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
type Intervals = CustomSeriesHandle<interval::View<SessionPoint, SessionKey>>;
struct App {
    chart: Chart<Sessions>,
    line: SeriesId,
    custom: Intervals,
    region: region::Scene,
    size: Size,
    ratio: f32,
    next: usize,
    step: usize,
    live: bool,
    long_labels: bool,
    status: String,
    exporter: Option<ChartExporter>,
    preview: Option<iced::widget::image::Handle>,
}
#[derive(Clone, Debug)]
enum Message {
    Chart(ChartMessage),
    Window(iced::window::Id, iced::window::Event),
    Ratio(f32),
    Tick(Instant),
    Latest,
    History,
    Append,
    Pop,
    Empty,
    Reset,
    Labels,
    Invalid,
    Transfer,
    Swap,
    Pause,
    Capture,
    Hide,
}
fn value(index: usize) -> f64 {
    4.5 + (index as f64 * 0.36).sin() * 0.5
}
// Deliberate gaps prove that logical bars are evenly spaced despite key distance.
fn point(index: usize) -> SessionPoint {
    SessionPoint::at(index * 2 + index / 5 * 3)
}
fn line_row(index: usize, delta: f64) -> BuiltInSeriesDataItem<SessionPoint> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: point(index),
        value: value(index) + delta,
        color: None,
        custom_values: None,
    }))
}
fn datum(
    index: usize,
    delta: f64,
) -> CustomSeriesDataItem<interval::Datum<SessionPoint>, SessionPoint> {
    let v = value(index) + delta;
    CustomSeriesDataItem::Data(interval::Datum {
        time: point(index),
        low: v - 0.2,
        high: v + 0.25,
        value: v,
    })
}
impl App {
    fn new(live: bool, exporter: Option<ChartExporter>) -> Self {
        let mut chart = Chart::with_custom_behavior(
            || Sessions::new(Default::default()),
            Default::default(),
            Default::default(),
            LocalizationOptions::new("en-US", ""),
        )
        .unwrap();
        let line = chart
            .add_series(
                SeriesOptionsMap::Line(Default::default()),
                0,
                PriceScalePosition::Right,
            )
            .unwrap();
        chart.add_pane();
        let custom = chart
            .add_custom_series(
                interval::View::<SessionPoint, SessionKey>::default(),
                Default::default(),
                1,
                PriceScalePosition::Right,
            )
            .unwrap();
        let region = region::Scene::new(&mut chart, line, 6.0..10.0);
        let mut app = Self {
            chart,
            line,
            custom,
            region,
            size: Size::ZERO,
            ratio: 1.,
            next: 24,
            step: 0,
            live,
            long_labels: false,
            status: "Session/phase domain; hover for original items and interval values".into(),
            exporter,
            preview: None,
        };
        app.action(&Message::Reset).unwrap();
        app
    }
    fn prepare(&mut self, now: Instant) -> Result<(), ChartError> {
        self.chart.prepare(self.size, self.ratio, now)
    }
    fn latest(&mut self) -> Result<(), ChartError> {
        if let Some(last) = self.chart.series_data(self.line)?.last() {
            let last = *last.time();
            let index = (0..self.next)
                .find(|i| point(*i) == last)
                .expect("demo-owned point");
            self.step += 1;
            let delta = (self.step as f64 * 0.4).sin() * 0.12;
            self.chart
                .update_data(self.line, line_row(index, delta), false)?;
            self.chart
                .update_custom_data(&self.custom, datum(index, delta), false)?;
        }
        Ok(())
    }
    fn action(&mut self, message: &Message) -> Result<(), ChartError> {
        match message {
            Message::Latest | Message::Tick(_) => self.latest()?,
            Message::History => {
                if let Some(first) = self.chart.series_data(self.line)?.first() {
                    let key = *first.time();
                    let index = (0..self.next).find(|i| point(*i) == key).unwrap();
                    self.chart
                        .update_data(self.line, line_row(index, 0.15), true)?;
                    self.chart
                        .update_custom_data(&self.custom, datum(index, 0.15), true)?;
                }
            }
            Message::Append => {
                self.chart
                    .update_data(self.line, line_row(self.next, 0.), false)?;
                self.chart
                    .update_custom_data(&self.custom, datum(self.next, 0.), false)?;
                self.next += 1;
            }
            Message::Pop => {
                let removed = self.chart.pop_data(self.line, 1)?;
                self.chart.pop_custom_data(&self.custom, 1)?;
                self.status = format!(
                    "Popped original items: {:?}",
                    removed.iter().map(|r| r.time()).collect::<Vec<_>>()
                );
            }
            Message::Empty => {
                self.chart.set_data(self.line, vec![])?;
                self.chart.set_custom_data(&self.custom, vec![])?;
                self.live = false;
            }
            Message::Reset => {
                self.next = 24;
                self.step = 0;
                self.chart
                    .set_data(self.line, (0..24).map(|i| line_row(i, 0.)).collect())?;
                self.chart
                    .set_custom_data(&self.custom, (0..24).map(|i| datum(i, 0.)).collect())?;
                self.chart.fit_content();
            }
            Message::Labels => {
                self.long_labels = !self.long_labels;
                self.chart.configure_horizontal_behavior(|| Options {
                    prefix: if self.long_labels { "Run " } else { "S" }.into(),
                    fast_append: false,
                });
            }
            Message::Invalid => {
                let invalid = BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                    time: SessionPoint {
                        session: 0,
                        phase: behavior::Phase::Open,
                    },
                    value: 5.,
                    color: None,
                    custom_values: None,
                }));
                match self.chart.update_data(self.line, invalid, false) {
                    Err(e) => self.status = format!("Rejected as expected: {e}"),
                    Ok(()) => panic!("invalid session accepted"),
                }
            }
            Message::Transfer => {
                let pane = self.chart.model().pane_for_series(self.line).unwrap();
                self.chart.move_series_to_pane(
                    self.line,
                    if self.chart.model().panes().len() == 1 {
                        1
                    } else {
                        1 - pane
                    },
                )?;
            }
            Message::Swap => {
                if self.chart.model().panes().len() > 1 {
                    self.chart.swap_panes(0, 1)?;
                }
            }
            Message::Pause => self.live = !self.live,
            Message::Capture => {
                let shot = self.chart.prepare_screenshot(
                    ScreenshotLayers {
                        include_top_layer: true,
                        include_crosshair: true,
                    },
                    Instant::now(),
                )?;
                if let Some(exporter) = &mut self.exporter {
                    let image = exporter
                        .render(
                            &shot,
                            ScreenshotRasterOptions {
                                pixel_ratio: 1.5,
                                ..Default::default()
                            },
                        )
                        .map_err(ChartError::Screenshot)?;
                    self.status = match captures::save(
                        std::path::Path::new("target/custom-horizontal/preview"),
                        &image,
                    ) {
                        Ok(()) => format!(
                            "Saved {}×{} RGBA to target/custom-horizontal/preview.rgba",
                            image.size().width,
                            image.size().height
                        ),
                        Err(error) => format!("Saving export failed: {error}"),
                    };
                    self.preview = Some(image.into_handle());
                }
            }
            Message::Hide => self.preview = None,
            _ => {}
        }
        for event in self.chart.take_events() {
            if let ChartEvent::PrimitiveEdit(event) = event {
                self.region.edit(&mut self.chart, &event)?;
            }
        }
        Ok(())
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        let result = match message {
            Message::Chart(message) => {
                if let ChartMessage::Bounds(size) = message {
                    self.size = size;
                }
                self.chart.update(message).and_then(|events| {
                    let mut edited = false;
                    for e in events {
                        match e {
                            ChartEvent::PrimitiveEdit(e) => {
                                self.region.edit(&mut self.chart, &e)?;
                                edited = true;
                            }
                            ChartEvent::CrosshairMoved(p) | ChartEvent::Click(p) => {
                                self.status = format!(
                                    "{:?} | interval {:?}",
                                    p.time,
                                    p.custom_data(&self.custom).map(|d| d.value)
                                );
                            }
                            _ => {}
                        }
                    }
                    if edited {
                        self.prepare(Instant::now())
                    } else {
                        Ok(())
                    }
                })
            }
            Message::Window(id, iced::window::Event::Opened { .. }) => {
                return iced::window::scale_factor(id).map(Message::Ratio);
            }
            Message::Window(_, iced::window::Event::Rescaled(r)) | Message::Ratio(r) => {
                self.ratio = r;
                self.prepare(Instant::now())
            }
            Message::Window(_, _) => Ok(()),
            Message::Tick(now) => self
                .action(&Message::Tick(now))
                .and_then(|_| self.prepare(now)),
            _ => self
                .action(&message)
                .and_then(|_| self.prepare(Instant::now())),
        };
        if let Err(error) = result {
            self.status = error.to_string();
        }
        Task::none()
    }
    fn view(&self) -> Element<'_, Message> {
        let mut controls = row![];
        for (label, message) in [
            ("Latest", Message::Latest),
            ("History", Message::History),
            ("Append", Message::Append),
            ("Pop", Message::Pop),
            ("Empty", Message::Empty),
            ("Reset", Message::Reset),
            ("Labels", Message::Labels),
            ("Invalid", Message::Invalid),
            ("Line → next/new pane", Message::Transfer),
            ("Swap panes", Message::Swap),
            (
                if self.live {
                    "Pause feed"
                } else {
                    "Resume feed"
                },
                Message::Pause,
            ),
            ("Capture + save", Message::Capture),
            ("Hide preview", Message::Hide),
        ] {
            controls = controls.push(button(label).on_press(message));
        }
        let mut content = column![
            iced::widget::scrollable(controls.spacing(6)).direction(
                iced::widget::scrollable::Direction::Horizontal(Default::default())
            ),
            text(&self.status).size(12).wrapping(text::Wrapping::None)
        ];
        if let Some(handle) = &self.preview {
            content = content.push(iced::widget::image(handle.clone()).height(100));
        }
        content
            .push(self.chart.element().map(Message::Chart))
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
    let backend = if args.iter().any(|a| a == "--wgpu")
        || std::env::var("ICED_BACKEND").as_deref() != Ok("tiny-skia")
    {
        ScreenshotBackend::Wgpu
    } else {
        ScreenshotBackend::Software
    };
    if let Some(i) = args.iter().position(|a| a == "--captures") {
        captures::run(std::path::Path::new(&args[i + 1]), backend);
        return Ok(());
    }
    let live = args.iter().any(|a| a == "--live-feed");
    iced::application(
        move || {
            let exporter = iced::futures::executor::block_on(ChartExporter::new(backend))
                .expect("explicit screenshot backend");
            App::new(live, Some(exporter))
        },
        App::update,
        App::view,
    )
    .title("Feather Charts — application session/phase domain")
    .subscription(App::subscription)
    .run()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn controls_preserve_domain_after_empty_and_pane_transfers() {
        let mut app = App::new(false, None);
        app.size = Size::new(800., 500.);
        for message in [
            Message::Latest,
            Message::History,
            Message::Pop,
            Message::Append,
            Message::Labels,
            Message::Invalid,
            Message::Transfer,
            Message::Swap,
            Message::Empty,
        ] {
            app.action(&message).unwrap();
            app.prepare(Instant::now()).unwrap();
        }
        assert!(app.chart.series_data(app.line).unwrap().is_empty());
        assert!(app.custom.data().is_empty());
        app.action(&Message::Append).unwrap();
        assert_eq!(
            *app.chart.series_data(app.line).unwrap()[0].time(),
            point(25)
        );
        app.action(&Message::Reset).unwrap();
        assert_eq!(app.custom.data().len(), 24);
        for _ in 0..100 {
            app.action(&Message::Tick(Instant::now())).unwrap();
        }
        assert!(
            app.chart.take_events().is_empty(),
            "idle feed must drain host events"
        );
    }
}
