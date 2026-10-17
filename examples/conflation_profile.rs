//! Fixed dense-view work/timing fixture; no renderer or presentation timing.
use feather_charts::{
    api::Chart,
    model::{
        data_consumer::*,
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::*,
        time_data::LogicalRange,
        time_scale_options::HorzScaleOptionsPatch,
    },
    ui::hit_testing::HitGeometry,
};
use std::{hint::black_box, time::Instant};
fn row(histogram: bool, i: usize, revision: usize) -> BuiltInSeriesDataItem<Time> {
    let time = UtcTimestamp::new(1_700_000_000. + i as f64 * 60.).into();
    let value = 100. + (i as f64 * 0.03).sin() * 10. + revision as f64 * 0.0001;
    if histogram {
        BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
            time,
            value,
            color: None,
            custom_values: None,
        }))
    } else {
        BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time,
            value,
            color: None,
            custom_values: None,
        }))
    }
}
fn prepare(chart: &mut Chart) -> u128 {
    let start = Instant::now();
    chart
        .prepare(iced::Size::new(1000., 500.), 1., Instant::now())
        .unwrap();
    black_box(chart.frame());
    start.elapsed().as_nanos()
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let rows: usize = args[1].parse().unwrap();
    let histogram = args[2] == "histogram";
    let enabled = args[3] == "on";
    assert!([8192, 100_000].contains(&rows));
    assert!(["line", "histogram"].contains(&args[2].as_str()));
    assert!(["on", "off"].contains(&args[3].as_str()));
    let mut chart = Chart::default();
    let id = chart
        .add_series(
            if histogram {
                SeriesOptionsMap::Histogram(Default::default())
            } else {
                SeriesOptionsMap::Line(Default::default())
            },
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(id, (0..rows).map(|i| row(histogram, i, 0)).collect())
        .unwrap();
    chart.apply_time_scale_options(HorzScaleOptionsPatch {
        enable_conflation: Some(enabled),
        min_bar_spacing: Some(0.001),
        ..Default::default()
    });
    // Settle fonts/axes at factor one before the cold full-history aggregation.
    prepare(&mut chart);
    chart.set_logical_range(LogicalRange {
        from: 0.0.into(),
        to: ((rows - 1) as f64).into(),
    });
    let cold = prepare(&mut chart);
    for _ in 0..20 {
        prepare(&mut chart);
    }
    let warm: Vec<_> = (0..100).map(|_| prepare(&mut chart)).collect();
    let projected = match &chart.frame().plot.hit_series[0].geometry {
        HitGeometry::Line { data, .. } => data.items.len(),
        HitGeometry::Range { items, .. } => items.len(),
        _ => unreachable!(),
    };
    assert!(projected > 0);
    if enabled {
        assert!(projected < rows / 2);
    } else {
        assert!(projected > rows / 2);
    }
    let start = Instant::now();
    chart
        .update_data(id, row(histogram, rows - 1, 1), false)
        .unwrap();
    let latest_update = start.elapsed().as_nanos();
    let latest_prepare = prepare(&mut chart);
    let start = Instant::now();
    chart
        .update_data(id, row(histogram, rows, 1), false)
        .unwrap();
    let append_update = start.elapsed().as_nanos();
    let append_prepare = prepare(&mut chart);
    assert_eq!(
        chart.model().series(id).unwrap().bars().rows().len(),
        rows + 1
    );
    println!(
        "{{\"rows\":{rows},\"kind\":\"{}\",\"enabled\":{enabled},\"factor\":{},\"projected\":{projected},\"cold_ns\":{cold},\"warm_ns\":{warm:?},\"latest_update_ns\":{latest_update},\"latest_prepare_ns\":{latest_prepare},\"append_update_ns\":{append_update},\"append_prepare_ns\":{append_prepare}}}",
        args[2],
        chart.model().time_scale().conflation_factor()
    );
}
