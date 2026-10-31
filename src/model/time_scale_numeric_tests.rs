use super::*;
use crate::model::horz_scale_behavior_price::{HorizontalPrice, HorzScaleBehaviorPrice};
fn scale(values: &[f64]) -> TimeScale<HorzScaleBehaviorPrice> {
    let mut s = TimeScale::new(
        HorzScaleBehaviorPrice::default(),
        HorzScaleOptions::default(),
        LocalizationOptions::new("en-US", ""),
    );
    s.enable_numeric();
    s.update(TimeScaleUpdate {
        points: values
            .iter()
            .map(|&v| {
                let value = HorizontalPrice::new(v).unwrap();
                TimeScalePoint {
                    time: value,
                    original_time: value,
                    time_weight: Default::default(),
                }
            })
            .collect(),
        first_changed_point_index: 0,
    });
    s.set_base_index(
        values
            .len()
            .checked_sub(1)
            .map(|i| TimePointIndex::new(i as f64)),
    );
    s.set_width(1000.);
    s.fit_content();
    s
}
fn ratio(s: &TimeScale<HorzScaleBehaviorPrice>) -> f64 {
    let xs = [0., 1., 2.].map(|i| s.index_to_coordinate(TimePointIndex::new(i)).value());
    (xs[2] - xs[1]) / (xs[1] - xs[0])
}
#[test]
fn numeric_pan_zoom_resize_ticks_and_nearest_ties() {
    let mut s = scale(&[1., 2., 100.]);
    let fitted = s.numeric_range();
    assert!((ratio(&s) - 98.).abs() < 1e-9);
    let at = s.coordinate_to_numeric(350.).unwrap();
    s.zoom(350_f64.into(), 5.);
    assert!((s.coordinate_to_numeric(350.).unwrap() - at).abs() < 1e-10);
    let before = s.numeric_range().unwrap();
    s.start_scroll(100_f64.into());
    s.scroll_to(250_f64.into());
    s.end_scroll();
    let after = s.numeric_range().unwrap();
    assert!(after.0 < before.0);
    assert!((ratio(&s) - 98.).abs() < 1e-9);
    s.set_width(450.);
    assert_eq!(s.numeric_range(), Some(after));
    assert!((ratio(&s) - 98.).abs() < 1e-9);
    let px = s.numeric_to_coordinate(1.5).unwrap();
    assert_eq!(s.coordinate_to_index(px.into(), false).value(), 0.);
    let marks = s
        .marks(crate::model::time_scale_host::TimeScaleLayoutContext { font_size: 12. })
        .unwrap()
        .to_vec();
    for mark in marks {
        let x = mark.label.parse::<f64>().unwrap();
        assert!((s.numeric_to_coordinate(x).unwrap() - mark.coordinate).abs() < 1e-9);
    }
    s.restore_default();
    assert_eq!(s.numeric_range(), fitted);
}
#[test]
fn finite_fallbacks_negative_fractional_large_offset_and_tiny_span() {
    assert!(scale(&[]).is_empty());
    for values in [
        vec![0.],
        vec![1e100],
        vec![-1.5, -0.5, 97.5],
        vec![1e12, 1e12 + 1., 1e12 + 99.],
        vec![1e-100, 2e-100, 100e-100],
    ] {
        let s = scale(&values);
        let (from, to) = s.numeric_range().unwrap();
        assert!(from.is_finite() && to.is_finite() && to > from);
        if values.len() == 3 {
            assert!((ratio(&s) - 98.).abs() < 1e-7);
        }
        for x in values {
            let px = s.numeric_to_coordinate(x).unwrap();
            let back = s.coordinate_to_numeric(px).unwrap();
            assert!((back - x).abs() <= x.abs() * 1e-14 + 1e-110);
        }
    }
    let mut s = scale(&[1., 2.]);
    for (a, b) in [
        (0., 0.),
        (2., 1.),
        (-f64::MAX, f64::MAX),
        (0., f64::from_bits(1)),
    ] {
        assert!(!s.set_numeric_range(a, b));
    }
}
