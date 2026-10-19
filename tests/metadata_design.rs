//! Concrete engine prototypes for the two public metadata designs.
use feather_charts::model::{
    data_consumer::*,
    data_layer::{DataLayer, SeriesId},
    horz_scale_behavior_time::{
        horz_scale_behavior_time::HorzScaleBehaviorTime, types::UtcTimestamp,
    },
    series_data::SeriesPlotRow,
    series_options::SeriesType,
};
use std::{any::Any, cell::Cell, rc::Rc};
// Deliberately neither Clone, Debug, PartialEq nor Send/Sync.
struct Record {
    id: u32,
    note: Cell<u32>,
}
fn roundtrip<M: Clone>(payload: M, read: impl FnOnce(&M)) {
    let mut layer = DataLayer::<_, M>::new(HorzScaleBehaviorTime::default());
    let update = layer
        .set_series_data(
            SeriesId::new(1),
            SeriesType::Line,
            vec![BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                time: UtcTimestamp::new(1.).into(),
                value: 10.,
                color: None,
                custom_values: Some(payload),
            }))],
        )
        .unwrap();
    let SeriesPlotRow::Line(row) = &update.series[&SeriesId::new(1)].rows[0] else {
        panic!()
    };
    read(row.base.custom_values.as_ref().unwrap());
}
#[test]
fn typed_and_erased_payloads_use_the_existing_engine_without_thread_bounds() {
    let value = Rc::new(Record {
        id: 7,
        note: Cell::new(3),
    });
    roundtrip(value.clone(), |p| {
        assert!(Rc::ptr_eq(&value, p));
        assert_eq!(p.id, 7);
        assert_eq!(p.note.get(), 3)
    });
    let erased: Rc<dyn Any> = value.clone();
    roundtrip(erased, |p| {
        assert_eq!(p.downcast_ref::<Record>().unwrap().id, 7)
    });
}
