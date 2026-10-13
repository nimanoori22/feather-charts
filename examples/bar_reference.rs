use feather_charts::{
    model::time_data::SeriesItemsIndexesRange,
    renderers::{
        bars_renderer::{BarPoint, BarsRendererData},
        grid_renderer::PixelRatio,
    },
};
fn main() {
    let mut cases = vec![];
    for h in [0.75, 1., 1.25, 1.5, 2., 3.] {
        for v in [1., 1.5, 2.] {
            for spacing in [0.5, 1., 2., 2.49, 2.5, 3., 4., 4.01, 5., 8., 20., 100.] {
                for open in [false, true] {
                    for thin in [false, true] {
                        for shape in 0..3 {
                            let items = (0..4)
                                .map(|i| BarPoint {
                                    x: -0.5 + i as f64 * spacing,
                                    open_y: 20.2,
                                    high_y: if shape == 2 { 50.5 } else { -0.5 },
                                    low_y: if shape == 2 { -0.5 } else { 50.5 },
                                    close_y: match shape {
                                        0 => 30.8,
                                        1 => 20.2,
                                        _ => 20.3,
                                    },
                                    color: format!("body{i}"),
                                })
                                .collect::<Vec<_>>();
                            for (from, to) in [(0, 4), (1, 3), (2, 2)] {
                                let data = BarsRendererData {
                                    items: items.clone(),
                                    visible_range: SeriesItemsIndexesRange { from, to },
                                    bar_spacing: spacing,
                                    open_visible: open,
                                    thin_bars: thin,
                                };
                                let result = data
                                    .draw_commands(PixelRatio {
                                        horizontal: h as f32,
                                        vertical: v as f32,
                                    })
                                    .unwrap();
                                let bars=items.iter().map(|b|format!("{{\"x\":{},\"openY\":{},\"highY\":{},\"lowY\":{},\"closeY\":{},\"barColor\":{:?}}}",b.x,b.open_y,b.high_y,b.low_y,b.close_y,b.color)).collect::<Vec<_>>().join(",");
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
                                cases.push(format!("{{\"h\":{h},\"v\":{v},\"bars\":[{bars}],\"barSpacing\":{spacing},\"openVisible\":{open},\"thinBars\":{thin},\"visibleRange\":{{\"from\":{from},\"to\":{to}}},\"result\":[{rects}]}}"));
                            }
                        }
                    }
                }
            }
        }
    }
    println!("[{}]", cases.join(","));
}
