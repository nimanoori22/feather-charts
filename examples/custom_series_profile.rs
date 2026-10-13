//! Fixed-visible custom payload preparation. Run in release without other builds.
#[path = "custom_series/box_whisker.rs"]
mod box_whisker;
use feather_charts::{
    api::Chart,
    model::{
        pane::PriceScalePosition, series_options::CustomSeriesOptions, time_data::LogicalRange,
    },
};
use std::time::Instant;
fn main() {
    for sources in [1, 16] {
        for rows in [1_200, 20_000, 100_000] {
            let mut chart = Chart::default();
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
                    .set_custom_data(&handle, (0..rows).map(|index| {
                        let mut row = box_whisker::sample(index);
                        if index + 1 == rows && let feather_charts::model::icustom_series::CustomSeriesDataItem::Data(data) = &mut row {
                            data.quartiles = [95., 98., 100., 103., 106.];
                            data.outliers.clear();
                        }
                        row
                    }).collect())
                    .unwrap();
            }
            let size = iced::Size::new(900., 600.);
            chart.prepare(size, 1.25, Instant::now()).unwrap();
            chart.set_logical_range(LogicalRange {
                from: 20.0.into(),
                to: 100.0.into(),
            });
            for _ in 0..20 {
                std::hint::black_box(chart.prepare_snapshot(size, 1.25, Instant::now()).unwrap());
            }
            let mut samples = vec![];
            for _ in 0..100 {
                let start = Instant::now();
                std::hint::black_box(chart.prepare_snapshot(size, 1.25, start).unwrap());
                samples.push(start.elapsed().as_secs_f64() * 1_000.);
            }
            samples.sort_by(f64::total_cmp);
            println!(
                "sources={sources} rows={rows} median_ms={:.6} p95_ms={:.6}",
                samples[50], samples[94]
            );
        }
    }
}
