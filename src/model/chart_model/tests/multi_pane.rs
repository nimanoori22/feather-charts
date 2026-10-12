use super::histogram::{histogram, histogram_options};
use super::*;
use crate::{
    renderers::{grid_renderer::PixelRatio, price_axis_renderer::AxisBounds},
    ui::{
        chart_frame::IcedAxisMeasurer,
        multi_pane_frame::{MultiPaneChartFrameInput, MultiPaneChartFrameOwner, SeriesPaneView},
        price_axis::AxisFontResolver,
    },
};

#[test]
fn multi_pane_frames_share_time_and_keep_price_labels_local() {
    let (mut layer, mut model, line) = fixture();
    let volume = SeriesId::new(2);
    model.add_pane();
    model
        .register_series(
            volume,
            SeriesType::Histogram,
            histogram_options(),
            1,
            PriceScalePosition::Right,
        )
        .unwrap();
    model
        .apply_data_update(
            layer
                .set_series_data(
                    volume,
                    SeriesType::Histogram,
                    (0..20)
                        .map(|i| histogram(i as f64, 0., 1000. + i as f64 * 100.))
                        .collect(),
                )
                .unwrap(),
        )
        .unwrap();
    let mut views = vec![
        SeriesPaneView::Line(Default::default()),
        SeriesPaneView::Histogram(Default::default()),
    ];
    let mut owner = MultiPaneChartFrameOwner::default();
    let mut controller = TimeScaleAnimationController::default();
    let fonts = AxisFontResolver::default();
    let mut input = MultiPaneChartFrameInput {
        series: vec![Some(line), Some(volume)],
        bounds: AxisBounds {
            width: 800.,
            height: 500.,
        },
        pixel_ratio: PixelRatio {
            horizontal: 1.,
            vertical: 1.,
        },
        background: "white".into(),
    };
    let first = owner
        .prepare(
            &mut model,
            &mut controller,
            &mut views,
            &input,
            &mut IcedAxisMeasurer { fonts: &fonts },
            Instant::now(),
        )
        .unwrap();
    let retained = format!("{first:?}");
    model.fit_content();
    flush(&mut model);
    for r in [1., 1.25, 1.5, 2.] {
        input.pixel_ratio = PixelRatio {
            horizontal: r,
            vertical: r,
        };
        for active in 0..2 {
            model
                .set_crosshair_position(
                    active,
                    Some(crate::renderers::line_renderer::Point { x: 200., y: 50. }),
                )
                .unwrap();
            model.zoom_time(200.0.into(), 0.32);
            let frame = owner
                .prepare(
                    &mut model,
                    &mut controller,
                    &mut views,
                    &input,
                    &mut IcedAxisMeasurer { fonts: &fonts },
                    Instant::now(),
                )
                .unwrap();
            for (i, pane) in frame.panes.iter().enumerate() {
                assert_eq!(pane.layout.plot.size.width, model.time_scale().width());
                assert_eq!(pane.layout.plot.size.height, model.panes()[i].height());
                assert_eq!(pane.plot.crosshair.len(), if i == active { 2 } else { 1 });
                let axes = pane.plot.axes.as_ref().unwrap();
                assert_eq!(axes.right.crosshair_label.is_some(), i == active);
                assert!(axes.time.crosshair_label.is_some());
                assert_eq!(pane.plot.crosshair_markers.is_empty(), i == 1);
            }
            assert!(!frame.panes[0].plot.lines.is_empty());
            assert!(!frame.panes[1].plot.candles.is_empty());
        }
    }
    assert_eq!(retained, format!("{first:?}"));
    model
        .remove_series(volume, layer.remove_series(volume).unwrap())
        .unwrap();
    assert_eq!(model.panes().len(), 1);
    input.series.pop();
    views.pop();
    let frame = owner
        .prepare(
            &mut model,
            &mut controller,
            &mut views,
            &input,
            &mut IcedAxisMeasurer { fonts: &fonts },
            Instant::now(),
        )
        .unwrap();
    assert_eq!(frame.panes.len(), 1);
    assert!(frame.panes[0].plot.crosshair.is_empty());
}

