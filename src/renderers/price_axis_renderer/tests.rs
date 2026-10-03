use super::*;
use crate::{
    model::{coordinate::Coordinate, layout_options::ColorSpace, price_scale::PriceMark},
    renderers::price_axis_renderer_options_provider::{
        PriceAxisRendererOptionsProvider, PriceAxisRendererStyle,
    },
};

pub(crate) fn snapshot(side: PriceAxisSide) -> PriceAxisSnapshot {
    let mut provider = PriceAxisRendererOptionsProvider::new();
    PriceAxisSnapshot {
        side,
        visible: true,
        marks: vec![
            PriceMark {
                coord: Coordinate::new(20.),
                label: "12".into(),
                logical: 12.,
            },
            PriceMark {
                coord: Coordinate::new(40.),
                label: "123456789".into(),
                logical: 20.,
            },
            PriceMark {
                coord: Coordinate::new(60.),
                label: "123".into(),
                logical: 30.,
            },
        ],
        background: Background::Solid {
            color: "#fff".into(),
        },
        color_space: ColorSpace::Srgb,
        border_visible: true,
        border_color: "#123".into(),
        ticks_visible: true,
        minimum_width: 0.,
        renderer_options: provider
            .options(PriceAxisRendererStyle {
                font_size: 12.,
                font_family: "sans-serif",
                text_color: "red",
                pane_background_color: "#fff",
            })
            .clone(),
    }
}
#[derive(Default)]
struct RecordingMeasurer {
    calls: Vec<String>,
    absent_ink: bool,
}
impl TextMeasurer for RecordingMeasurer {
    fn measure_text(&mut self, text: &str) -> TextMetrics {
        self.calls.push(text.into());
        TextMetrics {
            width: text.len() as f32 * 7.,
            actual_bounding_box_ascent: (!self.absent_ink).then_some(9.),
            actual_bounding_box_descent: (!self.absent_ink).then_some(3.),
        }
    }
}
fn ratio(value: f32) -> PixelRatio {
    PixelRatio {
        horizontal: value,
        vertical: value,
    }
}
fn commands(snapshot: &PriceAxisSnapshot, r: f32) -> PreparedPriceAxis {
    let measurement = measure_price_axis(snapshot, &mut RecordingMeasurer::default()).unwrap();
    prepare_price_axis(
        &measurement,
        AxisBounds {
            width: 100.,
            height: 80.,
        },
        ratio(r),
    )
    .unwrap()
}

#[test]
fn source_endpoint_width_fallback_minimum_and_even_rounding() {
    let mut snapshot = snapshot(PriceAxisSide::Right);
    let mut recorder = RecordingMeasurer::default();
    let measured = measure_price_axis(&snapshot, &mut recorder).unwrap();
    assert_eq!(recorder.calls, ["12", "123456789", "123"]);
    // A wider interior label intentionally does not affect source endpoint sizing.
    assert_eq!(measured.required_width(), 42.); // ceil(1+5+5+5+5+21)=42
    snapshot.minimum_width = 81.;
    assert_eq!(
        measure_price_axis(&snapshot, &mut recorder)
            .unwrap()
            .required_width(),
        82.
    );
    snapshot.minimum_width = 81.5;
    assert_eq!(
        measure_price_axis(&snapshot, &mut recorder)
            .unwrap()
            .required_width(),
        83.
    );
    snapshot.minimum_width = 0.;
    snapshot.marks.clear();
    assert_eq!(
        measure_price_axis(&snapshot, &mut recorder)
            .unwrap()
            .required_width(),
        56.
    );
    snapshot.visible = false;
    assert_eq!(
        measure_price_axis(&snapshot, &mut recorder)
            .unwrap()
            .required_width(),
        0.
    );
    assert!(commands(&snapshot, 1.).commands.is_empty());
}

#[test]
fn source_text_anchors_order_color_and_measured_y_alignment() {
    for side in [PriceAxisSide::Left, PriceAxisSide::Right] {
        let snapshot = snapshot(side);
        let prepared = commands(&snapshot, 1.);
        let mut texts = vec![];
        for command in prepared.commands {
            if let PriceAxisCommand::Text {
                label,
                position,
                alignment,
                color,
                ..
            } = command
            {
                assert_eq!(
                    position.x,
                    if side == PriceAxisSide::Left {
                        90.
                    } else {
                        10.
                    }
                );
                assert_eq!(
                    alignment,
                    if side == PriceAxisSide::Left {
                        AxisTextAlignment::Right
                    } else {
                        AxisTextAlignment::Left
                    }
                );
                assert_eq!(color, "red");
                texts.push((label, position.y));
            }
        }
        assert_eq!(
            texts,
            [
                ("123".into(), 63.),
                ("123456789".into(), 43.),
                ("12".into(), 23.)
            ]
        );
    }
}

