use super::*;
use crate::model::{
    axis_snapshots::PriceAxisSide,
    price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
};

#[test]
fn axis_scaling_uses_source_snapshot_coefficient_and_preserves_time() {
    let (_, mut model, _) = fixture();
    let before = model.panes()[0].right_price_scale().price_range().unwrap();
    let time = (
        model.time_scale().bar_spacing(),
        model.time_scale().right_offset(),
    );
    model
        .start_scale_price(0, PriceAxisSide::Right, 100.)
        .unwrap();
    model.scale_price_to(0, PriceAxisSide::Right, 80.).unwrap();
    let mut expected = before;
    expected.scale_around_center((100. + 199. * 0.2) / (120. + 199. * 0.2));
    assert_eq!(
        model.panes()[0].right_price_scale().price_range(),
        Some(expected)
    );
    model.scale_price_to(0, PriceAxisSide::Right, 60.).unwrap();
    let mut expected = before;
    expected.scale_around_center((100. + 199. * 0.2) / (140. + 199. * 0.2));
    assert_eq!(
        model.panes()[0].right_price_scale().price_range(),
        Some(expected)
    );
    assert!(!model.panes()[0].right_price_scale().is_auto_scale());
    model.end_scale_price(0, PriceAxisSide::Right).unwrap();
    model.scale_price_to(0, PriceAxisSide::Right, 150.).unwrap();
    assert_eq!(
        model.panes()[0].right_price_scale().price_range(),
        Some(expected)
    );
    assert_eq!(
        time,
        (
            model.time_scale().bar_spacing(),
            model.time_scale().right_offset()
        )
    );
    model.reset_price_scale(0, PriceAxisSide::Right).unwrap();
    assert!(model.panes()[0].right_price_scale().is_auto_scale());
    assert_eq!(
        model.panes()[0].right_price_scale().price_range(),
        Some(before)
    );
}
#[test]
fn axis_wheel_is_reversible_and_does_not_interrupt_an_existing_drag() {
    let (_, mut model, _) = fixture();
    let before = model.panes()[0].right_price_scale().price_range().unwrap();
    model
        .zoom_price_scale(0, PriceAxisSide::Right, 0.32)
        .unwrap();
    let after = model.panes()[0].right_price_scale().price_range().unwrap();
    assert!((after.length() / before.length() - (-0.064_f64).exp()).abs() < 1e-12);
    model
        .zoom_price_scale(0, PriceAxisSide::Right, -0.32)
        .unwrap();
    let restored = model.panes()[0].right_price_scale().price_range().unwrap();
    assert!((restored.length() - before.length()).abs() < 1e-10);
    model
        .start_scale_price(0, PriceAxisSide::Right, 100.)
        .unwrap();
    model.zoom_price_scale(0, PriceAxisSide::Right, 1.).unwrap();
    assert_eq!(
        model.panes()[0].right_price_scale().price_range(),
        Some(restored)
    );
    model.scale_price_to(0, PriceAxisSide::Right, 80.).unwrap();
    assert_ne!(
        model.panes()[0].right_price_scale().price_range(),
        Some(restored)
    );
}
#[test]
fn unsupported_modes_and_invalid_price_input_leave_range_unchanged() {
    for mode in [PriceScaleMode::Percentage, PriceScaleMode::IndexedTo100] {
        let (_, mut model, _) = fixture();
        model
            .apply_price_axis_options(
                0,
                PriceAxisSide::Right,
                PriceScaleOptionsPatch {
                    mode: Some(mode),
                    ..Default::default()
                },
            )
            .unwrap();
        let before = model.panes()[0].right_price_scale().price_range();
        model
            .start_scale_price(0, PriceAxisSide::Right, 100.)
            .unwrap();
        model.scale_price_to(0, PriceAxisSide::Right, 80.).unwrap();
        model.zoom_price_scale(0, PriceAxisSide::Right, 1.).unwrap();
        assert_eq!(model.panes()[0].right_price_scale().price_range(), before);
    }
    let (_, mut model, _) = fixture();
    let before = model.panes()[0].right_price_scale().price_range();
    for value in [f64::NAN, f64::INFINITY] {
        assert_eq!(
            model.start_scale_price(0, PriceAxisSide::Right, value),
            Err(ChartModelError::InvalidPriceInteraction)
        );
        assert_eq!(
            model.scale_price_to(0, PriceAxisSide::Right, value),
            Err(ChartModelError::InvalidPriceInteraction)
        );
        assert_eq!(
            model.zoom_price_scale(0, PriceAxisSide::Right, value),
            Err(ChartModelError::InvalidPriceInteraction)
        );
    }
    assert_eq!(
        model.reset_price_scale(99, PriceAxisSide::Right),
        Err(ChartModelError::InvalidPane(99))
    );
    assert_eq!(model.panes()[0].right_price_scale().price_range(), before);
}

