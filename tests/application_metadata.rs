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
pub struct Record {
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

use feather_charts::model::{
    data_layer::SeriesId, horz_scale_behavior_time::types::Time, plot_list::MismatchDirection,
    time_data::TimePointIndex,
};
type Payload = Rc<Record>;
fn payload(id: u32) -> Payload {
    Rc::new(Record {
        id,
        label: format!("trade-{id}"),
        count: Cell::new(0),
    })
}
fn row(kind: usize, t: f64, p: Option<Payload>) -> BuiltInSeriesDataItem<Time, Payload> {
    let time = UtcTimestamp::new(t).into();
    match kind {
        0 => BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time,
            value: 10.,
            color: None,
            custom_values: p,
        })),
        1 => BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
            time,
            value: 10.,
            color: None,
            custom_values: p,
        })),
        2 => BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
            time,
            value: 10.,
            line_color: None,
            top_color: None,
            bottom_color: None,
            custom_values: p,
        })),
        3 => BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(BaselineData {
            time,
            value: 10.,
            top_fill_color1: None,
            top_fill_color2: None,
            top_line_color: None,
            bottom_fill_color1: None,
            bottom_fill_color2: None,
            bottom_line_color: None,
            custom_values: p,
        })),
        4 => BuiltInSeriesDataItem::Bar(BarDataItem::Data(BarData {
            time,
            open: 9.,
            high: 11.,
            low: 8.,
            close: 10.,
            color: None,
            custom_values: p,
        })),
        _ => BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(CandlestickData {
            time,
            open: 9.,
            high: 11.,
            low: 8.,
            close: 10.,
            color: None,
            border_color: None,
            wick_color: None,
            custom_values: p,
        })),
    }
}
fn metadata(row: &BuiltInSeriesDataItem<Time, Payload>) -> Option<&Payload> {
    match row {
        BuiltInSeriesDataItem::Line(LineDataItem::Data(r)) => r.custom_values.as_ref(),
        BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(r)) => r.custom_values.as_ref(),
        BuiltInSeriesDataItem::Area(AreaDataItem::Data(r)) => r.custom_values.as_ref(),
        BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(r)) => r.custom_values.as_ref(),
        BuiltInSeriesDataItem::Bar(BarDataItem::Data(r)) => r.custom_values.as_ref(),
        BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(r)) => {
            r.custom_values.as_ref()
        }
        _ => None,
    }
}
fn options(kind: usize) -> SeriesOptionsMap {
    match kind {
        0 => SeriesOptionsMap::Line(Default::default()),
        1 => SeriesOptionsMap::Histogram(Default::default()),
        2 => SeriesOptionsMap::Area(Default::default()),
        3 => SeriesOptionsMap::Baseline(Default::default()),
        4 => SeriesOptionsMap::Bar(Default::default()),
        _ => SeriesOptionsMap::Candlestick(Default::default()),
    }
}
fn prepare(c: &mut Chart<HorzScaleBehaviorTime, Payload>) {
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
}
#[test]
fn all_six_families_preserve_payloads_across_mutations_reindex_and_pop() {
    for kind in 0..6 {
        let mut c = Chart::<_, Payload>::new_with_metadata(Default::default());
        c.set_event_interests(EventInterests::ALL);
        let id = c
            .add_series(options(kind), 0, PriceScalePosition::Right)
            .unwrap();
        let a = payload(2);
        let b = payload(4);
        c.set_data(
            id,
            vec![
                row(kind, 2., Some(a.clone())),
                row(kind, 4., Some(b.clone())),
            ],
        )
        .unwrap();
        // Another series inserts points before and between this series' canonical rows.
        let peer = c
            .add_series(options(0), 0, PriceScalePosition::Right)
            .unwrap();
        c.set_data(peer, vec![row(0, 1., None), row(0, 3., None)])
            .unwrap();
        let data = c
            .data_by_index(id, TimePointIndex::new(3.), MismatchDirection::None)
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(metadata(&data).unwrap(), &b));
        assert!(
            c.set_data(
                id,
                vec![
                    row(kind, 4., Some(payload(8))),
                    row(kind, 2., Some(payload(9)))
                ]
            )
            .is_err()
        );
        assert!(Rc::ptr_eq(
            metadata(&c.series_data(id).unwrap()[0]).unwrap(),
            &a
        ));
        assert!(
            c.update_data(id, row(kind, 1.5, Some(payload(99))), true)
                .is_err()
        );
        c.update_data(id, row(kind, 2., Some(payload(20))), true)
            .unwrap();
        assert_eq!(metadata(&c.series_data(id).unwrap()[0]).unwrap().id, 20);
        c.update_data(id, row(kind, 4., Some(payload(40))), false)
            .unwrap();
        assert_eq!(metadata(&c.series_data(id).unwrap()[1]).unwrap().id, 40);
        c.update_data(id, row(kind, 4., None), false).unwrap();
        assert!(metadata(&c.series_data(id).unwrap()[1]).is_none());
        let removed = c.pop_data(id, 2).unwrap();
        assert_eq!(*removed[0].time(), UtcTimestamp::new(4.).into());
        assert_eq!(metadata(&removed[1]).unwrap().id, 20);
        assert!(c.series_data(id).unwrap().is_empty());
        assert!(c.take_events().iter().any(|e|matches!(e,ChartEvent::DataChanged{series,scope:DataChangeScope::Update} if *series==id)));
    }
}
#[test]
fn replacing_with_whitespace_clears_previous_metadata_and_retains_only_current_membership() {
    let mut c = Chart::<_, Payload>::new_with_metadata(Default::default());
    let id = c
        .add_series(options(0), 0, PriceScalePosition::Right)
        .unwrap();
    let old = payload(1);
    let weak = Rc::downgrade(&old);
    c.set_data(id, vec![row(0, 1., Some(old.clone()))]).unwrap();
    drop(old);
    c.update_data(
        id,
        BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(WhitespaceData {
            time: UtcTimestamp::new(1.).into(),
            custom_values: Some(payload(7)),
        })),
        false,
    )
    .unwrap();
    assert!(weak.upgrade().is_none());
    assert!(c.series_data(id).unwrap().is_empty());
    c.update_data(id, row(0, 1., None), false).unwrap();
    assert!(metadata(&c.series_data(id).unwrap()[0]).is_none());
    assert!(
        c.update_data(SeriesId::new(u64::MAX), row(0, 1., Some(payload(9))), false)
            .is_err()
    );
}
#[test]
fn pointer_events_capture_metadata_and_survive_later_update_removal_and_owner_drop() {
    let mut c = Chart::<_, Payload>::new_with_metadata(Default::default());
    let id = c
        .add_series(options(0), 0, PriceScalePosition::Right)
        .unwrap();
    let p = payload(10);
    let weak = Rc::downgrade(&p);
    c.set_data(id, vec![row(0, 1., Some(p.clone()))]).unwrap();
    drop(p);
    c.fit_content();
    prepare(&mut c);
    let x = c
        .horizontal_to_coordinate(&UtcTimestamp::new(1.).into())
        .unwrap()
        .unwrap();
    let events = c
        .update(ChartMessage::Click {
            pane: 0,
            position: iced::Point::new(x as f32, 100.),
        })
        .unwrap();
    let pointer = events
        .into_iter()
        .find_map(|e| {
            if let ChartEvent::Click(e) = e {
                Some(e)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(metadata(&pointer.series_data[0].1).unwrap().id, 10);
    c.update_data(id, row(0, 1., Some(payload(11))), false)
        .unwrap();
    c.remove_series(id).unwrap();
    drop(c);
    assert!(weak.upgrade().is_some());
    assert_eq!(metadata(&pointer.series_data[0].1).unwrap().id, 10);
    drop(pointer);
    assert!(weak.upgrade().is_none());
}
#[test]
fn large_rc_payloads_are_not_deep_copied_and_builtin_snapshots_do_not_pin_them() {
    struct Large {
        bytes: Vec<u8>,
        clones: Rc<Cell<usize>>,
        drops: Rc<Cell<usize>>,
    }
    impl Clone for Large {
        fn clone(&self) -> Self {
            self.clones.set(self.clones.get() + 1);
            Self {
                bytes: self.bytes.clone(),
                clones: self.clones.clone(),
                drops: self.drops.clone(),
            }
        }
    }
    impl Drop for Large {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1)
        }
    }
    let clones = Rc::new(Cell::new(0));
    let drops = Rc::new(Cell::new(0));
    let p = Rc::new(Large {
        bytes: vec![1; 1024 * 1024],
        clones: clones.clone(),
        drops: drops.clone(),
    });
    let weak = Rc::downgrade(&p);
    let mut c = Chart::<_, Rc<Large>>::new_with_metadata(Default::default());
    let id = c
        .add_series(options(0), 0, PriceScalePosition::Right)
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
    drop(p);
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let shot = c
        .screenshot_of_prepared(ScreenshotLayers::default())
        .unwrap();
    let read = c.series_data(id).unwrap();
    let popped = c.pop_data(id, 1).unwrap();
    drop(c);
    assert_eq!(clones.get(), 0);
    assert_eq!(drops.get(), 0);
    drop(read);
    drop(popped);
    assert!(weak.upgrade().is_none());
    assert_eq!(drops.get(), 1);
    drop(shot);
}

#[path = "../examples/custom_horizontal/behavior.rs"]
mod behavior;
#[path = "application_metadata/view.rs"]
mod view;
#[test]
fn custom_views_and_all_chart_families_accept_metadata_without_erasing_custom_data() {
    use feather_charts::model::{
        horz_scale_behavior_price::{HorizontalPrice, HorzScaleBehaviorPrice},
        horz_scale_behavior_time::types::TimePoint,
        icustom_series::*,
        localization_options::LocalizationOptions,
    };
    let mut c = Chart::<_, Payload>::new_with_metadata(Default::default());
    let h = c
        .add_custom_series(
            view::View::<Time, TimePoint>::default(),
            Default::default(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let p = payload(88);
    let ws = payload(99);
    let weak_ws = Rc::downgrade(&ws);
    c.set_custom_data(
        &h,
        vec![
            CustomSeriesDataItem::Data(view::Datum {
                time: UtcTimestamp::new(1.).into(),
                value: 10.,
                payload: p.clone(),
            }),
            CustomSeriesDataItem::Whitespace(CustomSeriesWhitespaceData {
                time: UtcTimestamp::new(2.).into(),
                custom_values: Some(ws.clone()),
            }),
        ],
    )
    .unwrap();
    drop(ws);
    c.fit_content();
    prepare(&mut c);
    let x = c
        .horizontal_to_coordinate(&UtcTimestamp::new(1.).into())
        .unwrap()
        .unwrap();
    let e = c
        .update(ChartMessage::Click {
            pane: 0,
            position: iced::Point::new(x as f32, 100.),
        })
        .unwrap()
        .into_iter()
        .find_map(|e| {
            if let ChartEvent::Click(e) = e {
                Some(e)
            } else {
                None
            }
        })
        .unwrap();
    assert!(Rc::ptr_eq(&e.custom_data(&h).unwrap().payload, &p));
    assert!(Rc::ptr_eq(&h.data()[0].payload, &p));
    c.move_series_to_pane(h.id(), 1).unwrap();
    prepare(&mut c);
    let shot = c
        .screenshot_of_prepared(ScreenshotLayers::default())
        .unwrap();
    c.remove_series(h.id()).unwrap();
    assert!(weak_ws.upgrade().is_none());
    assert_eq!(h.data()[0].payload.id, 88);
    assert!(h.with_view_options(&c, |_| ()).is_err());
    drop(c);
    drop(shot);
    assert_eq!(e.custom_data(&h).unwrap().payload.id, 88);
    let mut strikes =
        Chart::<HorzScaleBehaviorPrice, Payload>::new_options_with_metadata(Default::default())
            .unwrap();
    let id = strikes
        .add_series(options(0), 0, PriceScalePosition::Right)
        .unwrap();
    strikes
        .set_data(
            id,
            vec![BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                time: HorizontalPrice::new(100.).unwrap(),
                value: 10.,
                color: None,
                custom_values: Some(p.clone()),
            }))],
        )
        .unwrap();
    strikes
        .prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let BuiltInSeriesDataItem::Line(LineDataItem::Data(row)) = &strikes.pop_data(id, 1).unwrap()[0]
    else {
        panic!()
    };
    assert!(Rc::ptr_eq(row.custom_values.as_ref().unwrap(), &p));
    let mut yields = YieldCurveChart::<Payload>::new_with_metadata(Default::default()).unwrap();
    let id = yields
        .add_line_series(Default::default(), 0, PriceScalePosition::Left)
        .unwrap();
    yields
        .set_data(
            id,
            vec![BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                time: Maturity::from_months(12.).unwrap(),
                value: 4.,
                color: None,
                custom_values: Some(p.clone()),
            }))],
        )
        .unwrap();
    yields.add_pane();
    let h = yields
        .add_custom_series(
            view::View::<Maturity, Maturity>::default(),
            Default::default(),
            1,
            PriceScalePosition::Left,
        )
        .unwrap();
    yields
        .set_custom_data(
            &h,
            vec![CustomSeriesDataItem::Data(view::Datum {
                time: Maturity::from_months(12.).unwrap(),
                value: 4.,
                payload: p.clone(),
            })],
        )
        .unwrap();
    yields
        .prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    assert_eq!(h.data()[0].payload.id, 88);
    let BuiltInSeriesDataItem::Line(LineDataItem::Data(row)) = &yields.series_data(id).unwrap()[0]
    else {
        panic!()
    };
    assert!(Rc::ptr_eq(row.custom_values.as_ref().unwrap(), &p));
    assert_eq!(yields.pop_custom_data(&h, 1).unwrap()[0].payload.id, 88);
    let mut sessions = Chart::<behavior::Sessions, Payload>::with_custom_behavior_and_metadata(
        || behavior::Sessions::new(Default::default()),
        Default::default(),
        Default::default(),
        LocalizationOptions::new("en-US", ""),
    )
    .unwrap();
    let id = sessions
        .add_series(options(0), 0, PriceScalePosition::Right)
        .unwrap();
    sessions
        .set_data(
            id,
            vec![BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                time: behavior::SessionPoint::at(0),
                value: 10.,
                color: None,
                custom_values: Some(p.clone()),
            }))],
        )
        .unwrap();
    sessions
        .prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    assert_eq!(sessions.series_data(id).unwrap().len(), 1);
}
#[test]
fn primitive_contexts_can_read_typed_metadata_without_retaining_live_models() {
    #[derive(Debug)]
    struct Inspect(Rc<Cell<u32>>);
    impl DrawingPrimitive<HorzScaleBehaviorTime, Payload> for Inspect {
        fn prepare(
            &self,
            c: &PrimitiveContext<'_, HorzScaleBehaviorTime, Payload>,
        ) -> PreparedPrimitive {
            if let PrimitiveOwner::Series(id) = c.owner {
                let series = c.model.series(id).unwrap();
                let feather_charts::model::series_data::SeriesPlotRow::Line(row) =
                    &series.bars().rows()[0]
                else {
                    panic!()
                };
                self.0.set(row.base.custom_values.as_ref().unwrap().id);
            }
            PreparedPrimitive::default()
        }
    }
    let mut c = Chart::<_, Payload>::new_with_metadata(Default::default());
    let id = c
        .add_series(options(0), 0, PriceScalePosition::Right)
        .unwrap();
    c.set_data(id, vec![row(0, 1., Some(payload(55)))]).unwrap();
    let observed = Rc::new(Cell::new(0));
    c.add_series_primitive(id, Inspect(observed.clone()))
        .unwrap();
    prepare(&mut c);
    assert_eq!(observed.get(), 55);
    c.update_data(id, row(0, 1., Some(payload(56))), false)
        .unwrap();
    prepare(&mut c);
    assert_eq!(observed.get(), 56);
}

