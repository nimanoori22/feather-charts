//! Phase timings for fixed-visible-range preparation and incremental data updates.
use feather_charts::{
    api::Chart,
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
        time_data::LogicalRange,
    },
    ui::{multi_pane_frame::IcedMultiPaneChartFrame, price_axis::AxisFontResolver},
};
use std::{hint::black_box, time::Instant};

fn row(kind: SeriesType, index: usize, revision: usize) -> BuiltInSeriesDataItem<Time> {
    let time = Time::from(UtcTimestamp::new(1_700_000_000. + index as f64 * 60.));
    let value = 100. + (index as f64 * 0.03).sin() * 10. + revision as f64 * 0.0001;
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
fn profile_row(
    kind: SeriesType,
    index: usize,
    revision: usize,
    fixed_price: bool,
) -> BuiltInSeriesDataItem<Time> {
    let mut data = row(kind, index, revision);
    if fixed_price {
        let value = 100. + revision as f64 * 0.0001;
        match &mut data {
            BuiltInSeriesDataItem::Line(LineDataItem::Data(data)) => data.value = value,
            BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(data)) => data.value = value,
            BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(data)) => {
                data.open = value - 1.;
                data.high = value + 2.;
                data.low = value - 2.;
                data.close = value;
            }
            _ => unreachable!(),
        }
    }
    data
}
fn main() {
    // The response shape distinguishes the original full-history handoff from
    // the incremental engine when comparing builds in one Cargo target cache.
    {
        use feather_charts::model::{
            data_layer::{DataLayer, SeriesId},
            horz_scale_behavior_time::horz_scale_behavior_time::HorzScaleBehaviorTime,
        };
        let mut layer: DataLayer<HorzScaleBehaviorTime> =
            DataLayer::new(HorzScaleBehaviorTime::default());
        let id = SeriesId::new(1);
        layer
            .set_series_data(
                id,
                SeriesType::Line,
                (0..3).map(|i| row(SeriesType::Line, i, 0)).collect(),
            )
            .unwrap();
        let response = layer
            .update_series_data(id, row(SeriesType::Line, 2, 1), false)
            .unwrap();
        println!(
            "engine_sentinel existing_timestamp_response_rows={}",
            response.series[&id].rows.len()
        );
    }
    let args: Vec<_> = std::env::args().collect();
    let number = |key: &str, fallback| {
        args.iter()
            .position(|s| s == key)
            .map(|i| args[i + 1].parse::<usize>().unwrap())
            .unwrap_or(fallback)
    };
    let histories = if args.iter().any(|s| s == "--rows") {
        vec![number("--rows", 1200)]
    } else {
        vec![1200, 20_000, 100_000]
    };
    let counts = if args.iter().any(|s| s == "--series") {
        vec![number("--series", 1)]
    } else {
        vec![1, 16]
    };
    let kinds = if let Some(index) = args.iter().position(|s| s == "--kind") {
        vec![match args[index + 1].as_str() {
            "line" => SeriesType::Line,
            "candlestick" => SeriesType::Candlestick,
            "histogram" => SeriesType::Histogram,
            value => panic!("unknown profile series kind: {value}"),
        }]
    } else if args.iter().any(|s| s == "--line-only") {
        vec![SeriesType::Line]
    } else {
        vec![
            SeriesType::Line,
            SeriesType::Candlestick,
            SeriesType::Histogram,
        ]
    };
    let samples = number("--samples", 100);
    let fixed_last_price = args.iter().any(|arg| arg == "--fixed-last-price");
    let auxiliary = args.iter().any(|arg| arg == "--auxiliary-probes");
    let selected_phase = args
        .iter()
        .position(|s| s == "--phase")
        .map(|i| args[i + 1].as_str());
    assert!(selected_phase.is_none_or(|phase| ["static", "latest", "append"].contains(&phase)));
    assert!(samples > 0);
    if args.iter().any(|arg| arg == "--model-phases") {
        model_phases(&histories, &counts, &kinds, samples, selected_phase);
        return;
    }
    println!(
        "profile={} warmup=20 samples={samples} logical=20..100 size=917x870.80005 ratio=1.25 backend=none (CPU frame/glyph only)",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
    for rows in histories {
        for &count in &counts {
            for &kind in &kinds {
                for phase in ["static", "latest", "append"] {
                    if auxiliary && phase != "static" {
                        continue;
                    }
                    if selected_phase.is_some_and(|selected| selected != phase) {
                        continue;
                    }
                    let mut chart = Chart::default();
                    let mut ids = Vec::new();
                    for _ in 0..count {
                        let options = match kind {
                            SeriesType::Line => {
                                SeriesOptionsMap::Line(LineSeriesOptions::default())
                            }
                            SeriesType::Candlestick => {
                                SeriesOptionsMap::Candlestick(CandlestickSeriesOptions::default())
                            }
                            SeriesType::Histogram => {
                                SeriesOptionsMap::Histogram(HistogramSeriesOptions::default())
                            }
                            _ => unreachable!(),
                        };
                        let id = chart
                            .add_series(options, 0, PriceScalePosition::Right)
                            .unwrap();
                        chart
                            .set_data(
                                id,
                                (0..rows)
                                    .map(|i| {
                                        profile_row(kind, i, 0, fixed_last_price && i + 1 == rows)
                                    })
                                    .collect(),
                            )
                            .unwrap();
                        ids.push(id);
                    }
                    chart.set_logical_range(LogicalRange {
                        from: 20.0.into(),
                        to: 100.0.into(),
                    });
                    let size = iced::Size::new(917., 870.80005);
                    chart.prepare(size, 1.25, Instant::now()).unwrap();
                    let frame = chart.frame();
                    let hit_items: usize = frame
                        .plot
                        .hit_series
                        .iter()
                        .map(|hit| match &hit.geometry {
                            feather_charts::ui::hit_testing::HitGeometry::Line { data, .. } => {
                                data.items.len()
                            }
                            feather_charts::ui::hit_testing::HitGeometry::Range {
                                items, ..
                            } => items.len(),
                            // This built-in profile cannot count an opaque custom renderer.
                            feather_charts::ui::hit_testing::HitGeometry::Custom(_) => 0,
                        })
                        .sum();
                    println!(
                        "fixture {kind:?} rows={rows} sources={count} phase={phase} fixed_last_price={fixed_last_price} plot={:?} hit_items={hit_items} price_lines={} right_labels={}",
                        frame.layout.plot,
                        frame.plot.price_lines.len(),
                        frame
                            .plot
                            .axes
                            .as_ref()
                            .map_or(0, |axes| axes.right.labels.len())
                    );
                    if auxiliary {
                        auxiliary_probes(&mut chart, ids[0], samples);
                        continue;
                    }
                    let fonts = AxisFontResolver::default();
                    let mut phases = [Vec::new(), Vec::new(), Vec::new()];
                    for sample in 0..samples + 20 {
                        let start = Instant::now();
                        if phase != "static" {
                            let index = if phase == "latest" {
                                rows - 1
                            } else {
                                rows + sample
                            };
                            chart
                                .update_data(
                                    ids[0],
                                    profile_row(kind, index, sample, fixed_last_price),
                                    false,
                                )
                                .unwrap();
                        }
                        let updated = Instant::now();
                        let snapshot = chart.prepare_snapshot(size, 1.25, updated).unwrap();
                        let prepared = Instant::now();
                        black_box(
                            IcedMultiPaneChartFrame::from_snapshot(snapshot, &fonts, "#eee")
                                .unwrap(),
                        );
                        let converted = Instant::now();
                        if sample >= 20 {
                            phases[0].push((updated - start).as_secs_f64() * 1000.);
                            phases[1].push((prepared - updated).as_secs_f64() * 1000.);
                            phases[2].push((converted - prepared).as_secs_f64() * 1000.);
                        }
                    }
                    for (name, mut timings) in
                        ["update", "snapshot", "glyph"].into_iter().zip(phases)
                    {
                        timings.sort_by(f64::total_cmp);
                        println!(
                            "{kind:?} rows={rows} sources={count} phase={phase}/{name} median_ms={:.6} p95_ms={:.6}",
                            timings[samples / 2],
                            timings[(samples * 95 / 100).min(samples - 1)]
                        );
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct ProbeBox;
#[derive(Debug)]
struct ProbeBoxDrawing(iced::Rectangle);
impl feather_charts::api::DrawingPrimitive for ProbeBox {
    fn prepare(
        &self,
        context: &feather_charts::api::PrimitiveContext<'_>,
    ) -> feather_charts::api::PreparedPrimitive {
        use feather_charts::api::{PreparedPrimitive, PrimitiveLayer, PrimitiveView};
        let Some(transform) = context.coordinate_transform() else {
            return PreparedPrimitive::default();
        };
        let (Some(x0), Some(x1), Some(y0), Some(y1)) = (
            transform.logical_to_coordinate(25.),
            transform.logical_to_coordinate(75.),
            transform.price_to_coordinate(105.),
            transform.price_to_coordinate(109.),
        ) else {
            return PreparedPrimitive::default();
        };
        let rectangle = iced::Rectangle::new(
            iced::Point::new(x0.min(x1) as f32, y0.min(y1) as f32),
            iced::Size::new((x1 - x0).abs() as f32, (y1 - y0).abs() as f32),
        );
        PreparedPrimitive {
            views: vec![PrimitiveView::new(
                PrimitiveLayer::Normal,
                ProbeBoxDrawing(rectangle),
            )],
            ..Default::default()
        }
    }
}
impl feather_charts::api::PrimitiveRenderer for ProbeBoxDrawing {
    fn draw(&self, frame: &mut iced::widget::canvas::Frame) {
        frame.fill_rectangle(
            self.0.position(),
            self.0.size(),
            iced::Color::from_rgba(0., 0., 1., 0.2),
        );
    }
}

fn auxiliary_probes(
    chart: &mut Chart,
    series: feather_charts::model::data_layer::SeriesId,
    samples: usize,
) {
    use feather_charts::{
        api::DrawingPrimitive,
        renderers::{grid_renderer::PixelRatio, price_axis_renderer::AxisBounds},
        ui::{
            chart_frame::{AxisMeasurer, IcedAxisMeasurer},
            multi_pane_layout::allocate_multi_pane_layout,
        },
    };
    let axes = chart.frame().plot.axes.as_ref().unwrap().clone();
    let fonts = AxisFontResolver::default();
    let mut measurer = IcedAxisMeasurer { fonts: &fonts };
    let requests = measurer.measure(&axes).unwrap().requests();
    let primitive = ProbeBox;
    let id = chart.add_series_primitive(series, ProbeBox).unwrap();
    println!(
        "diagnostic=auxiliary independent font measurement, pane allocation, and box preparation; not summed with public pipeline"
    );
    let mut samples_by_phase: [Vec<f64>; 3] = std::array::from_fn(|_| Vec::new());
    for sample in 0..samples + 20 {
        let started = Instant::now();
        black_box(measurer.measure(&axes).unwrap());
        let measured = Instant::now();
        black_box(
            allocate_multi_pane_layout(
                AxisBounds {
                    width: 917.,
                    height: 870.80005,
                },
                requests,
                PixelRatio {
                    horizontal: 1.25,
                    vertical: 1.25,
                },
                &[1.],
            )
            .unwrap(),
        );
        let allocated = Instant::now();
        let context = chart.primitive_context(id).unwrap();
        black_box(primitive.prepare(&context));
        let prepared = Instant::now();
        if sample >= 20 {
            for (bucket, (from, to)) in samples_by_phase.iter_mut().zip([
                (started, measured),
                (measured, allocated),
                (allocated, prepared),
            ]) {
                bucket.push((to - from).as_secs_f64() * 1000.);
            }
        }
    }
    for (name, mut values) in ["font-measure", "pane-allocation", "box-prepare"]
        .into_iter()
        .zip(samples_by_phase)
    {
        values.sort_by(f64::total_cmp);
        println!(
            "diagnostic {name} median_ms={:.6} p95_ms={:.6}",
            values[samples / 2],
            values[(samples * 95 / 100).min(samples - 1)]
        );
    }
    chart.remove_primitive(id).unwrap();
}

fn model_phases(
    histories: &[usize],
    counts: &[usize],
    kinds: &[SeriesType],
    samples: usize,
    selected_phase: Option<&str>,
) {
    use feather_charts::{
        model::{
            chart_model::{ChartModel, ChartModelOptions},
            data_layer::{DataLayer, SeriesId},
            horz_scale_behavior_time::horz_scale_behavior_time::HorzScaleBehaviorTime,
            localization_options::LocalizationOptions,
            time_scale::TimeScale,
            time_scale_options::HorzScaleOptions,
        },
        renderers::grid_renderer::PixelRatio,
    };
    println!(
        "diagnostic=model-phases warmup=20 samples={samples} logical=20..100 model_size=800x500 ratio=1.25 backend=none; excludes Iced font measurement/layout/glyph work"
    );
    for &rows in histories {
        for &count in counts {
            for &kind in kinds {
                for phase in ["static", "latest", "append"] {
                    if selected_phase.is_some_and(|selected| selected != phase) {
                        continue;
                    }
                    let mut model: ChartModel<HorzScaleBehaviorTime> = ChartModel::new(
                        TimeScale::new(
                            HorzScaleBehaviorTime::default(),
                            HorzScaleOptions::default(),
                            LocalizationOptions::new("en-US", "dd MMM 'yy"),
                        ),
                        ChartModelOptions::default(),
                    );
                    model.set_width(800.).unwrap();
                    model.set_pane_height(0, 500.).unwrap();
                    let mut layer = DataLayer::new(HorzScaleBehaviorTime::default());
                    for i in 0..count {
                        let id = SeriesId::new(i as u64 + 1);
                        let options = match kind {
                            SeriesType::Line => {
                                SeriesOptionsMap::Line(LineSeriesOptions::default())
                            }
                            SeriesType::Candlestick => {
                                SeriesOptionsMap::Candlestick(CandlestickSeriesOptions::default())
                            }
                            SeriesType::Histogram => {
                                SeriesOptionsMap::Histogram(HistogramSeriesOptions::default())
                            }
                            _ => unreachable!(),
                        };
                        model
                            .register_series(id, kind, options, 0, PriceScalePosition::Right)
                            .unwrap();
                        let update = layer
                            .set_series_data(id, kind, (0..rows).map(|i| row(kind, i, 0)).collect())
                            .unwrap();
                        model.apply_data_update(update).unwrap();
                    }
                    model.set_logical_range(LogicalRange {
                        from: 20.0.into(),
                        to: 100.0.into(),
                    });
                    let ratio = PixelRatio {
                        horizontal: 1.25,
                        vertical: 1.25,
                    };
                    let mut timings: [Vec<f64>; 5] = std::array::from_fn(|_| Vec::new());
                    for sample in 0..samples + 20 {
                        let start = Instant::now();
                        let update = (phase != "static").then(|| {
                            let index = if phase == "latest" {
                                rows - 1
                            } else {
                                rows + sample
                            };
                            layer
                                .update_series_data(
                                    SeriesId::new(1),
                                    row(kind, index, sample),
                                    false,
                                )
                                .unwrap()
                        });
                        let normalized = Instant::now();
                        if let Some(update) = update {
                            model.apply_data_update(update).unwrap();
                        }
                        let applied = Instant::now();
                        if let Some(mask) = model.take_invalidation() {
                            model.apply_invalidation(&mask);
                        }
                        let settled = Instant::now();
                        for i in 0..count {
                            let id = SeriesId::new(i as u64 + 1);
                            match kind {
                                SeriesType::Line => {
                                    if let Some(data) = model.prepare_line(id).unwrap() {
                                        black_box(data.draw_commands(ratio));
                                    }
                                }
                                SeriesType::Candlestick => {
                                    if let Some(data) = model.prepare_candlesticks(id).unwrap() {
                                        black_box(data.draw_commands(ratio).unwrap());
                                    }
                                }
                                SeriesType::Histogram => {
                                    if let Some(data) = model.prepare_histogram(id).unwrap() {
                                        black_box(data.draw_commands(ratio).unwrap());
                                    }
                                }
                                _ => unreachable!(),
                            }
                        }
                        let projected = Instant::now();
                        black_box(model.prepare_axis_snapshots(0).unwrap());
                        let decorated = Instant::now();
                        if sample >= 20 {
                            for (bucket, (from, to)) in timings.iter_mut().zip([
                                (start, normalized),
                                (normalized, applied),
                                (applied, settled),
                                (settled, projected),
                                (projected, decorated),
                            ]) {
                                bucket.push((to - from).as_secs_f64() * 1000.);
                            }
                        }
                    }
                    for (name, mut values) in [
                        "normalize-index",
                        "model-apply",
                        "settlement",
                        "geometry-commands",
                        "axis-model",
                    ]
                    .into_iter()
                    .zip(timings)
                    {
                        values.sort_by(f64::total_cmp);
                        println!(
                            "diagnostic {kind:?} rows={rows} sources={count} phase={phase}/{name} median_ms={:.6} p95_ms={:.6}",
                            values[samples / 2],
                            values[(samples * 95 / 100).min(samples - 1)]
                        );
                    }
                }
            }
        }
    }
}
