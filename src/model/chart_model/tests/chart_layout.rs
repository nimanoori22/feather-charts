use super::*;
use crate::{
    model::{
        axis_snapshots::{AxisSnapshots, PriceAxisSide},
        invalidate_mask::{AnimationRequest, TimeScaleAnimation, TimeScaleInvalidation},
        price_scale::PriceScaleOptionsPatch,
        series::line_pane_view::LinePaneView,
        text_width_cache::{TextMeasurer, TextMetrics},
        time_scale_options::HorzScaleOptionsPatch,
    },
    renderers::{
        grid_renderer::PixelRatio,
        price_axis_renderer::{AxisBounds, measure_price_axis},
        time_axis_renderer::{TimeAxisCommand, measure_time_axis},
    },
    ui::chart_frame::{
        AxisMeasurements, AxisMeasurer, ChartFrameError, ChartFrameInput, ChartFrameOwner,
        ChartFrameSnapshot, prepare_corner,
    },
};
use std::{cell::Cell, rc::Rc};

struct Metrics(f32);
impl TextMeasurer for Metrics {
    fn measure_text(&mut self, _: &str) -> TextMetrics {
        TextMetrics {
            width: self.0,
            ..Default::default()
        }
    }
}
struct Fake {
    widths: Vec<f32>,
    calls: usize,
}
impl Fake {
    fn new(widths: &[f32]) -> Self {
        Self {
            widths: widths.to_vec(),
            calls: 0,
        }
    }
}
impl AxisMeasurer for Fake {
    fn measure(&mut self, s: &AxisSnapshots) -> Result<AxisMeasurements, ChartFrameError> {
        let width = self.widths[self.calls.min(self.widths.len() - 1)];
        self.calls += 1;
        Ok(AxisMeasurements {
            left: measure_price_axis(&s.left, &mut Metrics(width))
                .map_err(ChartFrameError::Price)?,
            right: measure_price_axis(&s.right, &mut Metrics(width))
                .map_err(ChartFrameError::Price)?,
            time: measure_time_axis(&s.time, &mut Metrics(width), &mut Metrics(width + 1.))
                .map_err(ChartFrameError::Time)?,
        })
    }
}
fn input(id: Option<SeriesId>, width: f64, height: f64, r: f32) -> ChartFrameInput {
    ChartFrameInput {
        series: id,
        bounds: AxisBounds { width, height },
        pixel_ratio: PixelRatio {
            horizontal: r,
            vertical: r,
        },
        background: "#fff".into(),
    }
}
fn prepare(
    owner: &mut ChartFrameOwner,
    model: &mut Model,
    id: Option<SeriesId>,
    fake: &mut Fake,
) -> ChartFrameSnapshot {
    owner
        .prepare(
            model,
            &mut TimeScaleAnimationController::default(),
            &mut LinePaneView::default(),
            &input(id, 800., 500., 1.5),
            fake,
            Instant::now(),
        )
        .unwrap()
}
fn assert_aligned(s: &ChartFrameSnapshot, model: &Model) {
    assert_eq!(model.time_scale().width(), s.layout.plot.size.width);
    assert_eq!(model.panes()[0].height(), s.layout.plot.size.height);
    assert_eq!(f64::from(s.plot.size.width), s.layout.plot.size.width);
    assert_eq!(f64::from(s.plot.size.height), s.layout.plot.size.height);
    assert_eq!(s.left.bounds.height, s.layout.plot.size.height);
    assert_eq!(s.right.bounds.height, s.layout.plot.size.height);
    assert_eq!(s.time.bounds.width, s.layout.plot.size.width);
    let axes = s.plot.axes.as_ref().unwrap();
    let grid = model.panes()[0]
        .grid()
        .pane_view()
        .renderer()
        .data()
        .unwrap();
    assert_eq!(axes.right.marks, grid.price_marks);
    assert_eq!(
        axes.time
            .ticks
            .iter()
            .map(|t| t.mark.coordinate)
            .collect::<Vec<_>>(),
        grid.time_marks
            .iter()
            .map(|t| t.coordinate.value())
            .collect::<Vec<_>>()
    );
}

