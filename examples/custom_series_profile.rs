//! Fixed-visible custom payload preparation and live append profiling.
#[path = "custom_series/box_whisker.rs"]
mod box_whisker;
use feather_charts::{
    api::Chart,
    model::{
        pane::PriceScalePosition, series_options::CustomSeriesOptions, time_data::LogicalRange,
    },
    ui::{multi_pane_frame::IcedMultiPaneChartFrame, price_axis::AxisFontResolver},
};
use std::{hint::black_box, time::Instant};
fn row(
    index: usize,
) -> feather_charts::model::icustom_series::CustomSeriesDataItem<
    box_whisker::Datum,
    feather_charts::model::horz_scale_behavior_time::types::Time,
> {
    let mut row = box_whisker::sample(index);
    if let feather_charts::model::icustom_series::CustomSeriesDataItem::Data(data) = &mut row {
        data.quartiles = [95., 98., 100., 103., 106.];
        data.outliers.clear();
    }
    row
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let number = |key: &str, defaults: Vec<usize>| {
        args.iter()
            .position(|a| a == key)
            .map_or(defaults, |at| vec![args[at + 1].parse().unwrap()])
    };
    let histories = number("--rows", vec![1200, 20_000, 100_000]);
    let sources = number("--series", vec![1, 16]);
    let samples = number("--samples", vec![100])[0];
    let phase = args
        .iter()
        .position(|a| a == "--phase")
        .map_or("static", |at| args[at + 1].as_str());
    assert!(["static", "latest", "append"].contains(&phase) && samples > 0);
    println!(
        "custom-profile warmup=20 samples={samples} logical=20..100 size=917x870.80005 ratio=1.25 backend=none"
    );
    for &sources in &sources {
        for &rows in &histories {
            let mut chart = Chart::default();
            let mut handles = Vec::new();
            for _ in 0..sources {
                let handle = chart
                    .add_custom_series(
                        box_whisker::View::default(),
                        CustomSeriesOptions::default(),
                        0,
                        PriceScalePosition::Right,
                    )
                    .unwrap();
                chart
                    .set_custom_data(
                        &handle,
                        (0..rows)
                            .map(|index| {
                                if index + 1 == rows {
                                    row(index)
                                } else {
                                    box_whisker::sample(index)
                                }
                            })
                            .collect(),
                    )
                    .unwrap();
                handles.push(handle);
            }
            let size = iced::Size::new(917., 870.80005);
            chart.set_logical_range(LogicalRange {
                from: 20.0.into(),
                to: 100.0.into(),
            });
            chart.prepare(size, 1.25, Instant::now()).unwrap();
            let fonts = AxisFontResolver::default();
            let mut timings: [Vec<f64>; 3] = std::array::from_fn(|_| Vec::with_capacity(samples));
            for sample in 0..samples + 20 {
                let data = (phase != "static").then(|| {
                    row(if phase == "latest" {
                        rows - 1
                    } else {
                        rows + sample
                    })
                });
                let start = Instant::now();
                if let Some(data) = data {
                    chart.update_custom_data(&handles[0], data, false).unwrap();
                }
                let updated = Instant::now();
                let snapshot = chart.prepare_snapshot(size, 1.25, updated).unwrap();
                let prepared = Instant::now();
                black_box(
                    IcedMultiPaneChartFrame::from_snapshot(snapshot, &fonts, "#eee").unwrap(),
                );
                let converted = Instant::now();
                if sample == 0 {
                    println!(
                        "cold Custom rows={rows} sources={sources} phase={phase}/update ms={:.6}",
                        (updated - start).as_secs_f64() * 1000.
                    );
                }
                if sample >= 20 {
                    for (bucket, duration) in timings.iter_mut().zip([
                        updated - start,
                        prepared - updated,
                        converted - prepared,
                    ]) {
                        bucket.push(duration.as_secs_f64() * 1000.);
                    }
                }
            }
            for (name, mut values) in ["update", "snapshot", "glyph"].into_iter().zip(timings) {
                println!(
                    "raw Custom rows={rows} sources={sources} phase={phase}/{name} ms={values:?}"
                );
                values.sort_by(f64::total_cmp);
                println!(
                    "Custom rows={rows} sources={sources} phase={phase}/{name} median_ms={:.6} p95_ms={:.6}",
                    values[samples / 2],
                    values[(samples * 95 / 100).min(samples - 1)]
                );
            }
        }
    }
}