#[test]
fn source_application_metadata_states() {
    let mut output = vec![];
    for (kind, family) in [
        "Line",
        "Histogram",
        "Area",
        "Baseline",
        "Bar",
        "Candlestick",
    ]
    .into_iter()
    .enumerate()
    {
        let mut c = Chart::<_, Payload>::new_with_metadata(Default::default());
        let id = c
            .add_series(options(kind), 0, PriceScalePosition::Right)
            .unwrap();
        let peer = c
            .add_series(options(0), 0, PriceScalePosition::Right)
            .unwrap();
        let a = payload(2);
        let b = payload(4);
        c.set_data(
            id,
            vec![row(kind, 2., Some(a.clone())), row(kind, 4., Some(b))],
        )
        .unwrap();
        c.set_data(peer, vec![row(0, 1., None), row(0, 3., None)])
            .unwrap();
        let ids = |c: &Chart<_, Payload>| {
            c.series_data(id)
                .unwrap()
                .iter()
                .map(|r| metadata(r).map_or(0, |p| p.id))
                .collect::<Vec<_>>()
        };
        let retained = c.series_data(id).unwrap();
        let initial = ids(&c);
        let same = Rc::ptr_eq(
            metadata(
                &c.data_by_index(id, TimePointIndex::new(1.), MismatchDirection::None)
                    .unwrap()
                    .unwrap(),
            )
            .unwrap(),
            &a,
        );
        c.fit_content();
        prepare(&mut c);
        let x = c
            .horizontal_to_coordinate(&UtcTimestamp::new(2.).into())
            .unwrap()
            .unwrap();
        let pointer = c
            .update(ChartMessage::Click {
                pane: 0,
                position: iced::Point::new(x as f32, 100.),
            })
            .unwrap()
            .into_iter()
            .find_map(|e| {
                if let ChartEvent::Click(e) = e {
                    Some(e)
                } else {
                    None
                }
            })
            .unwrap();
        a.count.set(3);
        let shared = metadata(
            &pointer
                .series_data
                .iter()
                .find(|(s, _)| *s == id)
                .unwrap()
                .1,
        )
        .unwrap()
        .count
        .get()
            == 3;
        let rejected = c
            .update_data(id, row(kind, 1.5, Some(payload(99))), true)
            .is_err();
        c.update_data(id, row(kind, 2., Some(payload(20))), true)
            .unwrap();
        let history = ids(&c);
        c.update_data(id, row(kind, 4., Some(payload(40))), false)
            .unwrap();
        let latest = ids(&c);
        c.update_data(id, row(kind, 4., None), false).unwrap();
        let cleared = ids(&c);
        let whitespace_row = WhitespaceData {
            time: UtcTimestamp::new(4.).into(),
            custom_values: Some(payload(88)),
        };
        let ws = match kind {
            0 => BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(whitespace_row)),
            1 => BuiltInSeriesDataItem::Histogram(HistogramDataItem::Whitespace(whitespace_row)),
            2 => BuiltInSeriesDataItem::Area(AreaDataItem::Whitespace(whitespace_row)),
            3 => BuiltInSeriesDataItem::Baseline(BaselineDataItem::Whitespace(whitespace_row)),
            4 => BuiltInSeriesDataItem::Bar(BarDataItem::Whitespace(whitespace_row)),
            _ => {
                BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Whitespace(whitespace_row))
            }
        };
        c.update_data(id, ws, false).unwrap();
        let whitespace = ids(&c);
        c.update_data(id, row(kind, 4., None), false).unwrap();
        let restored = ids(&c);
        c.update_data(id, row(kind, 4., Some(payload(44))), false)
            .unwrap();
        let popped = c
            .pop_data(id, 2)
            .unwrap()
            .iter()
            .map(|r| metadata(r).unwrap().id)
            .collect::<Vec<_>>();
        let empty = c.series_data(id).unwrap().is_empty();
        let retained = retained
            .iter()
            .map(|r| metadata(r).unwrap().id)
            .collect::<Vec<_>>();
        output.push(format!("{{\"family\":\"{family}\",\"initial\":{initial:?},\"same\":{same},\"shared\":{shared},\"rejected\":{rejected},\"history\":{history:?},\"latest\":{latest:?},\"cleared\":{cleared:?},\"whitespace\":{whitespace:?},\"restored\":{restored:?},\"popped\":{popped:?},\"retained\":{retained:?},\"empty\":{empty}}}"));
    }
    println!("METADATA_REFERENCE [{}]", output.join(","));
}

#[test]
fn retained_custom_snapshots_and_handles_release_payload_after_the_last_owner() {
    use feather_charts::model::{horz_scale_behavior_time::types::TimePoint, icustom_series::*};
    let p = payload(77);
    let weak = Rc::downgrade(&p);
    let mut c = Chart::<_, Payload>::new_with_metadata(Default::default());
    let h = c
        .add_custom_series(
            view::View::<Time, TimePoint>::default(),
            Default::default(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_custom_data(
        &h,
        vec![CustomSeriesDataItem::Data(view::Datum {
            time: UtcTimestamp::new(1.).into(),
            value: 10.,
            payload: p,
        })],
    )
    .unwrap();
    c.fit_content();
    prepare(&mut c);
    let shot = c
        .screenshot_of_prepared(ScreenshotLayers::default())
        .unwrap();
    c.remove_series(h.id()).unwrap();
    drop(c);
    assert!(weak.upgrade().is_some());
    drop(h);
    assert!(
        weak.upgrade().is_some(),
        "the example renderer deliberately owns frozen original data"
    );
    drop(shot);
    assert!(weak.upgrade().is_none());
}