#[test]
fn fit_and_range_commands_replay_at_final_measured_width_without_draining_new_work() {
    for fit in [true, false] {
        let (_, mut model, id) = fixture();
        if fit {
            model.fit_content();
        } else {
            model.set_logical_range(LogicalRange {
                from: Logical::new(3.),
                to: Logical::new(10.),
            });
        }
        let mut owner = ChartFrameOwner::default();
        let s = prepare(
            &mut owner,
            &mut model,
            Some(id),
            &mut Fake::new(&[10., 100., 100.]),
        );
        assert_eq!(s.passes, 2);
        assert_aligned(&s, &model);
        assert!(
            (model.time_scale().bar_spacing()
                - s.layout.plot.size.width / if fit { 20. } else { 8. })
            .abs()
                < 1e-8
        );
        assert!(
            owner
                .retained_mask()
                .unwrap()
                .time_scale_invalidations()
                .iter()
                .any(|c| if fit {
                    matches!(c, TimeScaleInvalidation::FitContent)
                } else {
                    matches!(c, TimeScaleInvalidation::ApplyRange(_))
                })
        );
        let pending = model
            .take_invalidation()
            .expect("layout/recalculation work stays pending");
        assert!(pending.time_scale_invalidations().is_empty());
    }
}

#[test]
fn alternating_measurements_settle_without_shrinking_inside_a_frame() {
    let (_, mut model, id) = fixture();
    model.fit_content();
    let mut owner = ChartFrameOwner::with_pass_limit(2).unwrap();
    let mut metrics = Fake::new(&[10., 100., 10.]);
    let snapshot = prepare(&mut owner, &mut model, Some(id), &mut metrics);
    assert_eq!(snapshot.passes, 2);
    assert_eq!(metrics.calls, 3);
    assert!(snapshot.requests.right_width < snapshot.layout.right_axis.size.width);
    assert_aligned(&snapshot, &model);
    assert!(
        (model.time_scale().bar_spacing() - snapshot.layout.plot.size.width / 20.).abs() < 1e-8
    );
}

