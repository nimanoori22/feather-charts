//! Reorganize live calendar, strike or maturity panes through the public API.
#[path = "pane_management/attribution.rs"]
mod attribution;
#[path = "pane_management/axis_interaction.rs"]
mod axis_interaction;
#[path = "pane_management/captures.rs"]
mod captures;
#[path = "pane_management/configuration.rs"]
mod configuration;
#[path = "pane_management/data.rs"]
mod data;
#[path = "pane_management/events.rs"]
mod events;
#[path = "pane_management/export.rs"]
mod export;
#[path = "pane_management/formatting.rs"]
mod formatting;
#[path = "pane_management/host.rs"]
mod host;
#[path = "numeric_custom_series/view.rs"]
mod interval;
#[path = "support/editable_box.rs"]
mod region;
#[path = "pane_management/scale_assignment.rs"]
mod scale_assignment;
#[path = "pane_management/screenshot_captures.rs"]
mod screenshot_captures;
#[path = "custom_horizontal/behavior.rs"]
mod session_behavior;
use feather_charts::{
    api::*,
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem, TimedData},
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
    axis_mode: bool,
    assignment: Option<scale_assignment::Panel>,
    formatting: Option<formatting::Panel>,
    attribution: Option<attribution::Panel>,
    export: Option<export::Panel>,
    events: Option<events::Monitor>,
    config_mode: bool,
    config_pane: usize,
    config_left: bool,
    chart: C,
    line: SeriesId,
    order_mode: bool,
    data_mode: bool,
    data_next: usize,
    data_status: String,
    selected_custom: bool,
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
    Axis(axis_interaction::Action),
    Assign(scale_assignment::Action),
    Formatting(formatting::Action),
    Attribution(attribution::Action),
    Export(export::Action),
    Events(events::Action),
    Config(configuration::Action),
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
    SelectSeries,
    Backward,
    Forward,
    ToBack,
    ToFront,
    HoverRaise,
    Pop(usize),
    Append,
    Inspect,
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
            axis_mode: false,
            assignment: None,
            formatting: None,
            attribution: None,
            export: None,
            events: None,
            config_mode: false,
            config_pane: 0,
            config_left: false,
            chart,
            line,
            order_mode: false,
            data_mode: false,
            data_next: 24,
            data_status: String::new(),
            selected_custom: false,
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
    fn with_order(live: bool, enabled: bool) -> Self {
        let mut app = Self::new(live);
        app.order_mode = enabled;
        if enabled {
            app.chart.move_series_to_pane(app.custom.id(), 0).unwrap();
            let area = app.chart.area(0);
            app.chart
                .raw_data(
                    area,
                    (0..24)
                        .map(|i| {
                            BuiltInSeriesDataItem::Area(
                                feather_charts::model::data_consumer::AreaDataItem::Data(
                                    feather_charts::model::data_consumer::AreaData {
                                        time: C::key(i),
                                        value: 4.5 + (i as f64 * 0.3).sin() * 0.4,
                                        line_color: None,
                                        top_color: None,
                                        bottom_color: None,
                                        custom_values: None,
                                    },
                                ),
                            )
                        })
                        .collect(),
                )
                .unwrap();
            app.chart.fit();
            app.status = "Select blue line or custom intervals, then change order. Hover raising is independent.".into();
        }
        app
    }
    fn selected(&self) -> SeriesId {
        if self.selected_custom {
            self.custom.id()
        } else {
            self.line
        }
    }
    fn action(&mut self, m: Message) -> Result<(), ChartError> {
        let n = self.chart.core().model().panes().len();
        match m {
            Message::Axis(action) => self.axis_action(action)?,
            Message::Assign(action) => self.assignment_action(action)?,
            Message::Formatting(action) => self.formatting_action(action)?,
            Message::Attribution(action) => self.attribution_action(action)?,
            Message::Export(action) => self.export_action(action)?,
            Message::Events(action) => self.event_action(action)?,
            Message::Config(action) => self.configuration_action(action)?,
            Message::Pop(count) => self.pop_selected(count)?,
            Message::Append => self.append_selected()?,
            Message::Inspect => self.inspect_data()?,
            Message::SelectSeries => self.selected_custom = !self.selected_custom,
            Message::HoverRaise => self
                .chart
                .set_hovered_series_on_top(!self.chart.core().hovered_series_on_top()),
            Message::Backward | Message::Forward | Message::ToBack | Message::ToFront => {
                let id = self.selected();
                let current = self.chart.core().series_order(id)?;
                let target = match m {
                    Message::Backward => current.saturating_sub(1),
                    Message::Forward => current.saturating_add(1),
                    Message::ToBack => 0,
                    _ => usize::MAX,
                };
                self.chart.set_series_order(id, target)?;
            }
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
                let axis_mode = self.axis_mode;
                let assignment = self.assignment.is_some();
                let formatting = self.formatting.is_some();
                let attribution = self.attribution.is_some();
                let export = self.export.take();
                let (size, ratio, live) = (self.size, self.ratio, self.live);
                *self = Self::with_public_events(
                    live,
                    self.order_mode,
                    self.data_mode,
                    self.config_mode,
                    self.events.is_some(),
                );
                self.export = export;
                if axis_mode {
                    self.enable_axis()?;
                }
                if assignment {
                    self.enable_assignment()?;
                }
                if formatting {
                    self.formatting = Some(Default::default());
                }
                if attribution {
                    self.enable_attribution()?;
                }
                self.size = size;
                self.ratio = ratio;
            }
            Message::Pause => self.live = !self.live,
            Message::Tick(at)
                if self.live && self.chart.core().series_ids().any(|id| id == self.line) =>
            {
                self.step += 1;
                let mut item = line_row::<C>(23, [-0.1, 0., 0.1][self.step % 3]);
                if self.data_mode {
                    let rows = self.chart.core().series_data(self.line)?;
                    let Some(last) = rows.last() else {
                        return Ok(());
                    };
                    *item.time_mut() = last.time().clone();
                }
                self.chart.update_data(self.markers, item, at)?;
            }
            _ => {}
        }
        Ok(())
    }
    fn consume_events(&mut self, events: Vec<ChartEvent<C::Item>>) {
        for event in events {
            if let Some(m) = &mut self.events {
                m.observe(&event);
            }
            match event {
                ChartEvent::AttributionActivated { url } => {
                    self.status = format!("Host link action: {url}");
                    if let Some(panel) = &mut self.attribution {
                        panel.activations += 1;
                        panel.status = format!("Host action {}: {url}", panel.activations);
                    }
                }
                ChartEvent::DataChanged { .. }
                | ChartEvent::DoubleClick(_)
                | ChartEvent::VisibleLogicalRangeChanged(_)
                | ChartEvent::VisibleHorizontalRangeChanged(_)
                | ChartEvent::HorizontalAxisSizeChanged { .. } => {}
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
    fn prepare(&mut self, at: Instant) {
        if let Err(e) = self.chart.prepare(self.size, self.ratio, at) {
            self.status = e.to_string();
        }
        let events = self.chart.take_events();
        self.consume_events(events);
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
                        self.consume_events(events);
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
        let order_controls = if self.order_mode {
            row![
                button(if self.selected_custom {
                    "Selected: custom"
                } else {
                    "Selected: blue line"
                })
                .on_press(Message::SelectSeries),
                button("Backward").on_press(Message::Backward),
                button("Forward").on_press(Message::Forward),
                button("To back").on_press(Message::ToBack),
                button("To front").on_press(Message::ToFront),
                button(if self.chart.core().hovered_series_on_top() {
                    "Hover raise: on"
                } else {
                    "Hover raise: off"
                })
                .on_press(Message::HoverRaise),
                text(format!(
                    "order {:?}",
                    self.chart.core().series_order(self.selected()).ok()
                ))
            ]
            .spacing(6)
        } else {
            row![]
        };
        let mut content = column![
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
            iced::widget::scrollable(order_controls).direction(
                iced::widget::scrollable::Direction::Horizontal(Default::default())
            ),
            iced::widget::scrollable(if self.data_mode {
                row![
                    button("Pop 1").on_press(Message::Pop(1)),
                    button("Pop 3").on_press(Message::Pop(3)),
                    button("Pop all").on_press(Message::Pop(usize::MAX)),
                    button("Append").on_press(Message::Append),
                    button("Inspect index 0 / range 0..10").on_press(Message::Inspect),
                    text(&self.data_status)
                        .size(12)
                        .wrapping(text::Wrapping::None)
                ]
                .spacing(6)
            } else {
                row![]
            })
            .direction(iced::widget::scrollable::Direction::Horizontal(
                Default::default()
            )),
            if self.config_mode {
                self.configuration_controls()
            } else {
                row![].into()
            },
            self.event_controls(),
        ];
        if self.axis_mode {
            content = content.push(self.axis_controls());
        }
        if self.assignment.is_some() {
            content = content.push(self.assignment_controls());
        }
        if self.formatting.is_some() {
            content = content.push(self.formatting_controls());
        }
        if self.attribution.is_some() {
            content = content.push(self.attribution_controls());
        }
        if self.export.is_some() {
            content = content.push(self.export_controls());
        }
        content
            .push(text(owners).size(12).wrapping(text::Wrapping::None))
            .push(text(&self.status).size(12).wrapping(text::Wrapping::None))
            .push(self.chart.core().element().map(Message::Chart))
            .spacing(6)
            .into()
    }
    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::window::events().map(|(id, e)| Message::Window(id, e)),
            if self.export.is_some() {
                iced::event::listen_with(|event, _, _| match event {
                    iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                        key: iced::keyboard::Key::Named(iced::keyboard::key::Named::F8),
                        repeat: false,
                        ..
                    }) => Some(Message::Export(export::Action::Capture)),
                    _ => None,
                })
            } else {
                Subscription::none()
            },
            if self.live {
                iced::time::every(Duration::from_millis(800)).map(Message::Tick)
            } else {
                Subscription::none()
            },
        ])
    }
}
struct RunOptions {
    live: bool,
    ordered: bool,
    data: bool,
    config: bool,
    events: bool,
    exports: bool,
    attribution: bool,
    formatting: bool,
    assignment: bool,
    axis_mode: bool,
}
fn run<C: Host>(options: RunOptions, title: &'static str) -> iced::Result {
    iced::application(
        move || {
            let mut app = App::<C>::with_public_events(
                options.live,
                options.ordered,
                options.data,
                options.config,
                options.events,
            );
            if options.axis_mode {
                app.enable_axis().unwrap();
            }
            if options.assignment {
                app.enable_assignment().unwrap();
            }
            if options.formatting {
                app.formatting = Some(Default::default());
            }
            if options.exports {
                app.export = Some(export::Panel::new());
            }
            if options.attribution {
                app.enable_attribution().unwrap();
            }
            app
        },
        App::update,
        App::view,
    )
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
            args.iter().any(|a| a == "--series-order"),
            args.iter().any(|a| a == "--series-data"),
            args.iter().any(|a| a == "--price-scale-configuration"),
            args.iter().any(|a| a == "--public-events"),
        );
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--screenshot-captures") {
        screenshot_captures::run(
            std::path::Path::new(&args[i + 1]),
            args.iter().any(|a| a == "--wgpu"),
        );
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--attribution-captures") {
        attribution::captures(
            std::path::Path::new(&args[i + 1]),
            args.iter().any(|a| a == "--wgpu"),
        );
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--formatting-captures") {
        formatting::captures(
            std::path::Path::new(&args[i + 1]),
            args.iter().any(|a| a == "--wgpu"),
        );
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--scale-assignment-captures") {
        scale_assignment::captures(
            std::path::Path::new(&args[i + 1]),
            args.iter().any(|a| a == "--wgpu"),
        );
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--axis-interaction-captures") {
        axis_interaction::captures(
            std::path::Path::new(&args[i + 1]),
            args.iter().any(|a| a == "--wgpu"),
        );
        return Ok(());
    }
    let options = RunOptions {
        live: args.iter().any(|a| a == "--live-feed"),
        ordered: args.iter().any(|a| a == "--series-order"),
        data: args.iter().any(|a| a == "--series-data"),
        config: args.iter().any(|a| a == "--price-scale-configuration"),
        events: args.iter().any(|a| a == "--public-events"),
        exports: args.iter().any(|a| a == "--screenshot-export"),
        attribution: args.iter().any(|a| a == "--attribution"),
        formatting: args.iter().any(|a| a == "--formatting"),
        assignment: args.iter().any(|a| a == "--scale-assignment"),
        axis_mode: args.iter().any(|a| a == "--axis-interaction"),
    };
    if args.iter().any(|a| a == "--custom-domain") {
        run::<Chart<session_behavior::Sessions>>(
            options,
            "Feather Charts — session/phase attribution",
        )
    } else if args.iter().any(|a| a == "--yield-curve") {
        run::<YieldCurveChart>(options, "Feather Charts — maturity pane management")
    } else if args.iter().any(|a| a == "--options") {
        run::<OptionsChart>(options, "Feather Charts — strike pane management")
    } else {
        run::<Chart>(options, "Feather Charts — calendar pane management")
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
    fn order_controls<C: Host>() {
        let mut app = App::<C>::with_order(true, true);
        app.size = Size::new(800., 500.);
        for action in [
            Message::ToFront,
            Message::Backward,
            Message::ToBack,
            Message::Forward,
            Message::SelectSeries,
            Message::ToBack,
            Message::ToFront,
            Message::HoverRaise,
            Message::LineNew,
            Message::CustomToLine,
            Message::ToBack,
            Message::Tick(Instant::now()),
            Message::Reset,
        ] {
            app.action(action).unwrap();
            app.chart
                .prepare(Size::new(300., 90.), 1.5, Instant::now())
                .unwrap();
            app.chart.prepare(app.size, 1., Instant::now()).unwrap();
            assert!(app.chart.core().series_order(app.selected()).is_ok());
        }
        assert!(app.order_mode);
        assert_eq!(
            app.chart.core().model().pane_for_series(app.line),
            app.chart.core().model().pane_for_series(app.custom.id())
        );
    }
    #[test]
    fn calendar_order_controls() {
        order_controls::<Chart>();
    }
    #[test]
    fn strike_order_controls() {
        order_controls::<OptionsChart>();
    }
    #[test]
    fn maturity_order_controls() {
        order_controls::<YieldCurveChart>();
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
