//! Run with `cargo run --example line_chart`; `--smoke` checks the demo headlessly.
#[path = "display_scenario.rs"]
mod display_scenario;
use display_scenario::{DisplayConfig, DisplayScenario};
use feather_charts::{
    model::{
        axis_snapshots::PriceAxisSide,
        chart_model::{ChartModel, ChartModelOptions},
        data_consumer::{
            BuiltInSeriesDataItem, CandlestickData, CandlestickDataItem, HistogramData,
            HistogramDataItem, LineData, LineDataItem, WhitespaceData,
        },
        data_layer::{DataLayer, SeriesId},
        grid::GridOptions,
        horz_scale_behavior_time::{
            horz_scale_behavior_time::HorzScaleBehaviorTime,
            types::{Time, UtcTimestamp},
        },
        layout_options::{Background, ColorSpace, LayoutOptions, LayoutPanesOptions},
        localization_options::LocalizationOptions,
        pane::{PaneOptions, PriceScalePosition},
        price_scale::PriceScaleOptionsPatch,
        series::{
            candlesticks_pane_view::CandlesticksPaneView, histogram_pane_view::HistogramPaneView,
            line_pane_view::LinePaneView,
        },
        series_options::{
            CandlestickStyleOptions, HistogramStyleOptions, LastPriceAnimationMode,
            LineStyleOptions, PriceFormat, PriceFormatBuiltIn, PriceFormatBuiltInType,
            PriceLineSource, SeriesOptions, SeriesOptionsCommon, SeriesOptionsMap, SeriesType,
        },
        time_data::{Logical, LogicalRange},
        time_scale::TimeScale,
        time_scale_options::{HorzScaleOptions, HorzScaleOptionsPatch},
    },
    renderers::{
        draw_line::{LineStyle, LineType, LineWidth},
        grid_renderer::PixelRatio,
        price_axis_renderer::AxisBounds,
    },
    ui::{
        chart_frame::{ChartFrameInput, ChartFrameOwner, IcedAxisMeasurer, IcedChartFrame},
        chart_input::{ChartInput, ChartInteraction, Navigation},
        price_axis::AxisFontResolver,
    },
    views::time_scale_animation::TimeScaleAnimationController,
};
use iced::{
    Element, Size, Subscription, Theme,
    widget::{
        button,
        canvas::{self, Canvas, Frame, Geometry},
        column, container, row, text,
    },
};
use std::time::{Duration, Instant};

