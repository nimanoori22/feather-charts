#[path = "../examples/custom_horizontal/behavior.rs"]
mod behavior;
use behavior::*;
use feather_charts::{
    api::*,
    model::{
        data_consumer::*, ihorz_scale_behavior::HorzScaleBehavior,
        localization_options::LocalizationOptions, pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
    },
};
use std::{cell::RefCell, error::Error, rc::Rc, time::Instant};
fn row(index: usize, value: f64) -> BuiltInSeriesDataItem<SessionPoint> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: SessionPoint::at(index),
        value,
        color: None,
        custom_values: None,
    }))
}
fn make() -> Chart<Sessions> {
    Chart::with_custom_behavior(
        || Sessions::new(Default::default()),
        Default::default(),
        Default::default(),
        LocalizationOptions::new("en-US", ""),
    )
    .unwrap()
}
fn series(c: &mut Chart<Sessions>) -> feather_charts::model::data_layer::SeriesId {
    c.add_series(
        SeriesOptionsMap::Line(Default::default()),
        0,
        PriceScalePosition::Right,
    )
    .unwrap()
}
#[test]
fn external_non_clone_factory_initializes_independent_owners_and_drops_factory() {
    let audits = Rc::new(RefCell::new(vec![]));
    let observed = audits.clone();
    let token = Rc::new(());
    let factory_token = token.clone();
    let mut c = Chart::with_custom_behavior(
        move || {
            let _ = &factory_token;
            let b = Sessions::new(Default::default());
            observed.borrow_mut().push(b.audit.clone());
            b
        },
        Default::default(),
        Default::default(),
        LocalizationOptions::new("en-US", ""),
    )
    .unwrap();
    assert_eq!(Rc::strong_count(&token), 1);
    let audits = audits.borrow();
    assert_eq!(audits.len(), 2);
    assert!(!Rc::ptr_eq(&audits[0], &audits[1]));
    for a in &*audits {
        assert_eq!(*a.calls.borrow(), [("scale", 0), ("formatter", 0)]);
    }
    c.configure_horizontal_behavior(|| Options {
        prefix: "Run ".into(),
        fast_append: false,
    });
    for a in &*audits {
        assert_eq!(
            &a.calls.borrow()[2..],
            [("options", 0), ("scale", 0), ("formatter", 0)]
        );
    }
    assert_eq!(c.horizontal_behavior_options().prefix, "Run ");
    assert_eq!(
        c.model()
            .time_scale()
            .behavior()
            .format_item(&SessionKey(SessionPoint::at(2).rank())),
        "Run 1 Close"
    );
    drop(c);
    for a in &*audits {
        assert_eq!(Rc::strong_count(a), 1);
    }
}
#[test]
fn public_factory_errors_keep_owned_non_send_causes() {
    let mut c = make();
    let id = series(&mut c);
    c.set_data(id, vec![row(0, 10.), row(2, 20.)]).unwrap();
    let invalid = BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: SessionPoint {
            session: 0,
            phase: Phase::Open,
        },
        value: 1.,
        color: None,
        custom_values: None,
    }));
    let error = c.set_data(id, vec![invalid]).unwrap_err();
    let cause = error
        .source()
        .unwrap()
        .downcast_ref::<SessionError>()
        .unwrap();
    assert_eq!(cause.0.as_str(), "session zero is invalid");
    assert_eq!(c.series_data(id).unwrap().len(), 2);
    drop(c);
    assert!(error.source().is_some());
    let bad = feather_charts::model::time_scale_options::HorzScaleOptions {
        bar_spacing: f64::NAN,
        ..Default::default()
    };
    assert!(
        Chart::with_custom_behavior(
            || Sessions::new(Default::default()),
            Default::default(),
            bad,
            LocalizationOptions::new("en-US", "")
        )
        .is_err()
    );
}
#[test]
fn runtime_labels_and_prepared_exports_preserve_original_items() {
    let mut c = make();
    c.set_event_interests(EventInterests::ALL);
    let id = series(&mut c);
    let rows = vec![row(0, 10.), row(2, 20.), row(7, 15.)];
    c.set_data(id, rows).unwrap();
    c.fit_content();
    let now = Instant::now();
    c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
    let before = c.screenshot_of_prepared(Default::default()).unwrap();
    c.configure_horizontal_behavior(|| Options {
        prefix: "Run ".into(),
        fast_append: false,
    });
    c.prepare_screenshot(Default::default(), now).unwrap();
    assert_eq!(before.logical_size(), iced::Size::new(800., 500.));
    assert_eq!(*c.series_data(id).unwrap()[1].time(), SessionPoint::at(2));
    assert!(c.take_events().iter().any(|e|matches!(e,ChartEvent::VisibleHorizontalRangeChanged(Some(r)) if r.from==SessionPoint::at(0)&&r.to==SessionPoint::at(7))));
}

