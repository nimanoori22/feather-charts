//! Pure source layout cases for scripts/check_chart_layout_reference.cjs.
use feather_charts::{
    renderers::{grid_renderer::PixelRatio, price_axis_renderer::AxisBounds},
    ui::chart_layout::{AxisSizeRequests, allocate_chart_layout},
};
fn main() {
    if std::env::args().any(|arg| arg == "--corners") {
        corners();
        return;
    }
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

fn corners() {
    use feather_charts::{
        model::{
            axis_snapshots::PriceAxisSide,
            chart_model::{ChartModel, ChartModelOptions},
            data_layer::DataLayer,
            horz_scale_behavior_time::horz_scale_behavior_time::HorzScaleBehaviorTime,
            layout_options::{Background, ColorSpace, LayoutOptions, LayoutPanesOptions},
            localization_options::LocalizationOptions,
            price_scale::PriceScaleOptionsPatch,
            time_scale::TimeScale,
            time_scale_options::{HorzScaleOptions, HorzScaleOptionsPatch},
        },
        renderers::time_axis_renderer::TimeAxisCommand,
        ui::chart_frame::prepare_corner,
    };
    let mut cases = vec![];
    for ratio in [1., 1.25, 1.5, 2.] {
        for left in [false, true] {
            for time in [false, true] {
                for gradient in [false, true] {
                    for side in [PriceAxisSide::Left, PriceAxisSide::Right] {
                        let mut model = ChartModel::<HorzScaleBehaviorTime>::new(
                            TimeScale::new(
                                HorzScaleBehaviorTime::default(),
                                HorzScaleOptions::default(),
                                LocalizationOptions::new("en-US", "dd MMM 'yy"),
                            ),
                            ChartModelOptions {
                                layout: LayoutOptions {
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
                                    text_color: "#000".into(),
                                    font_size: 12.,
                                    font_family: "sans-serif".into(),
                                    panes: LayoutPanesOptions {
                                        enable_resize: false,
                                        separator_color: String::new(),
                                        separator_hover_color: String::new(),
                                    },
                                    attribution_logo: false,
                                    color_space: ColorSpace::Srgb,
                                    color_parsers: vec![],
                                },
                                pane: Default::default(),
                                grid: Default::default(),
                                add_default_pane: true,
                            },
                        );
                        for axis in [PriceAxisSide::Left, PriceAxisSide::Right] {
                            model
                                .apply_price_axis_options(
                                    0,
                                    axis,
                                    PriceScaleOptionsPatch {
                                        visible: Some(true),
                                        border_visible: Some(if axis == PriceAxisSide::Left {
                                            left
                                        } else {
                                            !left
                                        }),
                                        ..Default::default()
                                    },
                                )
                                .unwrap();
                        }
                        model.apply_time_scale_options(
                            &mut DataLayer::new(HorzScaleBehaviorTime::default()),
                            HorzScaleOptionsPatch {
                                border_visible: Some(time),
                                border_color: Some("#123".into()),
                                ..Default::default()
                            },
                        );
                        let axes = model.prepare_axis_snapshots(0).unwrap();
                        let bounds = AxisBounds {
                            width: 83.5,
                            height: 28.,
                        };
                        let commands = prepare_corner(
                            &axes,
                            side,
                            bounds,
                            PixelRatio {
                                horizontal: ratio,
                                vertical: ratio,
                            },
                        )
                        .commands;
                        let output = commands
                            .into_iter()
                            .map(|command| match command {
                                TimeAxisCommand::Background(color) => {
                                    format!("[\"bg\",\"{color}\"]")
                                }
                                TimeAxisCommand::Rectangle { rect, color } => format!(
                                    "[\"rect\",{},{},{},{},\"{}\"]",
                                    rect.origin.x,
                                    rect.origin.y,
                                    rect.size.width,
                                    rect.size.height,
                                    color
                                ),
                                _ => unreachable!(),
                            })
                            .collect::<Vec<_>>();
                        cases.push(format!("{{\"ratio\":{ratio},\"leftBorder\":{left},\"timeBorder\":{time},\"gradient\":{gradient},\"side\":\"{}\",\"commands\":[{}]}}",if side==PriceAxisSide::Left {"left"} else {"right"},output.join(",")));
                    }
                }
            }
        }
    }
    println!("[{}]", cases.join(","));
}
