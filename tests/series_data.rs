use feather_charts::{
    api::*,
    model::{
        data_consumer::*,
        data_layer::SeriesId,
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        plot_list::MismatchDirection,
        series_options::SeriesOptionsMap,
        time_data::LogicalRange,
    },
};
fn row(time: f64, value: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: UtcTimestamp::new(time).into(),
        value,
        color: Some("red".into()),
        custom_values: None,
    }))
}
fn add(c: &mut Chart) -> SeriesId {
    c.add_series(
        SeriesOptionsMap::Line(Default::default()),
        0,
        PriceScalePosition::Right,
    )
    .unwrap()
}
#[test]
fn pop_returns_newest_first_and_preserves_a_registered_empty_owner() {
    let mut c = Chart::default();
    let id = add(&mut c);
    c.set_data(id, vec![row(1., 10.), row(2., 20.), row(3., 30.)])
        .unwrap();
    assert_eq!(
        c.series_data(id).unwrap(),
        vec![row(1., 10.), row(2., 20.), row(3., 30.)]
    );
    assert_eq!(c.pop_data(id, 2).unwrap(), vec![row(3., 30.), row(2., 20.)]);
    assert_eq!(c.pop_data(id, usize::MAX).unwrap(), vec![row(1., 10.)]);
    assert_eq!(c.series_ids().collect::<Vec<_>>(), [id]);
    assert!(c.pop_data(id, 1).unwrap().is_empty());
    c.update_data(id, row(2., 22.), false).unwrap();
    assert_eq!(
        c.data_by_index(id, 0.0.into(), MismatchDirection::None)
            .unwrap(),
        Some(row(2., 22.))
    );
    assert!(
        c.bars_in_logical_range(
            id,
            Some(LogicalRange {
                from: f64::NAN.into(),
                to: 1.0.into()
            })
        )
        .is_err()
    );
}

fn time(t: &Time) -> f64 {
    match t {
        Time::UtcTimestamp(t) => t.seconds(),
        _ => panic!("timestamp fixture"),
    }
}
fn value(d: &BuiltInSeriesDataItem<Time>) -> f64 {
    match d {
        BuiltInSeriesDataItem::Line(LineDataItem::Data(d)) => d.value,
        _ => panic!("line fixture"),
    }
}
#[test]
fn source_reference_sequence() {
    let mut c = Chart::default();
    let a = add(&mut c);
    let b = add(&mut c);
    let whitespace = |t| {
        BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(WhitespaceData {
            time: UtcTimestamp::new(t).into(),
            custom_values: None,
        }))
    };
    c.set_data(
        a,
        vec![row(1., 10.), whitespace(2.), row(3., 30.), whitespace(4.)],
    )
    .unwrap();
    c.set_data(b, vec![row(3., 99.), row(5., 50.)]).unwrap();
    let snapshot = |c: &Chart, name: &str, popped: Vec<BuiltInSeriesDataItem<Time>>| {
        let pairs = |rows: Vec<BuiltInSeriesDataItem<Time>>| {
            rows.iter()
                .map(|r| [time(r.time()), value(r)])
                .collect::<Vec<_>>()
        };
        let data = pairs(c.series_data(a).unwrap());
        let peer = pairs(c.series_data(b).unwrap());
        let popped = pairs(popped);
        let queries = [-1., 0., 0.5, 1., 2., 3., 4., 5.]
            .into_iter()
            .map(|i| {
                let values = [
                    MismatchDirection::None,
                    MismatchDirection::NearestLeft,
                    MismatchDirection::NearestRight,
                ]
                .into_iter()
                .map(|d| {
                    c.data_by_index(a, i.into(), d)
                        .unwrap()
                        .as_ref()
                        .map_or("null".into(), |d| value(d).to_string())
                })
                .collect::<Vec<_>>()
                .join(",");
                format!("[{i},[{values}]]")
            })
            .collect::<Vec<_>>()
            .join(",");
        let ranges = [
            None,
            Some((-1., 6.)),
            Some((0.25, 1.25)),
            Some((1., 1.)),
            Some((3., 3.)),
            Some((20., 30.)),
        ]
        .into_iter()
        .map(|r| {
            c.bars_in_logical_range(
                a,
                r.map(|(a, b)| LogicalRange {
                    from: a.into(),
                    to: b.into(),
                }),
            )
            .unwrap()
            .map_or("null".into(), |v| {
                let range = v.data_range.map_or("null".into(), |(a, b)| {
                    format!("[{},{}]", time(&a), time(&b))
                });
                format!(
                    "{{\"before\":{},\"after\":{},\"range\":{range}}}",
                    v.bars_before, v.bars_after
                )
            })
        })
        .collect::<Vec<_>>()
        .join(",");
        println!(
            "DATA_REFERENCE {{\"name\":\"{name}\",\"data\":{data:?},\"peer\":{peer:?},\"popped\":{popped:?},\"queries\":[{queries}],\"ranges\":[{ranges}]}}"
        );
    };
    snapshot(&c, "initial", vec![]);
    for (name, count) in [("zero", 0), ("shared", 1), ("unique", 99), ("empty", 1)] {
        let popped = c.pop_data(a, count).unwrap();
        snapshot(&c, name, popped);
    }
    c.update_data(a, row(3., 33.), false).unwrap();
    snapshot(&c, "append", vec![]);
}

#[path = "numeric_custom_series/support.rs"]
mod support;
#[test]
fn custom_pop_moves_payloads_before_shared_indices_change() {
    use support::*;
    let mut c = OptionsChart::new_options(Default::default()).unwrap();
    let view = View::<HorizontalPrice, HorizontalPrice>::default();
    let destroyed = view.destroyed.clone();
    let h = c
        .add_custom_series(view, Default::default(), 0, PriceScalePosition::Right)
        .unwrap();
    c.set_custom_data(
        &h,
        vec![
            datum(HorizontalPrice::new(1.).unwrap(), 10.),
            datum(HorizontalPrice::new(3.).unwrap(), 30.),
        ],
    )
    .unwrap();
    assert_eq!(h.data().len(), 2);
    assert_eq!(c.pop_custom_data(&h, 1).unwrap()[0].value, 30.);
    assert_eq!(
        h.data_by_index(0.0.into(), MismatchDirection::None)
            .unwrap()
            .unwrap()
            .value,
        10.
    );
    assert_eq!(
        h.bars_in_logical_range(Some(LogicalRange {
            from: 0.0.into(),
            to: 1.0.into()
        }))
        .unwrap()
        .unwrap()
        .data_range
        .unwrap()
        .0,
        HorizontalPrice::new(1.).unwrap()
    );
    assert_eq!(destroyed.get(), 0);
    assert_eq!(c.pop_custom_data(&h, 99).unwrap().len(), 1);
    c.update_custom_data(&h, datum(HorizontalPrice::new(2.).unwrap(), 20.), false)
        .unwrap();
    assert_eq!(h.data()[0].value, 20.);
    let mut other = OptionsChart::new_options(Default::default()).unwrap();
    assert!(other.pop_custom_data(&h, 1).is_err());
    c.remove_series(h.id()).unwrap();
    assert_eq!(h.data()[0].value, 20.);
    assert!(c.pop_custom_data(&h, 0).is_err());
}
