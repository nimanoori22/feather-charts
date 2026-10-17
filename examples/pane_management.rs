//! Reorganize live calendar, strike or maturity panes through the public API.
#[path = "pane_management/captures.rs"]
mod captures;
#[path = "pane_management/host.rs"]
mod host;
#[path = "numeric_custom_series/view.rs"]
mod interval;
#[path = "support/editable_box.rs"]
mod region;
use feather_charts::{
    api::*,
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        data_layer::SeriesId,
        icustom_series::CustomSeriesDataItem,
        pane::PriceScalePosition,
        series_options::*,
    },
};
use host::Host;
use iced::{
    Element, Size, Subscription, Task,
    widget::{button, column, row, text},
};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
struct App<C: Host> {
    chart: C,
    line: SeriesId,
    custom: CustomSeriesHandle<interval::View<C::Item, C::Internal>>,
    markers: UpDownMarkersId,
    region: region::Scene,
    size: Size,
    ratio: f32,
    live: bool,
    step: usize,
    status: String,
}
#[derive(Clone, Debug)]
enum Message {
    Chart(ChartMessage),
    Window(iced::window::Id, iced::window::Event),
    Ratio(f32),
    Tick(Instant),
    Down,
    Up,
    Swap,
    LineNext,
    LineNew,
    CustomToLine,
    Remove,
    Reset,
    Pause,
}
fn line_row<C: Host>(i: usize, shift: f64) -> BuiltInSeriesDataItem<C::Item> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: C::key(i),
        value: 4.5 + (i as f64 * 0.3).sin() * 0.4 + shift,
        color: None,
        custom_values: None,
    }))
}
impl<C: Host> App<C> {
    fn new(live: bool) -> Self {
        let mut chart = C::new();
        chart.add_pane();
        chart.add_pane();
        let line = chart.line(0, live);
        let peer = chart.line(1, false);
        let custom = chart.custom(2);
        let markers = chart.markers(line);
        chart
            .data(markers, (0..24).map(|i| line_row::<C>(i, 0.)).collect())
            .unwrap();
        let peer_markers = chart.markers(peer);
        chart
            .data(
                peer_markers,
                (0..24).map(|i| line_row::<C>(i, 1.)).collect(),
            )
            .unwrap();
        chart
            .custom_data(
                &custom,
                (0..24)
                    .map(|i| {
                        let value = 4.5 + (i as f64 * 0.3).sin() * 0.4;
                        CustomSeriesDataItem::Data(interval::Datum {
                            time: C::key(i),
                            low: value - 0.2,
                            high: value + 0.25,
                            value,
                        })
                    })
                    .collect(),
            )
            .unwrap();
        for pane in 0..3 {
            chart.decorate(pane);
        }
        let end = chart
            .core()
            .model()
            .time_scale()
            .base_index()
            .map_or(23., |i| i.value());
        let region = region::Scene::new(&mut chart, line, end * 0.15..end * 0.4);
        chart.fit();
        Self {
            chart,
            line,
            custom,
            markers,
            region,
            size: Size::ZERO,
            ratio: 1.,
            live,
            step: 0,
            status: "Pane labels follow panes; the box and update markers follow the blue series"
                .into(),
        }
    }
    fn action(&mut self, m: Message) -> Result<(), ChartError> {
        let n = self.chart.core().model().panes().len();
        match m {
            Message::Down => self.chart.move_pane(0, n - 1)?,
            Message::Up => self.chart.move_pane(n - 1, 0)?,
            Message::Swap => self.chart.swap_panes(0, n - 1)?,
            Message::LineNext => {
                let from = self
                    .chart
                    .core()
                    .model()
                    .pane_for_series(self.line)
                    .ok_or(ChartError::UnknownSeries(self.line))?;
                self.chart.move_series_to_pane(self.line, (from + 1) % n)?;
            }
            Message::LineNew => self.chart.move_series_to_pane(self.line, n)?,
            Message::CustomToLine => {
                let to = self
                    .chart
                    .core()
                    .model()
                    .pane_for_series(self.line)
                    .ok_or(ChartError::UnknownSeries(self.line))?;
                self.chart.move_series_to_pane(self.custom.id(), to)?;
            }
            Message::Remove => self.chart.remove_pane(n - 1)?,
            Message::Reset => {
                let (size, ratio, live) = (self.size, self.ratio, self.live);
                *self = Self::new(live);
                self.size = size;
                self.ratio = ratio;
            }
            Message::Pause => self.live = !self.live,
            Message::Tick(at)
                if self.live && self.chart.core().series_ids().any(|id| id == self.line) =>
            {
                self.step += 1;
                self.chart.update_data(
                    self.markers,
                    line_row::<C>(23, [-0.1, 0., 0.1][self.step % 3]),
                    at,
                )?;
            }
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
            Message::Window(id, iced::window::Event::Opened { .. }) => {
                return iced::window::scale_factor(id).map(Message::Ratio);
            }
            Message::Window(_, iced::window::Event::Rescaled(r)) | Message::Ratio(r) => {
                self.ratio = r;
                self.prepare(Instant::now());
            }
            Message::Window(_, _) => {}
            Message::Chart(m) => {
                if let ChartMessage::Bounds(s) = m {
                    self.size = s;
                }
                match self.chart.update(m) {
                    Ok(events) => {
                        for event in events {
                            match event {
                                ChartEvent::PrimitiveEdit(e) => {
                                    if let Err(e) = self.region.edit(&mut self.chart, &e) {
                                        self.status = e.to_string();
                                    }
                                }
                                ChartEvent::Click(p) | ChartEvent::CrosshairMoved(p) => {
                                    let data = p.custom_data(&self.custom).map(|d| d.value);
                                    let hit = p.custom_hit_data(&self.custom);
                                    self.status = format!(
                                        "Pane {:?} time={:?} original={data:?} hit={hit:?}",
                                        p.pane, p.time
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => self.status = e.to_string(),
                }
            }
            _ => {
                let at = if let Message::Tick(at) = m {
                    at
                } else {
                    Instant::now()
                };
                if let Err(e) = self.action(m) {
                    self.status = e.to_string();
                }
                self.prepare(at);
            }
        }
        Task::none()
    }
    fn view(&self) -> Element<'_, Message> {
        let owners = format!(
            "{} panes | line {:?} | custom {:?}",
            self.chart.core().model().panes().len(),
            self.chart.core().model().pane_for_series(self.line),
            self.chart.core().model().pane_for_series(self.custom.id())
        );
        column![
            iced::widget::scrollable(
                row![
                    button("First → last").on_press(Message::Down),
                    button("Last → first").on_press(Message::Up),
                    button("Swap ends").on_press(Message::Swap),
                    button("Line → next").on_press(Message::LineNext),
                    button("Line → new").on_press(Message::LineNew),
                    button("Custom → line").on_press(Message::CustomToLine),
                    button("Remove last").on_press(Message::Remove),
                    button("Reset").on_press(Message::Reset),
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
            text(owners).size(12).wrapping(text::Wrapping::None),
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
fn run<C: Host>(live: bool, title: &'static str) -> iced::Result {
    iced::application(move || App::<C>::new(live), App::update, App::view)
        .title(title)
        .subscription(App::subscription)
        .run()
}
fn main() -> iced::Result {
    let args: Vec<_> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--captures") {
        captures::run(
            std::path::Path::new(&args[i + 1]),
            args.iter().any(|a| a == "--wgpu"),
        );
        return Ok(());
    }
    let live = args.iter().any(|a| a == "--live-feed");
    if args.iter().any(|a| a == "--yield-curve") {
        run::<YieldCurveChart>(live, "Feather Charts — maturity pane management")
    } else if args.iter().any(|a| a == "--options") {
        run::<OptionsChart>(live, "Feather Charts — strike pane management")
    } else {
        run::<Chart>(live, "Feather Charts — calendar pane management")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn controls<C: Host>() {
        let mut app = App::<C>::new(false);
        app.size = Size::new(800., 500.);
        app.prepare(Instant::now());
        assert!(
            app.chart.core().model().panes().iter().all(|p| p
                .default_visible_price_scale()
                .and_then(|s| s.first_value())
                .is_some()),
            "each initial pane must have data on its visible scale"
        );
        let original = app.custom.id();
        for action in [
            Message::Down,
            Message::Up,
            Message::Swap,
            Message::LineNext,
            Message::LineNew,
            Message::CustomToLine,
        ] {
            app.action(action).unwrap();
            app.chart
                .prepare(Size::new(300., 90.), 1.5, Instant::now())
                .unwrap();
            app.chart.prepare(app.size, 1., Instant::now()).unwrap();
            assert_eq!(app.custom.id(), original);
            assert!(app.chart.core().model().pane_for_series(app.line).is_some());
            assert!(app.chart.core().model().pane_for_series(original).is_some());
        }
        app.action(Message::Pause).unwrap();
        app.action(Message::Tick(Instant::now())).unwrap();
        app.action(Message::Remove).unwrap();
        app.prepare(Instant::now());
        while app.chart.core().model().panes().len() > 1 {
            app.action(Message::Remove).unwrap();
        }
        app.action(Message::Remove).unwrap();
        app.action(Message::Reset).unwrap();
        app.prepare(Instant::now());
        assert_eq!(app.chart.core().model().panes().len(), 3);
        assert_ne!(app.custom.id(), original);
        assert_eq!(app.size, Size::new(800., 500.));
    }
    #[test]
    fn calendar_controls_keep_live_owners_and_recover() {
        controls::<Chart>();
    }
    #[test]
    fn price_controls_keep_live_owners_and_recover() {
        controls::<OptionsChart>();
    }
    #[test]
    fn yield_controls_keep_live_owners_and_recover() {
        controls::<YieldCurveChart>();
    }
}
