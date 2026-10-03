use super::*;
#[path = "../../../../examples/support/display_scenario.rs"]
mod scenarios;
use crate::{
    model::{
        axis_snapshots::{AxisSnapshots, PriceAxisSide},
        price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
        text_width_cache::{TextMeasurer, TextMetrics},
        time_scale_options::HorzScaleOptionsPatch,
    },
    renderers::{
        grid_renderer::PixelRatio,
        line_renderer::PathOperation,
        price_axis_renderer::{AxisBounds, PriceAxisCommand, measure_price_axis},
        time_axis_renderer::{TimeAxisCommand, measure_time_axis},
    },
    ui::chart_frame::{
        AxisMeasurements, AxisMeasurer, ChartFrameError, ChartFrameInput, ChartFrameOwner,
        ChartFrameSnapshot,
    },
};
use scenarios::{DisplayConfig, DisplayScenario};

struct Metrics(f32);
impl TextMeasurer for Metrics {
    fn measure_text(&mut self, text: &str) -> TextMetrics {
        TextMetrics {
            width: text.chars().count() as f32 * self.0 * 0.6,
            ..Default::default()
        }
    }
}
struct Measure;
impl AxisMeasurer for Measure {
    fn measure(&mut self, s: &AxisSnapshots) -> Result<AxisMeasurements, ChartFrameError> {
        Ok(AxisMeasurements {
            left: measure_price_axis(&s.left, &mut Metrics(s.left.renderer_options.font_size))
                .map_err(ChartFrameError::Price)?,
            right: measure_price_axis(&s.right, &mut Metrics(s.right.renderer_options.font_size))
                .map_err(ChartFrameError::Price)?,
            time: measure_time_axis(
                &s.time,
                &mut Metrics(s.time.renderer_options.font_size),
                &mut Metrics(s.time.renderer_options.font_size * 1.1),
            )
            .map_err(ChartFrameError::Time)?,
        })
    }
}
struct Display {
    layer: Layer,
    model: Model,
    owner: ChartFrameOwner,
    controller: TimeScaleAnimationController,
    view: crate::model::series::line_pane_view::LinePaneView,
    config: DisplayConfig,
    id: Option<SeriesId>,
    ratio: f32,
    now: Instant,
}
impl Display {
    fn new(config: DisplayConfig, ratio: f32) -> Self {
        let mut settings = layout();
        if config.gradient {
            settings.background = Background::VerticalGradient {
                top_color: "#101820".into(),
                bottom_color: "#36536a".into(),
            };
        }
        let mut model = ChartModel::new(
            TimeScale::new(
                HorzScaleBehaviorTime::default(),
                HorzScaleOptions::default(),
                LocalizationOptions::new("en-US", "dd MMM 'yy"),
            ),
            ChartModelOptions {
                layout: settings,
                pane: PaneOptions::default(),
                grid: GridOptions::default(),
                add_default_pane: true,
            },
        );
        model
            .set_layout_typography(config.font_size, &config.font_family)
            .unwrap();
        model
            .apply_price_axis_options(
                0,
                PriceAxisSide::Left,
                PriceScaleOptionsPatch {
                    visible: Some(config.left_axis),
                    ticks_visible: Some(config.ticks),
                    ..Default::default()
                },
            )
            .unwrap();
        model
            .apply_price_axis_options(
                0,
                PriceAxisSide::Right,
                PriceScaleOptionsPatch {
                    ticks_visible: Some(config.ticks),
                    ..Default::default()
                },
            )
            .unwrap();
        let mut s = Self {
            layer: Layer::new(HorzScaleBehaviorTime::default()),
            model,
            owner: ChartFrameOwner::default(),
            controller: TimeScaleAnimationController::default(),
            view: Default::default(),
            config,
            id: None,
            ratio,
            now: Instant::now(),
        };
        s.model.apply_time_scale_options(
            &mut s.layer,
            HorzScaleOptionsPatch {
                time_visible: Some(true),
                ticks_visible: Some(s.config.ticks),
                fix_left_edge: Some(s.config.fixed_edges),
                fix_right_edge: Some(s.config.fixed_edges),
                ..Default::default()
            },
        );
        s.reload();
        s
    }
    fn reload(&mut self) {
        let id = SeriesId::new(1);
        if self.id.is_none() {
            self.model
                .register_series(
                    id,
                    SeriesType::Line,
                    line_options(),
                    0,
                    PriceScalePosition::Right,
                )
                .unwrap();
            self.id = Some(id);
        }
        self.set_data(
            (0..120)
                .map(|i| line(1_700_000_000. + i as f64 * 60., self.config.value(i, 0.)))
                .collect(),
        );
        if self.config.left_axis && self.model.series(SeriesId::new(2)).is_none() {
            self.model
                .register_series(
                    SeriesId::new(2),
                    SeriesType::Line,
                    line_options(),
                    0,
                    PriceScalePosition::Left,
                )
                .unwrap();
            let update = self
                .layer
                .set_series_data(
                    SeriesId::new(2),
                    SeriesType::Line,
                    (0..120)
                        .map(|i| line(1_700_000_000. + i as f64 * 60., 2000. + i as f64 * 2.))
                        .collect(),
                )
                .unwrap();
            self.model.apply_data_update(update).unwrap();
        }
        self.model.fit_content();
    }
    fn set_data(&mut self, rows: Vec<BuiltInSeriesDataItem<Time>>) {
        let update = self
            .layer
            .set_series_data(self.id.unwrap(), SeriesType::Line, rows)
            .unwrap();
        self.model.apply_data_update(update).unwrap();
    }
    fn update(&mut self, index: usize, adjustment: f64, historical: bool) {
        let update = self
            .layer
            .update_series_data(
                self.id.unwrap(),
                line(
                    1_700_000_000. + index as f64 * 60.,
                    self.config.value(index, adjustment),
                ),
                historical,
            )
            .unwrap();
        self.model.apply_data_update(update).unwrap();
    }
    fn frame(&mut self) -> ChartFrameSnapshot {
        let result = self
            .owner
            .prepare(
                &mut self.model,
                &mut self.controller,
                &mut self.view,
                &ChartFrameInput {
                    series: self.id,
                    bounds: AxisBounds {
                        width: f64::from(self.config.width),
                        height: f64::from(self.config.height),
                    },
                    pixel_ratio: PixelRatio {
                        horizontal: self.ratio,
                        vertical: self.ratio,
                    },
                    background: "#fff".into(),
                },
                &mut Measure,
                self.now,
            )
            .unwrap_or_else(|e| {
                panic!(
                    "{e:?}, scenario {:?}, size {}x{}, ratio {}, mode {:?}",
                    self.config.scenario,
                    self.config.width,
                    self.config.height,
                    self.ratio,
                    self.model.panes()[0]
                        .price_scale_by_id("right")
                        .unwrap()
                        .options()
                        .mode
                )
            });
        assert_frame(&mut self.model, &self.view, self.id, self.ratio, &result);
        result
    }
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 0.0002, "{a} != {b}");
}
fn assert_frame(
    model: &mut Model,
    view: &crate::model::series::line_pane_view::LinePaneView,
    id: Option<SeriesId>,
    ratio: f32,
    s: &ChartFrameSnapshot,
) {
    let l = s.layout;
    assert_eq!(model.time_scale().width(), l.plot.size.width);
    assert_eq!(model.panes()[0].height(), l.plot.size.height);
    assert_eq!(f64::from(s.plot.size.width), l.plot.size.width);
    assert_eq!(f64::from(s.plot.size.height), l.plot.size.height);
    assert_eq!(
        l.left_axis.origin.x + l.left_axis.size.width,
        l.plot.origin.x
    );
    assert_eq!(l.plot.origin.x + l.plot.size.width, l.right_axis.origin.x);
    assert_eq!(l.time_axis.origin.x, l.plot.origin.x);
    assert_eq!(l.time_axis.size.width, l.plot.size.width);
    assert_eq!(l.time_axis.origin.y, l.plot.size.height);
    for r in [
        l.plot,
        l.left_axis,
        l.right_axis,
        l.time_axis,
        l.left_corner,
        l.right_corner,
    ] {
        assert!(
            [r.origin.x, r.origin.y, r.size.width, r.size.height]
                .iter()
                .all(|v| v.is_finite() && *v >= 0.)
        );
    }
    let axes = s.plot.axes.as_ref().unwrap();
    let grid = model.panes()[0]
        .grid()
        .pane_view()
        .renderer()
        .data()
        .unwrap();
    // The grid uses the pane's selected default scale, not necessarily the right.
    assert!(grid.price_marks == axes.left.marks || grid.price_marks == axes.right.marks);
    assert_eq!(
        grid.time_marks
            .iter()
            .map(|m| m.coordinate.value())
            .collect::<Vec<_>>(),
        axes.time
            .ticks
            .iter()
            .map(|t| t.mark.coordinate)
            .collect::<Vec<_>>()
    );
    let r = f64::from(ratio);
    for mark in &grid.time_marks {
        let expected = f64::from((mark.coordinate.value() as f32 * ratio).round() / ratio);
        assert!(
            s.plot
                .grid
                .iter()
                .any(|stroke| matches!(stroke.path.as_slice(),
            [PathOperation::MoveTo(a), PathOperation::LineTo(b)]
            if (a.x-expected).abs()<0.0002 && a.x==b.x))
        );
    }
    for mark in &grid.price_marks {
        let expected = f64::from((mark.coord.value() as f32 * ratio).round() / ratio);
        assert!(
            s.plot
                .grid
                .iter()
                .any(|stroke| matches!(stroke.path.as_slice(),
            [PathOperation::MoveTo(a), PathOperation::LineTo(b)]
            if (a.y-expected).abs()<0.0002 && a.y==b.y))
        );
    }
    for (axis, prepared) in [(&axes.left, &s.left), (&axes.right, &s.right)] {
        let texts = prepared
            .commands
            .iter()
            .filter_map(|c| {
                if let PriceAxisCommand::Text {
                    label, position, ..
                } = c
                {
                    Some((label, position))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if axis.visible && prepared.bounds.width > 0. && prepared.bounds.height > 0. {
            assert_eq!(texts.len(), axis.marks.len());
            for ((label, p), mark) in texts.into_iter().zip(axis.marks.iter().rev()) {
                assert_eq!(label, &mark.label);
                close(p.y, mark.coord.value());
            }
            if axis.border_visible && axis.ticks_visible {
                let ticks = prepared
                    .commands
                    .iter()
                    .filter_map(|c| {
                        if let PriceAxisCommand::Rectangle { rect, .. } = c {
                            Some(rect)
                        } else {
                            None
                        }
                    })
                    .skip(1);
                for (rect, mark) in ticks.zip(&axis.marks) {
                    close(
                        rect.origin.y + (r * 0.5).floor() / r,
                        (mark.coord.value() * r + 0.5).floor() / r,
                    );
                }
            }
        } else {
            assert!(texts.is_empty());
        }
    }
    let ticks = s
        .time
        .commands
        .iter()
        .filter_map(|c| {
            if let TimeAxisCommand::Rectangle { rect, .. } = c {
                Some(rect)
            } else {
                None
            }
        })
        .skip(usize::from(axes.time.border_visible));
    if axes.time.visible && axes.time.border_visible && axes.time.ticks_visible {
        for (rect, tick) in ticks.zip(axes.time.ticks.iter().rev()) {
            close(
                rect.origin.x + (r * 0.5).floor() / r,
                (tick.mark.coordinate * r + 0.5).floor() / r,
            );
        }
    }
    for stroke in s.plot.grid.iter().chain(&s.plot.lines) {
        assert!(stroke.width.is_finite());
        for op in &stroke.path {
            let points = match op {
                PathOperation::MoveTo(p) | PathOperation::LineTo(p) => vec![*p],
                PathOperation::CubicTo {
                    control1,
                    control2,
                    end,
                } => vec![*control1, *control2, *end],
            };
            assert!(points.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
        }
    }
    if let (Some(data), Some(id)) = (view.data(), id) {
        let visible = model.visible_strict_range().unwrap();
        let series = model.series(id).unwrap();
        let first = series.first_value(Some(&visible)).unwrap().value;
        let scale = model.panes()[0].price_scale_by_id("right").unwrap();
        if matches!(
            scale.options().mode,
            PriceScaleMode::Normal | PriceScaleMode::Logarithmic
        ) {
            for mark in &axes.right.marks {
                close(
                    mark.coord.value(),
                    scale.price_to_coordinate(mark.logical, first).value(),
                );
            }
        }
        for (p, row) in data.items.iter().zip(series.bars().rows()) {
            close(p.x, model.time_scale().index_to_coordinate(p.index).value());
            let crate::model::series_data::SeriesPlotRow::Line(row) = row else {
                panic!("line row expected")
            };
            close(
                p.y,
                scale.price_to_coordinate(row.base.value[3], first).value(),
            );
        }
    }
}

#[test]
fn complete_display_targeted_matrix_and_owned_frames() {
    for (n, scenario) in DisplayScenario::ALL.into_iter().enumerate() {
        assert_eq!(DisplayScenario::parse(scenario.name()), Some(scenario));
        for ratio in [1., 1.25, 1.5, 2.] {
            let mut config = DisplayConfig::for_scenario(scenario);
            config.font_size = if n % 2 == 0 { 12. } else { 18. };
            config.font_family = if n % 2 == 0 {
                "sans-serif"
            } else {
                "monospace"
            }
            .into();
            let mut d = Display::new(config, ratio);
            for mode in [
                PriceScaleMode::Normal,
                PriceScaleMode::Percentage,
                PriceScaleMode::IndexedTo100,
                PriceScaleMode::Logarithmic,
            ] {
                d.model
                    .apply_price_axis_options(
                        0,
                        PriceAxisSide::Right,
                        PriceScaleOptionsPatch {
                            mode: Some(mode),
                            ..Default::default()
                        },
                    )
                    .unwrap();
                let old = d.frame();
                let before = format!("{old:?}");
                for (w, h) in [
                    (1200., 700.),
                    (260., 180.),
                    (800., 40.),
                    (30., 10.),
                    (0., 0.),
                    (800., 500.),
                ] {
                    d.config.width = w;
                    d.config.height = h;
                    d.frame();
                }
                assert_eq!(format!("{old:?}"), before);
                for (left, right, time, border, ticks) in [
                    (true, false, true, true, true),
                    (false, true, true, false, true),
                    (true, true, true, true, false),
                    (false, false, false, false, false),
                ] {
                    for (side, visible) in
                        [(PriceAxisSide::Left, left), (PriceAxisSide::Right, right)]
                    {
                        d.model
                            .apply_price_axis_options(
                                0,
                                side,
                                PriceScaleOptionsPatch {
                                    visible: Some(visible),
                                    border_visible: Some(border),
                                    ticks_visible: Some(ticks),
                                    ..Default::default()
                                },
                            )
                            .unwrap();
                    }
                    d.model.apply_time_scale_options(
                        &mut d.layer,
                        HorzScaleOptionsPatch {
                            visible: Some(time),
                            border_visible: Some(border),
                            ticks_visible: Some(ticks),
                            ..Default::default()
                        },
                    );
                    d.frame();
                }
            }
        }
    }
}

#[test]
fn complete_display_updates_clear_whitespace_remove_and_reload() {
    let mut d = Display::new(DisplayConfig::default(), 1.25);
    let initial = d.frame();
    d.config.width = 600.;
    d.config.height = 300.;
    d.frame();
    d.update(120, 0., false);
    d.frame();
    d.update(120, 3., false);
    d.frame();
    d.model.set_logical_range(LogicalRange {
        from: Logical::new(10.),
        to: Logical::new(50.),
    });
    d.frame();
    let offset = d.model.time_scale().right_offset();
    d.update(121, 0., false);
    d.frame();
    assert_eq!(d.model.time_scale().right_offset(), offset - 1.);
    d.update(25, 30., true);
    d.frame();
    d.set_data(vec![]);
    let empty = d.frame();
    assert!(empty.plot.lines.is_empty());
    assert!(empty.plot.axes.as_ref().unwrap().time.ticks.is_empty());
    d.set_data(
        (0..20)
            .map(|i| whitespace(1_700_000_000. + i as f64 * 60.))
            .collect(),
    );
    d.model.fit_content();
    let blank = d.frame();
    assert!(blank.plot.lines.is_empty());
    assert!(blank.plot.axes.as_ref().unwrap().right.marks.is_empty());
    d.reload();
    assert!(!d.frame().plot.lines.is_empty());
    let id = d.id.take().unwrap();
    let response = d.layer.remove_series(id).unwrap();
    d.model.remove_series(id, response).unwrap();
    assert!(d.frame().plot.lines.is_empty());
    d.reload();
    assert!(!d.frame().plot.lines.is_empty());
    assert!(!initial.plot.lines.is_empty());
}

#[test]
fn complete_display_magnitude_typography_visibility_and_new_bar_policies() {
    for shifting in [false, true] {
        let config = DisplayConfig {
            multiplier: 0.001,
            ..Default::default()
        };
        let mut d = Display::new(config, 1.5);
        let small = d.frame();
        d.config.multiplier = 10000.;
        d.reload();
        let big = d.frame();
        assert!(big.requests.right_width > small.requests.right_width);
        d.model.set_layout_typography(24., "monospace").unwrap();
        let font = d.frame();
        assert!(font.requests.right_width > big.requests.right_width);
        assert!(font.requests.time_height > big.requests.time_height);
        for side in [PriceAxisSide::Left, PriceAxisSide::Right] {
            d.model
                .apply_price_axis_options(
                    0,
                    side,
                    PriceScaleOptionsPatch {
                        visible: Some(false),
                        ..Default::default()
                    },
                )
                .unwrap();
        }
        d.model.apply_time_scale_options(
            &mut d.layer,
            HorzScaleOptionsPatch {
                visible: Some(false),
                shift_visible_range_on_new_bar: Some(shifting),
                ..Default::default()
            },
        );
        let hidden = d.frame();
        assert_eq!(hidden.layout.plot.size.width, 800.);
        assert_eq!(hidden.layout.plot.size.height, 500.);
        d.model.set_right_offset(0.);
        d.frame();
        let offset = d.model.time_scale().right_offset();
        d.update(120, 0., false);
        d.frame();
        assert_eq!(
            d.model.time_scale().right_offset(),
            if shifting { offset } else { offset - 1. }
        );
    }
}

#[test]
fn completed_animation_does_not_return_after_resize_or_unrelated_redraw() {
    let mut d = Display::new(DisplayConfig::default(), 2.);
    d.frame();
    d.model
        .scroll_to_offset_animated(-20., Duration::from_secs(1));
    d.frame();
    assert!(d.controller.is_active());
    d.now += Duration::from_millis(500);
    d.config.width = 650.;
    d.update(120, 0., false);
    d.frame();
    assert!(d.controller.is_active());
    d.now += Duration::from_millis(500);
    d.frame();
    assert!(!d.controller.is_active());
    let offset = d.model.time_scale().right_offset();
    d.model.light_update();
    d.now += Duration::from_secs(1);
    d.frame();
    assert!(!d.controller.is_active());
    assert_eq!(d.model.time_scale().right_offset(), offset);
    d.config.width = 750.;
    d.frame();
    assert!(!d.controller.is_active());
}
