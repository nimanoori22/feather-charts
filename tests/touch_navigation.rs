use feather_charts::{
    api::{
        Chart, ChartEvent, ChartMessage, ChartState, TouchAction, TouchInteraction,
        TouchNavigationOptions, TrackingExitMode,
    },
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        horz_scale_behavior_time::types::UtcTimestamp,
        pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
    },
};
use iced::{
    Point, Rectangle, Size, mouse, touch,
    widget::canvas::{Event, Program},
};
use std::time::{Duration, Instant};

fn chart() -> Chart {
    let mut chart = Chart::default();
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
            (0..120)
                .map(|index| {
                    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                        time: UtcTimestamp::new(index as f64 + 1.).into(),
                        value: 100. + (index as f64).sin(),
                        color: None,
                        custom_values: None,
                    }))
                })
                .collect(),
        )
        .unwrap();
    chart
        .prepare(Size::new(640., 480.), 1.25, Instant::now())
        .unwrap();
    chart
}
fn native(chart: &mut Chart, state: &mut ChartState, event: Event) -> Vec<ChartEvent> {
    let message = chart
        .program(std::convert::identity)
        .update(
            state,
            &event,
            Rectangle::with_size(Size::new(640., 480.)),
            mouse::Cursor::Unavailable,
        )
        .and_then(|action| action.into_inner().0);
    message.map_or_else(Vec::new, |message| chart.update(message).unwrap())
}
fn point(chart: &Chart, x: f32, y: f32) -> Point {
    let origin = chart.frame().layout.plot.origin;
    Point::new(x + origin.x as f32, y + origin.y as f32)
}
fn finger(event: touch::Event) -> Event {
    Event::Touch(event)
}
fn touch_message(chart: &mut Chart, now: Instant, action: TouchAction) {
    chart
        .update(ChartMessage::Touch(TouchInteraction {
            pane: 0,
            now,
            action,
        }))
        .unwrap();
}
fn flick(chart: &mut Chart, now: Instant) {
    touch_message(chart, now, TouchAction::Press);
    for (ms, x, start) in [
        (10, 210., true),
        (30, 240., false),
        (50, 280., false),
        (70, 320., false),
    ] {
        let position = Point::new(x, 150.);
        touch_message(
            chart,
            now + Duration::from_millis(ms),
            TouchAction::Pan {
                start: start.then_some(position),
                position,
            },
        );
    }
    touch_message(chart, now + Duration::from_millis(80), TouchAction::End);
}

#[test]
fn native_tap_emits_one_click_without_moving_the_crosshair_or_mouse_duplication() {
    let mut chart = chart();
    let mut state = ChartState::default();
    let position = point(&chart, 200., 150.);
    let id = touch::Finger(17);
    assert!(
        native(
            &mut chart,
            &mut state,
            finger(touch::Event::FingerPressed { id, position })
        )
        .is_empty()
    );
    let events = native(
        &mut chart,
        &mut state,
        finger(touch::Event::FingerLifted { id, position }),
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, ChartEvent::Click(_)))
            .count(),
        1
    );
    assert!(chart.model().crosshair().position().is_none());
    for event in [
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
    ] {
        assert!(
            chart
                .program(std::convert::identity)
                .update(
                    &mut state,
                    &Event::Mouse(event),
                    Rectangle::with_size(Size::new(640., 480.)),
                    mouse::Cursor::Available(position)
                )
                .is_none()
        );
    }
}

#[test]
fn native_pan_keeps_primary_identity_and_cancels_on_loss() {
    let mut chart = chart();
    let mut state = ChartState::default();
    let position = point(&chart, 200., 150.);
    let id = touch::Finger(4);
    let initial = chart.model().time_scale().right_offset();
    native(
        &mut chart,
        &mut state,
        finger(touch::Event::FingerPressed { id, position }),
    );
    for dx in [10., 50., 90.] {
        let events = native(
            &mut chart,
            &mut state,
            finger(touch::Event::FingerMoved {
                id,
                position: Point::new(position.x + dx, position.y + dx / 2.),
            }),
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, ChartEvent::Click(_)))
        );
    }
    assert!(chart.is_panning());
    assert_ne!(chart.model().time_scale().right_offset(), initial);
    assert!(chart.model().crosshair().position().is_none());
    native(
        &mut chart,
        &mut state,
        finger(touch::Event::FingerLost { id, position }),
    );
    assert!(!chart.is_panning());
    assert!(!chart.is_animating());
}

