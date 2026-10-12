//! Deterministic navigation, crosshair geometry and magnet source oracle inputs.
use feather_charts::{
    model::{
        coordinate::Coordinate,
        crosshair::{CrosshairOptions, CrosshairPosition},
        horz_scale_behavior_time::{
            horz_scale_behavior_time::HorzScaleBehaviorTime,
            types::{Time, TimePoint, UtcTimestamp},
        },
        localization_options::LocalizationOptions,
        time_data::{TimePointIndex, TimeScalePoint},
        time_scale::{TimeScale, TimeScaleUpdate},
        time_scale_options::HorzScaleOptions,
    },
    renderers::{
        crosshair_renderer, grid_renderer::PixelRatio, line_renderer::PathOperation,
        price_axis_renderer::AxisBounds,
    },
};

struct Metrics;
impl feather_charts::model::text_width_cache::TextMeasurer for Metrics {
    fn measure_text(&mut self, text: &str) -> feather_charts::model::text_width_cache::TextMetrics {
        feather_charts::model::text_width_cache::TextMetrics {
            width: text.len() as f32 * 7.,
            ..Default::default()
        }
    }
}
fn label_cases(cases: &mut Vec<String>) {
    use feather_charts::{
        model::{
            axis_snapshots::*,
            layout_options::{Background, ColorSpace},
        },
        renderers::{
            price_axis_renderer::*, price_axis_renderer_options_provider::*, time_axis_renderer::*,
        },
    };
    for ratio in [1., 1.25, 1.5, 2.] {
        for coordinate in [1., 30.8, 99.] {
            let label = FloatingAxisLabel {
                coordinate,
                text: "98.11".into(),
                background: "#4c525e".into(),
                color: "white".into(),
            };
            let pixel_ratio = PixelRatio {
                horizontal: ratio,
                vertical: ratio,
            };
            for side in [PriceAxisSide::Left, PriceAxisSide::Right] {
                let mut provider = PriceAxisRendererOptionsProvider::new();
                let snapshot = PriceAxisSnapshot {
                    crosshair_label: Some(label.clone()),
                    crosshair_width_labels: vec![],
                    side,
                    visible: true,
                    marks: vec![],
                    background: Background::Solid {
                        color: "black".into(),
                    },
                    color_space: ColorSpace::Srgb,
                    border_visible: false,
                    border_color: "black".into(),
                    ticks_visible: true,
                    minimum_width: 0.,
                    renderer_options: provider
                        .options(PriceAxisRendererStyle {
                            font_size: 12.,
                            font_family: "sans-serif",
                            text_color: "white",
                            pane_background_color: "black",
                        })
                        .clone(),
                };
                let measurement = measure_price_axis(&snapshot, &mut Metrics).unwrap();
                let frame = prepare_price_axis(
                    &measurement,
                    AxisBounds {
                        width: 100.,
                        height: 80.,
                    },
                    pixel_ratio,
                )
                .unwrap();
                let mut result = frame
                    .commands
                    .iter()
                    .find_map(|c| {
                        if let PriceAxisCommand::RoundedRectangle { rect, .. } = c {
                            Some(vec![
                                rect.origin.x,
                                rect.origin.y,
                                rect.size.width,
                                rect.size.height,
                            ])
                        } else {
                            None
                        }
                    })
                    .unwrap();
                if let Some(rect) = frame.commands.iter().find_map(|c| {
                    if let PriceAxisCommand::Rectangle { rect, color } = c {
                        (color == "white").then_some(rect)
                    } else {
                        None
                    }
                }) {
                    result.extend([
                        rect.origin.x,
                        rect.origin.y,
                        rect.size.width,
                        rect.size.height,
                    ]);
                }
                cases.push(format!("{{\"kind\":\"price-label\",\"ratio\":{ratio},\"coordinate\":{coordinate},\"left\":{},\"result\":{result:?}}}",side==PriceAxisSide::Left));
            }
            let snapshot = TimeAxisSnapshot {
                crosshair_label: Some(label),
                visible: true,
                ticks: vec![],
                background: Background::Solid {
                    color: "black".into(),
                },
                color_space: ColorSpace::Srgb,
                border_visible: false,
                border_color: "black".into(),
                ticks_visible: true,
                minimum_height: 0.,
                renderer_options: TimeAxisRendererOptions {
                    font: "12px sans-serif".into(),
                    bold_font: "bold 12px sans-serif".into(),
                    font_family: "sans-serif".into(),
                    font_size: 12.,
                    text_color: "white".into(),
                    border_size: 1.,
                    tick_length: 5.,
                    padding_top: 3.,
                    padding_bottom: 3.,
                    padding_horizontal: 9.,
                    baseline_offset: 0.,
                    label_bottom_offset: 4.,
                },
            };
            let measurement = measure_time_axis(&snapshot, &mut Metrics, &mut Metrics).unwrap();
            let frame = prepare_time_axis(
                &measurement,
                AxisBounds {
                    width: 100.,
                    height: 40.,
                },
                pixel_ratio,
            )
            .unwrap();
            let result = frame
                .commands
                .iter()
                .find_map(|c| {
                    if let TimeAxisCommand::RoundedRectangle { rect, .. } = c {
                        Some(vec![
                            rect.origin.x,
                            rect.origin.y,
                            rect.size.width,
                            rect.size.height,
                        ])
                    } else {
                        None
                    }
                })
                .unwrap();
            cases.push(format!("{{\"kind\":\"time-label\",\"ratio\":{ratio},\"coordinate\":{coordinate},\"result\":{result:?}}}"));
        }
    }
}
fn main() {
    let mut cases = vec![];
    label_cases(&mut cases);
    for candidates in [
        vec![],
        vec![8., 12.],
        vec![12., 8.],
        vec![40., 9., 12.],
        vec![10.],
    ] {
        let result =
            feather_charts::model::magnet::nearest_coordinate(10., candidates.iter().copied());
        cases.push(format!(
            "{{\"kind\":\"magnet\",\"candidates\":{candidates:?},\"result\":{result}}}"
        ));
    }
    for width in [40., 600.] {
        for stay in [false, true] {
            for fixed in [false, true] {
                for zoom in [-1., -0.32, 0.32, 1.] {
                    let mut scale = TimeScale::new(
                        HorzScaleBehaviorTime::default(),
                        HorzScaleOptions {
                            bar_spacing: 10.,
                            right_bar_stays_on_scroll: stay,
                            fix_left_edge: fixed,
                            fix_right_edge: fixed,
                            ..Default::default()
                        },
                        LocalizationOptions::new("en-US", "dd MMM 'yy"),
                    );
                    scale.update(TimeScaleUpdate {
                        points: (0..100)
                            .map(|i| {
                                let timestamp = UtcTimestamp::new(1700000000. + i as f64 * 60.);
                                TimeScalePoint {
                                    time_weight: Default::default(),
                                    time: TimePoint {
                                        timestamp,
                                        business_day: None,
                                    },
                                    original_time: Time::from(timestamp),
                                }
                            })
                            .collect(),
                        first_changed_point_index: 0,
                    });
                    scale.set_width(width);
                    scale.set_base_index(Some(TimePointIndex::new(99.)));
                    scale.set_right_offset(-40.);
                    let initial = [scale.bar_spacing(), scale.right_offset()];
                    scale.zoom(Coordinate::new(width * 0.4), zoom);
                    let zoomed = [scale.bar_spacing(), scale.right_offset()];
                    scale.start_scroll(Coordinate::new(20.));
                    scale.scroll_to(Coordinate::new(65.));
                    scale.end_scroll();
                    cases.push(format!("{{\"kind\":\"navigation\",\"width\":{width},\"stay\":{stay},\"fixed\":{fixed},\"zoom\":{zoom},\"initial\":{:?},\"zoomed\":{:?},\"scrolled\":{:?}}}",initial,zoomed,[scale.bar_spacing(),scale.right_offset()]));
                }
            }
        }
    }
    for ratio in [1., 1.25, 1.5, 2.] {
        for width in [1, 2, 3, 4] {
            let mut options = CrosshairOptions::default();
            options.vertical.width = width.try_into().unwrap();
            options.horizontal.width = width.try_into().unwrap();
            let commands = crosshair_renderer::draw_commands(
                CrosshairPosition {
                    pane: 0,
                    index: 0.0.into(),
                    x: 20.2,
                    y: Some(30.8),
                },
                &options,
                AxisBounds {
                    width: 100.,
                    height: 60.,
                },
                PixelRatio {
                    horizontal: ratio,
                    vertical: ratio,
                },
            );
            let values = commands
                .iter()
                .map(|c| {
                    let PathOperation::MoveTo(a) = c.path[0] else {
                        unreachable!()
                    };
                    let PathOperation::LineTo(b) = c.path[1] else {
                        unreachable!()
                    };
                    vec![a.x, a.y, b.x, b.y, c.width]
                })
                .collect::<Vec<_>>();
            cases.push(format!(
                "{{\"kind\":\"lines\",\"ratio\":{ratio},\"width\":{width},\"result\":{values:?}}}"
            ));
        }
    }
    for mode in [1, 3] {
        for origin in [-5., 0., 5., 10., 15., 20., 30.] {
            for values in [
                vec![],
                vec![[8., 20., 0., 12.]],
                vec![[8., 20., 0., 12.], [12., 25., -5., 18.]],
            ] {
                let candidates = values
                    .iter()
                    .flat_map(|v| if mode == 3 { v.to_vec() } else { vec![v[3]] })
                    .collect::<Vec<_>>();
                let result = feather_charts::model::magnet::nearest_coordinate(origin, candidates);
                cases.push(format!("{{\"kind\":\"ohlc-magnet\",\"mode\":{mode},\"origin\":{origin},\"values\":{values:?},\"result\":{result}}}"));
            }
        }
    }
    println!("[{}]", cases.join(","));
}
