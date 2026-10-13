//! Identical public API navigation/data-update workload for pre/post comparisons.
use feather_charts::{
    api::{Chart, ChartMessage},
    model::{
        data_consumer::{
            BuiltInSeriesDataItem, CandlestickData, CandlestickDataItem, HistogramData,
            HistogramDataItem, LineData, LineDataItem,
        },
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{
            CandlestickSeriesOptions, HistogramSeriesOptions, LineSeriesOptions, SeriesOptionsMap,
            SeriesType,
        },
    },
    ui::chart_input::{ChartInteraction, Navigation},
};
use std::time::Instant;
fn row(kind: SeriesType, index: usize) -> BuiltInSeriesDataItem<Time> {
    let time = Time::from(UtcTimestamp::new(1_700_000_000. + index as f64 * 60.));
    let value = 100. + (index as f64 * 0.03).sin() * 10.;
    match kind {
        SeriesType::Line => BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time,
            value,
            color: None,
            custom_values: None,
        })),
        SeriesType::Candlestick => {
            BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(CandlestickData {
                time,
                open: value - 1.,
                high: value + 2.,
                low: value - 2.,
                close: value,
                color: None,
                border_color: None,
                wick_color: None,
                custom_values: None,
            }))
        }
        SeriesType::Histogram => {
            BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
                time,
                value,
                color: None,
                custom_values: None,
            }))
        }
        _ => unreachable!(),
    }
}
fn prepare(chart: &mut Chart) {
    chart
        .prepare(iced::Size::new(917., 870.80005), 1.25, Instant::now())
        .unwrap();
}
fn main() {
    for kind in [
        SeriesType::Line,
        SeriesType::Candlestick,
        SeriesType::Histogram,
    ] {
        for updates in [false, true] {
            let mut c = Chart::default();
            let options = match kind {
                SeriesType::Line => SeriesOptionsMap::Line(LineSeriesOptions::default()),
                SeriesType::Candlestick => {
                    SeriesOptionsMap::Candlestick(CandlestickSeriesOptions::default())
                }
                SeriesType::Histogram => {
                    SeriesOptionsMap::Histogram(HistogramSeriesOptions::default())
                }
                _ => unreachable!(),
            };
            let id = c.add_series(options, 0, PriceScalePosition::Right).unwrap();
            c.set_data(id, (0..1200).map(|i| row(kind, i)).collect())
                .unwrap();
            c.fit_content();
            prepare(&mut c);
            let mut samples = vec![];
            for i in 0..120 {
                let start = Instant::now();
                if updates {
                    c.update_data(id, row(kind, 1200 + i), false).unwrap();
                    prepare(&mut c);
                } else {
                    c.update(ChartMessage::Interaction(ChartInteraction {
                        position: Some(iced::Point::new(200., 100.)),
                        navigation: Some(Navigation::Wheel {
                            x: 200.,
                            zoom: if i % 2 == 0 { 0.32 } else { -0.32 / 1.032 },
                            scroll: 0.,
                        }),
                        capture: true,
                    }))
                    .unwrap();
                }
                if i >= 20 {
                    samples.push(start.elapsed().as_secs_f64() * 1000.);
                }
                assert!(!c.frame().plot.lines.is_empty() || !c.frame().plot.candles.is_empty());
            }
            samples.sort_by(f64::total_cmp);
            println!(
                "{kind:?} {} model+frame+glyph median {:.3} ms p95 {:.3} ms",
                if updates { "append" } else { "zoom" },
                samples[50],
                samples[95]
            );
        }
    }
}