const ID: SeriesId = SeriesId::new(1);
const LEFT_ID: SeriesId = SeriesId::new(2);
fn options(kind: LineType, style: LineStyle, markers: bool) -> SeriesOptionsMap {
    SeriesOptionsMap::Line(SeriesOptions {
        common: SeriesOptionsCommon {
            last_value_visible: false,
            title: "Demo".into(),
            price_scale_id: None,
            series_last_value_mode: None,
            visible: true,
            hit_test_tolerance: 3.,
            price_line_visible: false,
            price_line_source: PriceLineSource::LastBar,
            price_line_width: LineWidth::One,
            price_line_color: String::new(),
            price_line_style: LineStyle::Solid,
            price_format: PriceFormat::BuiltIn(PriceFormatBuiltIn {
                kind: PriceFormatBuiltInType::Price,
                precision: 2,
                min_move: 0.01,
                base: Some(100.),
            }),
            base_line_visible: false,
            base_line_color: String::new(),
            base_line_width: LineWidth::One,
            base_line_style: LineStyle::Solid,
            autoscale_info_provider: None,
            conflation_threshold_factor: None,
        },
        style: LineStyleOptions {
            color: "#2196f3".into(),
            line_style: style,
            line_width: LineWidth::Three,
            line_type: kind,
            line_visible: true,
            point_markers_visible: markers,
            point_markers_radius: None,
            crosshair_marker_visible: true,
            crosshair_marker_radius: 4.,
            crosshair_marker_border_color: String::new(),
            crosshair_marker_background_color: String::new(),
            crosshair_marker_border_width: 2.,
            last_price_animation: LastPriceAnimationMode::Disabled,
        },
    })
}
fn bar(config: &DisplayConfig, index: usize, adjustment: f64) -> BuiltInSeriesDataItem<Time> {
    let value = config.value(index, adjustment);
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: Time::from(UtcTimestamp::new(1_700_000_000. + index as f64 * 60.)),
        value,
        color: Some(
            if (index / 20).is_multiple_of(2) {
                "#2196f3"
            } else {
                "#f59e0b"
            }
            .into(),
        ),
        custom_values: None,
    }))
}
struct Demo {
    config: DisplayConfig,
    candlesticks: bool,
    histogram: bool,
    histogram_view: HistogramPaneView,
    candle_view: CandlesticksPaneView,
    layer: DataLayer<HorzScaleBehaviorTime>,
    model: ChartModel<HorzScaleBehaviorTime>,
    controller: TimeScaleAnimationController,
    view: LinePaneView,
    frame: IcedChartFrame,
    frame_owner: ChartFrameOwner,
    fonts: AxisFontResolver,
    left_enabled: bool,
    left_registered: bool,
    ticks: bool,
    size: Size,
    scale: f32,
    count: usize,
    registered: bool,
    error: Option<String>,
    kind: LineType,
    style: LineStyle,
    markers: bool,
}
#[derive(Clone, Debug)]
enum Message {
    CrosshairMode,
    Interaction(ChartInteraction),
    Bounds(Size),
    Tick(Instant),
    Window(iced::window::Id, iced::window::Event),
    Scale(f32),
    Fit,
    Append,
    Replace,
    Older,
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
        let model = ChartModel::new(
            TimeScale::new(
                HorzScaleBehaviorTime::default(),
                HorzScaleOptions::default(),
                LocalizationOptions::new("en-US", "dd MMM 'yy"),
            ),
            ChartModelOptions {
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
                        enable_resize: false,
                        separator_color: String::new(),
                        separator_hover_color: String::new(),
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
            },
        );
        let mut demo = Self {
            config,
            candlesticks,
            histogram,
            histogram_view: HistogramPaneView::default(),
            candle_view: CandlesticksPaneView::default(),
            layer: DataLayer::new(HorzScaleBehaviorTime::default()),
            model,
            controller: TimeScaleAnimationController::default(),
            view: LinePaneView::default(),
            frame: IcedChartFrame::default(),
            frame_owner: ChartFrameOwner::default(),
            fonts: AxisFontResolver::default(),
            left_enabled: false,
            left_registered: false,
            ticks: false,
            size: Size::ZERO,
            scale: 1.,
            count: 0,
            registered: false,
            error: None,
            kind: LineType::Simple,
            style: LineStyle::Solid,
            markers: false,
        };
        demo.model.apply_time_scale_options(
            &mut demo.layer,
            HorzScaleOptionsPatch {
                time_visible: Some(true),
                fix_left_edge: Some(demo.config.fixed_edges),
                fix_right_edge: Some(demo.config.fixed_edges),
                ..Default::default()
            },
        );
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
        self.model.fit_content();
        match self.config.scenario {
            DisplayScenario::Historical => {
                let _ = self.update(Message::Older);
                let _ = self.update(Message::Append);
            }
            DisplayScenario::Empty => {
                let _ = self.update(Message::Remove);
            }
            DisplayScenario::Animation => {
                self.model
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
            self.model
                .register_series(
                    ID,
                    self.series_type(),
                    self.series_options(),
                    0,
                    PriceScalePosition::Right,
                )
                .unwrap();
            self.registered = true;
        }
        self.count = 120;
        let response = self
            .layer
            .set_series_data(
                ID,
                self.series_type(),
                (0..self.count).map(|i| self.bar(i, 0.)).collect(),
            )
            .unwrap();
        self.model.apply_data_update(response).unwrap();
        self.model.fit_content();
        self.sync_left();
    }
    fn sync_left(&mut self) {
        if self.left_enabled && self.registered {
            if !self.left_registered {
                self.model
                    .register_series(
                        LEFT_ID,
                        SeriesType::Line,
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
            let update = self
                .layer
                .set_series_data(LEFT_ID, SeriesType::Line, data)
                .unwrap();
            self.model.apply_data_update(update).unwrap();
        } else if self.left_registered {
            let response = self.layer.remove_series(LEFT_ID).unwrap();
            self.model.remove_series(LEFT_ID, response).unwrap();
            self.left_registered = false;
        }
    }
    fn prepare(&mut self, now: Instant) {
        let input = ChartFrameInput {
            series: self.registered.then_some(ID),
            bounds: AxisBounds {
                width: f64::from(self.size.width),
                height: f64::from(self.size.height),
            },
            pixel_ratio: PixelRatio {
                horizontal: self.scale,
                vertical: self.scale,
            },
            background: "#101820".into(),
        };
        let result = self
            .prepare_snapshot(&input, now)
            .and_then(|snapshot| IcedChartFrame::from_snapshot(snapshot, &self.fonts));
        match result {
            Ok(frame) => {
                self.frame = frame;
                self.error = None;
            }
            Err(error) => {
                self.frame = IcedChartFrame::default();
                self.frame.layout.available = input.bounds;
                self.error = Some(format!("{error:?}"));
            }
        }
    }
    fn prepare_snapshot(
        &mut self,
        input: &ChartFrameInput,
        now: Instant,
    ) -> Result<
        feather_charts::ui::chart_frame::ChartFrameSnapshot,
        feather_charts::ui::chart_frame::ChartFrameError,
    > {
        let mut measurer = IcedAxisMeasurer { fonts: &self.fonts };
        if self.histogram {
            self.frame_owner.prepare_histogram(
                &mut self.model,
                &mut self.controller,
                &mut self.histogram_view,
                input,
                &mut measurer,
                now,
            )
        } else if self.candlesticks {
            self.frame_owner.prepare_candlesticks(
                &mut self.model,
                &mut self.controller,
                &mut self.candle_view,
                input,
                &mut measurer,
                now,
            )
        } else {
            self.frame_owner.prepare(
                &mut self.model,
                &mut self.controller,
                &mut self.view,
                input,
                &mut measurer,
                now,
            )
        }
    }
    fn series_type(&self) -> SeriesType {
        if self.histogram {
            SeriesType::Histogram
        } else if self.candlesticks {
            SeriesType::Candlestick
        } else {
            SeriesType::Line
        }
    }
    fn series_options(&self) -> SeriesOptionsMap {
        let line = options(self.kind, self.style, self.markers);
        if self.histogram {
            let SeriesOptionsMap::Line(line) = line else {
                unreachable!()
            };
            return SeriesOptionsMap::Histogram(SeriesOptions {
                common: line.common,
                style: HistogramStyleOptions {
                    color: "#2196f3".into(),
                    base: 0.,
                },
            });
        }
        if !self.candlesticks {
            return line;
        }
        let SeriesOptionsMap::Line(line) = line else {
            unreachable!()
        };
        let (up, down) = if self.markers {
            ("rgba(38,166,154,0.35)", "rgba(239,83,80,0.35)")
        } else {
            ("#26a69a", "#ef5350")
        };
        SeriesOptionsMap::Candlestick(SeriesOptions {
            common: line.common,
            style: CandlestickStyleOptions {
                up_color: up.into(),
                down_color: down.into(),
                wick_visible: self.kind == LineType::Simple,
                border_visible: self.style == LineStyle::Solid,
                border_color: "#378658".into(),
                border_up_color: up.into(),
                border_down_color: down.into(),
                wick_color: "#737375".into(),
                wick_up_color: up.into(),
                wick_down_color: down.into(),
            },
        })
    }
    fn bar(&self, index: usize, adjustment: f64) -> BuiltInSeriesDataItem<Time> {
        if self.histogram {
            let time = Time::from(UtcTimestamp::new(1_700_000_000. + index as f64 * 60.));
            return BuiltInSeriesDataItem::Histogram(if index % 13 == 5 {
                HistogramDataItem::Whitespace(WhitespaceData {
                    time,
                    custom_values: None,
                })
            } else {
                HistogramDataItem::Data(HistogramData {
                    time,
                    value: self.config.value(index, adjustment),
                    color: index.is_multiple_of(19).then(|| "#f59e0b".into()),
                    custom_values: None,
                })
            });
        }
        if !self.candlesticks {
            return bar(&self.config, index, adjustment);
        }
        let time = Time::from(UtcTimestamp::new(1_700_000_000. + index as f64 * 60.));
        if index % 13 == 5 {
            return BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Whitespace(
                WhitespaceData {
                    time,
                    custom_values: None,
                },
            ));
        }
        let close = self.config.value(index, adjustment);
        let span = 2. * self.config.multiplier;
        let open = close
            + match index % 3 {
                0 => -span,
                1 => span,
                _ => 0.,
            };
        BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(CandlestickData {
            time,
            open,
            high: open.max(close) + span,
            low: open.min(close) - span,
            close,
            color: index.is_multiple_of(19).then(|| "#2196f3".into()),
            border_color: index.is_multiple_of(19).then(|| "#f59e0b".into()),
            wick_color: index.is_multiple_of(19).then(|| "#ddd".into()),
            custom_values: None,
        }))
    }
    fn has_series_geometry(&self) -> bool {
        if self.candlesticks || self.histogram {
            !self.frame.plot.candles.is_empty()
        } else {
            !self.frame.plot.lines.is_empty()
        }
    }
    fn update(&mut self, message: Message) -> iced::Task<Message> {
        let mut now = Instant::now();
        match message {
            Message::CrosshairMode => {
                use feather_charts::model::crosshair::CrosshairMode;
                let mut options = self.model.crosshair().options().clone();
                options.mode = match options.mode {
                    CrosshairMode::Magnet => CrosshairMode::Normal,
                    CrosshairMode::Normal if self.candlesticks => CrosshairMode::MagnetOhlc,
                    CrosshairMode::Normal | CrosshairMode::MagnetOhlc => CrosshairMode::Hidden,
                    _ => CrosshairMode::Magnet,
                };
                self.model.set_crosshair_options(options).unwrap();
            }
            Message::Interaction(input) => {
                use feather_charts::model::coordinate::Coordinate;
                match input.navigation {
                    Some(Navigation::Wheel { x, zoom, scroll }) => {
                        self.model.end_scroll_time();
                        self.model.zoom_time(Coordinate::new(x), zoom);
                        if scroll != 0. {
                            self.model.start_scroll_time(Coordinate::new(0.));
                            self.model.scroll_time_to(Coordinate::new(scroll));
                            self.model.end_scroll_time();
                        }
                    }
                    Some(Navigation::Scroll { start, x }) => {
                        if let Some(start) = start {
                            self.model.start_scroll_time(Coordinate::new(start));
                        }
                        self.model.scroll_time_to(Coordinate::new(x));
                    }
                    Some(Navigation::EndScroll) => self.model.end_scroll_time(),
                    None => {}
                }
                self.model
                    .set_crosshair_position(
                        0,
                        input.position.map(|point| {
                            feather_charts::renderers::line_renderer::Point {
                                x: f64::from(point.x),
                                y: f64::from(point.y),
                            }
                        }),
                    )
                    .unwrap();
            }
            Message::Bounds(size) => {
                let startup = self.size.width == 0. && size.width > 0.;
                self.size = size;
                if startup && self.registered {
                    self.model.fit_content();
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
            Message::Fit => self.model.fit_content(),
            Message::Append if self.registered => {
                let response = self
                    .layer
                    .update_series_data(ID, self.bar(self.count, 0.), false)
                    .unwrap();
                self.model.apply_data_update(response).unwrap();
                self.count += 1;
                self.sync_left();
            }
            Message::Replace if self.registered && self.count > 0 => {
                let response = self
                    .layer
                    .update_series_data(ID, self.bar(self.count - 1, 3.), false)
                    .unwrap();
                self.model.apply_data_update(response).unwrap();
            }
            Message::Older => self.model.set_logical_range(LogicalRange {
                from: Logical::new(10.),
                to: Logical::new(50.),
            }),
            Message::Latest => self.model.set_right_offset(0.),
            Message::Remove if self.registered => {
                let response = self.layer.remove_series(ID).unwrap();
                self.model.remove_series(ID, response).unwrap();
                self.registered = false;
                self.count = 0;
                self.sync_left();
            }
            Message::Reload => self.load(),
            Message::Animate => self
                .model
                .scroll_to_offset_animated(-40., Duration::from_secs(2)),
            Message::Stop => self.model.stop_time_scale_animation(),
            Message::Kind => {
                self.kind = if self.candlesticks {
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
                self.model
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
                self.model.apply_time_scale_options(
                    &mut self.layer,
                    HorzScaleOptionsPatch {
                        ticks_visible: Some(self.ticks),
                        ..Default::default()
                    },
                );
                for side in [PriceAxisSide::Left, PriceAxisSide::Right] {
                    self.model
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
            self.model
                .replace_series_options(ID, self.series_options())
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
            button("Latest").on_press(Message::Latest),
            button("Remove").on_press(Message::Remove),
            button("Reload").on_press(Message::Reload)
        ]
        .spacing(5);
        let styles = row![
            button(text(if self.candlesticks {
                format!("Wicks: {}", self.kind == LineType::Simple)
            } else {
                format!("{:?}", self.kind)
            }))
            .on_press(Message::Kind),
            button(text(if self.candlesticks {
                format!("Borders: {}", self.style == LineStyle::Solid)
            } else {
                format!("{:?}", self.style)
            }))
            .on_press(Message::Style),
            button(if self.candlesticks {
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
                self.model.crosshair().options().mode
            )))
            .on_press(Message::CrosshairMode)
        ]
        .spacing(5);
        let chart = Canvas::new(Chart {
            snapshot: &self.frame,
            animate: self.controller.is_active(),
            enabled: !self.model.time_scale().is_empty(),
        })
        .width(iced::Fill)
        .height(iced::Fill);
        container(
            column![
                controls,
                styles,
                text(self.error.clone().unwrap_or_else(|| format!(
                    "{} bars · axes L {:.0} / R {:.0} / T {:.0} px · plot {:.0} × {:.0} · {} passes",
                    self.count,
                    self.frame.layout.left_axis.size.width,
                    self.frame.layout.right_axis.size.width,
                    self.frame.layout.time_axis.size.height,
                    self.frame.layout.plot.size.width,
                    self.frame.layout.plot.size.height,
                    self.frame.passes
                ))),
                text(format!("{} · chart {:.0} × {:.0} · ratio {} · requested {:.0}/{:.0}/{:.0}",
                    self.config.scenario.name(), self.size.width, self.size.height, self.scale,
                    self.frame.requests.left_width, self.frame.requests.right_width, self.frame.requests.time_height)),
                chart
            ]
            .spacing(8),
        )
        .padding(12)
        .into()
    }
}
struct Chart<'a> {
    snapshot: &'a IcedChartFrame,
    animate: bool,
    enabled: bool,
}
#[derive(Default)]
struct ChartState {
    input: ChartInput,
    last_redraw: Option<Instant>,
}
impl canvas::Program<Message> for Chart<'_> {
    type State = ChartState;
    fn update(
        &self,
        state: &mut ChartState,
        event: &canvas::Event,
        bounds: iced::Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if let Some(action) = state.input.update(
            event,
            cursor
                .position()
                .map(|p| iced::Point::new(p.x - bounds.x, p.y - bounds.y)),
            self.snapshot.layout.plot,
            self.enabled,
        ) {
            let publish = canvas::Action::publish(Message::Interaction(action));
            return Some(if action.capture {
                publish.and_capture()
            } else {
                publish
            });
        }
        if let canvas::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            if bounds.size()
                != Size::new(
                    self.snapshot.layout.available.width as f32,
                    self.snapshot.layout.available.height as f32,
                )
            {
                return Some(canvas::Action::publish(Message::Bounds(bounds.size())));
            }
            if self.animate {
                let next = state
                    .last_redraw
                    .map(|last| last + Duration::from_millis(16))
                    .unwrap_or(*now);
                if *now < next {
                    return Some(canvas::Action::request_redraw_at(next));
                }
                state.last_redraw = Some(*now);
                return Some(canvas::Action::publish(Message::Tick(*now)));
            }
            state.last_redraw = None;
        }
        None
    }
    fn draw(
        &self,
        _state: &ChartState,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        self.snapshot.draw(&mut frame);
        vec![frame.into_geometry()]
    }
}
fn smoke(config: DisplayConfig, candlesticks: bool) {
    smoke_series(config, candlesticks, false);
}
fn smoke_series(config: DisplayConfig, candlesticks: bool, histogram: bool) {
    let mut demo = Demo::new_series(config, candlesticks, histogram);
    let _ = demo.update(Message::Bounds(Size::new(800., 500.)));
    assert!(demo.has_series_geometry());
    assert!(demo.frame.prices.right.required_width > 0.);
    assert_eq!(
        demo.frame.prices.right.size.height,
        demo.frame.plot.size.height
    );
    assert!(demo.frame.time.required_height > 0.);
    assert_eq!(demo.frame.time.size.width, demo.frame.plot.size.width);
    if !demo.left_enabled {
        let _ = demo.update(Message::LeftAxis);
    }
    if !demo.ticks {
        let _ = demo.update(Message::Ticks);
    }
    assert!(demo.frame.prices.left.required_width > 0.);
    let _ = demo.update(Message::Older);
    let before = demo.model.time_scale().right_offset();
    let _ = demo.update(Message::Append);
    assert_eq!(demo.model.time_scale().right_offset(), before - 1.);
    let _ = demo.update(Message::Replace);
    let _ = demo.update(Message::Kind);
    let _ = demo.update(Message::Kind);
    let _ = demo.update(Message::Style);
    let _ = demo.update(Message::Markers);
    let _ = demo.update(Message::Bounds(Size::new(500., 300.)));
    assert_eq!(
        demo.frame.layout.available,
        AxisBounds {
            width: 500.,
            height: 300.
        }
    );
    assert_eq!(demo.frame.time.size.width, demo.frame.plot.size.width);
    let _ = demo.update(Message::Animate);
    assert!(demo.controller.is_active());
    let _ = demo.update(Message::Stop);
    assert!(!demo.controller.is_active());
    let _ = demo.update(Message::Remove);
    assert!(!demo.has_series_geometry());
    assert!(demo.frame.plot.axes.as_ref().unwrap().time.ticks.is_empty());
    let _ = demo.update(Message::Reload);
    assert!(demo.has_series_geometry());
    println!(
        "Chart demo smoke test passed: load, fit, resize, append, replace, styles, animation, remove/reload."
    );
}
pub fn run(candlesticks: bool) -> iced::Result {
    run_series(candlesticks, false)
}
pub fn run_histogram() -> iced::Result {
    run_series(false, true)
}
fn run_series(candlesticks: bool, histogram: bool) -> iced::Result {
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
        smoke_series(config, candlesticks, histogram);
        return Ok(());
    }
    if let Some(directory) = args.windows(2).find(|a| a[0] == "--capture-dir") {
        capture::capture_display(
            std::path::Path::new(&directory[1]),
            args.iter().any(|a| a == "--wgpu"),
            candlesticks,
            histogram,
        );
        return Ok(());
    }
    let size = Size::new(config.width + 24., config.height + 145.);
    iced::application(
        move || {
            let mut demo = Demo::new_series(config.clone(), candlesticks, histogram);
            demo.apply_scenario(Instant::now());
            demo
        },
        Demo::update,
        Demo::view,
    )
    .title(if histogram {
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
