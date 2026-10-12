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