#[test]
fn native_pinch_zooms_and_disabled_direction_is_delegated() {
    let mut chart = chart();
    let mut state = ChartState::default();
    let left = point(&chart, 200., 150.);
    let right = point(&chart, 300., 150.);
    let spacing = chart.model().time_scale().bar_spacing();
    native(
        &mut chart,
        &mut state,
        finger(touch::Event::FingerPressed {
            id: touch::Finger(1),
            position: left,
        }),
    );
    native(
        &mut chart,
        &mut state,
        finger(touch::Event::FingerPressed {
            id: touch::Finger(2),
            position: right,
        }),
    );
    native(
        &mut chart,
        &mut state,
        finger(touch::Event::FingerMoved {
            id: touch::Finger(2),
            position: Point::new(right.x + 50., right.y),
        }),
    );
    assert!(chart.model().time_scale().bar_spacing() > spacing);
    native(
        &mut chart,
        &mut state,
        finger(touch::Event::FingerLost {
            id: touch::Finger(1),
            position: left,
        }),
    );
    chart.set_touch_navigation_options(TouchNavigationOptions {
        horizontal_drag: false,
        vertical_drag: false,
        pinch: false,
        ..Default::default()
    });
    let initial = chart.model().time_scale().right_offset();
    native(
        &mut chart,
        &mut state,
        finger(touch::Event::FingerPressed {
            id: touch::Finger(3),
            position: left,
        }),
    );
    let action = chart.program(std::convert::identity).update(
        &mut state,
        &finger(touch::Event::FingerMoved {
            id: touch::Finger(3),
            position: Point::new(left.x + 100., left.y),
        }),
        Rectangle::with_size(Size::new(640., 480.)),
        mouse::Cursor::Unavailable,
    );
    assert!(action.is_none());
    assert_eq!(chart.model().time_scale().right_offset(), initial);
}

#[test]
fn long_press_tracks_and_exit_modes_use_the_existing_redraw_deadline() {
    for exit in [TrackingExitMode::OnNextTap, TrackingExitMode::OnTouchEnd] {
        let mut chart = chart();
        chart.set_touch_navigation_options(TouchNavigationOptions {
            tracking_exit: exit,
            ..Default::default()
        });
        let mut state = ChartState::default();
        let position = point(&chart, 200., 150.);
        let id = touch::Finger(10);
        let message = chart
            .program(std::convert::identity)
            .update(
                &mut state,
                &finger(touch::Event::FingerPressed { id, position }),
                Rectangle::with_size(Size::new(640., 480.)),
                mouse::Cursor::Unavailable,
            )
            .unwrap()
            .into_inner()
            .0
            .unwrap();
        let ChartMessage::Touch(input) = &message else {
            panic!("native touch message");
        };
        let now = input.now;
        chart.update(message).unwrap();
        native(
            &mut chart,
            &mut state,
            Event::Window(iced::window::Event::RedrawRequested(
                now + Duration::from_millis(241),
            )),
        );
        assert!(chart.model().crosshair().position().is_some());
        native(
            &mut chart,
            &mut state,
            finger(touch::Event::FingerLifted { id, position }),
        );
        if exit == TrackingExitMode::OnNextTap {
            assert!(chart.model().crosshair().position().is_some());
            native(
                &mut chart,
                &mut state,
                finger(touch::Event::FingerPressed { id, position }),
            );
            native(
                &mut chart,
                &mut state,
                finger(touch::Event::FingerLifted { id, position }),
            );
        }
        assert!(chart.model().crosshair().position().is_none());
    }
}

#[test]
fn touch_scroll_waits_for_second_move_then_inertia_uses_one_controller_and_finishes() {
    let mut chart = chart();
    let now = Instant::now();
    let initial = chart.model().time_scale().right_offset();
    touch_message(&mut chart, now, TouchAction::Press);
    let position = Point::new(210., 150.);
    touch_message(
        &mut chart,
        now + Duration::from_millis(10),
        TouchAction::Pan {
            start: Some(position),
            position,
        },
    );
    assert!(!chart.is_panning());
    assert_eq!(chart.model().time_scale().right_offset(), initial);
    touch_message(
        &mut chart,
        now + Duration::from_millis(30),
        TouchAction::Pan {
            start: None,
            position: Point::new(240., 150.),
        },
    );
    assert!(chart.is_panning());
    assert_eq!(chart.model().time_scale().right_offset(), initial);
    touch_message(
        &mut chart,
        now + Duration::from_millis(50),
        TouchAction::Pan {
            start: None,
            position: Point::new(280., 150.),
        },
    );
    let before = chart.model().time_scale().right_offset();
    assert_ne!(before, initial);
    touch_message(
        &mut chart,
        now + Duration::from_millis(60),
        TouchAction::End,
    );
    assert!(!chart.is_panning());
    assert!(chart.is_animating());
    chart
        .update(ChartMessage::Tick(now + Duration::from_millis(160)))
        .unwrap();
    assert_ne!(chart.model().time_scale().right_offset(), before);
    chart
        .update(ChartMessage::Tick(now + Duration::from_secs(10)))
        .unwrap();
    assert!(!chart.is_animating());
    let mut state = ChartState::default();
    let idle = chart.program(std::convert::identity).update(
        &mut state,
        &Event::Window(iced::window::Event::RedrawRequested(
            now + Duration::from_secs(11),
        )),
        Rectangle::with_size(Size::new(640., 480.)),
        mouse::Cursor::Unavailable,
    );
    assert!(idle.is_none());
}

