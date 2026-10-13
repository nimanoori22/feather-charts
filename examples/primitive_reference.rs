//! Backend-independent records checked against the actual TypeScript methods.
use feather_charts::model::{
    autoscale_info_impl::{AutoScaleMargins, AutoscaleInfo},
    data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
    horz_scale_behavior_time::types::{Time, UtcTimestamp},
    price_range_impl::PriceRange,
    time_data::TimePointIndex,
};
use feather_charts::{
    api::{
        Chart, DrawingPrimitive, PreparedPrimitive, PrimitiveContext, PrimitiveHitCandidate,
        PrimitiveLayer, PrimitiveOwner, PrimitiveRenderer, PrimitiveView,
    },
    model::{
        axis_snapshots::{FloatingAxisLabel, SeriesAxisLabel},
        internal_hit_test::HitTestPriority,
        pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
    },
    renderers::{price_axis_labels::align_labels, range_hit_test::RangePoint},
    ui::hit_testing::{HitGeometry, PreparedSeriesHit},
};
use iced::{Point, Size, widget::canvas::Frame};
use std::time::Instant;
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Clone)]
struct Candidate(PrimitiveHitCandidate);
impl DrawingPrimitive for Candidate {
    fn prepare(&self, _: &PrimitiveContext<'_>) -> PreparedPrimitive {
        PreparedPrimitive {
            views: vec![PrimitiveView::new(self.0.layer, self.clone())],
            ..Default::default()
        }
    }
}
impl PrimitiveRenderer for Candidate {
    fn draw(&self, _: &mut Frame) {}
    fn hit_test(&self, _: Point) -> Option<PrimitiveHitCandidate> {
        Some(self.0.clone())
    }
}
#[derive(Debug, Clone)]
struct Contribution(AutoscaleInfo);
impl DrawingPrimitive for Contribution {
    fn prepare(&self, _: &PrimitiveContext<'_>) -> PreparedPrimitive {
        Default::default()
    }
    fn autoscale_info(&self, _: TimePointIndex, _: TimePointIndex) -> Option<AutoscaleInfo> {
        Some(self.0.clone())
    }
}
#[derive(Debug, Clone)]
struct LogView {
    label: String,
    calls: Rc<RefCell<Vec<String>>>,
}
impl PrimitiveRenderer for LogView {
    fn draw(&self, _: &mut Frame) {
        self.calls.borrow_mut().push(format!("fg:{}", self.label));
    }
    fn draw_background(&self, _: &mut Frame) {
        self.calls.borrow_mut().push(format!("bg:{}", self.label));
    }
}
#[derive(Debug)]
struct Logs {
    name: &'static str,
    calls: Rc<RefCell<Vec<String>>>,
}
impl DrawingPrimitive for Logs {
    fn prepare(&self, _: &PrimitiveContext<'_>) -> PreparedPrimitive {
        PreparedPrimitive {
            views: [
                PrimitiveLayer::Bottom,
                PrimitiveLayer::Normal,
                PrimitiveLayer::Top,
            ]
            .into_iter()
            .enumerate()
            .map(|(i, layer)| {
                PrimitiveView::new(
                    layer,
                    LogView {
                        label: format!("{}:{i}", self.name),
                        calls: self.calls.clone(),
                    },
                )
            })
            .collect(),
            ..Default::default()
        }
    }
}
fn drawing_records() -> Vec<String> {
    use iced::advanced::renderer::Headless;
    let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some("tiny-skia"),
    ))
    .unwrap();
    let calls = Rc::new(RefCell::new(vec![]));
    let mut chart = Chart::default();
    let ids: Vec<_> = (0..2)
        .map(|_| {
            chart
                .add_series(
                    SeriesOptionsMap::Line(Default::default()),
                    0,
                    PriceScalePosition::Right,
                )
                .unwrap()
        })
        .collect();
    for (&id, name) in ids.iter().zip(["a", "b"]) {
        chart
            .add_series_primitive(
                id,
                Logs {
                    name,
                    calls: calls.clone(),
                },
            )
            .unwrap();
    }
    chart
        .add_pane_primitive(
            0,
            Logs {
                name: "pane",
                calls: calls.clone(),
            },
        )
        .unwrap();
    chart
        .prepare(Size::new(400., 300.), 1., Instant::now())
        .unwrap();
    let mut records = vec![];
    for filled in [false, true] {
        for raised in [false, true] {
            let mut plot = chart.frame().plot.clone();
            plot.hovered_series = Some(ids[0]);
            plot.hovered_series_on_top = raised;
            plot.hit_series = vec![PreparedSeriesHit {
                series: ids[0],
                geometry: HitGeometry::Range {
                    items: vec![],
                    spacing: 1.,
                },
                tolerance: 0.,
                filled,
            }];
            calls.borrow_mut().clear();
            plot.draw(&mut Frame::new(&renderer, plot.size));
            let output = calls
                .borrow()
                .iter()
                .map(|x| format!("\"{x}\""))
                .collect::<Vec<_>>()
                .join(",");
            records.push(format!("{{\"kind\":\"draw\",\"filled\":{filled},\"raised\":{raised},\"output\":[{output}]}}"));
        }
    }
    records
}
fn autoscale_records() -> Vec<String> {
    let mut records = vec![];
    for empty in [false, true] {
        for range_mask in 0..4 {
            for margin_mask in 0..4 {
                let mut chart = Chart::default();
                let s = chart
                    .add_series(
                        SeriesOptionsMap::Line(Default::default()),
                        0,
                        PriceScalePosition::Right,
                    )
                    .unwrap();
                if !empty {
                    chart
                        .set_data(
                            s,
                            (0..10)
                                .map(|i| {
                                    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                                        time: Time::from(UtcTimestamp::new(1000. + i as f64 * 10.)),
                                        value: 100. + i as f64,
                                        color: None,
                                        custom_values: None,
                                    }))
                                })
                                .collect(),
                        )
                        .unwrap();
                }
                for i in 0..2 {
                    chart
                        .add_series_primitive(
                            s,
                            Contribution(AutoscaleInfo {
                                price_range: (range_mask & (1 << i) != 0).then_some(PriceRange {
                                    min_value: 20. + i as f64 * 60.,
                                    max_value: 130. + i as f64 * 20.,
                                }),
                                margins: (margin_mask & (1 << i) != 0).then_some(
                                    AutoScaleMargins {
                                        above: 7. - i as f64 * 4.,
                                        below: 7. - i as f64 * 4.,
                                    },
                                ),
                            }),
                        )
                        .unwrap();
                }
                let info = chart
                    .model()
                    .series(s)
                    .unwrap()
                    .autoscale_info(TimePointIndex::new(0.), TimePointIndex::new(9.));
                let output = info
                    .map(|x| {
                        let x = x.to_raw();
                        format!(
                            "{{\"priceRange\":{},\"margins\":{}}}",
                            x.price_range
                                .map(|r| format!(
                                    "{{\"minValue\":{},\"maxValue\":{}}}",
                                    r.min_value, r.max_value
                                ))
                                .unwrap_or("null".into()),
                            x.margins
                                .map(|m| format!("{{\"above\":{},\"below\":{}}}", m.above, m.below))
                                .unwrap_or("null".into())
                        )
                    })
                    .unwrap_or("null".into());
                records.push(format!("{{\"kind\":\"autoscale\",\"empty\":{empty},\"rangeMask\":{range_mask},\"marginMask\":{margin_mask},\"output\":{output}}}"));
            }
        }
    }
    records
}
fn main() {
    let mut chart = Chart::default();
    let ids: Vec<_> = (0..2)
        .map(|_| {
            chart
                .add_series(
                    SeriesOptionsMap::Line(Default::default()),
                    0,
                    PriceScalePosition::Right,
                )
                .unwrap()
        })
        .collect();
    for &id in &ids {
        chart
            .add_series_primitive(id, Candidate(PrimitiveHitCandidate::new("fixture")))
            .unwrap();
    }
    chart
        .add_pane_primitive(0, Candidate(PrimitiveHitCandidate::new("fixture")))
        .unwrap();
    chart
        .prepare(Size::new(400., 300.), 1., Instant::now())
        .unwrap();
    let base = chart.frame().plot.clone();
    let layers = [
        PrimitiveLayer::Bottom,
        PrimitiveLayer::Normal,
        PrimitiveLayer::Top,
    ];
    let priorities = [
        HitTestPriority::Range,
        HitTestPriority::Line,
        HitTestPriority::Point,
    ];
    let mut records = vec![];
    for mask in 0..4 {
        for owner in 0..3 {
            for layer in 0..3 {
                for background in [false, true] {
                    for (priority, &hit_priority) in priorities.iter().enumerate() {
                        for distance in [0., 3., 10.] {
                            for second_owner in 0..3 {
                                for second_layer in 0..3 {
                                    let mut plot = base.clone();
                                    plot.hit_series = ids
                                        .iter()
                                        .enumerate()
                                        .filter(|(i, _)| mask & (1 << i) != 0)
                                        .map(|(i, &series)| PreparedSeriesHit {
                                            series,
                                            filled: false,
                                            tolerance: 20.,
                                            geometry: HitGeometry::Range {
                                                spacing: 20.,
                                                items: vec![RangePoint {
                                                    index: 0.,
                                                    x: 50.,
                                                    top: 50. + i as f64 * 2.,
                                                    bottom: 50. + i as f64 * 2.,
                                                }],
                                            },
                                        })
                                        .collect();
                                    let mut first = PrimitiveHitCandidate::new("first");
                                    first.layer = layers[layer];
                                    first.is_background = background;
                                    first.priority = hit_priority;
                                    first.distance = distance;
                                    let mut second = PrimitiveHitCandidate::new("second");
                                    second.layer = layers[second_layer];
                                    second.distance = 3.;
                                    // Preserve attachment chronology within the same owner.
                                    let template = plot.primitives.clone();
                                    plot.primitives.clear();
                                    for (n, (which, hit)) in
                                        [(owner, first), (second_owner, second)]
                                            .into_iter()
                                            .enumerate()
                                    {
                                        let mut attachment = template
                                            .iter()
                                            .find(|a| match a.owner {
                                                PrimitiveOwner::Series(id) => {
                                                    which < 2 && id == ids[which]
                                                }
                                                PrimitiveOwner::Pane(_) => which == 2,
                                            })
                                            .unwrap()
                                            .clone();
                                        attachment.order = n as u64;
                                        attachment.output.views =
                                            vec![PrimitiveView::new(hit.layer, Candidate(hit))];
                                        plot.primitives.push(attachment);
                                    }
                                    let winner = if let Some(h) =
                                        plot.primitive_hit(Point::new(50., 50.))
                                    {
                                        format!("\"{}\"", h.external_id)
                                    } else if let Some(h) = plot.hit_test(Point::new(50., 50.)) {
                                        format!(
                                            "\"builtin{}\"",
                                            ids.iter().position(|id| *id == h.series).unwrap()
                                        )
                                    } else {
                                        "null".into()
                                    };
                                    records.push(format!("{{\"kind\":\"hit\",\"mask\":{mask},\"owner\":{owner},\"layer\":{layer},\"background\":{background},\"priority\":{priority},\"distance\":{distance},\"secondOwner\":{second_owner},\"secondLayer\":{second_layer},\"winner\":{winner}}}"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    for coordinates in [
        [2., 4., 6., 98.],
        [45., 46., 48., 50.],
        [-40., 0., 100., 140.],
    ] {
        for fixed in [false, true] {
            for padding in [0., 4., 4.5] {
                let mut labels: Vec<_> = coordinates
                    .into_iter()
                    .enumerate()
                    .map(|(i, coordinate)| SeriesAxisLabel {
                        source: None,
                        fixed_coordinate: (fixed && i == 1).then_some(35.),
                        tick_visible: None,
                        additional_padding: padding,
                        axis: FloatingAxisLabel {
                            coordinate,
                            text: "x".into(),
                            background: "blue".into(),
                            color: "white".into(),
                        },
                        pane_text: String::new(),
                        render_coordinate: coordinate,
                    })
                    .collect();
                align_labels(&mut labels, 50., 100., 17.);
                let output: Vec<_> = labels.iter().map(|l| l.render_coordinate).collect();
                records.push(format!("{{\"kind\":\"labels\",\"coordinates\":{coordinates:?},\"fixed\":{fixed},\"padding\":{padding},\"output\":{output:?}}}"));
            }
        }
    }
    records.extend(autoscale_records());
    records.extend(drawing_records());
    let markers = chart
        .add_series_markers(ids[0], vec![], Default::default())
        .unwrap();
    for (layer_index, &layer) in layers.iter().enumerate() {
        for (priority_index, &priority) in [HitTestPriority::Range, HitTestPriority::Point]
            .iter()
            .enumerate()
        {
            for marker_first in [false, true] {
                use feather_charts::{
                    api::{HitTarget, MarkerHit, MarkerShape, MarkerZOrder},
                    renderers::{
                        line_renderer::Point as RendererPoint,
                        series_markers_renderer::{PreparedMarker, PreparedMarkerCollection},
                    },
                };
                let mut plot = base.clone();
                plot.primitives
                    .retain(|a| a.owner == PrimitiveOwner::Series(ids[0]));
                let mut hit = PrimitiveHitCandidate::new("primitive");
                hit.layer = layer;
                hit.priority = priority;
                plot.primitives[0].order = 1;
                plot.primitives[0].output.views = vec![PrimitiveView::new(layer, Candidate(hit))];
                plot.marker_orders
                    .insert(markers, if marker_first { 0 } else { 2 });
                plot.annotations = vec![PreparedMarkerCollection {
                    series_layer: 0,
                    series: ids[0],
                    id: markers,
                    z_order: MarkerZOrder::Top,
                    font_size: 12.,
                    font_family: "sans-serif".into(),
                    items: vec![PreparedMarker {
                        index: 0.0.into(),
                        center: RendererPoint { x: 50., y: 50. },
                        size: 10.,
                        shape: MarkerShape::Circle,
                        color: "red".into(),
                        text: None,
                        hit: MarkerHit {
                            series: ids[0],
                            collection: markers,
                            index: 0,
                            external_id: Some("marker".into()),
                        },
                    }],
                }];
                let winner = if plot.primitive_hit(Point::new(50., 50.)).is_some() {
                    "primitive"
                } else {
                    assert!(matches!(
                        plot.hit_test(Point::new(50., 50.)).unwrap().target,
                        HitTarget::Marker(_)
                    ));
                    "marker"
                };
                records.push(format!("{{\"kind\":\"marker\",\"layer\":{layer_index},\"priority\":{priority_index},\"markerFirst\":{marker_first},\"winner\":\"{winner}\"}}"));
            }
        }
    }
    println!("[{}]", records.join(","));
}
