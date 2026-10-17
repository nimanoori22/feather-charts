use feather_charts::model::icustom_series::CustomSeriesDataItem;
use feather_charts::model::time_scale_host::TimeScaleHost;
#[path = "numeric_custom_series/support.rs"]
mod support;
use feather_charts::{api::*, model::pane::PriceScalePosition};
use iced::Size;
use std::time::Instant;
use support::*;
fn p(v: f64) -> HorizontalPrice {
    HorizontalPrice::new(v).unwrap()
}
#[test]
fn price_custom_data_and_original_payloads() {
    let mut c = OptionsChart::new_options(Default::default()).unwrap();
    let h = c
        .add_custom_series(
            View::<HorizontalPrice, HorizontalPrice>::default(),
            Default::default(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_custom_data(
        &h,
        vec![datum(p(90.), 2.), datum(p(100.), 3.), datum(p(150.), 4.)],
    )
    .unwrap();
    c.fit_content();
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    assert!(c.frame().plot.custom.is_some());
    assert_eq!(h.data_at(1.0.into()).unwrap().time, p(100.));
}

#[test]
fn yield_custom_extent_validation_is_transactional() {
    let mut c = YieldCurveChart::new(Default::default()).unwrap();
    let h = c
        .add_custom_series(
            View::<Maturity, Maturity>::default(),
            Default::default(),
            0,
            PriceScalePosition::Left,
        )
        .unwrap();
    let m = |v| Maturity::from_months(v).unwrap();
    c.set_custom_data(&h, vec![datum(m(1.), 5.), datum(m(360.), 4.)])
        .unwrap();
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    assert!(
        c.model()
            .time_scale()
            .time_to_index(&m(240.), false)
            .is_some()
    );
    let old = c.model().time_scale().base_index();
    assert!(c.update_custom_data(&h, datum(m(1e6), 3.), false).is_err());
    assert!(c.set_custom_data(&h, vec![datum(m(1e6), 3.)]).is_err());
    assert_eq!(c.model().time_scale().base_index(), old);
    assert_eq!(h.data_at(360.0.into()).unwrap().time, m(360.));
}

#[test]
fn numeric_custom_source_states() {
    use feather_charts::model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        icustom_series::*,
    };
    macro_rules! sequence {
        ($ty:ty,$c:expr,$view:expr,$family:expr,$keys:expr,$convert:expr,$number:expr,$side:expr) => {{
            let mut c = $c;
            let h = c
                .add_custom_series($view, Default::default(), 0, $side)
                .unwrap();
            let peer = c.add_series_for_reference();
            let keys = $keys;
            let convert = $convert;
            let rows = |time, value| datum(convert(time), value);
            let line = |time, value| {
                BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                    time: convert(time),
                    value,
                    color: None,
                    custom_values: None,
                }))
            };
            let mut expected = include_str!("../docs/porting/numeric-custom-series-states.tsv")
                .lines()
                .filter(|l| l.starts_with($family));
            let mut check = |c: &mut $ty, name: &str| {
                c.prepare(Size::new(800., 500.), 1., Instant::now())
                    .unwrap();
                let scale = c.model().time_scale();
                let points = (0..1000)
                    .map_while(|i| scale.index_to_time((i as f64).into()).map(|v| $number(*v)))
                    .collect::<Vec<_>>();
                let fulfilled = c
                    .model()
                    .fulfilled_time_indices()
                    .iter()
                    .map(|i| points[*i].to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                let values = (0..points.len())
                    .filter_map(|i| h.data_at((i as f64).into()).map(|r| r.value.to_string()))
                    .collect::<Vec<_>>()
                    .join(",");
                let actual = format!(
                    "{}\t{name}\t{}\t{fulfilled}\t{values}",
                    $family,
                    points
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(",")
                );
                assert_eq!(actual, expected.next().unwrap());
            };
            check(&mut c, "empty");
            c.set_custom_data(
                &h,
                keys[..3]
                    .iter()
                    .enumerate()
                    .map(|(i, t)| rows(*t, i as f64 + 2.))
                    .collect(),
            )
            .unwrap();
            check(&mut c, "set");
            c.set_data(peer, vec![line(keys[3], 5.)]).unwrap();
            check(&mut c, "union");
            c.update_custom_data(&h, rows(keys[1], 7.), true).unwrap();
            check(&mut c, "history");
            c.update_custom_data(&h, rows(keys[4], 9.), false).unwrap();
            check(&mut c, "append");
            c.update_custom_data(
                &h,
                CustomSeriesDataItem::Whitespace(CustomSeriesWhitespaceData {
                    time: convert(keys[4]),
                    custom_values: None,
                }),
                false,
            )
            .unwrap();
            check(&mut c, "whitespace");
            c.set_custom_data(&h, vec![rows(keys[0], 2.)]).unwrap();
            check(&mut c, "shorter");
            c.remove_series(peer).unwrap();
            check(&mut c, "remove_peer");
            c.set_custom_data(&h, vec![]).unwrap();
            check(&mut c, "clear");
            assert!(expected.next().is_none());
        }};
    }
    // Matching inherent method names preserve the real wrapper settlement path.
    trait ReferencePeer {
        fn add_series_for_reference(&mut self) -> feather_charts::model::data_layer::SeriesId;
    }
    impl ReferencePeer for OptionsChart {
        fn add_series_for_reference(&mut self) -> feather_charts::model::data_layer::SeriesId {
            self.add_series(
                feather_charts::model::series_options::SeriesOptionsMap::Line(Default::default()),
                0,
                PriceScalePosition::Right,
            )
            .unwrap()
        }
    }
    impl ReferencePeer for YieldCurveChart {
        fn add_series_for_reference(&mut self) -> feather_charts::model::data_layer::SeriesId {
            self.add_line_series(Default::default(), 0, PriceScalePosition::Left)
                .unwrap()
        }
    }
    sequence!(
        OptionsChart,
        OptionsChart::new_options(Default::default()).unwrap(),
        View::<HorizontalPrice, HorizontalPrice>::default(),
        "price",
        [90., 100., 150., 175., 200.],
        p,
        HorizontalPrice::value,
        PriceScalePosition::Right
    );
    sequence!(
        YieldCurveChart,
        YieldCurveChart::new(Default::default()).unwrap(),
        View::<Maturity, Maturity>::default(),
        "yield",
        [1., 6., 360., 420., 480.],
        |v| Maturity::from_months(v).unwrap(),
        Maturity::months,
        PriceScalePosition::Left
    );
}

