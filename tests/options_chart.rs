use feather_charts::{
    api::*,
    model::{
        chart_model::ChartModelOptions,
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        pane::PriceScalePosition,
        series_options::{LineSeriesOptions, SeriesOptionsMap},
        time_data::TimePointIndex,
    },
};
use std::time::Instant;
fn price(v: f64) -> HorizontalPrice {
    HorizontalPrice::new(v).unwrap()
}
fn row(t: f64, value: f64) -> BuiltInSeriesDataItem<HorizontalPrice> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: price(t),
        value,
        color: None,
        custom_values: None,
    }))
}
#[test]
fn price_constructor_preserves_calendar_inference_and_numeric_spacing() {
    let _calendar = Chart::new(ChartModelOptions::default());
    let _yield = YieldCurveChart::new(Default::default()).unwrap();
    let mut c = OptionsChart::new_options(OptionsChartOptions { precision: 2 }).unwrap();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(id, vec![row(90., 1.), row(100., 2.), row(150., 3.)])
        .unwrap();
    c.fit_content();
    c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let scale = c.model().time_scale();
    let x = [0., 1., 2.].map(|i| scale.index_to_coordinate(TimePointIndex::new(i)).value());
    assert!((x[1] - x[0] - (x[2] - x[1])).abs() < 1e-9);
    assert_eq!(
        scale.index_to_time(TimePointIndex::new(2.)),
        Some(&price(150.))
    );
}
#[test]
fn precision_replacement_is_transactional() {
    assert!(OptionsChart::new_options(OptionsChartOptions { precision: 101 }).is_err());
    let mut c = OptionsChart::new_options(Default::default()).unwrap();
    c.replace_options_chart_options(OptionsChartOptions { precision: 3 })
        .unwrap();
    assert!(
        c.replace_options_chart_options(OptionsChartOptions { precision: 101 })
            .is_err()
    );
    assert_eq!(c.options_chart_options().precision, 3);
}

#[test]
fn precision_refreshes_tick_and_crosshair_labels_without_changing_prices() {
    use feather_charts::model::{
        ihorz_scale_behavior::HorzScaleBehavior, time_data::TimeScalePoint,
    };
    let mut c = OptionsChart::new_options(Default::default()).unwrap();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(id, vec![row(90., 1.), row(100.5, 2.), row(150., 3.)])
        .unwrap();
    let now = Instant::now();
    c.fit_content();
    c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
    let point = TimeScalePoint {
        time: price(100.5),
        original_time: price(100.5),
        time_weight: Default::default(),
    };
    assert_eq!(c.model().time_scale().format_date_time(&point), "101");
    let before = c.frame().time.clone();
    c.replace_options_chart_options(OptionsChartOptions { precision: 2 })
        .unwrap();
    c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
    assert_eq!(c.model().time_scale().format_date_time(&point), "100.50");
    assert_ne!(format!("{before:?}"), format!("{:?}", c.frame().time));
    assert_eq!(c.model().time_scale().behavior().options().precision, 2);
    c.set_horizontal_price_formatter(Some(std::rc::Rc::new(|p| format!("Strike {}", p.value()))));
    assert_eq!(
        c.model().time_scale().format_date_time(&point),
        "Strike 100.5"
    );
    assert_eq!(
        c.model().time_scale().behavior().format_item(&price(100.5)),
        "100.50"
    );
    c.set_horizontal_price_formatter(None);
    assert_eq!(c.model().time_scale().format_date_time(&point), "100.50");
}

