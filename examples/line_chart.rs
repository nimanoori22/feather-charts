//! Run with `cargo run --example line_chart`; `--smoke` checks the demo headlessly.
#[path = "support/display_scenario.rs"]
mod display_scenario;
use display_scenario::{DisplayConfig, DisplayScenario};
use feather_charts::{
    model::{
        axis_snapshots::PriceAxisSide,
        chart_model::{ChartModel, ChartModelOptions},
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
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
        series::line_pane_view::LinePaneView,
        series_options::{
            LastPriceAnimationMode, LineStyleOptions, PriceFormat, PriceFormatBuiltIn,
            PriceFormatBuiltInType, PriceLineSource, SeriesOptions, SeriesOptionsCommon,
            SeriesOptionsMap, SeriesType,
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
            crosshair_marker_visible: false,
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
        Self::new(DisplayConfig::default())
    }
}
impl Demo {
    fn new(config: DisplayConfig) -> Self {
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
                    SeriesType::Line,
                    options(self.kind, self.style, self.markers),
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
                SeriesType::Line,
                (0..self.count).map(|i| bar(&self.config, i, 0.)).collect(),
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
            .frame_owner
            .prepare(
                &mut self.model,
                &mut self.controller,
                &mut self.view,
                &input,
                &mut IcedAxisMeasurer { fonts: &self.fonts },
                now,
            )
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
    fn update(&mut self, message: Message) -> iced::Task<Message> {
        let mut now = Instant::now();
        match message {
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
                    .update_series_data(ID, bar(&self.config, self.count, 0.), false)
                    .unwrap();
                self.model.apply_data_update(response).unwrap();
                self.count += 1;
                self.sync_left();
            }
            Message::Replace if self.registered && self.count > 0 => {
                let response = self
                    .layer
                    .update_series_data(ID, bar(&self.config, self.count - 1, 3.), false)
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
                self.kind = match self.kind {
                    LineType::Simple => LineType::WithSteps,
                    LineType::WithSteps => LineType::Curved,
                    LineType::Curved => LineType::Simple,
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
                .replace_series_options(ID, options(self.kind, self.style, self.markers))
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
            button(text(format!("{:?}", self.kind))).on_press(Message::Kind),
            button(text(format!("{:?}", self.style))).on_press(Message::Style),
            button("Markers").on_press(Message::Markers),
            button("Animate").on_press(Message::Animate),
            button("Stop").on_press(Message::Stop),
            button("Left axis").on_press(Message::LeftAxis),
            button("Ticks").on_press(Message::Ticks)
        ]
        .spacing(5);
        let chart = Canvas::new(Chart {
            snapshot: &self.frame,
            animate: self.controller.is_active(),
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
}
impl canvas::Program<Message> for Chart<'_> {
    type State = Option<Instant>;
    fn update(
        &self,
        state: &mut Option<Instant>,
        event: &canvas::Event,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
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
                    .map(|last| last + Duration::from_millis(16))
                    .unwrap_or(*now);
                if *now < next {
                    return Some(canvas::Action::request_redraw_at(next));
                }
                *state = Some(*now);
                return Some(canvas::Action::publish(Message::Tick(*now)));
            }
            *state = None;
        }
        None
    }
    fn draw(
        &self,
        _state: &Option<Instant>,
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
fn smoke(config: DisplayConfig) {
    let mut demo = Demo::new(config);
    let _ = demo.update(Message::Bounds(Size::new(800., 500.)));
    assert!(!demo.frame.plot.lines.is_empty());
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
    assert!(demo.frame.plot.lines.is_empty());
    assert!(demo.frame.plot.axes.as_ref().unwrap().time.ticks.is_empty());
    let _ = demo.update(Message::Reload);
    assert!(!demo.frame.plot.lines.is_empty());
    println!(
        "Line chart demo smoke test passed: load, fit, resize, append, replace, styles, animation, remove/reload."
    );
}
fn main() -> iced::Result {
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
        smoke(config);
        return Ok(());
    }
    if let Some(directory) = args.windows(2).find(|a| a[0] == "--capture-dir") {
        capture_display(
            std::path::Path::new(&directory[1]),
            args.iter().any(|a| a == "--wgpu"),
        );
        return Ok(());
    }
    let size = Size::new(config.width + 24., config.height + 145.);
    iced::application(
        move || {
            let mut demo = Demo::new(config.clone());
            demo.apply_scenario(Instant::now());
            demo
        },
        Demo::update,
        Demo::view,
    )
    .title("Feather Charts — price and time axes")
    .theme(Theme::Dark)
    .subscription(Demo::subscription)
    .window_size(size)
    .run()
}

/// Real Iced backend output at explicit physical sizes; no window-scale spoofing.
/// Captures are PPM plus JSON metadata, and can be converted to PNG externally.
fn capture_display(directory: &std::path::Path, wgpu: bool) {
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
    .expect("requested Iced headless backend unavailable");
    std::fs::create_dir_all(directory).unwrap();
    for scenario in DisplayScenario::ALL {
        for ratio in [1., 1.25, 1.5, 2.] {
            let mut demo = Demo::new(DisplayConfig::for_scenario(scenario));
            demo.scale = ratio;
            demo.apply_scenario(Instant::now());
            assert!(demo.error.is_none(), "{:?}", demo.error);
            let before = format!("{:?}", demo.frame);
            let offset = demo.model.time_scale().right_offset();
            let active = demo.controller.is_active();
            let size = Size::new(
                (demo.size.width * ratio).round() as u32,
                (demo.size.height * ratio).round() as u32,
            );
            let mut last = None;
            for _ in 0..2 {
                renderer.reset(iced::Rectangle::with_size(demo.size));
                let mut frame = Frame::new(&renderer, demo.size);
                demo.frame.draw(&mut frame);
                renderer.draw_geometry(frame.into_geometry());
                let rgba = renderer.screenshot(size, ratio, iced::Color::from_rgb8(255, 0, 255));
                assert_eq!(rgba.len(), size.width as usize * size.height as usize * 4);
                if let Some(previous) = &last {
                    assert_eq!(previous, &rgba, "drawing must be repeatable");
                }
                last = Some(rgba);
            }
            assert_eq!(before, format!("{:?}", demo.frame));
            assert_eq!(offset, demo.model.time_scale().right_offset());
            assert_eq!(active, demo.controller.is_active());
            let name = format!("{}-{backend}-{ratio}", scenario.name());
            let rgba = last.unwrap();
            if !demo.frame.plot.lines.is_empty() {
                assert!(
                    rgba.as_chunks::<4>()
                        .0
                        .iter()
                        .any(|p| p[2] > 180 && p[1] > 80 && p[0] < 80),
                    "prepared blue line must remain visible in {name}"
                );
            }
            // Magenta is the offscreen clear color, never a chart color. An
            // exposed interior pixel indicates a hole between region clips.
            for pixel in rgba.as_chunks::<4>().0 {
                assert!(
                    !(pixel[0] > pixel[1].saturating_add(30)
                        && pixel[2] > pixel[1].saturating_add(30)),
                    "unpainted chart seam in {name}: {pixel:?}"
                );
            }
            let mut ppm = format!("P6\n{} {}\n255\n", size.width, size.height).into_bytes();
            for pixel in rgba.as_chunks::<4>().0 {
                ppm.extend_from_slice(&pixel[..3]);
            }
            std::fs::write(directory.join(format!("{name}.ppm")), ppm).unwrap();
            std::fs::write(directory.join(format!("{name}.json")), format!(
                "{{\"scenario\":\"{}\",\"backend\":\"{}\",\"font\":\"sans-serif\",\"fontSize\":12,\"logicalWidth\":{},\"logicalHeight\":{},\"physicalWidth\":{},\"physicalHeight\":{},\"scaleFactor\":{},\"plotWidth\":{},\"plotHeight\":{},\"leftWidth\":{},\"rightWidth\":{},\"timeHeight\":{},\"passes\":{}}}\n",
                scenario.name(), renderer.name(), demo.size.width, demo.size.height, size.width, size.height,
                ratio, demo.frame.layout.plot.size.width, demo.frame.layout.plot.size.height,
                demo.frame.layout.left_axis.size.width, demo.frame.layout.right_axis.size.width,
                demo.frame.layout.time_axis.size.height, demo.frame.passes)).unwrap();
        }
    }
    println!(
        "28 {backend} captures and metadata written to {}",
        directory.display()
    );
}
#[cfg(test)]
mod tests {
    #[test]
    fn connected_demo_operations() {
        super::smoke(super::DisplayConfig::default());
    }
    #[test]
    fn explicit_display_scenarios_prepare_without_global_argument_state() {
        for scenario in super::DisplayScenario::ALL {
            let mut demo = super::Demo::new(super::DisplayConfig::for_scenario(scenario));
            demo.apply_scenario(std::time::Instant::now());
            assert!(demo.error.is_none(), "{scenario:?}: {:?}", demo.error);
            assert_eq!(demo.config.scenario, scenario);
            assert_eq!(
                demo.frame.layout.available.width,
                f64::from(demo.size.width)
            );
            assert_eq!(demo.frame.time.size.width, demo.frame.plot.size.width);
            if scenario == super::DisplayScenario::Empty {
                assert!(demo.frame.plot.lines.is_empty());
                let _ = demo.update(super::Message::Reload);
            }
            assert!(!demo.frame.plot.lines.is_empty());
        }
    }
}
