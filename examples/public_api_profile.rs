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
#[derive(Debug)]
struct ProfilePrimitive(f32);
impl feather_charts::api::DrawingPrimitive for ProfilePrimitive {
    fn prepare(
        &self,
        c: &feather_charts::api::PrimitiveContext<'_>,
    ) -> feather_charts::api::PreparedPrimitive {
        use feather_charts::api::{PreparedPrimitive, PrimitiveLayer, PrimitiveView};
        PreparedPrimitive {
            views: vec![PrimitiveView::new(
                PrimitiveLayer::Normal,
                ProfilePrimitive(self.0.min(c.size.width)),
            )],
            ..Default::default()
        }
    }
}
impl feather_charts::api::PrimitiveRenderer for ProfilePrimitive {
    fn draw(&self, f: &mut iced::widget::canvas::Frame) {
        f.fill_rectangle(
            iced::Point::new(self.0, 10.),
            iced::Size::new(20., 20.),
            iced::Color::from_rgba(0., 0., 1., 0.2),
        );
    }
    fn hit_test(&self, p: iced::Point) -> Option<feather_charts::api::PrimitiveHitCandidate> {
        (p.x >= self.0 && p.x < self.0 + 20. && p.y >= 10. && p.y < 30.)
            .then(|| feather_charts::api::PrimitiveHitCandidate::new("profile"))
    }
}
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let primitive_count = args
        .iter()
        .position(|s| s == "--primitive-count")
        .map(|i| args[i + 1].parse::<usize>().unwrap())
        .unwrap_or(0);
    if args.iter().any(|s| s == "--selection-profile") {
        selection_profile(primitive_count);
        return;
    }
    if args.iter().any(|s| s == "--empty-profile") {
        empty_profile();
        return;
    }
    let marker_count = args
        .iter()
        .position(|s| s == "--marker-count")
        .map(|i| args[i + 1].parse::<usize>().unwrap())
        .unwrap_or(0);
    let row_count = args
        .iter()
        .position(|s| s == "--rows")
        .map(|i| args[i + 1].parse::<usize>().unwrap())
        .unwrap_or(1200);
    let overlap_series = args
        .iter()
        .position(|s| s == "--overlap-series")
        .map(|i| args[i + 1].parse::<usize>().unwrap())
        .unwrap_or(1)
        .max(1);
    let hover = args.iter().any(|s| s == "--hover");
    for kind in [
        SeriesType::Line,
        SeriesType::Candlestick,
        SeriesType::Histogram,
    ] {
        for updates in [false, true] {
            let mut c = Chart::default();
            let options = || match kind {
                SeriesType::Line => SeriesOptionsMap::Line(LineSeriesOptions::default()),
                SeriesType::Candlestick => {
                    SeriesOptionsMap::Candlestick(CandlestickSeriesOptions::default())
                }
                SeriesType::Histogram => {
                    SeriesOptionsMap::Histogram(HistogramSeriesOptions::default())
                }
                _ => unreachable!(),
            };
            let id = c
                .add_series(options(), 0, PriceScalePosition::Right)
                .unwrap();
            c.set_data(id, (0..row_count).map(|i| row(kind, i)).collect())
                .unwrap();
            for _ in 1..overlap_series {
                let extra = c
                    .add_series(options(), 0, PriceScalePosition::Right)
                    .unwrap();
                c.set_data(extra, (0..row_count).map(|i| row(kind, i)).collect())
                    .unwrap();
            }
            c.fit_content();
            let primitives: Vec<_> = (0..primitive_count)
                .map(|i| {
                    c.add_series_primitive(id, ProfilePrimitive(i as f32 * 10.))
                        .unwrap()
                })
                .collect();
            prepare(&mut c);
            if marker_count > 0 {
                use feather_charts::api::{
                    MarkerPosition, MarkerShape, SeriesMarker, SeriesMarkersOptions,
                };
                let markers = (0..marker_count)
                    .map(|i| {
                        let index = i * row_count / marker_count;
                        let mut m = SeriesMarker::new(
                            Time::from(UtcTimestamp::new(1_700_000_000. + index as f64 * 60.)),
                            MarkerPosition::InBar,
                            MarkerShape::Circle,
                            "red",
                        );
                        m.text = Some("Signal".into());
                        m
                    })
                    .collect();
                c.add_series_markers(id, markers, SeriesMarkersOptions::default())
                    .unwrap();
                c.set_logical_range(feather_charts::model::time_data::LogicalRange {
                    from: (row_count.saturating_sub(400) as f64).into(),
                    to: ((row_count - 1) as f64).into(),
                });
                prepare(&mut c);
            }
            if let Some(i) = args.iter().position(|s| s == "--visible-rows") {
                let visible = args[i + 1].parse::<usize>().unwrap();
                c.set_logical_range(feather_charts::model::time_data::LogicalRange {
                    from: (row_count.saturating_sub(visible) as f64).into(),
                    to: ((row_count - 1) as f64).into(),
                });
                prepare(&mut c);
            }
            let mut samples = vec![];
            for i in 0..120 {
                let start = Instant::now();
                if updates {
                    c.update_data(id, row(kind, row_count + i), false).unwrap();
                    if let Some(&id) = primitives.first() {
                        c.replace_primitive(id, ProfilePrimitive((i % 30) as f32))
                            .unwrap();
                    }
                    prepare(&mut c);
                } else {
                    c.update(ChartMessage::Interaction(ChartInteraction {
                        position: Some(iced::Point::new(200., 100.)),
                        navigation: (!hover).then_some(Navigation::Wheel {
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
                if updates {
                    "append"
                } else if hover {
                    "hover"
                } else {
                    "zoom"
                },
                samples[50],
                samples[95]
            );
        }
    }
}

fn selection_profile(primitive_count: usize) {
    use feather_charts::ui::hit_testing::HitGeometry;
    for (name, count, rows, visible) in [
        ("empty", 0, 0, 0),
        ("single", 1, 1200, 1200),
        ("offscreen", 1, 20000, 200),
        ("many", 16, 1200, 200),
        ("crowded", 64, 1200, 200),
    ] {
        let mut chart = Chart::default();
        let mut ids = vec![];
        for _ in 0..count {
            let mut options = LineSeriesOptions::default();
            options.common.price_line_visible = false;
            let id = chart
                .add_series(
                    SeriesOptionsMap::Line(options),
                    0,
                    PriceScalePosition::Right,
                )
                .unwrap();
            chart
                .set_data(id, (0..rows).map(|i| row(SeriesType::Line, i)).collect())
                .unwrap();
            ids.push(id);
        }
        for i in 0..primitive_count {
            if ids.is_empty() {
                chart
                    .add_pane_primitive(0, ProfilePrimitive(i as f32 * 10.))
                    .unwrap();
            } else {
                chart
                    .add_series_primitive(ids[i % ids.len()], ProfilePrimitive(i as f32 * 10.))
                    .unwrap();
            }
        }
        chart.fit_content();
        if rows > visible {
            chart.set_logical_range(feather_charts::model::time_data::LogicalRange {
                from: ((rows - visible) as f64).into(),
                to: ((rows - 1) as f64).into(),
            });
        }
        prepare(&mut chart);
        let retained: usize = chart
            .frame()
            .plot
            .hit_series
            .iter()
            .map(|h| match &h.geometry {
                HitGeometry::Line { data, .. } => data.items.len(),
                HitGeometry::Range { items, .. } => items.len(),
            })
            .sum();
        let mut samples = vec![];
        for batch in 0..120 {
            let start = Instant::now();
            for i in 0..100 {
                std::hint::black_box(chart.hit_test(
                    0,
                    iced::Point::new(
                        40. + ((i * 37 + batch * 7) % 750) as f32,
                        (if primitive_count == 0 { 40. } else { 20. }) + ((i * 13) % 650) as f32,
                    ),
                ));
            }
            if batch >= 20 {
                samples.push(start.elapsed().as_secs_f64() * 1000. / 100.);
            }
        }
        samples.sort_by(f64::total_cmp);
        println!(
            "{name}: {count} series, {rows} rows/series, {retained} retained hit points, lookup median {:.6} ms p95 {:.6} ms",
            samples[50], samples[95]
        );
    }
}

fn empty_profile() {
    let mut chart = Chart::default();
    prepare(&mut chart);
    let mut samples = vec![];
    for i in 0..120 {
        let start = Instant::now();
        chart
            .update(ChartMessage::Interaction(ChartInteraction {
                position: Some(iced::Point::new(200., 100.)),
                navigation: None,
                capture: true,
            }))
            .unwrap();
        if i >= 20 {
            samples.push(start.elapsed().as_secs_f64() * 1000.);
        }
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "Empty hover model+frame+glyph median {:.3} ms p95 {:.3} ms",
        samples[50], samples[95]
    );
}
