//! Run with `cargo run --example line_chart`; `--smoke` checks the demo headlessly.
use feather_charts::{
    model::{
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
        series::line_pane_view::LinePaneView,
        series_options::{
            LastPriceAnimationMode, LineStyleOptions, PriceFormat, PriceFormatBuiltIn,
            PriceFormatBuiltInType, PriceLineSource, SeriesOptions, SeriesOptionsCommon,
            SeriesOptionsMap, SeriesType,
        },
        time_data::{Logical, LogicalRange},
        time_scale::TimeScale,
        time_scale_options::HorzScaleOptions,
    },
    renderers::{
        draw_line::{LineStyle, LineType, LineWidth},
        grid_renderer::PixelRatio,
    },
    ui::line_chart::{FrameLayout, PlotSnapshot, prepare_frame},
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
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: Time::from(UtcTimestamp::new(1_700_000_000. + index as f64 * 60.)),
        value: 100. + (index as f64 * 0.2).sin() * 8. + index as f64 * 0.04 + adjustment,
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
    snapshot: PlotSnapshot,
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
                    background: Background::Solid {
                        color: "#101820".into(),
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
                pane: PaneOptions::default(),
                grid: GridOptions::default(),
                add_default_pane: true,
            },
        );
        let mut demo = Self {
            layer: DataLayer::new(HorzScaleBehaviorTime::default()),
            model,
            controller: TimeScaleAnimationController::default(),
            view: LinePaneView::default(),
            snapshot: PlotSnapshot::default(),
            size: Size::ZERO,
            scale: 1.,
            count: 0,
            registered: false,
            error: None,
            kind: LineType::Simple,
            style: LineStyle::Solid,
            markers: false,
        };
        demo.load();
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
    }
    fn prepare(&mut self, now: Instant) {
        let layout = FrameLayout {
            pane: 0,
            size: self.size,
            pixel_ratio: PixelRatio {
                horizontal: self.scale,
                vertical: self.scale,
            },
            background: "#101820".into(),
        };
        match prepare_frame(
            &mut self.model,
            &mut self.controller,
            &mut self.view,
            self.registered.then_some(ID),
            &layout,
            now,
        ) {
            Ok((snapshot, _retained_mask)) => {
                self.snapshot = snapshot;
                self.error = None;
            }
            Err(error) => {
                self.snapshot = PlotSnapshot {
                    size: self.size,
                    ..PlotSnapshot::default()
                };
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
            button("Stop").on_press(Message::Stop)
        ]
        .spacing(5);
        let plot = Canvas::new(Plot {
            snapshot: &self.snapshot,
            animate: self.controller.is_active(),
        })
        .width(iced::Fill)
        .height(iced::Fill);
        container(
            column![
                controls,
                styles,
                text(self.error.clone().unwrap_or_else(|| format!(
                    "{} bars · plot-only · resize the window",
                    self.count
                ))),
                plot
            ]
            .spacing(8),
        )
        .padding(12)
        .into()
    }
}
struct Plot<'a> {
    snapshot: &'a PlotSnapshot,
    animate: bool,
}
impl canvas::Program<Message> for Plot<'_> {
    type State = Option<Instant>;
    fn update(
        &self,
        state: &mut Option<Instant>,
        event: &canvas::Event,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if let canvas::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            if bounds.size() != self.snapshot.size {
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
    assert!(!demo.snapshot.lines.is_empty());
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
    assert_eq!(demo.snapshot.size, Size::new(500., 300.));
    let _ = demo.update(Message::Animate);
    assert!(demo.controller.is_active());
    let _ = demo.update(Message::Stop);
    assert!(!demo.controller.is_active());
    let _ = demo.update(Message::Remove);
    assert!(demo.snapshot.lines.is_empty());
    let _ = demo.update(Message::Reload);
    assert!(!demo.snapshot.lines.is_empty());
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
        .title("Feather Charts — line rendering")
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