#[test]
fn light_frames_grow_but_full_frames_can_shrink_and_hidden_axes_release_space() {
    let (mut layer, mut model, id) = fixture();
    let mut owner = ChartFrameOwner::default();
    let large = prepare(&mut owner, &mut model, Some(id), &mut Fake::new(&[100.]));
    model.light_update();
    let retained = prepare(&mut owner, &mut model, Some(id), &mut Fake::new(&[10.]));
    assert_eq!(
        retained.layout.right_axis.size.width,
        large.layout.right_axis.size.width
    );
    assert!(retained.requests.right_width < retained.layout.right_axis.size.width);
    model.full_update();
    let small = prepare(&mut owner, &mut model, Some(id), &mut Fake::new(&[10.]));
    assert!(small.layout.right_axis.size.width < large.layout.right_axis.size.width);
    for side in [PriceAxisSide::Left, PriceAxisSide::Right] {
        model
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
    model.apply_time_scale_options(
        &mut layer,
        HorzScaleOptionsPatch {
            visible: Some(false),
            ..Default::default()
        },
    );
    let hidden = prepare(&mut owner, &mut model, Some(id), &mut Fake::new(&[10.]));
    assert_eq!(
        hidden.layout.plot.size,
        AxisBounds {
            width: 800.,
            height: 500.
        }
    );
    assert!(
        hidden.left.commands.is_empty()
            && hidden.right.commands.is_empty()
            && hidden.time.commands.is_empty()
    );
}

#[test]
fn replay_preserves_command_order_and_does_not_accumulate_duplicate_spacing_commands() {
    let (_, mut model, id) = fixture();
    model.set_bar_spacing(5.);
    model.fit_content();
    model.set_right_offset(-3.);
    let mut owner = ChartFrameOwner::default();
    let s = prepare(
        &mut owner,
        &mut model,
        Some(id),
        &mut Fake::new(&[10., 100., 100.]),
    );
    assert_aligned(&s, &model);
    assert_eq!(model.time_scale().right_offset(), -3.);
    assert!(matches!(
        owner.retained_mask().unwrap().time_scale_invalidations(),
        [
            TimeScaleInvalidation::FitContent,
            TimeScaleInvalidation::StopAnimation,
            TimeScaleInvalidation::ApplyRightOffset(-3.)
        ]
    ));
    assert!(
        model
            .take_invalidation()
            .unwrap()
            .time_scale_invalidations()
            .is_empty()
    );
}

#[derive(Debug)]
struct CountingAnimation {
    calls: Rc<Cell<usize>>,
    start: Instant,
}
impl TimeScaleAnimation for CountingAnimation {
    fn sample(&self, now: Instant) -> (f64, bool) {
        self.calls.set(self.calls.get() + 1);
        let p = now.saturating_duration_since(self.start).as_secs_f64();
        (-10. * p.min(1.), p >= 1.)
    }
}
#[test]
fn animation_samples_once_per_frame_survives_resize_and_navigation_cancels_continuation() {
    let (_, mut model, id) = fixture();
    let start = Instant::now();
    let calls = Rc::new(Cell::new(0));
    model.set_time_scale_animation(AnimationRequest::new(CountingAnimation {
        calls: calls.clone(),
        start,
    }));
    let mut owner = ChartFrameOwner::default();
    let mut controller = TimeScaleAnimationController::default();
    let mut view = LinePaneView::default();
    let first = owner
        .prepare(
            &mut model,
            &mut controller,
            &mut view,
            &input(Some(id), 800., 500., 1.25),
            &mut Fake::new(&[10., 100., 100.]),
            start + Duration::from_millis(500),
        )
        .unwrap();
    assert_eq!(first.passes, 2);
    assert_eq!(calls.get(), 1);
    assert_eq!(model.time_scale().right_offset(), -5.);
    assert!(controller.is_active());
    assert_aligned(&first, &model);
    let second = owner
        .prepare(
            &mut model,
            &mut controller,
            &mut view,
            &input(Some(id), 500., 300., 1.25),
            &mut Fake::new(&[100.]),
            start + Duration::from_millis(750),
        )
        .unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(model.time_scale().right_offset(), -7.5);
    assert_aligned(&second, &model);
    model.set_right_offset(-2.);
    owner
        .prepare(
            &mut model,
            &mut controller,
            &mut view,
            &input(Some(id), 500., 300., 1.25),
            &mut Fake::new(&[100.]),
            start + Duration::from_millis(900),
        )
        .unwrap();
    assert_eq!(calls.get(), 2);
    assert!(!controller.is_active());
    assert_eq!(model.time_scale().right_offset(), -2.);
    assert!(
        !model
            .take_invalidation()
            .unwrap()
            .time_scale_invalidations()
            .iter()
            .any(|c| matches!(c, TimeScaleInvalidation::Animation(_)))
    );
}

#[test]
fn pass_limit_is_explicit_and_does_not_publish_unsettled_geometry() {
    let (_, mut model, id) = fixture();
    model.fit_content();
    let mut owner = ChartFrameOwner::with_pass_limit(2).unwrap();
    let mut view = LinePaneView::default();
    let result = owner.prepare(
        &mut model,
        &mut TimeScaleAnimationController::default(),
        &mut view,
        &input(Some(id), 800., 500., 1.),
        &mut Fake::new(&[10., 20., 30.]),
        Instant::now(),
    );
    assert!(matches!(
        result,
        Err(ChartFrameError::LayoutDidNotConverge { passes: 2 })
    ));
    assert!(view.data().is_none());
    assert!(owner.retained_mask().is_some());
    assert!(model.take_invalidation().is_some());
    assert!(ChartFrameOwner::with_pass_limit(0).is_err());
}

#[test]
fn connected_dimensions_minima_history_empty_removal_and_reload() {
    let (mut layer, mut model, id) = fixture();
    let mut owner = ChartFrameOwner::default();
    model
        .apply_price_axis_options(
            0,
            PriceAxisSide::Left,
            PriceScaleOptionsPatch {
                visible: Some(true),
                minimum_width: Some(81.5),
                ..Default::default()
            },
        )
        .unwrap();
    model.apply_time_scale_options(
        &mut layer,
        HorzScaleOptionsPatch {
            minimum_height: Some(31.5),
            ..Default::default()
        },
    );
    let first = prepare(&mut owner, &mut model, Some(id), &mut Fake::new(&[10.]));
    assert_aligned(&first, &model);
    assert_eq!(first.layout.left_axis.size.width, 83.);
    assert_eq!(first.layout.time_axis.size.height, 33.);
    model.set_logical_range(LogicalRange {
        from: Logical::new(2.),
        to: Logical::new(10.),
    });
    prepare(&mut owner, &mut model, Some(id), &mut Fake::new(&[10.]));
    let before = model.visible_logical_range();
    model
        .apply_data_update(
            layer
                .update_series_data(id, line(21., 1000000.), false)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(model.visible_logical_range(), before);
    let appended = prepare(&mut owner, &mut model, Some(id), &mut Fake::new(&[30.]));
    assert_aligned(&appended, &model);
    model
        .apply_data_update(
            layer
                .update_series_data(id, line(8., -1000000.), true)
                .unwrap(),
        )
        .unwrap();
    let history = prepare(&mut owner, &mut model, Some(id), &mut Fake::new(&[30.]));
    assert!(!history.plot.lines.is_empty());
    model
        .remove_series(id, layer.remove_series(id).unwrap())
        .unwrap();
    let removed = prepare(&mut owner, &mut model, None, &mut Fake::new(&[30.]));
    assert!(removed.plot.lines.is_empty());
    assert!(removed.plot.axes.as_ref().unwrap().time.ticks.is_empty());
    model
        .register_series(
            id,
            SeriesType::Line,
            line_options(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    model
        .apply_data_update(
            layer
                .set_series_data(id, SeriesType::Line, vec![line(1., 10.), line(2., 20.)])
                .unwrap(),
        )
        .unwrap();
    model.fit_content();
    assert!(
        !prepare(&mut owner, &mut model, Some(id), &mut Fake::new(&[30.]))
            .plot
            .lines
            .is_empty()
    );
    let empty = owner
        .prepare(
            &mut model,
            &mut TimeScaleAnimationController::default(),
            &mut LinePaneView::default(),
            &input(Some(id), 0., 0., 1.),
            &mut Fake::new(&[30.]),
            Instant::now(),
        )
        .unwrap();
    assert!(empty.plot.lines.is_empty());
    assert_eq!(empty.layout.plot.size, AxisBounds::default());
    assert!(!first.plot.lines.is_empty()); // old owned frame survives mutations
}

#[test]
fn corner_background_border_getter_and_fractional_geometry_match_source() {
    let (_, mut model, _) = fixture();
    model
        .apply_price_axis_options(
            0,
            PriceAxisSide::Left,
            PriceScaleOptionsPatch {
                visible: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
    let mut axes = model.prepare_axis_snapshots(0).unwrap();
    axes.time.background = Background::VerticalGradient {
        top_color: "red".into(),
        bottom_color: "blue".into(),
    };
    for r in [1., 1.25, 1.5, 2.] {
        for side in [PriceAxisSide::Left, PriceAxisSide::Right] {
            let c = prepare_corner(
                &axes,
                side,
                AxisBounds {
                    width: 60.,
                    height: 28.,
                },
                PixelRatio {
                    horizontal: r,
                    vertical: r,
                },
            );
            assert_eq!(c.commands[0], TimeAxisCommand::Background("blue".into()));
            match &c.commands[1] {
                TimeAxisCommand::Rectangle { rect, color } => {
                    let size = f64::from(r).floor() / f64::from(r);
                    assert_eq!(
                        rect.size,
                        AxisBounds {
                            width: size,
                            height: size
                        }
                    );
                    assert_eq!(
                        rect.origin.x,
                        if side == PriceAxisSide::Left {
                            60. - size
                        } else {
                            0.
                        }
                    );
                    assert_eq!(color, &axes.time.border_color);
                }
                _ => panic!("expected corner border"),
            }
        }
    }
    axes.left.border_visible = false;
    assert_eq!(
        prepare_corner(
            &axes,
            PriceAxisSide::Right,
            AxisBounds {
                width: 60.,
                height: 28.
            },
            PixelRatio {
                horizontal: 1.,
                vertical: 1.
            }
        )
        .commands
        .len(),
        1
    );
}

#[test]
fn invalid_inputs_leave_pending_commands_and_dimensions_untouched() {
    let (_, mut model, id) = fixture();
    model.fit_content();
    let width = model.time_scale().width();
    let result = ChartFrameOwner::default().prepare(
        &mut model,
        &mut TimeScaleAnimationController::default(),
        &mut LinePaneView::default(),
        &input(Some(id), -1., 100., 1.),
        &mut Fake::new(&[10.]),
        Instant::now(),
    );
    assert!(matches!(result, Err(ChartFrameError::Layout(_))));
    assert_eq!(model.time_scale().width(), width);
    assert!(
        model
            .take_invalidation()
            .unwrap()
            .time_scale_invalidations()
            .iter()
            .any(|c| matches!(c, TimeScaleInvalidation::FitContent))
    );
}
