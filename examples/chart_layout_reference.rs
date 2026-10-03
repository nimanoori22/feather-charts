//! Pure source layout cases for scripts/check_chart_layout_reference.cjs.
use feather_charts::{
    renderers::{grid_renderer::PixelRatio, price_axis_renderer::AxisBounds},
    ui::chart_layout::{AxisSizeRequests, allocate_chart_layout},
};
fn main() {
    let mut cases = vec![];
    for ratio in [1., 1.25, 1.5, 2.] {
        for (width, height) in [(800.75, 500.25), (100., 10.), (400., 200.)] {
            for left in [false, true] {
                for right in [false, true] {
                    for time in [false, true] {
                        for fractional in [false, true] {
                            let requests = AxisSizeRequests {
                                left_width: if left {
                                    if fractional { 83. } else { 62. }
                                } else {
                                    0.
                                },
                                right_width: if right { 80. } else { 0. },
                                time_height: if time {
                                    if fractional { 33. } else { 28. }
                                } else {
                                    0.
                                },
                            };
                            let layout = allocate_chart_layout(
                                AxisBounds { width, height },
                                requests,
                                PixelRatio {
                                    horizontal: ratio,
                                    vertical: ratio,
                                },
                            )
                            .unwrap();
                            cases.push(format!("{{\"ratio\":{},\"width\":{},\"height\":{},\"left\":{},\"right\":{},\"time\":{},\"fractional\":{},\"result\":[{},{},{},{},{}]}}",ratio,width,height,left,right,time,fractional,
                layout.plot.size.width,layout.plot.size.height,layout.left_axis.size.width,layout.right_axis.size.width,layout.time_axis.size.height));
                        }
                    }
                }
            }
        }
    }
    println!("[{}]", cases.join(","));
}
