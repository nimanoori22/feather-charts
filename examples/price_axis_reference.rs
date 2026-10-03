//! Deterministic oracle cases for scripts/check_price_axis_reference.cjs.
use feather_charts::{
    model::{
        axis_snapshots::{PriceAxisSide, PriceAxisSnapshot},
        coordinate::Coordinate,
        layout_options::{Background, ColorSpace},
        price_scale::PriceMark,
        text_width_cache::{TextMeasurer, TextMetrics},
    },
    renderers::{
        grid_renderer::PixelRatio,
        price_axis_renderer::{
            AxisBounds, PriceAxisCommand, measure_price_axis, prepare_price_axis,
        },
        price_axis_renderer_options_provider::{
            PriceAxisRendererOptionsProvider, PriceAxisRendererStyle,
        },
    },
};
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure_text(&mut self, label: &str) -> TextMetrics {
        TextMetrics {
            width: label.chars().count() as f32 * 7.,
            actual_bounding_box_ascent: Some(9.),
            actual_bounding_box_descent: Some(3.),
        }
    }
}
fn main() {
    let mut cases = vec![];
    for side in [PriceAxisSide::Left, PriceAxisSide::Right] {
        for ratio in [1., 1.25, 1.5, 2.] {
            for border in [false, true] {
                for ticks in [false, true] {
                    for empty in [false, true] {
                        for gradient in [false, true] {
                            let mut provider = PriceAxisRendererOptionsProvider::new();
                            let snapshot = PriceAxisSnapshot {
                                side,
                                visible: true,
                                marks: if empty {
                                    vec![]
                                } else {
                                    [(10.5, "−123456.78"), (35.25, "0.00"), (60.75, "1234567.89")]
                                        .into_iter()
                                        .enumerate()
                                        .map(|(i, (coord, label))| PriceMark {
                                            coord: Coordinate::new(coord),
                                            label: label.into(),
                                            logical: i as f64,
                                        })
                                        .collect()
                                },
                                background: if gradient {
                                    Background::VerticalGradient {
                                        top_color: "#fff".into(),
                                        bottom_color: "#ddd".into(),
                                    }
                                } else {
                                    Background::Solid {
                                        color: "#fff".into(),
                                    }
                                },
                                color_space: ColorSpace::Srgb,
                                border_visible: border,
                                border_color: "#123".into(),
                                ticks_visible: ticks,
                                minimum_width: 0.,
                                renderer_options: provider
                                    .options(PriceAxisRendererStyle {
                                        font_size: 12.,
                                        font_family: "sans-serif",
                                        text_color: "red",
                                        pane_background_color: "#fff",
                                    })
                                    .clone(),
                            };
                            let measured = measure_price_axis(&snapshot, &mut Metrics).unwrap();
                            let prepared = prepare_price_axis(
                                &measured,
                                AxisBounds {
                                    width: 100.,
                                    height: 80.,
                                },
                                PixelRatio {
                                    horizontal: ratio,
                                    vertical: ratio,
                                },
                            )
                            .unwrap();
                            let commands = prepared
                                .commands
                                .iter()
                                .map(|cmd| match cmd {
                                    PriceAxisCommand::Background(Background::Solid { color }) => {
                                        format!("[\"bg\",{:?}]", color)
                                    }
                                    PriceAxisCommand::Background(
                                        Background::VerticalGradient {
                                            top_color,
                                            bottom_color,
                                        },
                                    ) => {
                                        format!("[\"gradient\",{:?},{:?}]", top_color, bottom_color)
                                    }
                                    PriceAxisCommand::Rectangle { rect, color } => format!(
                                        "[\"rect\",{},{},{},{},{:?}]",
                                        rect.origin.x,
                                        rect.origin.y,
                                        rect.size.width,
                                        rect.size.height,
                                        color
                                    ),
                                    PriceAxisCommand::Text {
                                        label,
                                        position,
                                        color,
                                        ..
                                    } => format!(
                                        "[\"text\",{:?},{},{},{:?}]",
                                        label, position.x, position.y, color
                                    ),
                                })
                                .collect::<Vec<_>>()
                                .join(",");
                            cases.push(format!("{{\"left\":{},\"ratio\":{},\"border\":{},\"ticks\":{},\"empty\":{},\"gradient\":{},\"width\":{},\"commands\":[{}]}}", side == PriceAxisSide::Left, ratio, border, ticks, empty, gradient, measured.required_width(), commands));
                        }
                    }
                }
            }
        }
    }
    println!("[{}]", cases.join(","));
}
