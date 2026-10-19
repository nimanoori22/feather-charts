use feather_charts::{
    api::*,
    model::{
        data_consumer::*,
        horz_scale_behavior_time::{
            horz_scale_behavior_time::HorzScaleBehaviorTime, types::UtcTimestamp,
        },
        pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
    },
};
use iced::Size;
use std::{cell::Cell, rc::Rc, time::Instant};
struct Record {
    id: u32,
    label: String,
    count: Cell<usize>,
}
#[test]
fn public_application_record_round_trip_needs_no_debug_equality_or_thread_bounds() {
    let p = Rc::new(Record {
        id: 17,
        label: "execution".into(),
        count: Cell::new(0),
    });
    let mut c = Chart::<HorzScaleBehaviorTime, Rc<Record>>::new_with_metadata(Default::default());
    let id = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(
        id,
        vec![BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time: UtcTimestamp::new(1.).into(),
            value: 10.,
            color: None,
            custom_values: Some(p.clone()),
        }))],
    )
    .unwrap();
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let rows = c.series_data(id).unwrap();
    let BuiltInSeriesDataItem::Line(LineDataItem::Data(row)) = &rows[0] else {
        panic!()
    };
    let value = row.custom_values.as_ref().unwrap();
    assert!(Rc::ptr_eq(value, &p));
    assert_eq!(value.id, 17);
    assert_eq!(value.label, "execution");
    assert_eq!(value.count.get(), 0);
}
