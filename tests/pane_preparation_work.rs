use feather_charts::{
    api::Chart,
    model::{
        data_consumer::{
            BuiltInSeriesDataItem, HistogramData, HistogramDataItem, LineData, LineDataItem,
        },
        horz_scale_behavior_time::types::UtcTimestamp,
        pane::PriceScalePosition,
        series_options::{
            HistogramSeriesOptions, LineSeriesOptions, PriceFormat, PriceFormatCustom,
            SeriesOptionsMap,
        },
        time_data::LogicalRange,
    },
};
use std::{cell::Cell, collections::BTreeSet, rc::Rc, time::Instant};

fn pane_formatter_calls(sources: usize, histogram: bool) -> usize {
    let calls = Rc::new(Cell::new(0));
    let mut chart = Chart::default();
    let mut ids = BTreeSet::new();
    for source in 0..sources {
        let mut options = if histogram {
            SeriesOptionsMap::Histogram(HistogramSeriesOptions::default())
        } else {
            SeriesOptionsMap::Line(LineSeriesOptions::default())
        };
        let common = match &mut options {
            SeriesOptionsMap::Histogram(options) => &mut options.common,
            SeriesOptionsMap::Line(options) => &mut options.common,
            _ => unreachable!(),
        };
        // Additional sources contribute geometry, not additional label content.
        common.last_value_visible = false;
        common.price_line_visible = false;
        if source == 0 {
            let calls = calls.clone();
            common.price_format = PriceFormat::Custom(PriceFormatCustom {
                formatter: Rc::new(move |price| {
                    calls.set(calls.get() + 1);
                    format!("{:.2}", price.value())
                }),
                tickmarks_formatter: None,
                min_move: 0.01,
                base: Some(100.),
            });
        }
        let id = chart
            .add_series(options, 0, PriceScalePosition::Right)
            .unwrap();
        ids.insert(id);
        chart
            .set_data(
                id,
                (0..1200)
                    .map(|index| {
                        let time = UtcTimestamp::new(1_700_000_000. + index as f64 * 60.).into();
                        let value = 100. + (index as f64 * 0.03).sin() * 10.;
                        if histogram {
                            BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(
                                HistogramData {
                                    time,
                                    value,
                                    color: None,
                                    custom_values: None,
                                },
                            ))
                        } else {
                            BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                                time,
                                value,
                                color: None,
                                custom_values: None,
                            }))
                        }
                    })
                    .collect(),
            )
            .unwrap();
    }
    chart.set_logical_range(LogicalRange {
        from: 20.0.into(),
        to: 100.0.into(),
    });
    let size = iced::Size::new(917., 870.80005);
    // Compare settled snapshots so initial font/axis allocation cannot affect counts.
    for _ in 0..2 {
        chart.prepare_snapshot(size, 1.25, Instant::now()).unwrap();
    }
    calls.set(0);
    let snapshot = chart.prepare_snapshot(size, 1.25, Instant::now()).unwrap();
    let work = calls.get();
    assert_eq!(snapshot.panes.len(), 1);
    let plot = &snapshot.panes[0].plot;
    assert!(plot.axes.is_some());
    assert!(!plot.grid.is_empty());
    assert_eq!(plot.hit_series.len(), sources);
    assert_eq!(
        plot.hit_series
            .iter()
            .map(|hit| hit.series)
            .collect::<BTreeSet<_>>(),
        ids
    );
    assert_eq!(plot.series_layers.len(), sources - 1);
    if histogram {
        assert!(!plot.candles.is_empty());
        assert!(
            plot.series_layers
                .iter()
                .all(|layer| !layer.candles.is_empty())
        );
    } else {
        assert!(!plot.lines.is_empty());
        assert!(
            plot.series_layers
                .iter()
                .all(|layer| !layer.lines.is_empty())
        );
    }
    work
}

#[test]
fn composed_pane_formatting_does_not_multiply_work_for_every_series() {
    for histogram in [false, true] {
        let sources = 16;
        let single = pane_formatter_calls(1, histogram);
        let composed = pane_formatter_calls(sources, histogram);
        assert!(
            single > 0,
            "the owner formatter must observe pane-axis work"
        );
        // The shared scale also formats legitimate per-series values; allow linear work.
        assert!(
            composed <= sources * single,
            "histogram={histogram}: {sources} series made {composed} formatter calls versus {single} for one; pane-wide decoration must not multiply per-series formatting"
        );
    }
}
