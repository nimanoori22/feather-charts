use feather_charts::model::time_scale_host::TimeScaleHost;
use feather_charts::{
    api::{
        ChartError, ChartEvent, ChartMessage, MarkerSign, Maturity, UpDownMarker, YieldCurveChart,
        YieldCurveOptions,
    },
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        pane::PriceScalePosition,
        time_data::TimePointIndex,
    },
};
use iced::Size;
use std::time::{Duration, Instant};
fn m(v: f64) -> Maturity {
    Maturity::from_months(v).unwrap()
}
fn row(t: f64, v: f64) -> BuiltInSeriesDataItem<Maturity> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: m(t),
        value: v,
        color: None,
        custom_values: None,
    }))
}
fn chart() -> (YieldCurveChart, feather_charts::model::data_layer::SeriesId) {
    let mut c = YieldCurveChart::new(Default::default()).unwrap();
    let id = c
        .add_line_series(Default::default(), 0, PriceScalePosition::Left)
        .unwrap();
    c.set_data(
        id,
        vec![row(1., 5.), row(6., 4.8), row(12., 4.5), row(120., 4.)],
    )
    .unwrap();
    c.fit_content();
    (c, id)
}
#[test]
fn sparse_curve_draws_and_source_defaults_keep_private_scaffold_out_of_public_data() {
    let (mut c, id) = chart();
    c.prepare(Size::new(800., 500.), 1.25, Instant::now())
        .unwrap();
    assert_eq!(c.series_ids().collect::<Vec<_>>(), vec![id]);
    assert!(!c.frame().plot.lines.is_empty());
    assert_eq!(c.model().series(id).unwrap().bars().rows().len(), 4);
    assert!(
        c.model()
            .time_scale()
            .time_to_index(&m(60.), false)
            .is_some()
    );
    assert!(!c.model().fulfilled_time_indices().contains(&60));
    let owner = c.model().series(id).unwrap();
    assert!(!owner.common_options().last_value_visible);
    assert!(!owner.common_options().price_line_visible);
}
#[test]
fn failed_writes_preserve_data_domain_and_marker_deadlines() {
    let (mut c, id) = chart();
    let now = Instant::now();
    let markers = c.add_up_down_markers(id, Default::default()).unwrap();
    c.set_up_down_data(markers, vec![row(1., 5.), row(120., 4.)])
        .unwrap();
    c.update_up_down_data(markers, row(1., 5.1), true, now)
        .unwrap();
    let before = c.up_down_markers(markers, now).unwrap();
    let deadline = c.next_up_down_expiration();
    for rows in [
        vec![row(2., 5.), row(1., 5.)],
        vec![row(200000., 4.)],
        vec![row(1., f64::NAN)],
    ] {
        assert!(c.set_up_down_data(markers, rows).is_err());
        assert_eq!(c.up_down_markers(markers, now).unwrap(), before);
        assert_eq!(c.next_up_down_expiration(), deadline);
        assert_eq!(c.model().series(id).unwrap().bars().rows().len(), 2);
    }
    assert!(matches!(
        c.replace_yield_curve_options(YieldCurveOptions {
            base_resolution: f64::NAN,
            ..Default::default()
        }),
        Err(ChartError::YieldCurve(_))
    ));
    c.prepare(Size::new(800., 500.), 1., now).unwrap();
    assert!(
        c.model()
            .time_scale()
            .time_to_index(&m(200000.), false)
            .is_none()
    );
}
#[test]
fn pointer_payloads_are_numeric_and_crosshair_ignores_scaffold_only_maturities() {
    let (mut c, _) = chart();
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    for maturity in [6., 8.] {
        let scale = c.model().time_scale();
        let index = scale.time_to_index(&m(maturity), false).unwrap();
        let x = scale.index_to_coordinate(index).value() as f32;
        let plot = c.frame().layout.plot;
        let events = c
            .update(ChartMessage::Click {
                pane: 0,
                position: iced::Point::new(plot.origin.x as f32 + x, plot.origin.y as f32 + 20.),
            })
            .unwrap();
        let event = events
            .iter()
            .find_map(|e| {
                if let ChartEvent::Click(p) = e {
                    Some(p)
                } else {
                    None
                }
            })
            .unwrap();
        assert!(event.time.is_some());
        assert_ne!(
            event.time,
            Some(m(8.)),
            "support-only points are not crosshair targets"
        );
        assert_eq!(event.series_data.len(), 1);
    }
}
#[test]
fn numeric_markers_expire_exactly_and_manual_markers_persist() {
    let (mut c, id) = chart();
    let now = Instant::now();
    let markers = c.add_up_down_markers(id, Default::default()).unwrap();
    c.set_up_down_data(markers, vec![row(1., 5.), row(6., 4.8), row(12., 4.5)])
        .unwrap();
    c.update_up_down_data(markers, row(6., 4.9), true, now)
        .unwrap();
    assert_eq!(
        c.up_down_markers(markers, now).unwrap()[0].sign,
        MarkerSign::Positive
    );
    c.prepare(Size::new(800., 500.), 1., now).unwrap();
    assert!(!c.frame().plot.primitives.is_empty());
    assert_eq!(
        c.up_down_markers(markers, now + Duration::from_millis(4999))
            .unwrap()
            .len(),
        1
    );
    c.update(ChartMessage::Tick(now + Duration::from_secs(5)))
        .unwrap();
    assert!(
        c.up_down_markers(markers, now + Duration::from_secs(5))
            .unwrap()
            .is_empty()
    );
    c.set_up_down_markers(
        markers,
        vec![UpDownMarker {
            time: m(6.),
            value: 4.8,
            sign: MarkerSign::Neutral,
        }],
    )
    .unwrap();
    assert_eq!(
        c.up_down_markers(markers, now + Duration::from_secs(100))
            .unwrap()
            .len(),
        1
    );
    c.remove_series(id).unwrap();
    assert!(c.next_up_down_expiration().is_none());
    assert!(c.up_down_markers(markers, now).is_err());
    assert!(
        c.model()
            .time_scale()
            .index_to_time(TimePointIndex::new(120.))
            .is_some()
    );
}
