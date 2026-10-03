//! Deterministic cases consumed by scripts/check_time_axis_reference.cjs.
use feather_charts::{
    model::{
        axis_snapshots::{
            TimeAxisRendererOptions, TimeAxisSnapshot, TimeAxisTick, TimeLabelEmphasis,
        },
        ihorz_scale_behavior::TimeMark,
        layout_options::{Background, ColorSpace},
        text_width_cache::{TextMeasurer, TextMetrics},
        time_data::TickMarkWeightValue,
    },
    renderers::{
        grid_renderer::PixelRatio,
        price_axis_renderer::AxisBounds,
        time_axis_renderer::{TimeAxisCommand, measure_time_axis, prepare_time_axis},
    },
};
struct Metrics(f32);
impl TextMeasurer for Metrics {
    fn measure_text(&mut self, label: &str) -> TextMetrics {
        TextMetrics {
            width: label.chars().count() as f32 * self.0,
            ..Default::default()
        }
    }
}
fn main() {
    let mut cases = vec![];
    for ratio in [1., 1.25, 1.5, 2.] {
        for width in [8., 100., 300.] {
            for border in [false, true] {
                for ticks in [false, true] {
                    for bold in [false, true] {
                        for empty in [false, true] {
                            let snapshot = TimeAxisSnapshot {
                                visible: true,
                                ticks: if empty {
                                    vec![]
                                } else {
                                    [
                                        (1., "Jan", 70, true),
                                        (50.25, "12:30", 20, false),
                                        (99., "Feb", 70, true),
                                    ]
                                    .into_iter()
                                    .map(|(coordinate, label, weight, align)| TimeAxisTick {
                                        mark: TimeMark {
                                            coordinate,
                                            label: label.into(),
                                            weight: TickMarkWeightValue::new(weight),
                                            need_align_coordinate: align,
                                        },
                                        priority: weight >= 70,
                                        emphasis: if bold && weight >= 70 {
                                            TimeLabelEmphasis::Emphasized
                                        } else {
                                            TimeLabelEmphasis::Normal
                                        },
                                    })
                                    .collect()
                                },
                                background: Background::VerticalGradient {
                                    top_color: "#fff".into(),
                                    bottom_color: "#ddd".into(),
                                },
                                color_space: ColorSpace::Srgb,
                                border_visible: border,
                                border_color: "#123".into(),
                                ticks_visible: ticks,
                                minimum_height: 0.,
                                renderer_options: TimeAxisRendererOptions {
                                    font: "12px sans-serif".into(),
                                    bold_font: "bold 12px sans-serif".into(),
                                    font_family: "sans-serif".into(),
                                    font_size: 12.,
                                    text_color: "#456".into(),
                                    border_size: 1.,
                                    tick_length: 5.,
                                    padding_top: 3.,
                                    padding_bottom: 3.,
                                    padding_horizontal: 9.,
                                    baseline_offset: 0.,
                                    label_bottom_offset: 4.,
                                },
                            };
                            let measured =
                                measure_time_axis(&snapshot, &mut Metrics(6.), &mut Metrics(8.))
                                    .unwrap();
                            let axis = prepare_time_axis(
                                &measured,
                                AxisBounds { width, height: 40. },
                                PixelRatio {
                                    horizontal: ratio,
                                    vertical: ratio,
                                },
                            )
                            .unwrap();
                            let commands = axis
                                .commands
                                .iter()
                                .map(|c| match c {
                                    TimeAxisCommand::Background(value) => {
                                        format!("[\"bg\",{:?}]", value)
                                    }
                                    TimeAxisCommand::Rectangle { rect, color } => {
                                        format!(
                                            "[\"rect\",{},{},{},{},{:?}]",
                                            rect.origin.x,
                                            rect.origin.y,
                                            rect.size.width,
                                            rect.size.height,
                                            color
                                        )
                                    }
                                    TimeAxisCommand::Text {
                                        label,
                                        position,
                                        emphasis,
                                        color,
                                        ..
                                    } => format!(
                                        "[\"text\",{:?},{},{},{},{:?}]",
                                        label,
                                        position.x,
                                        position.y,
                                        emphasis == &TimeLabelEmphasis::Emphasized,
                                        color
                                    ),
                                })
                                .collect::<Vec<_>>()
                                .join(",");
                            cases.push(format!("{{\"ratio\":{},\"plotWidth\":{},\"border\":{},\"ticks\":{},\"bold\":{},\"empty\":{},\"height\":{},\"commands\":[{}]}}", ratio,width,border,ticks,bold,empty,axis.required_height,commands));
                        }
                    }
                }
            }
        }
    }
    println!("[{}]", cases.join(","));
}