#[test]
fn kinetic_defaults_and_all_viewport_interruptions_stop_animation() {
    use feather_charts::{
        api::KineticScrollOptions,
        model::{axis_snapshots::PriceAxisSide, price_scale::PriceScaleOptionsPatch},
        ui::chart_input::{ChartInteraction, Navigation},
    };
    for interruption in 0..10 {
        let mut chart = chart();
        let now = Instant::now();
        assert_eq!(
            chart.kinetic_scroll_options(),
            KineticScrollOptions {
                mouse: false,
                touch: true
            }
        );
        flick(&mut chart, now);
        assert!(chart.is_animating(), "flick {interruption}");
        match interruption {
            0 => touch_message(
                &mut chart,
                now + Duration::from_millis(90),
                TouchAction::Press,
            ),
            1 => touch_message(
                &mut chart,
                now + Duration::from_millis(90),
                TouchAction::Pinch {
                    anchor: Point::new(200., 150.),
                    zoom: 0.,
                },
            ),
            2 => touch_message(
                &mut chart,
                now + Duration::from_millis(90),
                TouchAction::Cancel,
            ),
            3 => chart.fit_content(),
            4 => chart.set_right_offset(-20.),
            5 => chart
                .apply_price_axis_options(
                    0,
                    PriceAxisSide::Right,
                    PriceScaleOptionsPatch {
                        invert_scale: Some(true),
                        ..Default::default()
                    },
                )
                .unwrap(),
            6 => {
                chart
                    .prepare(Size::new(400., 300.), 1.25, now + Duration::from_millis(90))
                    .unwrap();
            }
            7 => {
                chart
                    .update(ChartMessage::Interaction(ChartInteraction {
                        position: None,
                        navigation: Some(Navigation::Wheel {
                            x: 200.,
                            zoom: 1.,
                            scroll: 0.,
                        }),
                        capture: true,
                    }))
                    .unwrap();
            }
            8 => {
                let id = chart.series_ids().next().unwrap();
                chart.remove_series(id).unwrap();
            }
            _ => {
                let mut state = ChartState::default();
                native(
                    &mut chart,
                    &mut state,
                    Event::Window(iced::window::Event::Unfocused),
                );
            }
        }
        assert!(!chart.is_animating(), "interruption {interruption}");
    }
    let mut chart = chart();
    chart.set_kinetic_scroll_options(KineticScrollOptions {
        mouse: false,
        touch: false,
    });
    flick(&mut chart, Instant::now());
    assert!(!chart.is_animating());
}

#[test]
fn ordinary_live_update_keeps_touch_capture_and_options_cancel_stale_fingers() {
    let mut chart = chart();
    let mut state = ChartState::default();
    let position = point(&chart, 200., 150.);
    let id = touch::Finger(8);
    native(
        &mut chart,
        &mut state,
        finger(touch::Event::FingerPressed { id, position }),
    );
    let series = chart.series_ids().next().unwrap();
    chart
        .update_data(
            series,
            BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                time: UtcTimestamp::new(120.).into(),
                value: 102.,
                color: None,
                custom_values: None,
            })),
            false,
        )
        .unwrap();
    chart
        .prepare(Size::new(640., 480.), 1.25, Instant::now())
        .unwrap();
    for dx in [10., 50., 90.] {
        native(
            &mut chart,
            &mut state,
            finger(touch::Event::FingerMoved {
                id,
                position: Point::new(position.x + dx, position.y),
            }),
        );
    }
    assert!(chart.is_panning());
    chart.set_touch_navigation_options(TouchNavigationOptions {
        horizontal_drag: false,
        ..Default::default()
    });
    native(
        &mut chart,
        &mut state,
        finger(touch::Event::FingerMoved {
            id,
            position: Point::new(position.x + 100., position.y),
        }),
    );
    assert!(!chart.is_panning());
    assert!(!chart.is_animating());
    assert!(
        native(
            &mut chart,
            &mut state,
            finger(touch::Event::FingerLifted { id, position })
        )
        .is_empty()
    );
}

