use feather_charts::{
    api::Chart,
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{LineSeriesOptions, SeriesOptionsMap},
        time_data::LogicalRange,
        time_scale_options::HorzScaleOptionsPatch,
    },
};
use std::time::Instant;
fn row(i: usize, value: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: UtcTimestamp::new(1_700_000_000. + i as f64 * 60.).into(),
        value,
        color: None,
        custom_values: None,
    }))
}
#[test]
fn dense_line_reduces_geometry_without_changing_canonical_rows() {
    let mut chart = Chart::default();
    let id = chart
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(id, (0..8192).map(|i| row(i, (i % 31) as f64)).collect())
        .unwrap();
    chart.apply_time_scale_options(HorzScaleOptionsPatch {
        min_bar_spacing: Some(0.001),
        enable_conflation: Some(true),
        ..Default::default()
    });
    chart.set_logical_range(LogicalRange {
        from: 0.0.into(),
        to: 8191.0.into(),
    });
    chart
        .prepare(iced::Size::new(1000., 500.), 1., Instant::now())
        .unwrap();
    let feather_charts::ui::hit_testing::HitGeometry::Line { data, .. } =
        &chart.frame().plot.hit_series[0].geometry
    else {
        panic!("expected line")
    };
    assert!(data.items.len() <= 2050);
    assert_eq!(chart.model().series(id).unwrap().bars().rows().len(), 8192);
}

#[test]
fn historical_rejection_uses_effective_level_and_preserves_data() {
    let mut chart = Chart::default();
    let id = chart
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(id, (0..32).map(|i| row(i, i as f64)).collect())
        .unwrap();
    chart.apply_time_scale_options(HorzScaleOptionsPatch {
        min_bar_spacing: Some(0.001),
        bar_spacing: Some(0.125),
        enable_conflation: Some(true),
        ..Default::default()
    });
    assert!(chart.update_data(id, row(1, 500.), true).is_err());
    use feather_charts::model::plot_data::PlotRowLike;
    assert_eq!(
        chart.model().series(id).unwrap().bars().rows()[1].values()[3],
        1.
    );
    chart.apply_time_scale_options(HorzScaleOptionsPatch {
        bar_spacing: Some(6.),
        ..Default::default()
    });
    chart.update_data(id, row(1, 500.), true).unwrap();
    assert_eq!(
        chart.model().series(id).unwrap().bars().rows()[1].values()[3],
        500.
    );
}

