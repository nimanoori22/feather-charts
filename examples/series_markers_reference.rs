use feather_charts::{
    api::{Chart, MarkerPosition, MarkerShape, SeriesMarker, SeriesMarkersOptions},
    model::{
        data_consumer::{BuiltInSeriesDataItem, CandlestickData, CandlestickDataItem},
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{CandlestickSeriesOptions, SeriesOptionsMap},
    },
    renderers::{
        line_renderer::Point,
        series_markers_renderer::{MarkerShapeCommand, PreparedMarker},
    },
};
fn main() {
    let mut cases = vec![];
    for spacing in [0.5, 6., 12., 12.1, 20., 30., 100.] {
        for shape in [
            MarkerShape::Circle,
            MarkerShape::Square,
            MarkerShape::ArrowUp,
            MarkerShape::ArrowDown,
        ] {
            for size in [-1., 0., 0.1, 1., 2.5] {
                let mut c = Chart::default();
                let series = c
                    .add_series(
                        SeriesOptionsMap::Candlestick(CandlestickSeriesOptions::default()),
                        0,
                        PriceScalePosition::Right,
                    )
                    .unwrap();
                c.set_data(
                    series,
                    (0..10)
                        .map(|i| {
                            BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(
                                CandlestickData {
                                    time: Time::from(UtcTimestamp::new(i as f64)),
                                    open: 100.,
                                    high: 110.,
                                    low: 90.,
                                    close: 105.,
                                    color: None,
                                    border_color: None,
                                    wick_color: None,
                                    custom_values: None,
                                },
                            ))
                        })
                        .collect(),
                )
                .unwrap();
                c.fit_content();
                c.prepare_snapshot(iced::Size::new(800., 500.), 1., std::time::Instant::now())
                    .unwrap();
                c.apply_time_scale_options(
                    feather_charts::model::time_scale_options::HorzScaleOptionsPatch {
                        bar_spacing: Some(spacing),
                        ..Default::default()
                    },
                );
                let positions = [
                    MarkerPosition::AboveBar,
                    MarkerPosition::AboveBar,
                    MarkerPosition::BelowBar,
                    MarkerPosition::BelowBar,
                    MarkerPosition::InBar,
                    MarkerPosition::AtPriceTop(103.),
                    MarkerPosition::AtPriceBottom(103.),
                    MarkerPosition::AtPriceMiddle(103.),
                ];
                let names = [
                    "aboveBar",
                    "aboveBar",
                    "belowBar",
                    "belowBar",
                    "inBar",
                    "atPriceTop",
                    "atPriceBottom",
                    "atPriceMiddle",
                ];
                let collection = c
                    .add_series_markers(
                        series,
                        positions
                            .iter()
                            .map(|p| {
                                let mut m = SeriesMarker::new(
                                    Time::from(UtcTimestamp::new(8.)),
                                    *p,
                                    shape,
                                    "red",
                                );
                                m.size = size;
                                m.text = Some("test".into());
                                m
                            })
                            .collect(),
                        SeriesMarkersOptions::default(),
                    )
                    .unwrap();
                for inverted in [false, true] {
                    c.apply_price_axis_options(
                        0,
                        feather_charts::model::axis_snapshots::PriceAxisSide::Right,
                        feather_charts::model::price_scale::PriceScaleOptionsPatch {
                            invert_scale: Some(inverted),
                            ..Default::default()
                        },
                    )
                    .unwrap();
                    let snap = c
                        .prepare_snapshot(
                            iced::Size::new(800., 500.),
                            1.,
                            std::time::Instant::now(),
                        )
                        .unwrap();
                    let scale = c.model().panes()[0].right_price_scale();
                    let values =
                        [90., 103., 105., 110.].map(|p| scale.price_to_coordinate(p, 105.).value());
                    let items = &snap.panes[0].plot.annotations[0].items;
                    let placement = items
                        .iter()
                        .map(|m| {
                            format!(
                                "[{}, {}, {}]",
                                m.center.y,
                                m.size,
                                m.text.as_ref().unwrap().y
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    let margins = c
                        .model()
                        .series(series)
                        .unwrap()
                        .autoscale_info(0f64.into(), 9f64.into())
                        .unwrap()
                        .margins()
                        .unwrap();
                    cases.push(format!("{{\"kind\":\"placement\",\"spacing\":{},\"inverted\":{inverted},\"size\":{size},\"positions\":{:?},\"prices\":{:?},\"margins\":[{},{}],\"result\":[{placement}]}}",c.model().time_scale().bar_spacing(),names,values,margins.above,margins.below));
                }
                let hit = feather_charts::api::MarkerHit {
                    series,
                    collection,
                    index: 0,
                    external_id: None,
                };
                let shape_name = match shape {
                    MarkerShape::Circle => "circle",
                    MarkerShape::Square => "square",
                    MarkerShape::ArrowUp => "arrowUp",
                    MarkerShape::ArrowDown => "arrowDown",
                };
                for ratio in [0.75, 1., 1.25, 1.5, 2.] {
                    let marker = PreparedMarker {
                        index: 8f64.into(),
                        center: Point { x: -0.5, y: 20.25 },
                        size: feather_charts::model::series_markers::shape_height(spacing)
                            * size.max(0.),
                        shape,
                        color: "red".into(),
                        hit: hit.clone(),
                        text: None,
                    };
                    let command = match marker.shape_command(ratio) {
                        None => "[]".into(),
                        Some(MarkerShapeCommand::Circle { center, radius }) => {
                            format!("[\"circle\",{},{},{}]", center.x, center.y, radius)
                        }
                        Some(MarkerShapeCommand::Square { origin, size }) => {
                            format!("[\"square\",{},{},{}]", origin.x, origin.y, size)
                        }
                        Some(MarkerShapeCommand::Polygon(p)) => format!(
                            "[\"polygon\",{}]",
                            p.iter()
                                .map(|p| format!("[{},{}]", p.x, p.y))
                                .collect::<Vec<_>>()
                                .join(",")
                        ),
                    };
                    let hits = (-25..=25)
                        .step_by(5)
                        .flat_map(|dx| (-25..=25).step_by(5).map(move |dy| (dx, dy)))
                        .map(|(dx, dy)| {
                            format!(
                                "[{dx},{dy},{}]",
                                marker.hit_test(
                                    marker.center.x + dx as f64,
                                    marker.center.y + dy as f64
                                )
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    cases.push(format!("{{\"kind\":\"shape\",\"shape\":{shape_name:?},\"spacing\":{spacing},\"size\":{},\"ratio\":{ratio},\"command\":{command},\"hits\":[{hits}]}}",marker.size));
                }
            }
        }
    }
    println!("[{}]", cases.join(","));
}
