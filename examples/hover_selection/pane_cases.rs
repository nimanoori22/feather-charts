use feather_charts::{
    api::{
        Chart, HitTarget, MarkerHit, MarkerShape, MarkerZOrder, PriceLineKind, SeriesMarkersOptions,
    },
    model::{
        data_layer::SeriesId,
        pane::PriceScalePosition,
        series_options::{LineSeriesOptions, SeriesOptionsMap},
        time_data::SeriesItemsIndexesRange,
    },
    renderers::{
        draw_line::{LineStyle, LineType, LineWidth},
        line_renderer::{LinePoint, LineRendererData, Point},
        range_hit_test::RangePoint,
        series_markers_renderer::{PreparedMarker, PreparedMarkerCollection},
    },
    ui::{
        hit_testing::{HitGeometry, PreparedPriceLineHit, PreparedSeriesHit},
        line_chart::{CanvasSeriesGeometry, PlotSnapshot},
    },
};
pub fn records() -> Vec<String> {
    // Obtain a valid attachment handle; fixture geometry below remains backend independent.
    let mut chart = Chart::default();
    let owner = chart
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let collection = chart
        .add_series_markers(owner, vec![], SeriesMarkersOptions::default())
        .unwrap();
    let mut records = vec![];
    for code in 0..64 {
        let kinds = [code % 4, (code / 4) % 4, code / 16];
        for distances in [
            [0., 0., 0.],
            [2., 0., 5.],
            [5., 2., 0.],
            [0., 5., 2.],
            [2., 2., 5.],
            [9., 9., 9.],
        ] {
            for marker in -1i32..3 {
                let mut plot = PlotSnapshot {
                    size: iced::Size::new(100., 100.),
                    series_id: Some(SeriesId::new(1)),
                    ..Default::default()
                };
                plot.series_layers = (2..=3)
                    .map(|i| CanvasSeriesGeometry {
                        series_id: Some(SeriesId::new(i)),
                        ..Default::default()
                    })
                    .collect();
                for i in 0..3 {
                    let id = SeriesId::new(i as u64 + 1);
                    let d = distances[i];
                    let geometry = if kinds[i] == 0 || kinds[i] == 3 {
                        HitGeometry::Range {
                            items: vec![RangePoint {
                                index: 0.,
                                x: 50.,
                                top: 50. + d,
                                bottom: 50. + d,
                            }],
                            spacing: 20.,
                        }
                    } else {
                        let xs: &[f64] = if kinds[i] == 1 { &[0., 100.] } else { &[50.] };
                        HitGeometry::Line {
                            kind: LineType::Simple,
                            data: LineRendererData {
                                items: xs
                                    .iter()
                                    .enumerate()
                                    .map(|(n, &x)| LinePoint {
                                        index: (n as f64).into(),
                                        x,
                                        y: 50. + d,
                                        color: String::new(),
                                    })
                                    .collect(),
                                visible_range: SeriesItemsIndexesRange {
                                    from: 0,
                                    to: xs.len(),
                                },
                                bar_width: 20.,
                                line_type: Some(LineType::Simple),
                                line_width: LineWidth::One,
                                line_style: LineStyle::Solid,
                                point_markers_radius: None,
                            },
                        }
                    };
                    plot.hit_series.push(PreparedSeriesHit {
                        series: id,
                        geometry,
                        filled: false,
                        tolerance: 20.,
                    });
                    if kinds[i] == 3 {
                        plot.hit_price_lines.push(PreparedPriceLineHit {
                            series: id,
                            y: 50. + d,
                            width: 4.,
                            kind: PriceLineKind::LastValue,
                            external_id: None,
                        });
                    }
                }
                if marker >= 0 {
                    let id = SeriesId::new(marker as u64 + 1);
                    plot.annotations.push(PreparedMarkerCollection {
                        series_layer: marker as usize,
                        series: id,
                        id: collection,
                        z_order: MarkerZOrder::Top,
                        items: vec![PreparedMarker {
                            index: 0.0.into(),
                            center: Point { x: 50., y: 50. },
                            size: 10.,
                            shape: MarkerShape::Circle,
                            color: "red".into(),
                            hit: MarkerHit {
                                series: id,
                                collection,
                                index: 0,
                                external_id: None,
                            },
                            text: None,
                        }],
                        font_size: 12.,
                        font_family: "sans-serif".into(),
                    });
                }
                let hit = plot.hit_test(iced::Point::new(50., 50.)).unwrap();
                let is_marker = matches!(hit.target, HitTarget::Marker(_));
                records.push(format!("{{\"kind\":\"pane\",\"kinds\":{:?},\"distances\":{:?},\"marker\":{marker},\"series\":{},\"isMarker\":{is_marker}}}",kinds,distances,hit.series.value()-1));
            }
        }
    }
    for width in [1., 2., 3., 4.] {
        for d in [-12., -11., -8., -7., 0., 7., 8., 11., 12.] {
            let mut plot = PlotSnapshot {
                size: iced::Size::new(100., 100.),
                series_id: Some(owner),
                ..Default::default()
            };
            plot.hit_series.push(PreparedSeriesHit {
                series: owner,
                geometry: HitGeometry::Range {
                    items: vec![],
                    spacing: 20.,
                },
                filled: false,
                tolerance: 0.,
            });
            plot.hit_price_lines.push(PreparedPriceLineHit {
                series: owner,
                y: 50.,
                width,
                kind: PriceLineKind::LastValue,
                external_id: None,
            });
            records.push(format!(
                "{{\"kind\":\"horizontal\",\"width\":{width},\"offset\":{d},\"hit\":{}}}",
                plot.hit_test(iced::Point::new(50., (50. + d) as f32))
                    .is_some()
            ));
        }
    }

    for hovered in 0..3 {
        for filled in [false, true] {
            for enabled in [false, true] {
                let mut plot = PlotSnapshot {
                    series_id: Some(SeriesId::new(1)),
                    hovered_series: Some(SeriesId::new(hovered + 1)),
                    hovered_series_on_top: enabled,
                    ..Default::default()
                };
                plot.series_layers = (2..=3)
                    .map(|i| CanvasSeriesGeometry {
                        series_id: Some(SeriesId::new(i)),
                        ..Default::default()
                    })
                    .collect();
                plot.hit_series.push(PreparedSeriesHit {
                    series: SeriesId::new(hovered + 1),
                    geometry: HitGeometry::Range {
                        items: vec![],
                        spacing: 1.,
                    },
                    filled,
                    tolerance: 0.,
                });
                let passes = plot
                    .series_draw_passes()
                    .map(|p| format!("[{},{}]", p.layer, p.kind as u8))
                    .collect::<Vec<_>>()
                    .join(",");
                records.push(format!("{{\"kind\":\"order\",\"hovered\":{hovered},\"filled\":{filled},\"enabled\":{enabled},\"passes\":[{passes}]}}"));
            }
        }
    }
    records
}