#[test]
fn numeric_events_errors_and_cleanup_keep_owned_payloads() {
    use feather_charts::model::icustom_series::*;
    let mut c = OptionsChart::new_options(Default::default()).unwrap();
    let view = View::<HorizontalPrice, HorizontalPrice>::default();
    let destroyed = view.destroyed.clone();
    let h = c
        .add_custom_series(view, Default::default(), 0, PriceScalePosition::Right)
        .unwrap();
    c.set_custom_data(
        &h,
        vec![datum(p(90.), 2.), datum(p(100.), 3.), datum(p(150.), 4.)],
    )
    .unwrap();
    c.fit_content();
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let x = c
        .model()
        .time_scale()
        .index_to_coordinate(1.0.into())
        .value();
    let y = c.model().panes()[0]
        .right_price_scale()
        .price_to_coordinate(3., 2.)
        .value();
    let events = c
        .update(ChartMessage::Click {
            pane: 0,
            position: iced::Point::new(x as f32, y as f32),
        })
        .unwrap();
    let ChartEvent::Click(event) = &events[0] else {
        panic!("missing click")
    };
    assert_eq!(event.time, Some(p(100.)));
    assert_eq!(event.custom_data(&h).unwrap().value, 3.);
    assert_eq!(event.custom_hit_data(&h), Some(&3.));
    for bad in [
        vec![datum(p(100.), 2.), datum(p(90.), 1.)],
        vec![datum(p(90.), 1.), datum(p(90.), 2.)],
        vec![datum(p(200.), f64::NAN)],
    ] {
        assert!(c.set_custom_data(&h, bad).is_err());
        assert_eq!(h.data_at(1.0.into()).unwrap().value, 3.);
    }
    let mut other = OptionsChart::new_options(Default::default()).unwrap();
    assert!(other.set_custom_data(&h, vec![]).is_err());
    c.update_custom_data(
        &h,
        CustomSeriesDataItem::Whitespace(CustomSeriesWhitespaceData {
            time: p(100.),
            custom_values: None,
        }),
        true,
    )
    .unwrap();
    assert!(h.data_at(1.0.into()).is_none());
    let old = c.frame().clone();
    let clone = h.clone();
    c.remove_series(h.id()).unwrap();
    assert_eq!(destroyed.get(), 1);
    assert!(c.set_custom_data(&clone, vec![]).is_err());
    assert_eq!(h.data_at(0.0.into()).unwrap().value, 2.);
    assert_eq!(event.custom_data(&h).unwrap().value, 3.);
    assert!(old.plot.custom.is_some());
    drop(c);
    drop(h);
    drop(clone);
    assert_eq!(destroyed.get(), 1);
}

