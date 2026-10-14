//! Run with `cargo run --example line_chart`; `--smoke` checks the demo headlessly.
#[path = "display_scenario.rs"]
mod display_scenario;
use display_scenario::{DisplayConfig, DisplayScenario};
#[cfg(test)]
use feather_charts::{
    api::ChartState,
    renderers::grid_renderer::PixelRatio,
    ui::{
        chart_frame::{ChartFrameInput, IcedChartFrame},
        price_axis::AxisFontResolver,
    },
};
use feather_charts::{
    api::{Chart, ChartMessage},
    model::{
        axis_snapshots::PriceAxisSide,
        chart_model::ChartModelOptions,
        data_consumer::{
            BuiltInSeriesDataItem, CandlestickData, CandlestickDataItem, HistogramData,
            HistogramDataItem, LineData, LineDataItem, WhitespaceData,
        },
        data_layer::SeriesId,
        grid::GridOptions,
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        layout_options::{Background, ColorSpace, LayoutOptions, LayoutPanesOptions},
        pane::{PaneOptions, PriceScalePosition},
        price_scale::PriceScaleOptionsPatch,
        series_options::{
            CandlestickStyleOptions, HistogramStyleOptions, LastPriceAnimationMode,
            LineStyleOptions, PriceFormat, PriceFormatBuiltIn, PriceFormatBuiltInType,
            PriceLineSource, SeriesOptions, SeriesOptionsCommon, SeriesOptionsMap, SeriesType,
        },
        time_data::{Logical, LogicalRange},
        time_scale_options::HorzScaleOptionsPatch,
    },
    renderers::{
        draw_line::{LineStyle, LineType, LineWidth},
        price_axis_renderer::AxisBounds,
    },
    ui::{
        chart_input::{ChartInteraction, MultiPaneInteraction, Navigation},
        price_axis_input::PriceAxisInteraction,
    },
};
#[cfg(test)]
use iced::widget::canvas;
use iced::{
    Element, Size, Subscription, Theme,
    widget::{button, canvas::Frame, column, container, row, text},
};
use std::time::{Duration, Instant};