#[test]
fn pan_stays_with_its_pane_outside_the_plot() {
    let mut chart = chart();
    let pane = chart.add_pane();
    let id = chart
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            pane,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(
            id,
            (0..120)
                .map(|index| {
                    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                        time: UtcTimestamp::new(index as f64 + 1.).into(),
                        value: 100. + (index as f64).sin(),
                        color: None,
                        custom_values: None,
                    }))
                })
                .collect(),
        )
        .unwrap();
    chart
        .prepare(Size::new(640., 480.), 1.25, Instant::now())
        .unwrap();
    let plot = chart.multi_frame().unwrap().layout.panes[pane].plot;
    let position = Point::new(plot.origin.x as f32 + 200., plot.origin.y as f32 + 100.);
    let finger_id = touch::Finger(4);
    let mut state = ChartState::default();
    native(
        &mut chart,
        &mut state,
        finger(touch::Event::FingerPressed {
            id: finger_id,
            position,
        }),
    );
    for (dx, dy) in [(10., 0.), (50., -50.), (90., -300.)] {
        native(
            &mut chart,
            &mut state,
            finger(touch::Event::FingerMoved {
                id: finger_id,
                position: Point::new(position.x + dx, position.y + dy),
            }),
        );
    }
    assert_eq!(chart.pan_pane(), Some(pane));
    assert!(
        !chart.model().panes()[pane]
            .right_price_scale()
            .options()
            .auto_scale
    );
    assert!(
        chart.model().panes()[0]
            .right_price_scale()
            .options()
            .auto_scale
    );
    native(
        &mut chart,
        &mut state,
        Event::Window(iced::window::Event::Unfocused),
    );
    assert!(!chart.is_panning());
    assert!(!chart.is_animating());
}

#[test]
fn wheel_outside_canvas_does_not_discard_a_captured_touch_pan() {
    let mut chart = chart();
    let mut state = ChartState::default();
    let position = point(&chart, 200., 150.);
    let id = touch::Finger(5);
    native(
        &mut chart,
        &mut state,
        finger(touch::Event::FingerPressed { id, position }),
    );
    for dx in [10., 50., 90.] {
        native(
            &mut chart,
            &mut state,
            finger(touch::Event::FingerMoved {
                id,
                position: Point::new(position.x + dx, position.y),
            }),
        );
    }
    let before = chart.model().time_scale().right_offset();
    let ignored = chart.program(std::convert::identity).update(
        &mut state,
        &Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0., y: 50. },
        }),
        Rectangle::with_size(Size::new(640., 480.)),
        mouse::Cursor::Unavailable,
    );
    assert!(ignored.is_none());
    native(
        &mut chart,
        &mut state,
        finger(touch::Event::FingerMoved {
            id,
            position: Point::new(position.x + 120., position.y),
        }),
    );
    assert_ne!(chart.model().time_scale().right_offset(), before);
    native(
        &mut chart,
        &mut state,
        finger(touch::Event::FingerLost { id, position }),
    );
    assert!(!chart.is_panning());
}

#[test]
fn native_resize_precedes_a_due_long_press_deadline() {
    let mut chart = chart();
    let mut state = ChartState::default();
    let position = point(&chart, 200., 150.);
    let id = touch::Finger(9);
    let message = chart
        .program(std::convert::identity)
        .update(
            &mut state,
            &finger(touch::Event::FingerPressed { id, position }),
            Rectangle::with_size(Size::new(640., 480.)),
            mouse::Cursor::Unavailable,
        )
        .unwrap()
        .into_inner()
        .0
        .unwrap();
    let ChartMessage::Touch(input) = &message else {
        panic!("touch press");
    };
    let now = input.now;
    chart.update(message).unwrap();
    let resized = Size::new(400., 300.);
    let message = chart
        .program(std::convert::identity)
        .update(
            &mut state,
            &Event::Window(iced::window::Event::RedrawRequested(
                now + Duration::from_millis(241),
            )),
            Rectangle::with_size(resized),
            mouse::Cursor::Unavailable,
        )
        .unwrap()
        .into_inner()
        .0
        .unwrap();
    assert!(matches!(message,ChartMessage::Bounds(size) if size==resized));
    chart.update(message).unwrap();
    assert!(chart.model().crosshair().position().is_none());
    let release = chart
        .program(std::convert::identity)
        .update(
            &mut state,
            &finger(touch::Event::FingerLifted { id, position }),
            Rectangle::with_size(resized),
            mouse::Cursor::Unavailable,
        )
        .and_then(|a| a.into_inner().0);
    if let Some(message) = release {
        assert!(chart.update(message).unwrap().is_empty());
    }
    assert!(chart.model().crosshair().position().is_none());
    assert!(!chart.is_panning());
}