#[test]
fn numeric_custom_reducer_contexts_and_sparse_fallback() {
    use feather_charts::model::time_scale_options::HorzScaleOptionsPatch;
    macro_rules! run {($chart:expr,$ty:ty,$key:expr,$number:expr)=>{{
        for reduce in [false,true] {
            let mut c=$chart;let mut view=View::<$ty,$ty>::default();view.reduce=reduce;view.order_sensitive=true;view.count_during_reduce=true;
            let observed=view.observed.clone();let seen=view.seen.clone();
            let h=c.add_custom_series(view,Default::default(),0,PriceScalePosition::Right).unwrap();
            c.set_custom_data(&h,(0..7).map(|i|datum($key(90.+i as f64*2.5),i as f64+1.)).collect()).unwrap();
            c.apply_time_scale_options(HorzScaleOptionsPatch {enable_conflation:Some(true),min_bar_spacing:Some(0.001),bar_spacing:Some(0.125),..Default::default()});
            c.prepare(Size::new(800.,500.),1.,Instant::now()).unwrap();
            let initial=observed.borrow().last().unwrap().1.clone();
            assert_eq!(initial,if reduce{vec![2107.]}else{vec![1.,2.,3.,4.,5.,6.,7.]});
            assert_eq!(h.data_at(c.model().time_scale().time_to_index(&$key(105.),false).unwrap()).unwrap().value,7.);
            for (original,internal) in seen.borrow().iter(){assert_eq!($number(*original),$number(*internal));}
            assert!(c.update_custom_data(&h,datum($key(92.5),4.),true).is_err());
            c.update_custom_data(&h,datum($key(105.),9.),false).unwrap();c.prepare(Size::new(800.,500.),1.,Instant::now()).unwrap();
            let latest=observed.borrow().last().unwrap().1.clone();assert_eq!(latest,if reduce{vec![2109.]}else{vec![1.,2.,3.,4.,5.,6.,9.]});
            println!("NUMERIC_CUSTOM_REDUCTION {{\"reduce\":{reduce},\"initial\":{initial:?},\"latest\":{latest:?}}}");
            c.apply_time_scale_options(HorzScaleOptionsPatch {enable_conflation:Some(false),..Default::default()});
            c.prepare(Size::new(800.,500.),1.,Instant::now()).unwrap();assert_eq!(observed.borrow().last().unwrap().1.len(),7);
        }
    }}}
    run!(
        OptionsChart::new_options(Default::default()).unwrap(),
        HorizontalPrice,
        p,
        HorizontalPrice::value
    );
    // Use unit scaffold steps matching these points so source factor comparisons are meaningful.
    run!(
        YieldCurveChart::new(YieldCurveOptions {
            base_resolution: 2.5,
            minimum_time_range: 0.,
            start_time_range: 90.
        })
        .unwrap(),
        Maturity,
        |v| Maturity::from_months(v).unwrap(),
        Maturity::months
    );
}