#[path = "chart_demo/series.rs"]
mod series;
use series::{SeriesKind, bar, options};
struct Demo {
    config: DisplayConfig,
    series_kind: SeriesKind,
    volume_registered: bool,
    chart: Chart,
    id: SeriesId,
    left_id: SeriesId,
    volume_id: SeriesId,
    #[cfg(test)]
    fonts: AxisFontResolver,
    left_enabled: bool,
    left_registered: bool,
    ticks: bool,
    size: Size,
    scale: f32,
    count: usize,
    history_start: i64,
    registered: bool,
    error: Option<String>,
    kind: LineType,
    style: LineStyle,
    markers: bool,
}
#[derive(Clone, Debug)]
enum Message {
    Touch(feather_charts::api::TouchInteraction),
    PrimitiveEdit(feather_charts::api::PrimitiveEditEvent),
    Click { pane: usize, position: iced::Point },
    CrosshairMode,
    Interaction(ChartInteraction),
    PaneInteraction(MultiPaneInteraction),
    PriceAxis(PriceAxisInteraction),
    Bounds(Size),
    Tick(Instant),
    Window(iced::window::Id, iced::window::Event),
    Scale(f32),
    Fit,
    Append,
    Replace,
    Older,
    Prepend,
    Clear,
    Latest,
    Remove,
    Reload,
    Animate,
    Stop,
    Kind,
    Style,
    Markers,
    LeftAxis,
    Ticks,
}
impl Default for Demo {
    fn default() -> Self {
        Self::new_kind(DisplayConfig::default(), false)
    }
}
impl Demo {
    fn new_kind(config: DisplayConfig, candlesticks: bool) -> Self {
        Self::new_series(config, candlesticks, false)
    }
    fn new_series(config: DisplayConfig, candlesticks: bool, histogram: bool) -> Self {
        Self::new_chart(
            config,
            if histogram {
                SeriesKind::Histogram
            } else if candlesticks {
                SeriesKind::Candlesticks
            } else {
                SeriesKind::Line
            },
        )
    }
    fn new_chart(config: DisplayConfig, series_kind: SeriesKind) -> Self {
        let chart = Chart::new(ChartModelOptions {
            layout: LayoutOptions {
                background: if config.gradient {
                    Background::VerticalGradient {
                        top_color: "#101820".into(),
                        bottom_color: "#36536a".into(),
                    }
                } else {
                    Background::Solid {
                        color: "#101820".into(),
                    }
                },
                text_color: "#eee".into(),
                font_size: config.font_size,
                font_family: config.font_family.clone(),
                panes: LayoutPanesOptions {
                    enable_resize: series_kind.volume(),
                    separator_color: "#36536a".into(),
                    separator_hover_color: "#758696".into(),
                },
                attribution_logo: false,
                color_space: ColorSpace::Srgb,
                color_parsers: vec![],
            },
            pane: PaneOptions {
                left_price_scale: feather_charts::model::price_scale::PriceScaleOptions {
                    visible: false,
                    ..Default::default()
                },
                ..Default::default()
            },
            grid: GridOptions::default(),
            add_default_pane: true,
        });
        let mut demo = Self {
            config,
            series_kind,
            volume_registered: false,
            chart,
            id: SeriesId::new(0),
            left_id: SeriesId::new(0),
            volume_id: SeriesId::new(0),
            #[cfg(test)]
            fonts: AxisFontResolver::default(),
            left_enabled: false,
            left_registered: false,
            ticks: false,
            size: Size::ZERO,
            scale: 1.,
            count: 0,
            history_start: 0,
            registered: false,
            error: None,
            kind: LineType::Simple,
            style: LineStyle::Solid,
            markers: false,
        };
        demo.chart.apply_time_scale_options(HorzScaleOptionsPatch {
            time_visible: Some(true),
            fix_left_edge: Some(demo.config.fixed_edges),
            fix_right_edge: Some(demo.config.fixed_edges),
            ..Default::default()
        });
        demo.load();
        if demo.config.left_axis {
            let _ = demo.update(Message::LeftAxis);
        }
        if demo.config.ticks {
            let _ = demo.update(Message::Ticks);
        }
        demo
    }
}
impl Demo {
    fn apply_scenario(&mut self, now: Instant) {
        self.size = Size::new(self.config.width, self.config.height);
        self.chart.fit_content();
        match self.config.scenario {
            DisplayScenario::Historical => {
                let _ = self.update(Message::Older);
                let _ = self.update(Message::Append);
            }
            DisplayScenario::Empty => {
                let _ = self.update(Message::Remove);
            }
            DisplayScenario::Animation => {
                self.chart
                    .scroll_to_offset_animated(-40., Duration::from_secs(2));
                self.prepare(now);
                self.size = Size::new(620., 360.);
                self.prepare(now + Duration::from_secs(1));
                return;
            }
            _ => {}
        }
        self.prepare(now);
    }
    fn load(&mut self) {
        if !self.registered {
            self.id = self
                .chart
                .add_series(self.series_options(), 0, PriceScalePosition::Right)
                .unwrap();
            self.registered = true;
        }
        self.count = 120;
        self.history_start = 0;
        let data = (0..self.count).map(|i| self.bar(i, 0.)).collect();
        self.chart.set_data(self.id, data).unwrap();
        self.sync_volume();
        self.chart.fit_content();
        self.sync_left();
    }
    fn sync_left(&mut self) {
        if self.left_enabled && self.registered {
            if !self.left_registered {
                self.left_id = self
                    .chart
                    .add_series(
                        options(self.kind, self.style, false),
                        0,
                        PriceScalePosition::Left,
                    )
                    .unwrap();
                self.left_registered = true;
            }
            let data = (0..self.count)
                .map(|index| {
                    let mut item = bar(&self.config, index, 0.);
                    if let BuiltInSeriesDataItem::Line(LineDataItem::Data(row)) = &mut item {
                        row.value = row.value * 100. - 12000.;
                    }
                    item
                })
                .collect();
            self.chart.set_data(self.left_id, data).unwrap();
        } else if self.left_registered {
            self.chart.remove_series(self.left_id).unwrap();
            self.left_registered = false;
        }
    }
    fn prepare(&mut self, now: Instant) {
        self.error = self
            .chart
            .prepare(self.size, self.scale, now)
            .err()
            .map(|e| format!("{e:?}"));
    }
    #[cfg(test)]
    fn prepare_snapshot(
        &mut self,
        input: &ChartFrameInput,
        now: Instant,
    ) -> Result<feather_charts::ui::chart_frame::ChartFrameSnapshot, feather_charts::api::ChartError>
    {
        Ok(self
            .chart
            .prepare_snapshot(
                Size::new(input.bounds.width as f32, input.bounds.height as f32),
                input.pixel_ratio.horizontal,
                now,
            )?
            .panes
            .remove(0))
    }
    fn has_series_geometry(&self) -> bool {
        if self.series_kind.candlesticks() || self.series_kind.histogram() {
            !self.chart.frame().plot.candles.is_empty()
        } else {
            !self.chart.frame().plot.lines.is_empty()
        }
    }
    fn update(&mut self, message: Message) -> iced::Task<Message> {
        let mut now = Instant::now();
        if matches!(
            message,
            Message::Bounds(_)
                | Message::Scale(_)
                | Message::Remove
                | Message::Reload
                | Message::Clear
                | Message::LeftAxis
        ) {
            self.chart.cancel_interaction();
        }
        match message {
            Message::Touch(input) => {
                self.error = self
                    .chart
                    .update(ChartMessage::Touch(input))
                    .err()
                    .map(|e| format!("{e:?}"));
                return iced::Task::none();
            }
            Message::PriceAxis(input) => {
                self.error = self
                    .chart
                    .update(ChartMessage::PriceAxis(input))
                    .err()
                    .map(|e| format!("{e:?}"));
                return iced::Task::none();
            }
            Message::PrimitiveEdit(input) => {
                self.error = self
                    .chart
                    .update(ChartMessage::PrimitiveEdit(input))
                    .err()
                    .map(|e| e.to_string());
            }
            Message::Click { pane, position } => {
                self.error = self
                    .chart
                    .update(ChartMessage::Click { pane, position })
                    .err()
                    .map(|e| format!("{e:?}"));
                return iced::Task::none();
            }
            Message::CrosshairMode => {
                use feather_charts::model::crosshair::CrosshairMode;
                let mut options = self.chart.model().crosshair().options().clone();
                options.mode = match options.mode {
                    CrosshairMode::Magnet => CrosshairMode::Normal,
                    CrosshairMode::Normal if self.series_kind.candlesticks() => {
                        CrosshairMode::MagnetOhlc
                    }
                    CrosshairMode::Normal | CrosshairMode::MagnetOhlc => CrosshairMode::Hidden,
                    _ => CrosshairMode::Magnet,
                };
                self.chart.set_crosshair_options(options).unwrap();
            }
            Message::Interaction(input) => {
                self.error = self
                    .chart
                    .update(ChartMessage::Interaction(input))
                    .err()
                    .map(|e| format!("{e:?}"));
                return iced::Task::none();
            }
            Message::PaneInteraction(input) => {
                self.error = self
                    .chart
                    .update(ChartMessage::PaneInteraction(input))
                    .err()
                    .map(|e| format!("{e:?}"));
                return iced::Task::none();
            }
            Message::Bounds(size) => {
                let startup = self.size.width == 0. && size.width > 0.;
                self.size = size;
                if startup && self.registered {
                    self.chart.fit_content();
                }
            }
            Message::Tick(time) => now = time,
            Message::Window(id, iced::window::Event::Opened { .. }) => {
                return iced::window::scale_factor(id).map(Message::Scale);
            }
            Message::Window(_, iced::window::Event::Rescaled(scale)) | Message::Scale(scale) => {
                self.scale = scale
            }
            Message::Window(_, _) => return iced::Task::none(),
            Message::Fit => self.chart.fit_content(),
            Message::Append if self.registered => {
                self.chart
                    .update_data(self.id, self.bar(self.count, 0.), false)
                    .unwrap();
                if self.volume_registered {
                    self.chart
                        .update_data(self.volume_id, self.volume_bar(self.count, 0.), false)
                        .unwrap();
                }
                self.count += 1;
                self.sync_left();
            }
            Message::Replace if self.registered && self.count > 0 => {
                self.chart
                    .update_data(self.id, self.bar(self.count - 1, 3.), false)
                    .unwrap();
                if self.volume_registered {
                    self.chart
                        .update_data(self.volume_id, self.volume_bar(self.count - 1, 3.), false)
                        .unwrap();
                }
            }
            Message::Prepend if self.registered => {
                self.history_start -= 20;
                let data = (self.history_start..self.count as i64)
                    .map(|i| self.history_bar(i, false))
                    .collect();
                self.chart.set_data(self.id, data).unwrap();
                if self.volume_registered {
                    self.sync_volume();
                }
            }
            Message::Clear if self.registered => {
                self.chart.set_data(self.id, vec![]).unwrap();
                if self.volume_registered {
                    self.chart.set_data(self.volume_id, vec![]).unwrap();
                }
                self.count = 0;
                self.history_start = 0;
                self.sync_left();
            }
            Message::Older => self.chart.set_logical_range(LogicalRange {
                from: Logical::new(10.),
                to: Logical::new(50.),
            }),
            Message::Latest => self.chart.set_right_offset(0.),
            Message::Remove if self.registered => {
                if self.volume_registered {
                    self.chart.remove_series(self.volume_id).unwrap();
                    self.volume_registered = false;
                }
                self.chart.remove_series(self.id).unwrap();
                self.registered = false;
                self.count = 0;
                self.sync_left();
            }
            Message::Reload => self.load(),
            Message::Animate => self
                .chart
                .scroll_to_offset_animated(-40., Duration::from_secs(2)),
            Message::Stop => self.chart.stop_time_scale_animation(),
            Message::Kind => {
                self.kind = if self.series_kind.candlesticks() {
                    if self.kind == LineType::Simple {
                        LineType::WithSteps
                    } else {
                        LineType::Simple
                    }
                } else {
                    match self.kind {
                        LineType::Simple => LineType::WithSteps,
                        LineType::WithSteps => LineType::Curved,
                        LineType::Curved => LineType::Simple,
                    }
                };
                self.apply_style();
            }
            Message::Style => {
                self.style = match self.style {
                    LineStyle::Solid => LineStyle::Dashed,
                    _ => LineStyle::Solid,
                };
                self.apply_style();
            }
            Message::Markers => {
                self.markers = !self.markers;
                self.apply_style();
            }
            Message::LeftAxis => {
                self.left_enabled = !self.left_enabled;
                self.chart
                    .apply_price_axis_options(
                        0,
                        PriceAxisSide::Left,
                        PriceScaleOptionsPatch {
                            visible: Some(self.left_enabled),
                            ..Default::default()
                        },
                    )
                    .unwrap();
                self.sync_left();
            }
            Message::Ticks => {
                self.ticks = !self.ticks;
                self.chart.apply_time_scale_options(HorzScaleOptionsPatch {
                    ticks_visible: Some(self.ticks),
                    ..Default::default()
                });
                for side in [PriceAxisSide::Left, PriceAxisSide::Right] {
                    self.chart
                        .apply_price_axis_options(
                            0,
                            side,
                            PriceScaleOptionsPatch {
                                ticks_visible: Some(self.ticks),
                                ..Default::default()
                            },
                        )
                        .unwrap();
                }
            }
            _ => {}
        }
        self.prepare(now);
        iced::Task::none()
    }
    fn apply_style(&mut self) {
        if self.registered {
            self.chart
                .replace_series_options(self.id, self.series_options())
                .unwrap();
        }
    }
    fn subscription(&self) -> Subscription<Message> {
        iced::window::events().map(|(id, event)| Message::Window(id, event))
    }
    fn view(&self) -> Element<'_, Message> {
        let controls = row![
            button("Fit").on_press(Message::Fit),
            button("Append").on_press(Message::Append),
            button("Replace").on_press(Message::Replace),
            button("Older").on_press(Message::Older),
            button("Prepend").on_press(Message::Prepend),
            button("Clear").on_press(Message::Clear),
            button("Latest").on_press(Message::Latest),
            button("Remove").on_press(Message::Remove),
            button("Reload").on_press(Message::Reload)
        ]
        .spacing(5);
        let styles = row![
            button(text(if self.series_kind.candlesticks() {
                format!("Wicks: {}", self.kind == LineType::Simple)
            } else {
                format!("{:?}", self.kind)
            }))
            .on_press(Message::Kind),
            button(text(if self.series_kind.candlesticks() {
                format!("Borders: {}", self.style == LineStyle::Solid)
            } else {
                format!("{:?}", self.style)
            }))
            .on_press(Message::Style),
            button(if self.series_kind.candlesticks() {
                "Transparency"
            } else {
                "Markers"
            })
            .on_press(Message::Markers),
            button("Animate").on_press(Message::Animate),
            button("Stop").on_press(Message::Stop),
            button("Left axis").on_press(Message::LeftAxis),
            button("Ticks").on_press(Message::Ticks),
            button(text(format!(
                "Crosshair: {:?}",
                self.chart.model().crosshair().options().mode
            )))
            .on_press(Message::CrosshairMode)
        ]
        .spacing(5);
        let chart = self.chart.element().map(Message::from);
        container(
            column![
                controls,
                styles,
                text(self.error.clone().unwrap_or_else(|| format!(
                    "{} bars · axes L {:.0} / R {:.0} / T {:.0} px · plot {:.0} × {:.0} · {} passes",
                    self.count,
                    self.chart.frame().layout.left_axis.size.width,
                    self.chart.frame().layout.right_axis.size.width,
                    self.chart.frame().layout.time_axis.size.height,
                    self.chart.frame().layout.plot.size.width,
                    self.chart.frame().layout.plot.size.height,
                    self.chart.frame().passes
                ))),
                text(format!("{} · chart {:.0} × {:.0} · ratio {} · requested {:.0}/{:.0}/{:.0}",
                    self.config.scenario.name(), self.size.width, self.size.height, self.scale,
                    self.chart.frame().requests.left_width, self.chart.frame().requests.right_width, self.chart.frame().requests.time_height)),
                chart
            ]
            .spacing(8),
        )
        .padding(12)
        .into()
    }
}
impl From<ChartMessage> for Message {
    fn from(message: ChartMessage) -> Self {
        match message {
            ChartMessage::Touch(input) => Self::Touch(input),
            ChartMessage::PrimitiveEdit(input) => Self::PrimitiveEdit(input),
            ChartMessage::Interaction(input) => Self::Interaction(input),
            ChartMessage::PaneInteraction(input) => Self::PaneInteraction(input),
            ChartMessage::PriceAxis(input) => Self::PriceAxis(input),
            ChartMessage::Bounds(size) => Self::Bounds(size),
            ChartMessage::Tick(now) => Self::Tick(now),
            ChartMessage::Click { pane, position } => Self::Click { pane, position },
        }
    }
}
fn smoke(config: DisplayConfig, candlesticks: bool) {
    smoke_series(config, candlesticks, false);
}
fn smoke_series(config: DisplayConfig, candlesticks: bool, histogram: bool) {
    let mut demo = Demo::new_series(config, candlesticks, histogram);
    let _ = demo.update(Message::Bounds(Size::new(800., 500.)));
    assert!(demo.has_series_geometry());
    assert!(demo.chart.frame().prices.right.required_width > 0.);
    assert_eq!(
        demo.chart.frame().prices.right.size.height,
        demo.chart.frame().plot.size.height
    );
    assert!(demo.chart.frame().time.required_height > 0.);
    assert_eq!(
        demo.chart.frame().time.size.width,
        demo.chart.frame().plot.size.width
    );
    if !demo.left_enabled {
        let _ = demo.update(Message::LeftAxis);
    }
    if !demo.ticks {
        let _ = demo.update(Message::Ticks);
    }
    assert!(demo.chart.frame().prices.left.required_width > 0.);
    let _ = demo.update(Message::Older);
    let before = demo.chart.model().time_scale().right_offset();
    let _ = demo.update(Message::Append);
    assert_eq!(demo.chart.model().time_scale().right_offset(), before - 1.);
    let _ = demo.update(Message::Replace);
    let _ = demo.update(Message::Kind);
    let _ = demo.update(Message::Kind);
    let _ = demo.update(Message::Style);
    let _ = demo.update(Message::Markers);
    let _ = demo.update(Message::Bounds(Size::new(500., 300.)));
    assert_eq!(
        demo.chart.frame().layout.available,
        AxisBounds {
            width: 500.,
            height: 300.
        }
    );
    assert_eq!(
        demo.chart.frame().time.size.width,
        demo.chart.frame().plot.size.width
    );
    let _ = demo.update(Message::Animate);
    assert!(demo.chart.is_animating());
    let _ = demo.update(Message::Stop);
    assert!(!demo.chart.is_animating());
    let _ = demo.update(Message::Remove);
    assert!(!demo.has_series_geometry());
    assert!(
        demo.chart
            .frame()
            .plot
            .axes
            .as_ref()
            .unwrap()
            .time
            .ticks
            .is_empty()
    );
    let _ = demo.update(Message::Reload);
    assert!(demo.has_series_geometry());
    println!(
        "Chart demo smoke test passed: load, fit, resize, append, replace, styles, animation, remove/reload."
    );
}
fn smoke_volume(config: DisplayConfig) {
    let mut demo = Demo::new_chart(config, SeriesKind::PriceVolume);
    let _ = demo.update(Message::Bounds(Size::new(800., 500.)));
    assert!(demo.error.is_none(), "{:?}", demo.error);
    assert_eq!(demo.chart.multi_frame().unwrap().panes.len(), 2);
    for message in [
        Message::Append,
        Message::Replace,
        Message::Older,
        Message::Prepend,
        Message::Latest,
        Message::Fit,
        Message::Clear,
        Message::Remove,
        Message::Reload,
    ] {
        let _ = demo.update(message);
        assert!(demo.error.is_none(), "{:?}", demo.error);
    }
    let frame = demo.chart.multi_frame().unwrap();
    assert!(!frame.panes[0].plot.candles.is_empty());
    assert!(!frame.panes[1].plot.candles.is_empty());
    assert_eq!(
        frame.panes[0].plot.size.width,
        frame.panes[1].plot.size.width
    );
    println!(
        "Price/volume smoke passed: aligned panes, paired updates, history, removal and reload."
    );
}
pub fn run(candlesticks: bool) -> iced::Result {
    run_series(candlesticks, false)
}
pub fn run_histogram() -> iced::Result {
    run_series(false, true)
}
pub fn run_price_volume() -> iced::Result {
    run_chart(SeriesKind::PriceVolume)
}
fn run_series(candlesticks: bool, histogram: bool) -> iced::Result {
    run_chart(if histogram {
        SeriesKind::Histogram
    } else if candlesticks {
        SeriesKind::Candlesticks
    } else {
        SeriesKind::Line
    })
}
fn run_chart(series_kind: SeriesKind) -> iced::Result {
    let candlesticks = series_kind.candlesticks();
    let histogram = series_kind.histogram();
    let volume = series_kind.volume();
    let args = std::env::args().collect::<Vec<_>>();
    let scenario = args
        .windows(2)
        .find(|a| a[0] == "--scenario")
        .map(|a| DisplayScenario::parse(&a[1]).expect("unknown display scenario"))
        .unwrap_or_default();
    let mut config = DisplayConfig::for_scenario(scenario);
    config.multiplier = if args.iter().any(|a| a == "--large") {
        10000.
    } else {
        config.multiplier
    };
    config.negative |= args.iter().any(|a| a == "--negative");
    config.gradient |= args.iter().any(|a| a == "--gradient");
    config.left_axis |= args.iter().any(|a| a == "--left-axis");
    config.ticks |= args.iter().any(|a| a == "--ticks");
    config.fixed_edges |= args.iter().any(|a| a == "--fixed-edges");
    if std::env::args().any(|arg| arg == "--smoke") {
        if volume {
            smoke_volume(config);
        } else {
            smoke_series(config, candlesticks, histogram);
        }
        return Ok(());
    }
    if let Some(directory) = args.windows(2).find(|a| a[0] == "--capture-dir") {
        capture::capture_display(
            std::path::Path::new(&directory[1]),
            args.iter().any(|a| a == "--wgpu"),
            candlesticks,
            histogram,
            volume,
        );
        return Ok(());
    }
    let size = Size::new(config.width + 24., config.height + 145.);
    iced::application(
        move || {
            let mut demo = Demo::new_chart(config.clone(), series_kind);
            demo.apply_scenario(Instant::now());
            demo
        },
        Demo::update,
        Demo::view,
    )
    .title(if volume {
        "Feather Charts — price and volume"
    } else if histogram {
        "Feather Charts — histogram"
    } else if candlesticks {
        "Feather Charts — candlesticks"
    } else {
        "Feather Charts — price and time axes"
    })
    .theme(Theme::Dark)
    .subscription(Demo::subscription)
    .window_size(size)
    .run()
}

#[path = "chart_demo/capture.rs"]
mod capture;

#[cfg(test)]
#[path = "chart_demo/tests.rs"]
mod tests;