#[test]
fn multi_pane_animation_samples_once_across_layout_passes_and_failure_clears_views() {
    use super::chart_layout::{CountingAnimation, Fake};
    use crate::model::invalidate_mask::AnimationRequest;
    use std::{cell::Cell, rc::Rc};
    let (mut layer, mut model, id) = fixture();
    let volume = SeriesId::new(2);
    model.add_pane();
    model
        .register_series(
            volume,
            SeriesType::Histogram,
            histogram_options(),
            1,
            PriceScalePosition::Right,
        )
        .unwrap();
    model
        .apply_data_update(
            layer
                .set_series_data(
                    volume,
                    SeriesType::Histogram,
                    (1..=20)
                        .map(|i| histogram(i as f64, 0., 1000. + i as f64))
                        .collect(),
                )
                .unwrap(),
        )
        .unwrap();
    let start = Instant::now();
    let calls = Rc::new(Cell::new(0));
    model.set_time_scale_animation(AnimationRequest::new(CountingAnimation {
        calls: calls.clone(),
        start,
    }));
    let mut controller = TimeScaleAnimationController::default();
    let mut views = vec![
        SeriesPaneView::Line(Default::default()),
        SeriesPaneView::Histogram(Default::default()),
    ];
    let mut owner = MultiPaneChartFrameOwner::default();
    let input = MultiPaneChartFrameInput {
        series: vec![Some(id), Some(volume)],
        bounds: AxisBounds {
            width: 800.,
            height: 500.,
        },
        pixel_ratio: PixelRatio {
            horizontal: 1.25,
            vertical: 1.25,
        },
        background: "white".into(),
    };
    let first = owner
        .prepare(
            &mut model,
            &mut controller,
            &mut views,
            &input,
            &mut Fake::new(&[10., 10., 100., 100., 100., 100.]),
            start + Duration::from_millis(500),
        )
        .unwrap();
    assert_eq!(first.panes[0].passes, 2);
    assert_eq!(calls.get(), 1);
    assert_eq!(model.time_scale().right_offset(), -5.);
    assert!(controller.is_active());
    assert!(model.take_invalidation().is_some());
    let mut invalid = histogram_options();
    if let SeriesOptionsMap::Histogram(options) = &mut invalid {
        options.style.color = "invalid-css-color".into();
    }
    model.replace_series_options(volume, invalid).unwrap();
    model.fit_content();
    assert!(
        owner
            .prepare(
                &mut model,
                &mut controller,
                &mut views,
                &input,
                &mut Fake::new(&[100.]),
                start + Duration::from_millis(750)
            )
            .is_err()
    );
    for view in &views {
        match view {
            SeriesPaneView::Line(v) => assert!(v.data().is_none()),
            SeriesPaneView::Histogram(v) => assert!(v.data().is_none()),
            _ => unreachable!(),
        }
    }
    model
        .replace_series_options(volume, histogram_options())
        .unwrap();
    let mut limited = MultiPaneChartFrameOwner::with_pass_limit(1).unwrap();
    assert!(matches!(
        limited.prepare(
            &mut model,
            &mut controller,
            &mut views,
            &input,
            &mut Fake::new(&[10., 10., 100., 100.]),
            Instant::now()
        ),
        Err(crate::ui::chart_frame::ChartFrameError::LayoutDidNotConverge { passes: 1 })
    ));
    assert!(MultiPaneChartFrameOwner::with_pass_limit(0).is_err());
}

#[test]
fn multi_pane_rejects_mismatched_membership_before_draining_invalidation() {
    let (_, mut model, id) = fixture();
    model.full_update();
    let mut owner = MultiPaneChartFrameOwner::default();
    let mut controller = TimeScaleAnimationController::default();
    let mut views = vec![SeriesPaneView::Line(Default::default())];
    let fonts = AxisFontResolver::default();
    let input = MultiPaneChartFrameInput {
        series: vec![Some(id), None],
        bounds: AxisBounds {
            width: 800.,
            height: 500.,
        },
        pixel_ratio: PixelRatio {
            horizontal: 1.,
            vertical: 1.,
        },
        background: "white".into(),
    };
    assert!(
        owner
            .prepare(
                &mut model,
                &mut controller,
                &mut views,
                &input,
                &mut IcedAxisMeasurer { fonts: &fonts },
                Instant::now()
            )
            .is_err()
    );
    assert!(owner.retained_mask().is_none());
    assert!(model.take_invalidation().is_some());
}
