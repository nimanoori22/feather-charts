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
        let mode_number = match mode {
            PriceScaleMode::Normal => 0,
            PriceScaleMode::Logarithmic => 1,
            PriceScaleMode::Percentage => 2,
            PriceScaleMode::IndexedTo100 => 3,
        };
        let emit = |c: &Chart, name: &str| {
            let r = c.visible_price_range(0, &side).unwrap().unwrap();
            println!(
                "CONFIG_REFERENCE {{\"mode\":{mode_number},\"name\":\"{name}\",\"auto\":{},\"range\":{{\"from\":{},\"to\":{}}}}}",
                c.price_scale_options(0, &side).unwrap().auto_scale,
                r.min_value,
                r.max_value
            );
        };
        assert_eq!(c.visible_price_range(0, &side).unwrap(), Some(expected));
        emit(&c, "manual");
        c.update_data(id, row(3., 30.), false).unwrap();
        c.prepare(iced::Size::new(300., 90.), 1.5, Instant::now())
            .unwrap();
        assert_eq!(c.visible_price_range(0, &side).unwrap(), Some(expected));
        emit(&c, "update");
        assert!(!c.price_scale_options(0, &side).unwrap().auto_scale);
        c.set_price_scale_auto_scale(0, &side, true).unwrap();
        assert!(c.price_scale_options(0, &side).unwrap().auto_scale);
        let r = c.visible_price_range(0, &side).unwrap().unwrap();
        assert!(r.min_value.is_finite() && r.max_value.is_finite());
        println!(
            "CONFIG_REFERENCE {{\"mode\":{mode_number},\"name\":\"restored\",\"auto\":true,\"finite\":true}}"
        );
    }
}

#[test]
fn range_belongs_to_pane_and_existing_overlays_survive_updates() {
    use feather_charts::model::series_options::SeriesOptionsMap;
    let mut c = Chart::default();
    c.add_pane();
    c.set_preserve_empty_pane(0, true).unwrap();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(id, vec![row(1., 10.), row(2., 20.)]).unwrap();
    let peer = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(peer, vec![row(1., 11.), row(2., 21.)]).unwrap();
    c.fit_content();
    c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let range = PriceRange {
        min_value: 1.,
        max_value: 9.,
    };
    c.set_visible_price_range(0, &PriceScalePosition::Right, range)
        .unwrap();
    c.move_series_to_pane(id, 1).unwrap();
    assert_eq!(
        c.visible_price_range(0, &PriceScalePosition::Right)
            .unwrap(),
        Some(range)
    );
    assert!(
        c.price_scale_options(1, &PriceScalePosition::Right)
            .unwrap()
            .auto_scale
    );
    c.swap_panes(0, 1).unwrap();
    assert_eq!(
        c.visible_price_range(1, &PriceScalePosition::Right)
            .unwrap(),
        Some(range)
    );
    let overlay = PriceScalePosition::Overlay("study".into());
    let h = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            overlay.clone(),
        )
        .unwrap();
    c.set_data(h, vec![row(1., 2.), row(2., 3.)]).unwrap();
    c.set_visible_price_range(0, &overlay, range).unwrap();
    c.pop_data(h, usize::MAX).unwrap();
    assert_eq!(c.visible_price_range(0, &overlay).unwrap(), Some(range));
    c.update_data(h, row(3., 4.), false).unwrap();
    assert_eq!(c.visible_price_range(0, &overlay).unwrap(), Some(range));
    c.set_price_scale_auto_scale(0, &overlay, true).unwrap();
    c.remove_series(h).unwrap();
    assert!(c.visible_price_range(0, &overlay).is_err());
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

