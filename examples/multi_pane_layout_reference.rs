use feather_charts::{
    renderers::{grid_renderer::PixelRatio, price_axis_renderer::AxisBounds},
    ui::{chart_layout::AxisSizeRequests, multi_pane_layout::allocate_multi_pane_layout},
};
fn main() {
    let mut cases = vec![];
    for ratio in [1., 1.25, 1.5, 2.] {
        for (width, height) in [(800.75, 500.25), (100., 10.), (400., 200.)] {
            for left in [false, true] {
                for right in [false, true] {
                    for time in [false, true] {
                        for stretches in [vec![2., 1.], vec![1., 1., 1.], vec![0.01, 10., 0.1]] {
                            let layout = allocate_multi_pane_layout(
                                AxisBounds { width, height },
                                AxisSizeRequests {
                                    left_width: if left { 62. } else { 0. },
                                    right_width: if right { 80. } else { 0. },
                                    time_height: if time { 28. } else { 0. },
                                },
                                PixelRatio {
                                    horizontal: ratio,
                                    vertical: ratio,
                                },
                                &stretches,
                            )
                            .unwrap();
                            let result = layout
                                .panes
                                .iter()
                                .map(|p| format!("{},{}", p.plot.size.width, p.plot.size.height))
                                .collect::<Vec<_>>()
                                .join(",");
                            cases.push(format!("{{\"ratio\":{ratio},\"width\":{width},\"height\":{height},\"left\":{left},\"right\":{right},\"time\":{time},\"fractional\":false,\"stretches\":{stretches:?},\"result\":[{result}]}}"));
                        }
                    }
                }
            }
        }
    }
    println!("[{}]", cases.join(","));
}
