use feather_charts::{
    api::*,
    model::{
        data_consumer::*,
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        price_range_impl::PriceRange,
        price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
    },
};
use std::time::Instant;
fn row(t: f64, v: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: UtcTimestamp::new(t).into(),
        value: v,
        color: None,
        custom_values: None,
    }))
}
#[test]
fn source_manual_range_sequence() {
    for (mode, lo, hi) in [
        (PriceScaleMode::Normal, 1., 9.),
        (PriceScaleMode::Logarithmic, 4., 5.),
        (PriceScaleMode::Percentage, 1., 9.),
        (PriceScaleMode::IndexedTo100, 1., 9.),
    ] {
        let mut c = Chart::default();
        let side = PriceScalePosition::Right;
        let id = c
            .add_series(
                feather_charts::model::series_options::SeriesOptionsMap::Line(Default::default()),
                0,
                side.clone(),
            )
            .unwrap();
        c.set_data(id, vec![row(1., 10.), row(2., 20.)]).unwrap();
        c.fit_content();
        c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
            .unwrap();
        c.apply_price_scale_options(
            0,
            &side,
            PriceScaleOptionsPatch {
                mode: Some(mode),
                ..Default::default()
            },
        )
        .unwrap();
        c.set_visible_price_range(
            0,
            &side,
            PriceRange {
                min_value: lo,
                max_value: hi,
            },
        )
        .unwrap();
        let expected = PriceRange {
            min_value: 1.,
            max_value: if mode == PriceScaleMode::Logarithmic {
                10.
            } else {
                9.
            },
        };
        assert_eq!(c.visible_price_range(0, &side).unwrap(), Some(expected));
        c.update_data(id, row(3., 30.), false).unwrap();
        c.prepare(iced::Size::new(300., 90.), 1.5, Instant::now())
            .unwrap();
        assert_eq!(c.visible_price_range(0, &side).unwrap(), Some(expected));
        assert!(!c.price_scale_options(0, &side).unwrap().auto_scale);
        c.set_price_scale_auto_scale(0, &side, true).unwrap();
        assert!(c.price_scale_options(0, &side).unwrap().auto_scale);
        let r = c.visible_price_range(0, &side).unwrap().unwrap();
        assert!(r.min_value.is_finite() && r.max_value.is_finite());
    }
}
#[test]
fn invalid_ranges_and_missing_scales_do_not_mutate() {
    let mut c = Chart::default();
    let side = PriceScalePosition::Right;
    assert_eq!(c.visible_price_range(0, &side).unwrap(), None);
    for (lo, hi) in [
        (f64::NAN, 3.),
        (1., f64::INFINITY),
        (4., 3.),
        (3., 3.),
        (-f64::MAX, f64::MAX),
    ] {
        assert!(
            c.set_visible_price_range(
                0,
                &side,
                PriceRange {
                    min_value: lo,
                    max_value: hi
                }
            )
            .is_err()
        );
        assert!(c.price_scale_options(0, &side).unwrap().auto_scale);
    }
    assert!(c.visible_price_range(1, &side).is_err());
    assert!(
        c.visible_price_range(0, &PriceScalePosition::Overlay("missing".into()))
            .is_err()
    );
    assert!(c.model().panes()[0].price_scale_by_id("missing").is_none());
}