#[test]
fn failed_preprocessing_keeps_shared_data_and_error_ownership() {
    let audits = Rc::new(RefCell::new(vec![]));
    let observed = audits.clone();
    let mut c = Chart::with_custom_behavior(
        move || {
            let b = Sessions::new(Default::default());
            observed.borrow_mut().push(b.audit.clone());
            b
        },
        Default::default(),
        Default::default(),
        LocalizationOptions::new("en-US", ""),
    )
    .unwrap();
    let a = series(&mut c);
    let b = series(&mut c);
    c.set_data(a, vec![row(0, 10.), row(2, 20.)]).unwrap();
    c.set_data(b, vec![row(1, 30.)]).unwrap();
    let cause = Rc::new("application preprocessing failed".to_string());
    *audits.borrow()[0].fail_preprocess.borrow_mut() = Some(SessionError(cause.clone()));
    let error = c.set_data(a, vec![row(8, 40.)]).unwrap_err();
    let data_error = error
        .source()
        .unwrap()
        .downcast_ref::<feather_charts::model::data_layer::DataLayerError<SessionError>>()
        .unwrap();
    assert!(Rc::ptr_eq(
        &data_error
            .source()
            .unwrap()
            .downcast_ref::<SessionError>()
            .unwrap()
            .0,
        &cause
    ));
    // A later mutation rebuilds shared indexes from the layer, exposing any
    // partial cleanup hidden by the model's previously displayed rows.
    c.update_data(b, row(5, 50.), false).unwrap();
    assert_eq!(c.series_data(a).unwrap().len(), 2);
    assert_eq!(*c.series_data(a).unwrap()[0].time(), SessionPoint::at(0));
    assert_eq!(c.pop_data(a, 1).unwrap().len(), 1);
    assert_eq!(c.series_data(a).unwrap().len(), 1);
    drop(c);
    assert!(Rc::ptr_eq(
        &data_error
            .source()
            .unwrap()
            .downcast_ref::<SessionError>()
            .unwrap()
            .0,
        &cause
    ));
}

#[path = "../examples/numeric_custom_series/view.rs"]
mod interval;