#[test]
fn option_mode_transitions_keep_source_internal_units_and_separate_defaults() {
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
    c.set_visible_price_range(
        0,
        &side,
        PriceRange {
            min_value: 4.,
            max_value: 5.,
        },
    )
    .unwrap();
    assert!(!c.runtime_options().model.pane.right_price_scale.auto_scale);
    let emit = |c: &Chart, mode: u8| {
        let r = c.visible_price_range(0, &side).unwrap().unwrap();
        println!(
            "TRANSITION_REFERENCE {{\"mode\":{mode},\"auto\":{},\"range\":{{\"from\":{},\"to\":{}}},\"defaultAuto\":{}}}",
            c.price_scale_options(0, &side).unwrap().auto_scale,
            r.min_value,
            r.max_value,
            c.runtime_options().model.pane.right_price_scale.auto_scale
        );
    };
    for (mode, lo, hi) in [
        (PriceScaleMode::Logarithmic, 1., 10.),
        (PriceScaleMode::Normal, 4., 5.),
    ] {
        c.apply_price_scale_options(
            0,
            &side,
            PriceScaleOptionsPatch {
                mode: Some(mode),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            c.visible_price_range(0, &side).unwrap(),
            Some(PriceRange {
                min_value: lo,
                max_value: hi
            })
        );
        emit(&c, mode as u8);
    }
    c.apply_chart_options(ChartOptionsPatch {
        model: ChartModelOptionsPatch {
            right_price_scale: Some(PriceScaleOptionsPatch {
                mode: Some(PriceScaleMode::Percentage),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
    assert!(c.price_scale_options(0, &side).unwrap().auto_scale);
    assert!(!c.runtime_options().model.pane.right_price_scale.auto_scale);
    emit(&c, 2);
    c.apply_price_scale_options(
        0,
        &side,
        PriceScaleOptionsPatch {
            mode: Some(PriceScaleMode::IndexedTo100),
            ..Default::default()
        },
    )
    .unwrap();
    emit(&c, 3);
    c.apply_price_scale_options(
        0,
        &side,
        PriceScaleOptionsPatch {
            mode: Some(PriceScaleMode::Percentage),
            ..Default::default()
        },
    )
    .unwrap();
    let pane = c.add_pane();
    assert!(c.price_scale_options(pane, &side).unwrap().auto_scale);
    c.set_price_scale_auto_scale(0, &side, false).unwrap();
    c.apply_chart_options(ChartOptionsPatch {
        model: ChartModelOptionsPatch {
            right_price_scale: Some(PriceScaleOptionsPatch {
                mode: Some(PriceScaleMode::Percentage),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
    assert!(c.price_scale_options(0, &side).unwrap().auto_scale);
}

#[test]
fn removing_the_last_source_restores_the_empty_scale_auto_policy() {
    let mut c = Chart::default();
    c.set_preserve_empty_pane(0, true).unwrap();
    let side = PriceScalePosition::Right;
    let id = c
        .add_series(
            feather_charts::model::series_options::SeriesOptionsMap::Line(Default::default()),
            0,
            side.clone(),
        )
        .unwrap();
    c.set_data(id, vec![row(1., 10.), row(2., 20.)]).unwrap();
    c.set_visible_price_range(
        0,
        &side,
        PriceRange {
            min_value: 1.,
            max_value: 9.,
        },
    )
    .unwrap();
    c.remove_series(id).unwrap();
    assert_eq!(c.visible_price_range(0, &side).unwrap(), None);
    assert!(c.price_scale_options(0, &side).unwrap().auto_scale);
}

#[test]
fn manual_range_set_before_first_frame_survives_initial_layout_and_resize() {
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
    let range = PriceRange {
        min_value: 3.5,
        max_value: 6.5,
    };
    c.set_visible_price_range(0, &side, range).unwrap();
    c.update_data(id, row(2., 21.), false).unwrap();
    for size in [
        iced::Size::new(800., 500.),
        iced::Size::new(300., 90.),
        iced::Size::new(300., 300.),
        iced::Size::new(800., 500.),
    ] {
        c.prepare(size, 1.5, Instant::now()).unwrap();
        assert_eq!(c.visible_price_range(0, &side).unwrap(), Some(range));
    }
}