#[test]
fn numeric_visible_preparation_and_value_updates_have_bounded_payload_work() {
    use feather_charts::model::time_data::LogicalRange;
    use std::{cell::Cell, rc::Rc};
    let copies = Rc::new(Cell::new(0));
    let mut c = OptionsChart::new_options(Default::default()).unwrap();
    let h = c
        .add_custom_series(
            View::<HorizontalPrice, HorizontalPrice>::default(),
            Default::default(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_custom_data(
        &h,
        (0..100_000)
            .map(|i| {
                CustomSeriesDataItem::Data(Datum {
                    time: p(i as f64),
                    low: 1.,
                    high: 3.,
                    value: 2.,
                    clones: Some(copies.clone()),
                })
            })
            .collect(),
    )
    .unwrap();
    c.set_logical_range(LogicalRange {
        from: 20.0.into(),
        to: 100.0.into(),
    });
    copies.set(0);
    c.prepare(Size::new(900., 600.), 1.25, Instant::now())
        .unwrap();
    assert!(copies.get() <= 170, "copied {} payloads", copies.get());
    copies.set(0);
    c.update_custom_data(
        &h,
        CustomSeriesDataItem::Data(Datum {
            time: p(99_999.),
            low: 1.,
            high: 3.,
            value: 2.5,
            clones: Some(copies.clone()),
        }),
        false,
    )
    .unwrap();
    assert!(copies.get() <= 4, "updated {} copies", copies.get());
}

#[test]
fn yield_custom_defaults_and_overrides_do_not_inherit_line_visibility() {
    use feather_charts::model::{
        chart_model::ChartModelOptions, localization_options::LocalizationOptions,
        series_options::CustomSeriesOptions,
    };
    use std::rc::Rc;
    let mut localization = LocalizationOptions::new("en-US", "");
    localization.base.price_formatter = Some(Rc::new(|v| format!("Y{:.2}", v.value())));
    let mut c = YieldCurveChart::with_chart_options(
        Default::default(),
        ChartModelOptions::default(),
        localization,
    )
    .unwrap();
    let mut options = CustomSeriesOptions::default();
    options.common.price_line_visible = true;
    options.common.last_value_visible = true;
    let m = |v| Maturity::from_months(v).unwrap();
    let h = c
        .add_custom_series(
            View::<Maturity, Maturity>::default(),
            options,
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_custom_data(&h, vec![datum(m(1.), 5.), datum(m(360.), 5.378)])
        .unwrap();
    c.fit_content();
    let now = Instant::now();
    c.prepare(Size::new(800., 500.), 1., now).unwrap();
    assert!(!c.frame().plot.price_lines.is_empty());
    assert_eq!(
        c.model().panes()[0]
            .right_price_scale()
            .format_price(5.378, 5.),
        "Y5.38"
    );
    let storage = c.model().time_scale().index_to_time(120.0.into()).unwrap() as *const _;
    c.update_custom_data(&h, datum(m(360.), 5.4), false)
        .unwrap();
    c.prepare(Size::new(800., 500.), 1., now).unwrap();
    assert_eq!(
        storage,
        c.model().time_scale().index_to_time(120.0.into()).unwrap() as *const _
    );
    let mut options = CustomSeriesOptions::default();
    options.common.price_line_visible = false;
    options.common.last_value_visible = false;
    c.replace_custom_series_options(&h, options).unwrap();
    c.prepare(Size::new(800., 500.), 1., now).unwrap();
    assert!(c.frame().plot.price_lines.is_empty());
    c.remove_series(h.id()).unwrap();
    assert!(c.series_ids().next().is_none());
    c.prepare(Size::new(800., 500.), 1., now).unwrap();
    assert!(
        c.model()
            .time_scale()
            .time_to_index(&m(240.), false)
            .is_some()
    );
}