#[test]
fn custom_rows_keep_items_through_reindex_pop_and_owner_drop() {
    use feather_charts::model::icustom_series::CustomSeriesDataItem;
    use feather_charts::model::plot_list::MismatchDirection;
    use feather_charts::model::time_data::TimePointIndex;
    let mut c = make();
    let peer = series(&mut c);
    let h = c
        .add_custom_series(
            interval::View::<SessionPoint, SessionKey>::default(),
            Default::default(),
            0,
            PriceScalePosition::Left,
        )
        .unwrap();
    let datum = |i| {
        CustomSeriesDataItem::Data(interval::Datum {
            time: SessionPoint::at(i),
            low: 2.,
            high: 5.,
            value: 4.,
        })
    };
    c.set_custom_data(&h, vec![datum(0), datum(7)]).unwrap();
    c.set_data(peer, vec![row(2, 10.)]).unwrap();
    assert_eq!(
        h.data_by_index(TimePointIndex::new(2.), MismatchDirection::None)
            .unwrap()
            .unwrap()
            .time,
        SessionPoint::at(7)
    );
    c.update_custom_data(&h, datum(7), false).unwrap();
    c.update_custom_data(&h, datum(0), true).unwrap();
    c.fit_content();
    c.prepare(iced::Size::new(800., 500.), 1.5, Instant::now())
        .unwrap();
    let snapshot = c.screenshot_of_prepared(Default::default()).unwrap();
    c.pop_data(peer, usize::MAX).unwrap();
    assert_eq!(
        h.data_by_index(TimePointIndex::new(1.), MismatchDirection::None)
            .unwrap()
            .unwrap()
            .time,
        SessionPoint::at(7)
    );
    assert_eq!(
        c.pop_custom_data(&h, 1).unwrap()[0].time,
        SessionPoint::at(7)
    );
    drop(c);
    assert_eq!(h.data()[0].time, SessionPoint::at(0));
    assert_eq!(snapshot.logical_size(), iced::Size::new(800., 500.));
}

#[test]
fn weighting_falls_back_to_full_context_unless_explicitly_supported() {
    for fast in [false, true] {
        let observed = Rc::new(RefCell::new(vec![]));
        let audits = observed.clone();
        let mut c = Chart::with_custom_behavior(
            move || {
                let b = Sessions::new(Options {
                    fast_append: fast,
                    ..Default::default()
                });
                observed.borrow_mut().push(b.audit.clone());
                b
            },
            Default::default(),
            Default::default(),
            LocalizationOptions::new("en-US", ""),
        )
        .unwrap();
        let id = series(&mut c);
        c.set_data(id, vec![row(0, 10.), row(2, 20.)]).unwrap();
        audits.borrow()[0].calls.borrow_mut().clear();
        c.update_data(id, row(7, 30.), false).unwrap();
        let calls = audits.borrow()[0].calls.borrow().clone();
        assert!(calls.contains(&("append", 0)));
        assert_eq!(calls.contains(&("weights", 3)), !fast);
        // The fast path must have exactly the same visible weights as a full rebuild.
        let mut layer = feather_charts::model::data_layer::DataLayer::new(Sessions::new(Options {
            fast_append: fast,
            ..Default::default()
        }));
        layer
            .set_series_data(
                id,
                feather_charts::model::series_options::SeriesType::Line,
                vec![row(0, 10.), row(2, 20.)],
            )
            .unwrap();
        let response = layer.update_series_data(id, row(7, 30.), false).unwrap();
        if fast {
            assert_eq!(
                response
                    .time_scale
                    .append
                    .unwrap()
                    .point
                    .time_weight
                    .value(),
                1
            );
        } else {
            assert_eq!(
                response
                    .time_scale
                    .points
                    .unwrap()
                    .iter()
                    .map(|p| p.time_weight.value())
                    .collect::<Vec<_>>(),
                [7, 1, 1]
            );
        }
    }
}

