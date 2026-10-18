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

#[test]
fn all_builtin_queries_and_pop_preserve_original_times_and_style_fields() {
    use feather_charts::model::series_options::*;
    let t = Time::from("2024-01-01");
    let cases = [
        (
            SeriesOptionsMap::Line(Default::default()),
            BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                time: t.clone(),
                value: 12.,
                color: Some("red".into()),
                custom_values: Some(()),
            })),
        ),
        (
            SeriesOptionsMap::Histogram(Default::default()),
            BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
                time: t.clone(),
                value: 12.,
                color: Some("red".into()),
                custom_values: Some(()),
            })),
        ),
        (
            SeriesOptionsMap::Area(Default::default()),
            BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
                time: t.clone(),
                value: 12.,
                line_color: Some("red".into()),
                top_color: Some("blue".into()),
                bottom_color: Some("green".into()),
                custom_values: Some(()),
            })),
        ),
        (
            SeriesOptionsMap::Baseline(Default::default()),
            BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(BaselineData {
                time: t.clone(),
                value: 12.,
                top_line_color: Some("red".into()),
                bottom_line_color: Some("blue".into()),
                top_fill_color1: Some("green".into()),
                top_fill_color2: Some("pink".into()),
                bottom_fill_color1: Some("cyan".into()),
                bottom_fill_color2: Some("black".into()),
                custom_values: Some(()),
            })),
        ),
        (
            SeriesOptionsMap::Bar(Default::default()),
            BuiltInSeriesDataItem::Bar(BarDataItem::Data(BarData {
                time: t.clone(),
                open: 10.,
                high: 15.,
                low: 9.,
                close: 12.,
                color: Some("red".into()),
                custom_values: Some(()),
            })),
        ),
        (
            SeriesOptionsMap::Candlestick(Default::default()),
            BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(CandlestickData {
                time: t,
                open: 10.,
                high: 15.,
                low: 9.,
                close: 12.,
                color: Some("red".into()),
                border_color: Some("blue".into()),
                wick_color: Some("green".into()),
                custom_values: Some(()),
            })),
        ),
    ];
    for (options, item) in cases {
        let mut c = Chart::default();
        let id = c.add_series(options, 0, PriceScalePosition::Right).unwrap();
        c.set_data(id, vec![item.clone()]).unwrap();
        assert_eq!(
            c.series_data(id).unwrap().as_slice(),
            std::slice::from_ref(&item)
        );
        assert_eq!(
            c.data_by_index(id, 0.0.into(), MismatchDirection::None)
                .unwrap(),
            Some(item.clone())
        );
        assert_eq!(c.pop_data(id, 1).unwrap(), [item]);
    }
}

#[test]
fn custom_survivors_reindex_without_payload_copies_and_numeric_support_stays_private() {
    use std::{cell::Cell, rc::Rc};
    use support::*;
    let mut c = OptionsChart::new_options(Default::default()).unwrap();
    let a = c
        .add_custom_series(
            View::<HorizontalPrice, HorizontalPrice>::default(),
            Default::default(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let b = c
        .add_custom_series(
            View::<HorizontalPrice, HorizontalPrice>::default(),
            Default::default(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let copies = Rc::new(Cell::new(0));
    c.set_custom_data(&a, vec![datum(HorizontalPrice::new(1.).unwrap(), 10.)])
        .unwrap();
    c.set_custom_data(
        &b,
        vec![
            feather_charts::model::icustom_series::CustomSeriesDataItem::Data(Datum {
                time: HorizontalPrice::new(3.).unwrap(),
                low: 29.,
                high: 31.,
                value: 30.,
                clones: Some(copies.clone()),
            }),
        ],
    )
    .unwrap();
    copies.set(0);
    c.pop_custom_data(&a, 1).unwrap();
    assert_eq!(copies.get(), 0);
    assert_eq!(b.data_at(0.0.into()).unwrap().value, 30.);
    assert!(b.data_at(1.0.into()).is_none());
    let mut y = YieldCurveChart::new(Default::default()).unwrap();
    let line = y
        .add_line_series(Default::default(), 0, PriceScalePosition::Left)
        .unwrap();
    y.set_data(
        line,
        vec![BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time: Maturity::from_months(420.).unwrap(),
            value: 4.,
            color: None,
            custom_values: None,
        }))],
    )
    .unwrap();
    y.prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    let support = y.model().panes()[0].attached_source_ids()[0];
    assert!(y.pop_data(support, 1).is_err());
    assert!(y.series_data(support).is_err());
    y.pop_data(line, 99).unwrap();
    y.prepare(iced::Size::new(300., 90.), 1.5, std::time::Instant::now())
        .unwrap();
    assert!(
        y.model()
            .time_scale()
            .time_to_index(&Maturity::from_months(420.).unwrap(), false)
            .is_some()
    );
    assert_eq!(y.series_ids().collect::<Vec<_>>(), [line]);
}

#[test]
fn canonical_queries_stay_pure_under_conflation_and_pop_invalidates_derived_rows() {
    use feather_charts::model::time_scale_options::HorzScaleOptionsPatch;
    use std::time::Instant;
    let mut c = Chart::default();
    let id = add(&mut c);
    c.apply_time_scale_options(HorzScaleOptionsPatch {
        enable_conflation: Some(true),
        ..Default::default()
    });
    c.set_data(
        id,
        (0..8192)
            .map(|i| row(i as f64 + 1., 100. + (i % 10) as f64))
            .collect(),
    )
    .unwrap();
    c.fit_content();
    c.prepare(iced::Size::new(300., 300.), 1., Instant::now())
        .unwrap();
    assert!(c.model().time_scale().conflation_factor() > 1);
    let old = c.frame().clone();
    assert_eq!(c.series_data(id).unwrap().len(), 8192);
    assert_eq!(
        c.data_by_index(id, 8191.0.into(), MismatchDirection::None)
            .unwrap(),
        Some(row(8192., 101.))
    );
    assert_eq!(c.pop_data(id, 17).unwrap().len(), 17);
    assert_eq!(c.series_data(id).unwrap().len(), 8175);
    c.prepare(iced::Size::new(300., 90.), 1.5, Instant::now())
        .unwrap();
    c.update_data(id, row(8175., 123.), false).unwrap();
    c.update_data(id, row(8193., 120.), false).unwrap();
    c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    assert_eq!(value(c.series_data(id).unwrap().last().unwrap()), 120.);
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            c.data_by_index(id, bad.into(), MismatchDirection::NearestRight),
            Err(ChartError::InvalidDataQuery)
        ));
    }
    assert!(matches!(
        c.bars_in_logical_range(
            id,
            Some(LogicalRange {
                from: 3.0.into(),
                to: 1.0.into()
            })
        ),
        Err(ChartError::InvalidDataQuery)
    ));
    assert_eq!(c.series_data(id).unwrap().len(), 8176);
    drop(c);
    assert_eq!(old.plot.series_id, Some(id));
}
