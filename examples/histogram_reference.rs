use feather_charts::{
    model::time_data::SeriesItemsIndexesRange,
    renderers::{
        grid_renderer::PixelRatio,
        histogram_renderer::{HistogramPoint, HistogramRendererData},
    },
};
fn main() {
    let mut cases = vec![];
    for h in [0.75, 1., 1.25, 1.5, 2., 3.] {
        for v in [0.75, 1., 1.5, 2., 3.] {
            for spacing in [0.25, 0.5, 1., 1.01, 2., 2.49, 3., 4., 4.01, 8., 20.] {
                for phase in [-0.5, 0., 0.2, 0.5, 0.8] {
                    for gap in [false, true] {
                        let items = (0..5)
                            .map(|i| {
                                let index = (if gap && i > 1 { i + 2 } else { i }) as f64;
                                HistogramPoint {
                                    index,
                                    x: phase + index * spacing,
                                    y: [-0.5, 10.2, 20., 20.3, 50.5][i],
                                    color: format!("row{i}"),
                                }
                            })
                            .collect::<Vec<_>>();
                        for (from, to) in [(0, 5), (1, 4), (2, 2)] {
                            let data = HistogramRendererData {
                                items: items.clone(),
                                visible_range: SeriesItemsIndexesRange { from, to },
                                bar_spacing: spacing,
                                baseline: 20.,
                            };
                            let result = data
                                .draw_commands(PixelRatio {
                                    horizontal: h as f32,
                                    vertical: v as f32,
                                })
                                .unwrap();
                            let bars = items
                                .iter()
                                .map(|b| {
                                    format!(
                                        "{{\"time\":{},\"x\":{},\"y\":{},\"barColor\":{:?}}}",
                                        b.index, b.x, b.y, b.color
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(",");
                            let rects = result
                                .iter()
                                .map(|r| {
                                    format!(
                                        "[{},{},{},{},{:?}]",
                                        r.x, r.y, r.width, r.height, r.color
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(",");
                            cases.push(format!("{{\"h\":{h},\"v\":{v},\"items\":[{bars}],\"barSpacing\":{spacing},\"histogramBase\":20,\"visibleRange\":{{\"from\":{from},\"to\":{to}}},\"result\":[{rects}]}}"));
                        }
                    }
                }
            }
        }
    }
    println!("[{}]", cases.join(","));
}