fn sample(kind: usize, i: usize, value: f64) -> BuiltInSeriesDataItem<Time> {
    use feather_charts::model::data_consumer::*;
    let time = UtcTimestamp::new(1_700_000_000. + i as f64 * 60.).into();
    match kind {
        0 => row(i, value),
        1 => BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(CandlestickData {
            time,
            open: value - 1.,
            high: value + 2.,
            low: value - 2.,
            close: value,
            color: None,
            border_color: None,
            wick_color: None,
            custom_values: None,
        })),
        2 => BuiltInSeriesDataItem::Bar(BarDataItem::Data(BarData {
            time,
            open: value - 1.,
            high: value + 2.,
            low: value - 2.,
            close: value,
            color: None,
            custom_values: None,
        })),
        3 => BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
            time,
            value,
            color: None,
            custom_values: None,
        })),
        4 => BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
            time,
            value,
            line_color: None,
            top_color: None,
            bottom_color: None,
            custom_values: None,
        })),
        5 => BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(BaselineData {
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
        _ => unreachable!(),
    }
}
fn options(kind: usize) -> SeriesOptionsMap {
    match kind {
        0 => SeriesOptionsMap::Line(Default::default()),
        1 => SeriesOptionsMap::Candlestick(Default::default()),
        2 => SeriesOptionsMap::Bar(Default::default()),
        3 => SeriesOptionsMap::Histogram(Default::default()),
        4 => SeriesOptionsMap::Area(Default::default()),
        5 => SeriesOptionsMap::Baseline(Default::default()),
        _ => unreachable!(),
    }
}
fn frame_points(frame: &feather_charts::ui::chart_frame::IcedChartFrame) -> Vec<(f64, f64, f64)> {
    use feather_charts::ui::hit_testing::HitGeometry;
    match &frame.plot.hit_series[0].geometry {
        HitGeometry::Line { data, .. } => data
            .items
            .iter()
            .map(|p| (p.index.value(), p.x, p.y))
            .collect(),
        HitGeometry::Range { items, .. } => {
            items.iter().map(|p| (p.index, p.top, p.bottom)).collect()
        }
        _ => unreachable!(),
    }
}
fn points(chart: &Chart) -> Vec<(f64, f64, f64)> {
    frame_points(chart.frame())
}
fn settle(chart: &mut Chart) {
    chart
        .prepare(iced::Size::new(600., 400.), 1., Instant::now())
        .unwrap();
    // Normalize the viewport after axes have settled their label widths.
    chart.set_logical_range(LogicalRange {
        from: 0.0.into(),
        to: 4095.0.into(),
    });
    chart
        .prepare(iced::Size::new(600., 400.), 1., Instant::now())
        .unwrap();
}
#[test]
fn every_builtin_reduces_work_and_live_cache_matches_fresh() {
    for kind in 0..6 {
        let build = |last: f64| {
            let mut chart = Chart::default();
            // Keep cache comparisons independent of retained axis label widths.
            chart
                .apply_price_axis_options(
                    0,
                    feather_charts::model::axis_snapshots::PriceAxisSide::Right,
                    feather_charts::model::price_scale::PriceScaleOptionsPatch {
                        minimum_width: Some(100.),
                        ..Default::default()
                    },
                )
                .unwrap();
            let id = chart
                .add_series(options(kind), 0, PriceScalePosition::Right)
                .unwrap();
            chart
                .set_data(
                    id,
                    (0..4096)
                        .map(|i| {
                            sample(
                                kind,
                                i,
                                if i == 4095 {
                                    last
                                } else {
                                    30. + (i % 31) as f64
                                },
                            )
                        })
                        .collect(),
                )
                .unwrap();
            chart.apply_time_scale_options(HorzScaleOptionsPatch {
                min_bar_spacing: Some(0.001),
                enable_conflation: Some(true),
                ..Default::default()
            });
            chart.set_logical_range(LogicalRange {
                from: 0.0.into(),
                to: 4095.0.into(),
            });
            (chart, id)
        };
        let (mut chart, id) = build(50.);
        settle(&mut chart);
        assert!(!points(&chart).is_empty());
        assert!(points(&chart).len() <= 1026, "kind={kind}");
        let old = points(&chart);
        let retained = chart.frame().clone();
        chart
            .update_data(id, sample(kind, 4095, 200.), false)
            .unwrap();
        settle(&mut chart);
        let (mut fresh, fresh_id) = build(200.);
        settle(&mut fresh);
        assert_eq!(points(&chart), points(&fresh), "kind={kind}");
        assert_eq!(frame_points(&retained), old);
        chart
            .update_data(id, sample(kind, 4096, 100.), false)
            .unwrap();
        settle(&mut chart);
        assert_eq!(chart.model().series(id).unwrap().bars().rows().len(), 4097);
        fresh
            .update_data(fresh_id, sample(kind, 4096, 100.), false)
            .unwrap();
        settle(&mut fresh);
        assert_eq!(points(&chart), points(&fresh), "append kind={kind}");
        chart
            .set_data(
                id,
                (0..4096)
                    .map(|i| sample(kind, i, 30. + (i % 31) as f64))
                    .collect(),
            )
            .unwrap();
        let (mut reset, reset_id) = build(30. + (4095 % 31) as f64);
        settle(&mut reset);
        settle(&mut chart);
        assert_eq!(points(&chart), points(&reset), "reload kind={kind}");
        chart.remove_series(id).unwrap();
        reset.remove_series(reset_id).unwrap();
    }
}

