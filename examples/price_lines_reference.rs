//! Differential fixtures for the original price-line and label algorithms.
use feather_charts::{
    api::Chart,
    model::{
        axis_snapshots::{FloatingAxisLabel, PriceAxisSide, SeriesAxisLabel},
        price_line::PriceLine,
        text_width_cache::{TextMeasurer, TextMetrics},
    },
    renderers::{
        draw_line::{LineStyle, LineWidth},
        grid_renderer::PixelRatio,
        horizontal_line_renderer,
        line_renderer::PathOperation,
        price_axis_labels::align_labels,
        price_axis_renderer::{
            AxisBounds, PriceAxisCommand, measure_price_axis, prepare_pane_price_labels,
            prepare_price_axis,
        },
    },
};
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure_text(&mut self, text: &str) -> TextMetrics {
        TextMetrics {
            width: text.len() as f32 * 7.,
            ..Default::default()
        }
    }
}
fn label(y: f64) -> SeriesAxisLabel {
    SeriesAxisLabel {
        axis: FloatingAxisLabel {
            coordinate: y,
            text: "123.45".into(),
            background: "blue".into(),
            color: "white".into(),
        },
        pane_text: "Entry".into(),
        render_coordinate: y,
    }
}
fn main() {
    let mut cases = vec![];
    for r in [1., 1.25, 1.5, 2.] {
        let ratio = PixelRatio {
            horizontal: r,
            vertical: r,
        };
        for width in [
            LineWidth::One,
            LineWidth::Two,
            LineWidth::Three,
            LineWidth::Four,
        ] {
            for y in [-1., -0.25, 0., 30.8, 60., 61.] {
                let line = PriceLine {
                    coordinate: y,
                    color: "red".into(),
                    width,
                    style: LineStyle::Dashed,
                };
                let commands = horizontal_line_renderer::draw_commands(
                    &line,
                    AxisBounds {
                        width: 100.,
                        height: 60.,
                    },
                    ratio,
                );
                let result = if let Some(stroke) = commands {
                    let [PathOperation::MoveTo(a), PathOperation::LineTo(b)] =
                        stroke.path.as_slice()
                    else {
                        panic!()
                    };
                    format!("[{}, {}, {}, {}, {}]", a.x, a.y, b.x, b.y, stroke.width)
                } else {
                    "[]".into()
                };
                cases.push(format!(
                    "{{\"kind\":\"line\",\"ratio\":{r},\"width\":{},\"y\":{y},\"result\":{result}}}",
                    width.pixels()
                ));
            }
        }
        let mut chart = Chart::default();
        let snapshot = chart
            .prepare_snapshot(iced::Size::new(800., 500.), r, std::time::Instant::now())
            .unwrap();
        let base = snapshot.panes[0].plot.axes.as_ref().unwrap().right.clone();
        for left in [false, true] {
            for pane in [false, true] {
                for y in [-2., 0., 30.8, 79., 82.] {
                    let mut axis = base.clone();
                    axis.marks.clear();
                    axis.crosshair_width_labels.clear();
                    axis.crosshair_label = None;
                    axis.side = if left {
                        PriceAxisSide::Left
                    } else {
                        PriceAxisSide::Right
                    };
                    axis.labels = vec![label(y)];
                    axis.align_labels = false;
                    let m = measure_price_axis(&axis, &mut Metrics).unwrap();
                    let bounds = AxisBounds {
                        width: 100.,
                        height: 80.,
                    };
                    let prepared = if pane {
                        prepare_pane_price_labels(&m, bounds, ratio).unwrap()
                    } else {
                        prepare_price_axis(&m, bounds, ratio).unwrap()
                    };
                    let rect = prepared
                        .commands
                        .iter()
                        .find_map(|c| {
                            if let PriceAxisCommand::RoundedRectangle { rect, .. } = c {
                                Some(rect)
                            } else {
                                None
                            }
                        })
                        .unwrap();
                    let text = prepared
                        .commands
                        .iter()
                        .find_map(|c| {
                            if let PriceAxisCommand::Text { position, .. } = c {
                                Some(position)
                            } else {
                                None
                            }
                        })
                        .unwrap();
                    let o = &axis.renderer_options;
                    cases.push(format!("{{\"kind\":\"label\",\"ratio\":{r},\"left\":{left},\"pane\":{pane},\"y\":{y},\"options\":[{},{},{},{},{},{},{}],\"result\":[{},{},{},{},{},{}]}}",
                        o.font_size,o.border_size,o.tick_length,o.padding_top,o.padding_bottom,o.padding_inner,o.padding_outer,
                        rect.origin.x,rect.origin.y,rect.size.width,rect.size.height,text.x,text.y));
                }
            }
        }
    }
    for height in [20., 80., 300.] {
        for h in [17., 25.] {
            for center in [0., height / 2., height] {
                for coords in [
                    vec![],
                    vec![0.],
                    vec![-8., 0., 1., 2., height - 1., height + 2.],
                    vec![20., 20., 20., 20.],
                    vec![1., 3., 40., 42., 44., height - 5.],
                ] {
                    let mut labels = coords.iter().copied().map(label).collect::<Vec<_>>();
                    align_labels(&mut labels, center, height, h);
                    let positions = labels
                        .iter()
                        .map(|l| l.render_coordinate)
                        .collect::<Vec<_>>();
                    cases.push(format!("{{\"kind\":\"align\",\"height\":{height},\"labelHeight\":{h},\"center\":{center},\"coordinates\":{coords:?},\"result\":{positions:?}}}"));
                }
            }
        }
    }
    use feather_charts::model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem, WhitespaceData},
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{LineSeriesOptions, SeriesOptionsMap},
    };
    for right in [0., 1., 2., 4., 5., 8., 9., 12.] {
        let mut chart = Chart::default();
        let id = chart
            .add_series(
                SeriesOptionsMap::Line(LineSeriesOptions::default()),
                0,
                PriceScalePosition::Right,
            )
            .unwrap();
        chart
            .set_data(
                id,
                (0..13)
                    .map(|i| {
                        let time = Time::from(UtcTimestamp::new(i as f64));
                        if [0, 2, 5, 9, 12].contains(&i) {
                            BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                                time,
                                value: 100. + i as f64,
                                color: Some("red".into()),
                                custom_values: None,
                            }))
                        } else {
                            BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(WhitespaceData {
                                time,
                                custom_values: None,
                            }))
                        }
                    })
                    .collect(),
            )
            .unwrap();
        chart.fit_content();
        chart
            .prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
            .unwrap();
        let visible = Some(feather_charts::model::range_impl::RangeImpl::<
            feather_charts::model::time_data::TimePointIndex,
        >::new(0.0.into(), right.into()));
        let actual_right = visible.as_ref().unwrap().right().value();
        for global in [false, true] {
            let row = chart
                .model()
                .series(id)
                .unwrap()
                .last_value_data(
                    chart.model().panes()[0].right_price_scale(),
                    visible.as_ref(),
                    global,
                )
                .unwrap();
            cases.push(format!("{{\"kind\":\"last\",\"right\":{actual_right},\"global\":{global},\"result\":[{},{}]}}", row.index.value(),row.price));
        }
    }
    println!("[{}]", cases.join(",\n"));
}