#[test]
fn manual_price_pan_tracks_pixels_preserves_span_and_stops_after_end() {
    for mode in [PriceScaleMode::Normal, PriceScaleMode::Logarithmic] {
        for inverted in [false, true] {
            let (_, mut model, _) = fixture();
            model
                .apply_price_axis_options(
                    0,
                    PriceAxisSide::Right,
                    PriceScaleOptionsPatch {
                        mode: Some(mode),
                        invert_scale: Some(inverted),
                        ..Default::default()
                    },
                )
                .unwrap();
            let before = model.panes()[0].right_price_scale().price_range().unwrap();
            let y = model.panes()[0]
                .right_price_scale()
                .price_to_coordinate(15., 1.)
                .value();
            let time = (
                model.time_scale().bar_spacing(),
                model.time_scale().right_offset(),
            );
            model
                .start_scroll_price(0, PriceAxisSide::Right, 100.)
                .unwrap();
            model
                .scroll_price_to(0, PriceAxisSide::Right, 125.)
                .unwrap();
            let after = model.panes()[0].right_price_scale().price_range().unwrap();
            assert!((after.length() - before.length()).abs() < 1e-10);
            assert!(
                (model.panes()[0]
                    .right_price_scale()
                    .price_to_coordinate(15., 1.)
                    .value()
                    - y
                    - 25.)
                    .abs()
                    < 1e-8
            );
            model.scroll_price_to(0, PriceAxisSide::Right, 90.).unwrap();
            assert!(
                (model.panes()[0]
                    .right_price_scale()
                    .price_to_coordinate(15., 1.)
                    .value()
                    - y
                    + 10.)
                    .abs()
                    < 1e-8
            );
            assert!(!model.panes()[0].right_price_scale().is_auto_scale());
            let stopped = model.panes()[0].right_price_scale().price_range();
            model.end_scroll_price(0, PriceAxisSide::Right).unwrap();
            model.end_scroll_price(0, PriceAxisSide::Right).unwrap();
            model
                .scroll_price_to(0, PriceAxisSide::Right, 170.)
                .unwrap();
            assert_eq!(model.panes()[0].right_price_scale().price_range(), stopped);
            assert_eq!(
                time,
                (
                    model.time_scale().bar_spacing(),
                    model.time_scale().right_offset()
                )
            );
            model.reset_price_scale(0, PriceAxisSide::Right).unwrap();
            assert!(model.panes()[0].right_price_scale().is_auto_scale());
            assert_eq!(
                model.panes()[0].right_price_scale().price_range(),
                Some(before)
            );
        }
    }
}
#[test]
fn rejected_price_pan_does_not_modify_a_scale_or_disable_autoscale() {
    let (_, mut model, _) = fixture();
    let before = model.panes()[0].right_price_scale().price_range();
    for value in [f64::NAN, f64::INFINITY] {
        assert_eq!(
            model.start_scroll_price(0, PriceAxisSide::Right, value),
            Err(ChartModelError::InvalidPriceInteraction)
        );
        assert_eq!(
            model.scroll_price_to(0, PriceAxisSide::Right, value),
            Err(ChartModelError::InvalidPriceInteraction)
        );
    }
    assert_eq!(
        model.start_scroll_price(99, PriceAxisSide::Right, 10.),
        Err(ChartModelError::InvalidPane(99))
    );
    assert_eq!(model.panes()[0].right_price_scale().price_range(), before);
    assert!(model.panes()[0].right_price_scale().is_auto_scale());
    model
        .start_scroll_price(0, PriceAxisSide::Left, 10.)
        .unwrap();
    assert!(model.panes()[0].left_price_scale().is_auto_scale());
    for mode in [PriceScaleMode::Percentage, PriceScaleMode::IndexedTo100] {
        model
            .apply_price_axis_options(
                0,
                PriceAxisSide::Right,
                PriceScaleOptionsPatch {
                    mode: Some(mode),
                    ..Default::default()
                },
            )
            .unwrap();
        let before = model.panes()[0].right_price_scale().price_range();
        model
            .start_scroll_price(0, PriceAxisSide::Right, 10.)
            .unwrap();
        model
            .scroll_price_to(0, PriceAxisSide::Right, 100.)
            .unwrap();
        assert_eq!(model.panes()[0].right_price_scale().price_range(), before);
        assert!(model.panes()[0].right_price_scale().is_auto_scale());
    }
}
