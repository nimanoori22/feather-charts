use super::*;
use crate::model::{
    data_consumer::{LineData, WhitespaceData},
    horz_scale_behavior_time::types::UtcTimestamp,
    pane::PriceScalePosition,
};
fn row(time: f64, value: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: UtcTimestamp::new(time).into(),
        value,
        color: None,
        custom_values: None,
    }))
}
fn snapshot(chart: &mut Chart, id: UpDownMarkersId, now: Instant, name: &str) {
    let values = chart
        .up_down_markers(id, now)
        .unwrap()
        .into_iter()
        .map(|m| {
            let time = convert_time(&m.time).unwrap().timestamp.seconds();
            let sign = match m.sign {
                MarkerSign::Negative => -1,
                MarkerSign::Neutral => 0,
                MarkerSign::Positive => 1,
            };
            format!("[{time},{},{sign}]", m.value)
        })
        .collect::<Vec<_>>()
        .join(",");
    println!("UP_DOWN_STATE {{\"name\":\"{name}\",\"markers\":[{values}]}}");
}
#[test]
fn up_down_markers_source_timeline() {
    let mut c = Chart::default();
    let series = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let id = c.add_up_down_markers(series, Default::default()).unwrap();
    let now = Instant::now();
    c.set_up_down_data(id, vec![row(60., 10.), row(120., 20.)])
        .unwrap();
    snapshot(&mut c, id, now, "seed");
    c.update_up_down_data(id, row(60., 12.), true, now).unwrap();
    snapshot(&mut c, id, now, "up");
    let t = now + Duration::from_millis(100);
    c.update_up_down_data(id, row(60., 11.), true, t).unwrap();
    snapshot(&mut c, id, t, "repeat");
    c.update_up_down_data(id, row(120., 20.), true, t).unwrap();
    snapshot(&mut c, id, t, "neutral");
    let t = now + Duration::from_millis(200);
    c.update_up_down_data(id, row(60., 9.), true, t).unwrap();
    snapshot(&mut c, id, t, "refresh_order");
    for (ms, name) in [
        (5099, "before_deadline"),
        (5100, "at_first_deadline"),
        (5200, "expired"),
    ] {
        snapshot(&mut c, id, now + Duration::from_millis(ms), name);
    }
    c.set_up_down_markers(
        id,
        vec![
            UpDownMarker {
                time: UtcTimestamp::new(60.).into(),
                value: 1.,
                sign: MarkerSign::Positive,
            },
            UpDownMarker {
                time: UtcTimestamp::new(120.).into(),
                value: 2.,
                sign: MarkerSign::Negative,
            },
            UpDownMarker {
                time: UtcTimestamp::new(60.).into(),
                value: 3.,
                sign: MarkerSign::Neutral,
            },
        ],
    )
    .unwrap();
    snapshot(&mut c, id, now + Duration::from_millis(5200), "manual");
    let t = now + Duration::from_secs(100);
    snapshot(&mut c, id, t, "manual_persists");
    c.set_up_down_data(id, vec![row(60., 5.)]).unwrap();
    snapshot(&mut c, id, t, "reset_preserves");
    c.update_up_down_data(id, row(180., 8.), false, t).unwrap();
    c.update_up_down_data(id, row(180., 9.), false, t).unwrap();
    snapshot(&mut c, id, t, "append_untracked");
    c.update_up_down_data(
        id,
        BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(WhitespaceData {
            time: UtcTimestamp::new(60.).into(),
            custom_values: None,
        })),
        true,
        t,
    )
    .unwrap();
    c.update_up_down_data(id, row(60., 7.), true, t).unwrap();
    snapshot(&mut c, id, t, "whitespace_untracks");
    c.clear_up_down_markers(id).unwrap();
    snapshot(&mut c, id, t, "clear");
}
#[test]
fn up_down_markers_share_live_state_but_frozen_frames_do_not_retain_it() {
    let mut c = Chart::default();
    let series = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let id = c.add_up_down_markers(series, Default::default()).unwrap();
    let now = Instant::now();
    c.set_up_down_data(id, vec![row(60., 10.), row(120., 20.)])
        .unwrap();
    c.update_up_down_data(id, row(60., 11.), true, now).unwrap();
    c.fit_content();
    c.prepare(iced::Size::new(640., 480.), 1., now).unwrap();
    let frame = c.frame().clone();
    let weak = Rc::downgrade(&c.up_down[&id.0].state);
    c.remove_primitive(id.primitive_id()).unwrap();
    assert!(weak.upgrade().is_none());
    assert!(!frame.plot.primitives.is_empty());
}
#[test]
fn up_down_markers_update_storage_is_bounded_by_active_keys_not_history_updates() {
    let mut c = Chart::default();
    let series = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let id = c.add_up_down_markers(series, Default::default()).unwrap();
    c.set_up_down_data(id, (1..=10000).map(|i| row(i as f64 * 60., 10.)).collect())
        .unwrap();
    let baseline_address = c.up_down[&id.0].baseline.first_key_value().unwrap().1 as *const f64;
    let now = Instant::now();
    for i in 0..100 {
        c.update_up_down_data(
            id,
            row(600000., 11. + i as f64),
            false,
            now + Duration::from_millis(i),
        )
        .unwrap();
    }
    assert_eq!(c.up_down[&id.0].baseline.len(), 10000);
    assert_eq!(
        c.up_down[&id.0].baseline.first_key_value().unwrap().1 as *const f64,
        baseline_address
    );
    let state = c.up_down[&id.0].state.borrow();
    assert_eq!(state.active.len(), 1);
    assert_eq!(state.next, Some(now + Duration::from_millis(5099)));
}
