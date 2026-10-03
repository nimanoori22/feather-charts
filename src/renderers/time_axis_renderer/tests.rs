use super::*;
use crate::model::{
    axis_snapshots::{TimeAxisRendererOptions, TimeAxisTick},
    ihorz_scale_behavior::TimeMark,
    layout_options::{Background, ColorSpace},
    text_width_cache::TextMetrics,
    time_data::TickMarkWeightValue,
};

pub(crate) fn snapshot() -> TimeAxisSnapshot {
    TimeAxisSnapshot {
        visible: true,
        ticks: [
            (1., "Jan", true),
            (50.25, "12:30", false),
            (99., "Feb", true),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (coordinate, label, priority))| TimeAxisTick {
            mark: TimeMark {
                coordinate,
                label: label.into(),
                weight: TickMarkWeightValue::new(i as i32),
                need_align_coordinate: i != 1,
            },
            priority,
            emphasis: if priority {
                TimeLabelEmphasis::Emphasized
            } else {
                TimeLabelEmphasis::Normal
            },
        })
        .collect(),
        background: Background::Solid {
            color: "#fff".into(),
        },
        color_space: ColorSpace::Srgb,
        border_visible: true,
        border_color: "#123".into(),
        ticks_visible: true,
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
    }
}
pub(crate) struct Metrics(pub f32);
impl TextMeasurer for Metrics {
    fn measure_text(&mut self, label: &str) -> TextMetrics {
        TextMetrics {
            width: label.chars().count() as f32 * self.0,
            ..Default::default()
        }
    }
}
fn prepared(snapshot: &TimeAxisSnapshot, width: f64, h: f32, v: f32) -> PreparedTimeAxis {
    prepare_time_axis(
        &measure_time_axis(snapshot, &mut Metrics(6.), &mut Metrics(8.)).unwrap(),
        AxisBounds { width, height: 40. },
        PixelRatio {
            horizontal: h,
            vertical: v,
        },
    )
    .unwrap()
}
fn texts(axis: &PreparedTimeAxis) -> Vec<(&str, Point, TimeLabelEmphasis)> {
    axis.commands
        .iter()
        .filter_map(|command| match command {
            TimeAxisCommand::Text {
                label,
                position,
                emphasis,
                ..
            } => Some((label.as_str(), *position, *emphasis)),
            _ => None,
        })
        .collect()
}

#[test]
fn priority_order_font_metrics_and_edge_alignment_match_source() {
    let axis = prepared(&snapshot(), 100., 1., 1.);
    assert_eq!(
        texts(&axis),
        vec![
            (
                "12:30",
                Point { x: 50.25, y: 15. },
                TimeLabelEmphasis::Normal
            ),
            (
                "Jan",
                Point { x: 11.5, y: 15. },
                TimeLabelEmphasis::Emphasized
            ),
            (
                "Feb",
                Point { x: 87.5, y: 15. },
                TimeLabelEmphasis::Emphasized
            ),
        ]
    );
    assert_eq!(axis.required_height, 28.);
    let mut normal = snapshot();
    for tick in &mut normal.ticks {
        tick.emphasis = TimeLabelEmphasis::Normal;
    }
    let normal = prepared(&normal, 100., 1., 1.);
    assert_eq!(texts(&normal)[1].1.x, 8.5);
    assert_eq!(
        texts(&normal).iter().map(|v| v.0).collect::<Vec<_>>(),
        ["12:30", "Jan", "Feb"]
    );
}

#[test]
fn tick_visibility_preserves_labels_and_reserved_spacing() {
    for border in [false, true] {
        for ticks in [false, true] {
            let mut s = snapshot();
            s.border_visible = border;
            s.ticks_visible = ticks;
            let axis = prepared(&s, 100., 1., 1.);
            assert_eq!(texts(&axis), texts(&prepared(&snapshot(), 100., 1., 1.)));
            assert_eq!(
                axis.commands
                    .iter()
                    .filter(|c| matches!(c, TimeAxisCommand::Rectangle { .. }))
                    .count(),
                usize::from(border) + if border && ticks { 3 } else { 0 }
            );
            assert_eq!(axis.required_height, 28.);
        }
    }
}