#[test]
fn numeric_pointer_payloads_whitespace_and_invalid_writes_are_transactional() {
    use feather_charts::model::{data_consumer::WhitespaceData, plot_data::MutablePlotRow};
    let mut c = OptionsChart::new_options(Default::default()).unwrap();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let rows = vec![
        row(90., 1.),
        BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(WhitespaceData {
            time: price(100.),
            custom_values: None,
        })),
        row(150., 3.),
    ];
    c.set_data(id, rows).unwrap();
    for bad in [
        vec![row(100., 1.), row(90., 2.)],
        vec![row(90., 1.), row(90., 2.)],
        vec![row(100., f64::NAN)],
    ] {
        assert!(c.set_data(id, bad).is_err());
    }
    assert!(c.update_data(id, row(90., 2.), false).is_err());
    let snapshot = c.model().series(id).unwrap().bars().rows().to_vec();
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot[1].index().value(), 2.);
    let now = Instant::now();
    c.fit_content();
    c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
    for (index, expected) in [(0., 1), (1., 0)] {
        let events = c
            .update(ChartMessage::Click {
                pane: 0,
                position: iced::Point::new(
                    c.model()
                        .time_scale()
                        .index_to_coordinate(index.into())
                        .value() as f32,
                    100.,
                ),
            })
            .unwrap();
        let ChartEvent::Click(pointer) = &events[0] else {
            panic!("missing click");
        };
        assert_eq!(
            pointer.time,
            Some(price(if index == 0. { 90. } else { 100. }))
        );
        assert_eq!(pointer.series_data.len(), expected);
    }
    c.update_data(id, row(100., 2.), true).unwrap();
    c.update_data(id, row(200., 4.), false).unwrap();
    assert_eq!(c.model().series(id).unwrap().bars().rows().len(), 4);
    let retained = c.frame().clone();
    c.remove_series(id).unwrap();
    c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
    assert!(c.model().time_scale().is_empty());
    assert!(!retained.plot.lines.is_empty());
    assert!(c.frame().plot.lines.is_empty());
}

#[test]
fn all_builtin_price_families_prepare_at_short_heights_and_recover() {
    use feather_charts::model::{data_consumer::*, series_options::*};
    let now = Instant::now();
    for family in 0..6 {
        let options = match family {
            0 => SeriesOptionsMap::Line(LineSeriesOptions::default()),
            1 => SeriesOptionsMap::Area(AreaSeriesOptions::default()),
            2 => SeriesOptionsMap::Baseline(BaselineSeriesOptions::default()),
            3 => SeriesOptionsMap::Histogram(HistogramSeriesOptions::default()),
            4 => SeriesOptionsMap::Bar(BarSeriesOptions::default()),
            _ => SeriesOptionsMap::Candlestick(CandlestickSeriesOptions::default()),
        };
        let rows = [90., 100., 150.]
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                let time = price(v);
                let value = i as f64 + 2.;
                match family {
                    0 => row(v, value),
                    1 => BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
                        time,
                        value,
                        line_color: None,
                        top_color: None,
                        bottom_color: None,
                        custom_values: None,
                    })),
                    2 => BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(BaselineData {
                        time,
                        value,
                        top_fill_color1: None,
                        top_fill_color2: None,
                        top_line_color: None,
                        bottom_fill_color1: None,
                        bottom_fill_color2: None,
                        bottom_line_color: None,
                        custom_values: None,
                    })),
                    3 => BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
                        time,
                        value,
                        color: None,
                        custom_values: None,
                    })),
                    4 => BuiltInSeriesDataItem::Bar(BarDataItem::Data(BarData {
                        time,
                        open: value,
                        high: value + 1.,
                        low: value - 1.,
                        close: value + 0.5,
                        color: None,
                        custom_values: None,
                    })),
                    _ => BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(
                        CandlestickData {
                            time,
                            open: value,
                            high: value + 1.,
                            low: value - 1.,
                            close: value + 0.5,
                            color: None,
                            border_color: None,
                            wick_color: None,
                            custom_values: None,
                        },
                    )),
                }
            })
            .collect();
        let mut c = OptionsChart::new_options(Default::default()).unwrap();
        let id = c.add_series(options, 0, PriceScalePosition::Right).unwrap();
        c.set_data(id, rows).unwrap();
        c.fit_content();
        for height in [500., 90., 35., 500.] {
            c.prepare(iced::Size::new(300., height), 1.25, now).unwrap();
        }
        assert_eq!(c.model().series(id).unwrap().bars().rows().len(), 3);
    }
}