#[test]
fn source_visibility_combinations_keep_labels_and_tick_spacing() {
    for border in [false, true] {
        for ticks in [false, true] {
            let mut snapshot = snapshot(PriceAxisSide::Right);
            snapshot.border_visible = border;
            snapshot.ticks_visible = ticks;
            let result = commands(&snapshot, 1.);
            let rectangles = result
                .commands
                .iter()
                .filter(|cmd| matches!(cmd, PriceAxisCommand::Rectangle { .. }))
                .count();
            assert_eq!(
                rectangles,
                usize::from(border) + if border && ticks { 3 } else { 0 }
            );
            assert_eq!(result.commands.iter().filter(|cmd| matches!(cmd, PriceAxisCommand::Text { position, .. } if position.x == 10.)).count(), 3);
            assert_eq!(result.required_width, 42.);
        }
    }
}

#[test]
fn source_snapping_at_fractional_and_nonuniform_ratios() {
    for side in [PriceAxisSide::Left, PriceAxisSide::Right] {
        for (h, v) in [(1., 1.), (1.25, 1.25), (1.5, 1.5), (2., 2.), (1.25, 2.)] {
            let snapshot = snapshot(side);
            let measurement =
                measure_price_axis(&snapshot, &mut RecordingMeasurer::default()).unwrap();
            let result = prepare_price_axis(
                &measurement,
                AxisBounds {
                    width: 100.,
                    height: 80.,
                },
                PixelRatio {
                    horizontal: h,
                    vertical: v,
                },
            )
            .unwrap();
            let rectangles = result
                .commands
                .iter()
                .filter_map(|cmd| match cmd {
                    PriceAxisCommand::Rectangle { rect, .. } => Some(rect),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let h = f64::from(h);
            let v = f64::from(v);
            let border_width = h.floor().max(1.) / h;
            assert_eq!(rectangles[0].size.width, border_width);
            assert_eq!(
                rectangles[0].origin.x,
                if side == PriceAxisSide::Left {
                    100. - border_width
                } else {
                    0.
                }
            );
            assert_eq!(rectangles[1].size.width, source_round(5. * h) / h);
            assert_eq!(
                rectangles[1].origin.y,
                (source_round(20. * v) - (v * 0.5).floor()) / v
            );
            assert_eq!(rectangles[1].size.height, v.floor().max(1.) / v);
        }
    }
}

#[test]
fn source_negative_half_coordinates_round_towards_positive_infinity() {
    let mut snapshot = snapshot(PriceAxisSide::Right);
    snapshot.marks[0].coord = Coordinate::new(-0.5);
    let result = commands(&snapshot, 1.);
    assert!(
        matches!(result.commands[2], PriceAxisCommand::Rectangle { rect, .. } if rect.origin.y == 0.)
    );
}

#[test]
fn gradient_owned_replay_and_zero_bounds_do_not_retain_commands() {
    let mut snapshot = snapshot(PriceAxisSide::Left);
    snapshot.background = Background::VerticalGradient {
        top_color: "white".into(),
        bottom_color: "black".into(),
    };
    let measured = measure_price_axis(&snapshot, &mut RecordingMeasurer::default()).unwrap();
    snapshot.marks.clear();
    let result = prepare_price_axis(
        &measured,
        AxisBounds {
            width: 10.,
            height: 80.,
        },
        ratio(1.),
    )
    .unwrap();
    assert_eq!(
        result.commands[0],
        PriceAxisCommand::Background(snapshot.background)
    );
    assert_eq!(result.required_width, 42.);
    assert!(
        result
            .commands
            .iter()
            .any(|cmd| matches!(cmd, PriceAxisCommand::Text { .. }))
    );
    let zero = prepare_price_axis(&measured, AxisBounds::default(), ratio(1.)).unwrap();
    assert!(zero.commands.is_empty());
}

#[test]
fn missing_ink_metrics_have_zero_vertical_correction() {
    let snapshot = snapshot(PriceAxisSide::Right);
    let measured = measure_price_axis(
        &snapshot,
        &mut RecordingMeasurer {
            absent_ink: true,
            ..Default::default()
        },
    )
    .unwrap();
    let result = prepare_price_axis(
        &measured,
        AxisBounds {
            width: 100.,
            height: 80.,
        },
        ratio(1.),
    )
    .unwrap();
    assert!(
        result
            .commands
            .iter()
            .any(|cmd| matches!(cmd, PriceAxisCommand::Text { position, .. } if position.y == 20.))
    );
}

#[test]
fn invalid_inputs_return_typed_errors() {
    let snapshot = snapshot(PriceAxisSide::Right);
    let measurement = measure_price_axis(&snapshot, &mut RecordingMeasurer::default()).unwrap();
    assert_eq!(
        prepare_price_axis(
            &measurement,
            AxisBounds {
                width: f64::NAN,
                height: 1.
            },
            ratio(1.)
        ),
        Err(PriceAxisRenderError::InvalidBounds)
    );
    assert_eq!(
        prepare_price_axis(&measurement, AxisBounds::default(), ratio(0.)),
        Err(PriceAxisRenderError::InvalidPixelRatio)
    );
    struct Invalid;
    impl TextMeasurer for Invalid {
        fn measure_text(&mut self, _: &str) -> TextMetrics {
            TextMetrics {
                width: f32::NAN,
                ..Default::default()
            }
        }
    }
    assert!(matches!(
        measure_price_axis(&snapshot, &mut Invalid),
        Err(PriceAxisRenderError::InvalidMetrics)
    ));
}