#[test]
fn source_reference() {
    fn json(p: SessionPoint) -> String {
        format!("{{\"session\":{},\"phase\":\"{:?}\"}}", p.session, p.phase)
    }
    fn times(rows: &[BuiltInSeriesDataItem<SessionPoint>]) -> String {
        rows.iter()
            .map(|r| json(*r.time()))
            .collect::<Vec<_>>()
            .join(",")
    }
    fn value(row: &BuiltInSeriesDataItem<SessionPoint>) -> f64 {
        match row {
            BuiltInSeriesDataItem::Line(LineDataItem::Data(d)) => d.value,
            _ => panic!("expected line"),
        }
    }
    let mut c = make();
    c.set_event_interests(EventInterests::ALL);
    let s = series(&mut c);
    c.set_data(s, vec![row(0, 10.), row(2, 20.), row(7, 15.)])
        .unwrap();
    c.fit_content();
    c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let initialized = {
        let calls = c.model().time_scale().behavior().audit.calls.borrow();
        calls.contains(&("scale", 0)) && calls.contains(&("formatter", 0))
    };
    let initial = c.series_data(s).unwrap();
    let scale = c.model().time_scale();
    let xs = (0..3)
        .map(|i| {
            scale
                .index_to_coordinate(feather_charts::model::time_data::TimePointIndex::new(
                    i as f64,
                ))
                .value()
        })
        .collect::<Vec<_>>();
    let equally_spaced = ((xs[1] - xs[0]) - (xs[2] - xs[1])).abs() < 1e-8;
    c.update_data(s, row(7, 16.), false).unwrap();
    c.update_data(s, row(0, 12.), true).unwrap();
    c.update_data(s, row(21, 18.), false).unwrap();
    let data = c.series_data(s).unwrap();
    let latest = value(data.last().unwrap());
    let history = value(&data[0]);
    let removed = c.pop_data(s, 2).unwrap();
    let whitespace = series(&mut c);
    c.set_data(
        whitespace,
        vec![BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(
            WhitespaceData {
                time: SessionPoint::at(3),
                custom_values: None,
            },
        ))],
    )
    .unwrap();
    c.fit_content();
    c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let range = c
        .take_events()
        .into_iter()
        .filter_map(|e| {
            if let ChartEvent::VisibleHorizontalRangeChanged(r) = e {
                r
            } else {
                None
            }
        })
        .next_back()
        .unwrap();
    c.configure_horizontal_behavior(|| Options {
        prefix: "Run ".into(),
        fast_append: false,
    });
    let formatted = c
        .model()
        .time_scale()
        .behavior()
        .format_item(&SessionKey(SessionPoint::at(2).rank()));
    let rejected = c
        .update_data(
            s,
            BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                time: SessionPoint {
                    session: 0,
                    phase: Phase::Open,
                },
                value: 1.,
                color: None,
                custom_values: None,
            })),
            false,
        )
        .is_err();
    println!(
        "CUSTOM_HORIZONTAL_REFERENCE {{\"initialTimes\":[{}],\"equallySpaced\":{},\"latestValue\":{},\"historyValue\":{},\"removedTimes\":[{}],\"range\":{{\"from\":{},\"to\":{}}},\"formatted\":\"{}\",\"rejected\":{},\"initialized\":{}}}",
        times(&initial),
        equally_spaced,
        latest,
        history,
        times(&removed),
        json(range.from),
        json(range.to),
        formatted,
        rejected,
        initialized
    );
}

#[test]
fn formatting_changes_invalidate_cached_tick_pixels_without_mutating_old_exports() {
    let mut exporter =
        iced::futures::executor::block_on(ChartExporter::new(ScreenshotBackend::Software)).unwrap();
    let mut c = make();
    let id = series(&mut c);
    c.set_data(
        id,
        (0..24)
            .map(|i| row(i, 10. + (i as f64 * 0.3).sin()))
            .collect(),
    )
    .unwrap();
    c.fit_content();
    let now = Instant::now();
    c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
    let old = c.screenshot_of_prepared(Default::default()).unwrap();
    let before = exporter.render(&old, Default::default()).unwrap();
    c.configure_horizontal_behavior(|| Options {
        prefix: "Run ".into(),
        fast_append: false,
    });
    let fresh = c.prepare_screenshot(Default::default(), now).unwrap();
    assert_ne!(
        before.rgba(),
        exporter.render(&fresh, Default::default()).unwrap().rgba()
    );
    assert_eq!(
        before.rgba(),
        exporter.render(&old, Default::default()).unwrap().rgba()
    );
    assert_eq!(c.series_data(id).unwrap().len(), 24);
}
