use feather_charts::{
    api::*,
    model::{
        data_consumer::*, horz_scale_behavior_time::types::UtcTimestamp, pane::PriceScalePosition,
        series_options::SeriesOptionsMap, time_scale_options::HorzScaleOptionsPatch,
    },
};
use iced::{
    Point, Rectangle, Size, mouse,
    widget::canvas::{Event, Program},
};
use std::time::Instant;

fn chart() -> Chart {
    let mut chart = Chart::default();
    chart
        .apply_chart_options(ChartOptionsPatch {
            model: ChartModelOptionsPatch {
                time_scale: Some(HorzScaleOptionsPatch {
                    bar_spacing: Some(9.),
                    right_offset: Some(3.),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    let id = chart
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(
            id,
            (1..=250)
                .map(|i| {
                    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                        time: UtcTimestamp::new(i as f64).into(),
                        value: 100. + (i % 10) as f64,
                        color: None,
                        custom_values: None,
                    }))
                })
                .collect(),
        )
        .unwrap();
    prepare(&mut chart);
    chart
}
fn prepare(c: &mut Chart) {
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
}
fn input(c: &mut Chart, state: &mut ChartState, event: Event, point: Option<Point>) {
    let message = c
        .program(std::convert::identity)
        .update(
            state,
            &event,
            Rectangle::with_size(Size::new(800., 500.)),
            point.map_or(mouse::Cursor::Unavailable, mouse::Cursor::Available),
        )
        .and_then(|a| a.into_inner().0);
    if let Some(m) = message {
        c.update(m).unwrap();
    }
}
fn press(c: &mut Chart, s: &mut ChartState, p: Point) {
    input(
        c,
        s,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Some(p),
    );
}
fn release(c: &mut Chart, s: &mut ChartState, p: Option<Point>) {
    input(
        c,
        s,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        p,
    );
}
fn movement(c: &mut Chart, s: &mut ChartState, p: Point) {
    input(
        c,
        s,
        Event::Mouse(mouse::Event::CursorMoved { position: p }),
        Some(p),
    );
}
fn axis(c: &Chart, fraction: f64) -> Point {
    let r = c.frame().layout.time_axis;
    Point::new(
        (r.origin.x + r.size.width * fraction) as f32,
        (r.origin.y + r.size.height * 0.5) as f32,
    )
}
#[test]
fn canvas_horizontal_axis_drag_scales_about_right_edge_and_releases_outside() {
    let mut c = chart();
    let mut s = ChartState::default();
    let start = axis(&c, 0.6);
    let end = axis(&c, 0.4);
    let before = c.model().time_scale().bar_spacing();
    let offset = c.model().time_scale().right_offset();
    press(&mut c, &mut s, start);
    movement(&mut c, &mut s, end);
    let width = c.model().time_scale().width();
    let origin = c.frame().layout.time_axis.origin.x;
    let expected =
        before * (width - (f64::from(end.x) - origin)) / (width - (f64::from(start.x) - origin));
    assert!(
        (c.model().time_scale().bar_spacing() - expected).abs() < 1e-8,
        "axis drag did not change spacing: {} expected {expected}",
        c.model().time_scale().bar_spacing()
    );
    assert_eq!(c.model().time_scale().right_offset(), offset);
    release(&mut c, &mut s, None);
    let spacing = c.model().time_scale().bar_spacing();
    movement(&mut c, &mut s, start);
    assert_eq!(c.model().time_scale().bar_spacing(), spacing);
}
#[test]
fn canvas_axis_double_click_restores_configured_spacing_and_offset() {
    let mut c = chart();
    let mut s = ChartState::default();
    c.set_logical_range(feather_charts::model::time_data::LogicalRange {
        from: 200.0.into(),
        to: 240.0.into(),
    });
    c.set_right_offset(8.);
    prepare(&mut c);
    let p = axis(&c, 0.6);
    for _ in 0..2 {
        press(&mut c, &mut s, p);
        release(&mut c, &mut s, Some(p));
    }
    prepare(&mut c);
    assert_eq!(c.model().time_scale().bar_spacing(), 9.);
    assert_eq!(c.model().time_scale().right_offset(), 3.);
}

#[test]
fn stale_axis_captures_end_on_focus_resize_dpr_and_pane_changes() {
    for reason in 0..7 {
        let mut c = chart();
        let mut s = ChartState::default();
        c.add_pane();
        c.set_preserve_empty_pane(1, true).unwrap();
        prepare(&mut c);
        let start = axis(&c, 0.6);
        let moved = axis(&c, 0.4);
        press(&mut c, &mut s, start);
        movement(&mut c, &mut s, moved);
        assert!(s.is_axis_dragging());
        match reason {
            0 => input(
                &mut c,
                &mut s,
                Event::Window(iced::window::Event::Unfocused),
                None,
            ),
            1 => input(&mut c, &mut s, Event::Mouse(mouse::Event::CursorLeft), None),
            2 => {
                c.prepare(Size::new(600., 300.), 1., Instant::now())
                    .unwrap();
            }
            3 => {
                c.prepare(Size::new(800., 500.), 1.5, Instant::now())
                    .unwrap();
            }
            4 => c.swap_panes(0, 1).unwrap(),
            5 => c.remove_pane(1).unwrap(),
            _ => c.cancel_interaction(),
        }
        let before = c.model().time_scale().bar_spacing();
        movement(&mut c, &mut s, start);
        release(&mut c, &mut s, None);
        assert_eq!(
            before,
            c.model().time_scale().bar_spacing(),
            "reason {reason}"
        );
        assert!(!s.is_axis_dragging(), "reason {reason}");
    }
}

#[test]
fn empty_hidden_and_side_stubs_do_not_scale_or_capture_the_plot() {
    for reason in 0..3 {
        let mut c = chart();
        let mut s = ChartState::default();
        let p = axis(&c, 0.6);
        if reason == 0 {
            let id = c.series_ids().next().unwrap();
            c.set_data(id, vec![]).unwrap();
        } else if reason == 1 {
            c.apply_chart_options(ChartOptionsPatch {
                model: ChartModelOptionsPatch {
                    time_scale: Some(HorzScaleOptionsPatch {
                        visible: Some(false),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();
        }
        prepare(&mut c);
        let p = if reason == 2 {
            Point::new(799., p.y)
        } else {
            p
        };
        let before = c.model().time_scale().bar_spacing();
        press(&mut c, &mut s, p);
        movement(&mut c, &mut s, Point::new(p.x - 100., p.y));
        release(&mut c, &mut s, None);
        assert_eq!(before, c.model().time_scale().bar_spacing());
        assert!(!s.is_axis_dragging());
    }
}

#[test]
fn reset_retains_pixel_offset_policy_and_runtime_defaults() {
    let mut c = chart();
    let mut s = ChartState::default();
    c.apply_chart_options(ChartOptionsPatch {
        model: ChartModelOptionsPatch {
            time_scale: Some(HorzScaleOptionsPatch {
                bar_spacing: Some(12.),
                right_offset_pixels: Some(Some(60.)),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
    c.set_logical_range(feather_charts::model::time_data::LogicalRange {
        from: 200.0.into(),
        to: 240.0.into(),
    });
    prepare(&mut c);
    let p = axis(&c, 0.6);
    for _ in 0..2 {
        press(&mut c, &mut s, p);
        release(&mut c, &mut s, Some(p));
    }
    prepare(&mut c);
    assert_eq!(c.model().time_scale().bar_spacing(), 12.);
    assert_eq!(c.model().time_scale().right_offset(), 5.);
    assert_eq!(
        c.runtime_options().model.time_scale.right_offset_pixels,
        Some(60.)
    );
}

#[test]
fn source_axis_flag_matrix() {
    for mask in 0..16 {
        let mut c = chart();
        let mut s = ChartState::default();
        let opts = AxisInteractionOptions {
            time_drag: mask & 1 != 0,
            time_reset: mask & 2 != 0,
            price_drag: mask & 4 != 0,
            price_reset: mask & 8 != 0,
        };
        c.apply_chart_options(ChartOptionsPatch {
            axis_interaction: Some(opts),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(c.runtime_options().axis_interaction, opts);
        let a = axis(&c, 0.6);
        let b = axis(&c, 0.4);
        press(&mut c, &mut s, a);
        movement(&mut c, &mut s, b);
        release(&mut c, &mut s, Some(b));
        let scaled = c.model().time_scale().bar_spacing() != 9.;
        assert_eq!(scaled, opts.time_drag, "mask {mask}");
        // The range contains exactly width/18 bars; this does not change defaults.
        let width = c.model().time_scale().width();
        c.set_logical_range(feather_charts::model::time_data::LogicalRange {
            from: 200.0.into(),
            to: (200. + width / 18. - 1.).into(),
        });
        c.set_right_offset(8.);
        prepare(&mut c);
        let p = axis(&c, 0.6);
        for _ in 0..2 {
            press(&mut c, &mut s, p);
            release(&mut c, &mut s, Some(p));
        }
        prepare(&mut c);
        let spacing = c.model().time_scale().bar_spacing();
        let offset = c.model().time_scale().right_offset();
        assert!((spacing - if opts.time_reset { 9. } else { 18. }).abs() < 1e-8);
        assert_eq!(offset, if opts.time_reset { 3. } else { 8. });
        let r = c.frame().layout.right_axis;
        let p = Point::new(
            (r.origin.x + r.size.width * 0.5) as f32,
            (r.size.height * 0.5) as f32,
        );
        press(&mut c, &mut s, p);
        let end = Point::new(p.x, p.y + r.size.height as f32 * 0.1);
        movement(&mut c, &mut s, end);
        release(&mut c, &mut s, Some(end));
        let manual = !c
            .price_scale_options(0, &PriceScalePosition::Right)
            .unwrap()
            .auto_scale;
        assert_eq!(manual, opts.price_drag, "mask {mask}");
        c.set_visible_price_range(
            0,
            &PriceScalePosition::Right,
            feather_charts::model::price_range_impl::PriceRange {
                min_value: 100.,
                max_value: 110.,
            },
        )
        .unwrap();
        prepare(&mut c);
        let r = c.frame().layout.right_axis;
        let p = Point::new(
            (r.origin.x + r.size.width * 0.6) as f32,
            (r.size.height * 0.5) as f32,
        );
        for _ in 0..2 {
            press(&mut c, &mut s, p);
            release(&mut c, &mut s, Some(p));
        }
        let auto = c
            .price_scale_options(0, &PriceScalePosition::Right)
            .unwrap()
            .auto_scale;
        assert_eq!(auto, opts.price_reset, "mask {mask}");
        println!(
            "AXIS_REFERENCE {{\"mask\":{mask},\"scaled\":{scaled},\"spacing\":{},\"offset\":{offset},\"manual\":{manual},\"autoAfterReset\":{auto}}}",
            (spacing * 1e6).round() / 1e6
        );
    }
}

#[test]
fn option_patches_are_atomic_and_cancel_only_the_disabled_axis_capture() {
    for owner in 0..3 {
        let mut c = chart();
        let mut s = ChartState::default();
        let p = match owner {
            0 => axis(&c, 0.6),
            1 => Point::new(790., 100.),
            _ => Point::new(200., 100.),
        };
        press(&mut c, &mut s, p);
        movement(&mut c, &mut s, Point::new(p.x - 20., p.y + 20.));
        let before = c.axis_interaction_options();
        assert!(
            c.apply_chart_options(ChartOptionsPatch {
                axis_interaction: Some(AxisInteractionOptions {
                    time_drag: false,
                    ..before
                }),
                model: ChartModelOptionsPatch {
                    time_scale: Some(HorzScaleOptionsPatch {
                        bar_spacing: Some(f64::NAN),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            })
            .is_err()
        );
        assert_eq!(c.axis_interaction_options(), before);
        c.apply_chart_options(Default::default()).unwrap();
        let unrelated = if owner == 0 {
            AxisInteractionOptions {
                price_drag: false,
                ..before
            }
        } else {
            AxisInteractionOptions {
                time_drag: false,
                ..before
            }
        };
        c.set_axis_interaction_options(unrelated);
        let spacing = c.model().time_scale().bar_spacing();
        movement(&mut c, &mut s, Point::new(p.x - 60., p.y + 60.));
        if owner == 0 {
            assert_ne!(spacing, c.model().time_scale().bar_spacing());
        }
        if owner == 1 {
            assert!(c.active_price_axis().is_some());
        }
        if owner == 2 {
            assert!(c.is_panning());
        }
        if owner < 2 {
            c.set_axis_interaction_options(AxisInteractionOptions {
                time_drag: false,
                price_drag: false,
                ..before
            });
            let spacing = c.model().time_scale().bar_spacing();
            movement(&mut c, &mut s, Point::new(p.x - 100., p.y + 100.));
            assert_eq!(spacing, c.model().time_scale().bar_spacing());
            assert!(!s.is_axis_dragging());
            assert!(c.active_price_axis().is_none());
        }
        release(&mut c, &mut s, None);
    }
}

fn finger(c: &mut Chart, s: &mut ChartState, kind: u8, p: Point) {
    let id = iced::touch::Finger(1);
    let e = match kind {
        0 => iced::touch::Event::FingerPressed { id, position: p },
        1 => iced::touch::Event::FingerMoved { id, position: p },
        2 => iced::touch::Event::FingerLifted { id, position: p },
        _ => iced::touch::Event::FingerLost { id, position: p },
    };
    input(c, s, Event::Touch(e), None);
}
#[test]
fn touch_axis_double_tap_resets_without_plot_events_or_synthetic_mouse_actions() {
    for price in [false, true] {
        let mut c = chart();
        let mut s = ChartState::default();
        c.set_event_interests(EventInterests {
            double_click: true,
            ..Default::default()
        });
        c.set_logical_range(feather_charts::model::time_data::LogicalRange {
            from: 200.0.into(),
            to: 240.0.into(),
        });
        c.set_visible_price_range(
            0,
            &PriceScalePosition::Right,
            feather_charts::model::price_range_impl::PriceRange {
                min_value: 100.,
                max_value: 110.,
            },
        )
        .unwrap();
        prepare(&mut c);
        let p = if price {
            Point::new(790., 100.)
        } else {
            axis(&c, 0.6)
        };
        for _ in 0..2 {
            finger(&mut c, &mut s, 0, p);
            finger(&mut c, &mut s, 2, p);
        }
        if price {
            assert!(
                c.price_scale_options(0, &PriceScalePosition::Right)
                    .unwrap()
                    .auto_scale
            );
        } else {
            assert_eq!(c.model().time_scale().bar_spacing(), 9.);
        }
        assert!(!s.is_axis_dragging());
        let spacing = c.model().time_scale().bar_spacing();
        let offset = c.model().time_scale().right_offset();
        press(&mut c, &mut s, p);
        movement(&mut c, &mut s, Point::new(p.x - 60., p.y + 30.));
        release(&mut c, &mut s, Some(p));
        assert_eq!(
            (spacing, offset),
            (
                c.model().time_scale().bar_spacing(),
                c.model().time_scale().right_offset()
            )
        );
        assert!(!s.is_axis_dragging());
        assert!(
            c.take_events()
                .iter()
                .all(|e| !matches!(e, ChartEvent::DoubleClick(_) | ChartEvent::Click(_)))
        );
    }
}
#[test]
fn touch_axis_drag_obeys_direction_and_touch_options_and_loss_cancels() {
    for price in [false, true] {
        for allowed in [false, true] {
            for correct_direction in [false, true] {
                let mut c = chart();
                let mut s = ChartState::default();
                c.set_touch_navigation_options(TouchNavigationOptions {
                    horizontal_drag: allowed,
                    vertical_drag: allowed,
                    ..Default::default()
                });
                let p = if price {
                    Point::new(790., 100.)
                } else {
                    axis(&c, 0.6)
                };
                let q = if price == correct_direction {
                    Point::new(p.x, p.y + 40.)
                } else {
                    Point::new(p.x - 80., p.y)
                };
                let spacing = c.model().time_scale().bar_spacing();
                let range = c
                    .visible_price_range(0, &PriceScalePosition::Right)
                    .unwrap();
                finger(&mut c, &mut s, 0, p);
                finger(&mut c, &mut s, 1, q);
                let changed = if price {
                    c.visible_price_range(0, &PriceScalePosition::Right)
                        .unwrap()
                        != range
                } else {
                    c.model().time_scale().bar_spacing() != spacing
                };
                assert_eq!(
                    changed,
                    allowed && correct_direction,
                    "price {price} allowed {allowed} direction {correct_direction}"
                );
                finger(&mut c, &mut s, 3, q);
                assert!(!s.is_axis_dragging());
                let before = c.model().time_scale().bar_spacing();
                finger(&mut c, &mut s, 1, p);
                assert_eq!(before, c.model().time_scale().bar_spacing());
            }
        }
    }
}

#[test]
fn disabling_axis_flags_preserves_wheel_navigation_and_blocks_raw_actions() {
    let mut c = chart();
    let mut s = ChartState::default();
    c.set_axis_interaction_options(AxisInteractionOptions {
        time_drag: false,
        time_reset: false,
        price_drag: false,
        price_reset: false,
    });
    let before = c.model().time_scale().bar_spacing();
    input(
        &mut c,
        &mut s,
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0., y: 1. },
        }),
        Some(Point::new(200., 100.)),
    );
    assert_ne!(before, c.model().time_scale().bar_spacing());
    let range = c
        .visible_price_range(0, &PriceScalePosition::Right)
        .unwrap();
    input(
        &mut c,
        &mut s,
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0., y: 1. },
        }),
        Some(Point::new(790., 100.)),
    );
    assert_ne!(
        range,
        c.visible_price_range(0, &PriceScalePosition::Right)
            .unwrap()
    );
    let before = c.model().time_scale().bar_spacing();
    for action in [
        feather_charts::ui::time_axis_input::TimeAxisAction::Start { x: 200. },
        feather_charts::ui::time_axis_input::TimeAxisAction::Move { x: 100. },
        feather_charts::ui::time_axis_input::TimeAxisAction::Reset,
    ] {
        c.update(ChartMessage::TimeAxis(action)).unwrap();
    }
    assert_eq!(before, c.model().time_scale().bar_spacing());
    assert!(
        c.update(ChartMessage::TimeAxis(
            feather_charts::ui::time_axis_input::TimeAxisAction::Move { x: f64::NAN }
        ))
        .is_err()
    );
}