#[test]
fn fractional_snapping_keeps_ticks_unshifted_and_text_logical() {
    for (h, v) in [(1., 1.), (1.25, 1.5), (1.5, 2.), (2., 1.25)] {
        let s = snapshot();
        let axis = prepared(&s, 100., h, v);
        assert_eq!(texts(&axis), texts(&prepared(&s, 100., 1., 1.)));
        let rects = axis
            .commands
            .iter()
            .filter_map(|c| match c {
                TimeAxisCommand::Rectangle { rect, .. } => Some(rect),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(rects[0].origin, Point { x: 0., y: 0. });
        assert_eq!(
            rects[0].size.height,
            (f64::from(v)).floor().max(1.) / f64::from(v)
        );
        for (rect, tick) in rects[1..].iter().zip(s.ticks.iter().rev()) {
            let h = f64::from(h);
            let v = f64::from(v);
            assert_eq!(
                rect.origin.x,
                (source_round(tick.mark.coordinate * h) - (h * 0.5).floor()) / h
            );
            assert_eq!(rect.size.width, h.floor().max(1.) / h);
            assert_eq!(rect.size.height, source_round(5. * v) / v);
        }
    }
    let mut s = snapshot();
    s.ticks[0].mark.coordinate = -0.5;
    s.ticks[0].mark.need_align_coordinate = false;
    assert!(
        prepared(&s, 100., 1., 1.)
            .commands
            .iter()
            .any(|c| matches!(c,
        TimeAxisCommand::Rectangle { rect, .. } if rect.origin.x == 0. && rect.size.height == 5.))
    );
}

#[test]
fn narrow_axes_preserve_labels_and_source_one_sided_clamping() {
    let axis = prepared(&snapshot(), 8., 1., 1.);
    assert_eq!(texts(&axis).len(), 3); // no second collision/selection algorithm
    assert_eq!(texts(&axis)[1].1.x, 11.5); // source left branch does not also clamp right
    let mut s = snapshot();
    s.ticks[0].mark.need_align_coordinate = false;
    assert_eq!(texts(&prepared(&s, 100., 1., 1.))[1].1.x, 1.);
}

#[test]
fn height_minimum_rounding_typography_and_hidden_empty_output() {
    let mut s = snapshot();
    s.minimum_height = 31.5;
    assert_eq!(prepared(&s, 100., 1., 1.).required_height, 33.);
    s.minimum_height = 0.;
    s.renderer_options.font_size = 24.;
    s.renderer_options.padding_top = 6.;
    s.renderer_options.padding_bottom = 6.;
    s.renderer_options.label_bottom_offset = 8.;
    assert_eq!(prepared(&s, 100., 1., 1.).required_height, 50.);
    s.ticks.clear();
    let empty = prepared(&s, 100., 1., 1.);
    assert!(texts(&empty).is_empty());
    assert_eq!(empty.commands.len(), 2);
    s.visible = false;
    let hidden = prepared(&s, 100., 1., 1.);
    assert_eq!(hidden.required_height, 0.);
    assert!(hidden.commands.is_empty());
    s.visible = true;
    assert!(prepared(&s, 0., 1., 1.).commands.is_empty());
    let m = measure_time_axis(&s, &mut Metrics(6.), &mut Metrics(8.)).unwrap();
    assert!(
        prepare_time_axis(
            &m,
            AxisBounds {
                width: 100.,
                height: 0.
            },
            PixelRatio {
                horizontal: 1.,
                vertical: 1.
            }
        )
        .unwrap()
        .commands
        .is_empty()
    );
}

#[test]
fn bottom_background_color_and_owned_measurement_survive_mutation() {
    let mut s = snapshot();
    s.background = Background::VerticalGradient {
        top_color: "red".into(),
        bottom_color: "blue".into(),
    };
    let m = measure_time_axis(&s, &mut Metrics(6.), &mut Metrics(8.)).unwrap();
    s.ticks.clear();
    s.renderer_options.font_size = 30.;
    let axis = prepare_time_axis(
        &m,
        AxisBounds {
            width: 100.,
            height: 40.,
        },
        PixelRatio {
            horizontal: 1.,
            vertical: 1.,
        },
    )
    .unwrap();
    assert_eq!(axis.commands[0], TimeAxisCommand::Background("blue".into()));
    assert_eq!(texts(&axis).len(), 3);
    assert_eq!(axis.required_height, 28.);
}

#[test]
fn invalid_snapshot_metrics_bounds_and_ratios_are_typed_errors() {
    let mut s = snapshot();
    s.ticks[0].mark.coordinate = f64::NAN;
    assert!(matches!(
        measure_time_axis(&s, &mut Metrics(6.), &mut Metrics(8.)),
        Err(TimeAxisRenderError::InvalidSnapshot)
    ));
    let s = snapshot();
    assert!(matches!(
        measure_time_axis(&s, &mut Metrics(f32::NAN), &mut Metrics(8.)),
        Err(TimeAxisRenderError::InvalidMetrics)
    ));
    let m = measure_time_axis(&s, &mut Metrics(6.), &mut Metrics(8.)).unwrap();
    assert_eq!(
        prepare_time_axis(
            &m,
            AxisBounds {
                width: -1.,
                height: 40.
            },
            PixelRatio {
                horizontal: 1.,
                vertical: 1.
            }
        ),
        Err(TimeAxisRenderError::InvalidBounds)
    );
    assert_eq!(
        prepare_time_axis(
            &m,
            AxisBounds {
                width: 100.,
                height: 40.
            },
            PixelRatio {
                horizontal: 0.,
                vertical: 1.
            }
        ),
        Err(TimeAxisRenderError::InvalidPixelRatio)
    );
}