#[test]
fn price_data_layer_states_match_actual_source() {
    use feather_charts::model::{
        data_consumer::WhitespaceData, ihorz_scale_behavior::HorzScaleBehavior,
    };
    let whitespace = |t| {
        BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(WhitespaceData {
            time: price(t),
            custom_values: None,
        }))
    };
    let mut c = OptionsChart::new_options(Default::default()).unwrap();
    let a = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let b = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let mut expected = include_str!("../docs/porting/options-chart-states.tsv").lines();
    let mut check = |c: &OptionsChart, name: &str| {
        let scale = c.model().time_scale();
        let keys = (0..100)
            .map_while(|i| scale.index_to_time((i as f64).into()).copied())
            .collect::<Vec<_>>();
        let mut behavior =
            feather_charts::model::horz_scale_behavior_price::HorzScaleBehaviorPrice::default();
        let weights = keys
            .iter()
            .map(|p| behavior.append_weight(None, p).unwrap().value())
            .collect::<Vec<_>>();
        let actual = format!(
            "{name}\t{}\t{}",
            keys.iter()
                .map(|p| p.value().to_string())
                .collect::<Vec<_>>()
                .join(","),
            weights
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        );
        assert_eq!(actual, expected.next().unwrap());
    };
    c.set_data(a, vec![]).unwrap();
    check(&c, "empty");
    c.set_data(a, vec![row(90., 1.), row(100., 2.), row(150., 3.)])
        .unwrap();
    check(&c, "line");
    c.set_data(b, vec![row(95., 3.), whitespace(100.), row(175., 2.)])
        .unwrap();
    check(&c, "union");
    c.update_data(a, row(100., 4.), true).unwrap();
    check(&c, "historical");
    c.update_data(a, row(200., 4.), false).unwrap();
    check(&c, "append");
    c.update_data(a, whitespace(200.), false).unwrap();
    check(&c, "whitespace");
    c.set_data(a, vec![row(90., 1.), row(100., 2.)]).unwrap();
    check(&c, "shorter");
    c.remove_series(b).unwrap();
    check(&c, "remove");
    c.set_data(a, vec![]).unwrap();
    check(&c, "clear");
    assert!(expected.next().is_none());
}

#[test]
fn price_update_markers_expire_without_input_and_manual_markers_persist() {
    use std::time::Duration;
    let now = Instant::now();
    let mut c = OptionsChart::new_options(Default::default()).unwrap();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let markers = c
        .add_up_down_markers(
            id,
            UpDownMarkersOptions {
                update_visibility_duration: Duration::from_secs(5),
                ..Default::default()
            },
        )
        .unwrap();
    c.set_up_down_data(markers, vec![row(90., 1.), row(100., 2.)])
        .unwrap();
    c.update_up_down_data(markers, row(100., 3.), false, now)
        .unwrap();
    assert_eq!(
        c.up_down_markers(markers, now).unwrap()[0].sign,
        MarkerSign::Positive
    );
    c.update_up_down_data(markers, row(100., 2.5), false, now)
        .unwrap();
    assert_eq!(
        c.up_down_markers(markers, now).unwrap()[0].sign,
        MarkerSign::Positive
    );
    let deadline = c.next_up_down_expiration();
    assert!(
        c.update_up_down_data(markers, row(90., 4.), false, now)
            .is_err()
    );
    assert_eq!(c.next_up_down_expiration(), deadline);
    c.prepare(
        iced::Size::new(300., 300.),
        1.,
        now + Duration::from_secs(5),
    )
    .unwrap();
    assert!(
        c.up_down_markers(markers, now + Duration::from_secs(5))
            .unwrap()
            .is_empty()
    );
    c.set_up_down_markers(
        markers,
        vec![UpDownMarker {
            time: price(90.),
            value: 1.,
            sign: MarkerSign::Neutral,
        }],
    )
    .unwrap();
    assert_eq!(
        c.up_down_markers(markers, now + Duration::from_secs(50))
            .unwrap()
            .len(),
        1
    );
    assert!(c.next_up_down_expiration().is_none());
}
