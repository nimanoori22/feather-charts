use feather_charts::{
    api::*,
    model::{
        data_consumer::*,
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::*,
    },
};
fn chart() -> (Chart, feather_charts::model::data_layer::SeriesId) {
    let mut c = Chart::default();
    let mut o = LineSeriesOptions::default();
    o.common.price_line_visible = false;
    let id = c
        .add_series(SeriesOptionsMap::Line(o), 0, PriceScalePosition::Right)
        .unwrap();
    c.set_data(
        id,
        (0..10)
            .map(|i| {
                BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                    time: Time::from(UtcTimestamp::new(1000. + i as f64 * 10.)),
                    value: 100. + i as f64,
                    color: None,
                    custom_values: None,
                }))
            })
            .collect(),
    )
    .unwrap();
    c.fit_content();
    c.prepare(iced::Size::new(640., 480.), 1.25, std::time::Instant::now())
        .unwrap();
    (c, id)
}
#[test]
fn settled_line_hit_is_owned_and_public_click_reports_it() {
    let (mut c, id) = chart();
    let stroke = &c.frame().plot.lines[0];
    let a = match stroke.path[0] {
        feather_charts::renderers::line_renderer::PathOperation::MoveTo(p) => p,
        _ => panic!("move"),
    };
    let p = iced::Point::new(a.x as f32, a.y as f32);
    let hit = c.hit_test(0, p).unwrap();
    assert_eq!(hit.series, id);
    assert!(matches!(
        hit.target,
        HitTarget::SeriesLine | HitTarget::SeriesPoint
    ));
    let events = c
        .update(ChartMessage::Click {
            pane: 0,
            position: p,
        })
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e,ChartEvent::Click(data) if data.hit.as_ref()==Some(&hit)))
    );
    c.set_data(id, vec![]).unwrap();
    c.prepare(iced::Size::new(640., 480.), 1.25, std::time::Instant::now())
        .unwrap();
    assert!(c.hit_test(0, p).is_none());
    assert_eq!(hit.series, id);
}

fn options(kind: SeriesType) -> SeriesOptionsMap {
    macro_rules! make {
        ($variant:ident,$ty:ident) => {{
            let mut o = $ty::default();
            o.common.price_line_visible = false;
            SeriesOptionsMap::$variant(o)
        }};
    }
    match kind {
        SeriesType::Line => make!(Line, LineSeriesOptions),
        SeriesType::Area => make!(Area, AreaSeriesOptions),
        SeriesType::Baseline => make!(Baseline, BaselineSeriesOptions),
        SeriesType::Bar => make!(Bar, BarSeriesOptions),
        SeriesType::Candlestick => make!(Candlestick, CandlestickSeriesOptions),
        SeriesType::Histogram => make!(Histogram, HistogramSeriesOptions),
        _ => unreachable!(),
    }
}
fn row(kind: SeriesType, i: usize) -> BuiltInSeriesDataItem<Time> {
    let time = Time::from(UtcTimestamp::new(1000. + i as f64 * 10.));
    let v = 100. + i as f64;
    match kind {
        SeriesType::Line => BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time,
            value: v,
            color: None,
            custom_values: None,
        })),
        SeriesType::Area => BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
            time,
            value: v,
            line_color: None,
            top_color: None,
            bottom_color: None,
            custom_values: None,
        })),
        SeriesType::Baseline => {
            BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(BaselineData {
                time,
                value: v,
                top_fill_color1: None,
                top_fill_color2: None,
                bottom_fill_color1: None,
                bottom_fill_color2: None,
                top_line_color: None,
                bottom_line_color: None,
                custom_values: None,
            }))
        }
        SeriesType::Bar => BuiltInSeriesDataItem::Bar(BarDataItem::Data(BarData {
            time,
            open: v - 1.,
            high: v + 2.,
            low: v - 2.,
            close: v,
            color: None,
            custom_values: None,
        })),
        SeriesType::Candlestick => {
            BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(CandlestickData {
                time,
                open: v - 1.,
                high: v + 2.,
                low: v - 2.,
                close: v,
                color: None,
                border_color: None,
                wick_color: None,
                custom_values: None,
            }))
        }
        SeriesType::Histogram => {
            BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
                time,
                value: v,
                color: None,
                custom_values: None,
            }))
        }
        _ => unreachable!(),
    }
}
fn position(
    c: &Chart,
    id: feather_charts::model::data_layer::SeriesId,
    index: f64,
    price: f64,
) -> iced::Point {
    let x = c
        .model()
        .time_scale()
        .index_to_coordinate(index.into())
        .value();
    let pane = c.model().pane_for_series(id).unwrap();
    let scale = c.model().panes()[pane].right_price_scale();
    let first = 100.;
    iced::Point::new(
        x as f32,
        scale.price_to_coordinate(price, first).value() as f32,
    )
}
#[test]
fn every_family_hits_its_source_geometry_at_fractional_ratios() {
    for kind in [
        SeriesType::Line,
        SeriesType::Area,
        SeriesType::Baseline,
        SeriesType::Bar,
        SeriesType::Candlestick,
        SeriesType::Histogram,
    ] {
        let mut c = Chart::default();
        let id = c
            .add_series(options(kind), 0, PriceScalePosition::Right)
            .unwrap();
        c.set_data(id, (0..10).map(|i| row(kind, i)).collect())
            .unwrap();
        c.fit_content();
        for ratio in [1., 1.25, 1.5, 2.] {
            c.prepare(
                iced::Size::new(640., 480.),
                ratio,
                std::time::Instant::now(),
            )
            .unwrap();
            let hit = c
                .hit_test(0, position(&c, id, 3., 103.))
                .expect("visible series hit");
            assert_eq!(hit.series, id);
            if matches!(
                kind,
                SeriesType::Bar | SeriesType::Candlestick | SeriesType::Histogram
            ) {
                assert_eq!(hit.target, HitTarget::SeriesRange);
            }
        }
    }
}