#[test]
fn optional_warmup_yields_between_frames_and_cancels_old_data() {
    let mut chart = Chart::default();
    let id = chart
        .add_series(options(0), 0, PriceScalePosition::Right)
        .unwrap();
    chart
        .set_data(id, (0..2048).map(|i| row(i, i as f64)).collect())
        .unwrap();
    chart.apply_time_scale_options(HorzScaleOptionsPatch {
        enable_conflation: Some(true),
        precompute_conflation_on_init: Some(true),
        min_bar_spacing: Some(0.001),
        ..Default::default()
    });
    chart
        .prepare(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    assert!(
        chart.is_animating(),
        "warmup must remain pending after one bounded step"
    );
    chart
        .set_data(id, (0..2048).map(|i| row(i, 10. + i as f64)).collect())
        .unwrap();
    let mut steps = 0;
    while chart.is_animating() {
        chart
            .update(feather_charts::api::ChartMessage::Tick(Instant::now()))
            .unwrap();
        steps += 1;
        assert!(steps < 100, "warmup failed to finish");
    }
    assert!(steps > 1);
    assert_eq!(chart.model().series(id).unwrap().bars().rows().len(), 2048);
}

#[test]
fn exact_events_and_series_override_survive_dense_dpr_transitions() {
    use feather_charts::api::{ChartEvent, ChartMessage};
    let mut chart = Chart::default();
    let mut opts = LineSeriesOptions::default();
    opts.common.conflation_threshold_factor = Some(0.25);
    let id = chart
        .add_series(SeriesOptionsMap::Line(opts), 0, PriceScalePosition::Right)
        .unwrap();
    chart
        .set_data(id, (0..4096).map(|i| row(i, i as f64)).collect())
        .unwrap();
    chart.apply_time_scale_options(HorzScaleOptionsPatch {
        enable_conflation: Some(true),
        min_bar_spacing: Some(0.001),
        conflation_threshold_factor: Some(Some(4.)),
        ..Default::default()
    });
    for ratio in [1., 1.25, 1.5, 2.] {
        settle(&mut chart);
        chart
            .prepare(iced::Size::new(600., 400.), ratio, Instant::now())
            .unwrap();
        let factor = chart.model().time_scale().conflation_factor();
        assert!(factor >= 8);
        assert!(
            points(&chart).len() > 512,
            "series threshold must override global threshold"
        );
        let x = chart
            .model()
            .time_scale()
            .index_to_coordinate(2049.0.into())
            .value() as f32;
        let events = chart
            .update(ChartMessage::Click {
                pane: 0,
                position: iced::Point::new(x, 100.),
            })
            .unwrap();
        let pointer = events
            .iter()
            .find_map(|e| {
                if let ChartEvent::Click(p) = e {
                    Some(p)
                } else {
                    None
                }
            })
            .unwrap();
        let (_, BuiltInSeriesDataItem::Line(LineDataItem::Data(data))) = &pointer.series_data[0]
        else {
            panic!("canonical line data")
        };
        assert_eq!(data.value, 2049.);
    }
    chart.apply_time_scale_options(HorzScaleOptionsPatch {
        enable_conflation: Some(false),
        ..Default::default()
    });
    settle(&mut chart);
    assert!(points(&chart).len() > 4000);
    chart.set_data(id, vec![]).unwrap();
    chart
        .prepare(iced::Size::new(600., 30.), 1., Instant::now())
        .unwrap();
    chart.remove_series(id).unwrap();
}

#[test]
fn dense_families_recover_from_short_sizes_and_preserve_price_modes() {
    use feather_charts::model::{
        axis_snapshots::PriceAxisSide,
        price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
    };
    for kind in 0..6 {
        let mut chart = Chart::default();
        let id = chart
            .add_series(options(kind), 0, PriceScalePosition::Right)
            .unwrap();
        chart
            .set_data(
                id,
                (0..2048)
                    .map(|i| sample(kind, i, 100. + (i % 31) as f64))
                    .collect(),
            )
            .unwrap();
        chart.apply_time_scale_options(HorzScaleOptionsPatch {
            enable_conflation: Some(true),
            min_bar_spacing: Some(0.001),
            bar_spacing: Some(0.125),
            ..Default::default()
        });
        for mode in [
            PriceScaleMode::Normal,
            PriceScaleMode::Logarithmic,
            PriceScaleMode::Percentage,
            PriceScaleMode::IndexedTo100,
        ] {
            chart
                .apply_price_axis_options(
                    0,
                    PriceAxisSide::Right,
                    PriceScaleOptionsPatch {
                        mode: Some(mode),
                        invert_scale: Some(true),
                        ..Default::default()
                    },
                )
                .unwrap();
            for ratio in [1., 1.25, 1.5, 2.] {
                for size in [iced::Size::new(300., 30.), iced::Size::new(600., 400.)] {
                    chart.prepare(size, ratio, Instant::now()).unwrap();
                    assert!(
                        points(&chart)
                            .iter()
                            .all(|p| p.0.is_finite() && p.1.is_finite() && p.2.is_finite())
                    );
                }
            }
        }
    }
}

#[test]
fn shared_time_reindex_invalidates_warm_aggregates() {
    let build = |earlier: bool| {
        let mut chart = Chart::default();
        chart
            .apply_price_axis_options(
                0,
                feather_charts::model::axis_snapshots::PriceAxisSide::Right,
                feather_charts::model::price_scale::PriceScaleOptionsPatch {
                    minimum_width: Some(100.),
                    ..Default::default()
                },
            )
            .unwrap();
        let id = chart
            .add_series(options(0), 0, PriceScalePosition::Right)
            .unwrap();
        let peer = chart
            .add_series(options(0), 0, PriceScalePosition::Right)
            .unwrap();
        chart
            .set_data(
                id,
                (0..2048).map(|i| row(i, 100. + (i % 31) as f64)).collect(),
            )
            .unwrap();
        if earlier {
            chart
                .set_data(
                    peer,
                    vec![BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                        time: UtcTimestamp::new(1_699_999_940.).into(),
                        value: 100.,
                        color: None,
                        custom_values: None,
                    }))],
                )
                .unwrap();
        }
        chart.apply_time_scale_options(HorzScaleOptionsPatch {
            enable_conflation: Some(true),
            min_bar_spacing: Some(0.001),
            ..Default::default()
        });
        settle(&mut chart);
        (chart, peer)
    };
    let (mut chart, peer) = build(false);
    let old = chart.frame().clone();
    let before = points(&chart);
    chart
        .set_data(
            peer,
            vec![BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                time: UtcTimestamp::new(1_699_999_940.).into(),
                value: 100.,
                color: None,
                custom_values: None,
            }))],
        )
        .unwrap();
    settle(&mut chart);
    let (fresh, _) = build(true);
    assert_eq!(points(&chart), points(&fresh));
    assert_eq!(frame_points(&old), before);
    assert_eq!(points(&chart)[0].0, 1.);
}
