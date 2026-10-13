//! Marker fixtures for the standalone public API example.
use feather_charts::{
    api::{
        Chart, ChartEvent, ChartMessage, MarkerCollectionId, MarkerPosition, MarkerShape,
        MarkerZOrder, PriceLineOptions, SeriesMarker, SeriesMarkersOptions,
    },
    model::{
        axis_snapshots::PriceAxisSide,
        data_layer::SeriesId,
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        price_scale::PriceScaleOptionsPatch,
    },
};
use iced::Size;
pub fn sample_markers(volume: bool) -> Vec<SeriesMarker<Time>> {
    [
        MarkerShape::ArrowUp,
        MarkerShape::ArrowDown,
        MarkerShape::Circle,
        MarkerShape::Square,
    ]
    .into_iter()
    .enumerate()
    .map(|(i, shape)| {
        let mut m = SeriesMarker::new(
            Time::from(UtcTimestamp::new(
                1_700_000_000. + (65 + i * 12) as f64 * 60.,
            )),
            if i.is_multiple_of(2) {
                MarkerPosition::BelowBar
            } else {
                MarkerPosition::AboveBar
            },
            shape,
            if i.is_multiple_of(2) {
                "#26a69a"
            } else {
                "#ef5350"
            },
        );
        m.id = Some(format!("{}-{i}", if volume { "volume" } else { "price" }));
        m.text = Some(
            if volume {
                "Event"
            } else if i.is_multiple_of(2) {
                "Buy"
            } else {
                "Sell"
            }
            .into(),
        );
        m
    })
    .collect()
}
pub fn setup_capture(
    chart: &mut Chart,
    series: SeriesId,
    id: MarkerCollectionId,
    scenario: &str,
    size: Size,
    ratio: f32,
) {
    match scenario {
        "annotation-stack"
        | "annotation-price"
        | "annotation-inverted"
        | "annotation-layers"
        | "annotation-navigation" => {
            let mut markers = sample_markers(false);
            if scenario == "annotation-stack" {
                for m in &mut markers {
                    m.time = Time::from(UtcTimestamp::new(1_700_000_000. + 80. * 60.));
                    m.text = Some("Stacked signal".into());
                }
            }
            if scenario == "annotation-price" {
                for (i, m) in markers.iter_mut().enumerate() {
                    m.position = match i % 3 {
                        0 => MarkerPosition::AtPriceTop(103.),
                        1 => MarkerPosition::AtPriceBottom(100.),
                        _ => MarkerPosition::AtPriceMiddle(97.),
                    };
                }
            }
            chart.set_series_markers(id, markers.clone()).unwrap();
            if scenario == "annotation-inverted" {
                chart
                    .apply_price_axis_options(
                        0,
                        PriceAxisSide::Right,
                        PriceScaleOptionsPatch {
                            invert_scale: Some(true),
                            ..Default::default()
                        },
                    )
                    .unwrap();
            }
            if scenario == "annotation-layers" {
                for z_order in [MarkerZOrder::AboveSeries, MarkerZOrder::Top] {
                    chart
                        .add_series_markers(
                            series,
                            markers.clone(),
                            SeriesMarkersOptions {
                                auto_scale: true,
                                z_order,
                            },
                        )
                        .unwrap();
                }
            }
            chart
                .prepare(size, ratio, std::time::Instant::now())
                .unwrap();
            if scenario == "annotation-navigation" {
                let p = chart.frame().plot.annotations[0].items[0].center;
                let events = chart
                    .update(ChartMessage::Click {
                        pane: 0,
                        position: iced::Point::new(p.x as f32, p.y as f32),
                    })
                    .unwrap();
                assert!(
                    events
                        .iter()
                        .any(|e| matches!(e, ChartEvent::Click(p) if p.marker.is_some()))
                );
                chart.zoom_time(p.x.into(), 1.);
                chart
                    .prepare(size, ratio, std::time::Instant::now())
                    .unwrap();
                let p = chart.frame().plot.annotations[0].items[0].center;
                assert!(
                    chart
                        .marker_hit(0, iced::Point::new(p.x as f32, p.y as f32))
                        .is_some()
                );
            }
            assert!(!chart.frame().plot.annotation_geometry.is_empty());
        }
        "annotation-line-overlap" => {
            let mut marker = SeriesMarker::new(
                Time::from(UtcTimestamp::new(1_700_000_000. + 80. * 60.)),
                MarkerPosition::AtPriceMiddle(100.),
                MarkerShape::Circle,
                "#ff0000",
            );
            marker.size = 2.;
            chart.set_series_markers(id, vec![marker]).unwrap();
            let mut line = PriceLineOptions::new(100.);
            line.color = "#0000ff".into();
            line.line_style = feather_charts::renderers::draw_line::LineStyle::Solid;
            chart.add_price_line(series, line).unwrap();
        }
        _ => unreachable!("known marker fixture"),
    }
}
