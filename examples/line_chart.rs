//! Run with `cargo run --example line_chart`; `--smoke` checks the demo headlessly.
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
fn bar(index: usize, adjustment: f64) -> BuiltInSeriesDataItem<Time> {
    let value = 100. + (index as f64 * 0.2).sin() * 8. + index as f64 * 0.04 + adjustment;
    let value = if std::env::args().any(|arg| arg == "--large") {
        value * 10000.
    } else {
        value
    };
    let value = if std::env::args().any(|arg| arg == "--negative") {
        -value
    } else {
        value
    };
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
        let model = ChartModel::new(
            TimeScale::new(
                HorzScaleBehaviorTime::default(),
                HorzScaleOptions::default(),
                LocalizationOptions::new("en-US", "dd MMM 'yy"),
            ),
            ChartModelOptions {
                layout: LayoutOptions {
                    background: if std::env::args().any(|arg| arg == "--gradient") {
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
                    font_size: 12.,
                    font_family: "sans-serif".into(),
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
                fix_left_edge: Some(std::env::args().any(|arg| arg == "--fixed-edges")),
                fix_right_edge: Some(std::env::args().any(|arg| arg == "--fixed-edges")),
                ..Default::default()
            },
        );
        demo.load();
        if std::env::args().any(|arg| arg == "--left-axis") {
            let _ = demo.update(Message::LeftAxis);
        }
        if std::env::args().any(|arg| arg == "--ticks") {
            let _ = demo.update(Message::Ticks);
        }
        demo
    }
}
impl Demo {
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
                (0..self.count).map(|i| bar(i, 0.)).collect(),
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
                    let mut item = bar(index, 0.);
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
                    .update_series_data(ID, bar(self.count, 0.), false)
                    .unwrap();
                self.model.apply_data_update(response).unwrap();
                self.count += 1;
                self.sync_left();
            }
            Message::Replace if self.registered && self.count > 0 => {
                let response = self
                    .layer
                    .update_series_data(ID, bar(self.count - 1, 3.), false)
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
fn smoke() {
    let mut demo = Demo::default();
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
    if std::env::args().any(|arg| arg == "--smoke") {
        smoke();
        return Ok(());
    }
    iced::application(Demo::default, Demo::update, Demo::view)
        .title("Feather Charts — price and time axes")
        .theme(Theme::Dark)
        .subscription(Demo::subscription)
        .run()
}
#[cfg(test)]
mod tests {
    #[test]
    fn connected_demo_operations() {
        super::smoke();
    }
}
