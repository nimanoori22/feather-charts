#[test]
fn profile_zoom_frame_stages() {
    profile_zoom(false);
}
#[test]
fn profile_candle_zoom_frame_stages() {
    profile_zoom(true);
}
#[test]
fn profile_price_volume_zoom() {
    use super::*;
    for continuous in [false, true] {
        let mut demo = Demo::new_chart(DisplayConfig::default(), SeriesKind::PriceVolume);
        let _ = demo.update(Message::Bounds(Size::new(917., 870.80005)));
        let mut values = vec![];
        for i in 0..120 {
            let start = Instant::now();
            let _ = demo.update(Message::PaneInteraction(MultiPaneInteraction::Plot {
                pane: i % 2,
                interaction: ChartInteraction {
                    position: Some(iced::Point::new(200., 50.)),
                    navigation: Some(Navigation::Wheel {
                        x: 200.,
                        zoom: if (continuous && i < 70) || (!continuous && i % 2 == 0) {
                            0.32
                        } else {
                            -0.32 / 1.032
                        },
                        scroll: 0.,
                    }),
                    capture: true,
                },
            }));
            assert!(demo.error.is_none());
            if i >= 20 {
                values.push(start.elapsed().as_secs_f64() * 1000.);
            }
        }
        values.sort_by(f64::total_cmp);
        println!(
            "price/volume continuous={continuous} complete model+frame+glyph update: median {:.3} ms, p95 {:.3} ms",
            values[50], values[95]
        );
    }
}
fn profile_zoom(candlesticks: bool) {
    use super::*;
    for continuous in [false, true] {
        let mut demo = Demo::new_kind(DisplayConfig::default(), candlesticks);
        let _ = demo.update(Message::Bounds(Size::new(917., 870.80005)));
        let mut samples = vec![];
        for i in 0..120 {
            let start = Instant::now();
            demo.chart.zoom_time(
                200.0.into(),
                if (continuous && i < 70) || (!continuous && i % 2 == 0) {
                    0.32
                } else {
                    -0.32 / 1.032
                },
            );
            demo.chart
                .set_crosshair_position(
                    0,
                    Some(feather_charts::renderers::line_renderer::Point { x: 200., y: 200. }),
                )
                .unwrap();
            let model = start.elapsed();
            let input = ChartFrameInput {
                series: Some(demo.id),
                bounds: AxisBounds {
                    width: f64::from(demo.size.width),
                    height: f64::from(demo.size.height),
                },
                pixel_ratio: PixelRatio {
                    horizontal: demo.scale,
                    vertical: demo.scale,
                },
                background: "#101820".into(),
            };
            let start = Instant::now();
            let snapshot = demo.prepare_snapshot(&input, Instant::now()).unwrap();
            let engine = start.elapsed();
            let start = Instant::now();
            let _frame = IcedChartFrame::from_snapshot(snapshot, &demo.fonts).unwrap();
            let glyphs = start.elapsed();
            if i >= 20 {
                samples.push([
                    model.as_secs_f64() * 1000.,
                    engine.as_secs_f64() * 1000.,
                    glyphs.as_secs_f64() * 1000.,
                ]);
            }
        }
        for (index, name) in ["model", "engine/measurement", "Iced glyph paths"]
            .into_iter()
            .enumerate()
        {
            let mut values = samples.iter().map(|s| s[index]).collect::<Vec<_>>();
            values.sort_by(f64::total_cmp);
            println!(
                "candles={candlesticks} continuous={continuous} {name}: median {:.3} ms, p95 {:.3} ms",
                values[50], values[95]
            );
        }
    }
}
#[test]
fn cursor_replay_and_redraw_settle_without_rebuilding_frames_forever() {
    use super::*;
    use iced::widget::canvas::Program;
    for candlesticks in [false, true] {
        let mut demo = Demo::new_kind(DisplayConfig::default(), candlesticks);
        let mut state = ChartState::default();
        for size in [Size::new(800., 500.), Size::new(917., 870.80005)] {
            let _ = demo.update(Message::Bounds(size));
            let bounds = iced::Rectangle::new(iced::Point::new(12., 150.), size);
            let point = iced::Point::new(
                bounds.x + demo.chart.frame().layout.plot.origin.x as f32 + 200.,
                bounds.y + 100.,
            );
            let cursor = iced::mouse::Cursor::Available(point);
            let movement =
                canvas::Event::Mouse(iced::mouse::Event::CursorMoved { position: point });
            let redraw =
                canvas::Event::Window(iced::window::Event::RedrawRequested(Instant::now()));
            let mut published = 0;
            for _ in 0..10 {
                for event in [&movement, &redraw] {
                    if let Some(action) = (demo.chart.program(Message::from))
                        .update(&mut state, event, bounds, cursor)
                    {
                        if let Some(message) = action.into_inner().0 {
                            published += 1;
                            let _ = demo.update(message);
                        }
                    }
                }
            }
            assert!(
                published <= 2,
                "unchanged cursor and redraw events must settle, got {published} messages"
            );
            assert!(demo.error.is_none());
            assert!(demo.chart.model().crosshair().position().is_some());
        }
    }
}
#[test]
fn native_event_sequence_navigation_crosshair_labels_and_cancel() {
    use super::*;
    use iced::widget::canvas::Program;
    use iced::{Point, Rectangle, mouse};
    for candlesticks in [false, true] {
        let mut demo = Demo::new_kind(DisplayConfig::default(), candlesticks);
        let _ = demo.update(Message::Bounds(Size::new(800., 500.)));
        let mut state = ChartState::default();
        for ratio in [1., 1.25, 1.5, 2.] {
            demo.scale = ratio;
            demo.prepare(Instant::now());
            let width_before = demo.chart.frame().layout.plot.size.width;
            let retained = format!("{:?}", demo.chart.frame());
            let point = Point::new(demo.chart.frame().layout.plot.origin.x as f32 + 200., 100.);
            let bounds = Rectangle::with_size(demo.size);
            let event = canvas::Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Lines { x: 0., y: 1. },
            });
            let before = demo.chart.model().time_scale().bar_spacing();
            let message = demo
                .chart
                .program(Message::from)
                .update(&mut state, &event, bounds, mouse::Cursor::Available(point))
                .unwrap()
                .into_inner()
                .0
                .unwrap();
            let _ = demo.update(message);
            assert!(demo.error.is_none());
            assert!(demo.chart.model().time_scale().bar_spacing() > before);
            assert!(!demo.chart.frame().plot.crosshair.is_empty());
            assert_eq!(
                demo.chart.frame().plot.crosshair_markers.is_empty(),
                candlesticks && !demo.left_registered
            );
            let axes = demo.chart.frame().plot.axes.as_ref().unwrap();
            assert!(axes.right.crosshair_label.is_some());
            assert!(axes.time.crosshair_label.is_some());
            assert_eq!(demo.chart.frame().layout.plot.size.width, width_before);
            assert_ne!(format!("{:?}", demo.chart.frame()), retained);
            let _ = demo.update(Message::Animate);
            let _ = demo.update(Message::Interaction(ChartInteraction {
                position: Some(Point::new(200., 100.)),
                navigation: Some(Navigation::Scroll {
                    start: Some(200.),
                    x: 200.,
                }),
                capture: true,
            }));
            assert!(!demo.chart.is_animating());
            let offset = demo.chart.model().time_scale().right_offset();
            let _ = demo.update(Message::Interaction(ChartInteraction {
                position: None,
                navigation: Some(Navigation::Scroll {
                    start: None,
                    x: 250.,
                }),
                capture: true,
            }));
            assert_ne!(demo.chart.model().time_scale().right_offset(), offset);
            let leave = canvas::Event::Window(iced::window::Event::Unfocused);
            let message = demo
                .chart
                .program(Message::from)
                .update(&mut state, &leave, bounds, mouse::Cursor::Unavailable)
                .unwrap()
                .into_inner()
                .0
                .unwrap();
            let _ = demo.update(message);
            assert!(demo.chart.frame().plot.crosshair.is_empty());
            assert!(
                demo.chart
                    .frame()
                    .plot
                    .axes
                    .as_ref()
                    .unwrap()
                    .time
                    .crosshair_label
                    .is_none()
            );
            let _ = demo.update(Message::Remove);
            assert!(demo.chart.frame().plot.crosshair_markers.is_empty());
            let _ = demo.update(Message::Reload);
        }
    }
}
#[test]
fn connected_demo_operations() {
    super::smoke(super::DisplayConfig::default(), false);
    super::smoke(super::DisplayConfig::default(), true);
    super::smoke_series(super::DisplayConfig::default(), false, true);
    super::smoke_volume(super::DisplayConfig::default());
}

#[test]
fn price_volume_native_input_resize_updates_and_cursor_replay() {
    use super::*;
    use iced::{Point, Rectangle, mouse, widget::canvas::Program};
    let mut demo = Demo::new_chart(DisplayConfig::default(), SeriesKind::PriceVolume);
    let _ = demo.update(Message::Bounds(Size::new(800., 500.)));
    let mut state = ChartState::default();
    for pane in 0..2 {
        let layout = demo.chart.multi_frame().unwrap().layout.panes[pane].plot;
        let point = Point::new(layout.origin.x as f32 + 200., layout.origin.y as f32 + 50.);
        let movement = canvas::Event::Mouse(mouse::Event::CursorMoved { position: point });
        let wheel = canvas::Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0., y: 1. },
        });
        for event in [&movement, &wheel] {
            let action = demo
                .chart
                .program(Message::from)
                .update(
                    &mut state,
                    event,
                    Rectangle::with_size(demo.size),
                    mouse::Cursor::Available(point),
                )
                .unwrap();
            let _ = demo.update(action.into_inner().0.unwrap());
            assert!(demo.error.is_none());
        }
        assert_eq!(
            demo.chart.model().crosshair().position().unwrap().pane,
            pane
        );
        let mut published = 0;
        for _ in 0..10 {
            if let Some(action) = (demo.chart.program(Message::from)).update(
                &mut state,
                &movement,
                Rectangle::with_size(demo.size),
                mouse::Cursor::Available(point),
            ) {
                if let Some(message) = action.into_inner().0 {
                    published += 1;
                    let _ = demo.update(message);
                }
            }
        }
        assert!(published <= 2, "stationary cursor must settle: {published}");
        let frame = demo.chart.multi_frame().unwrap();
        for (i, p) in frame.panes.iter().enumerate() {
            assert_eq!(p.plot.crosshair.len(), if i == pane { 2 } else { 1 });
            assert_eq!(
                p.plot
                    .axes
                    .as_ref()
                    .unwrap()
                    .right
                    .crosshair_label
                    .is_some(),
                i == pane
            );
            assert!(p.plot.axes.as_ref().unwrap().time.crosshair_label.is_some());
        }
    }
    let point = Point::new(
        200.,
        demo.chart.multi_frame().unwrap().layout.separators[0]
            .origin
            .y as f32,
    );
    let press = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
    let movement = canvas::Event::Mouse(mouse::Event::CursorMoved { position: point });
    let spacing = demo.chart.model().time_scale().bar_spacing();
    for (event, cursor) in [
        (&press, point),
        (&movement, Point::new(200., point.y - 50.)),
    ] {
        let action = demo
            .chart
            .program(Message::from)
            .update(
                &mut state,
                event,
                Rectangle::with_size(demo.size),
                mouse::Cursor::Available(cursor),
            )
            .unwrap();
        let _ = demo.update(action.into_inner().0.unwrap());
        assert!(demo.error.is_none());
    }
    assert_eq!(spacing, demo.chart.model().time_scale().bar_spacing());
    assert!(demo.chart.model().panes()[0].stretch_factor() < 2.);
    let _ = demo.update(Message::Append);
    let _ = demo.update(Message::Replace);
    let frame = demo.chart.multi_frame().unwrap();
    let top = &frame.panes[0].plot.candles;
    let bottom = &frame.panes[1].plot.candles;
    assert!(!top.is_empty() && !bottom.is_empty());
    assert_eq!(
        frame.panes[0].plot.size.width,
        frame.panes[1].plot.size.width
    );
}

#[test]
fn replacing_pane_membership_cancels_an_old_gesture_even_at_same_count() {
    use super::*;
    use iced::{Point, Rectangle, mouse, widget::canvas::Program};
    let mut demo = Demo::new_chart(DisplayConfig::default(), SeriesKind::PriceVolume);
    let _ = demo.update(Message::Bounds(Size::new(800., 500.)));
    let mut state = ChartState::default();
    let point = Point::new(200., 100.);
    let press = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
    let action = demo
        .chart
        .program(Message::from)
        .update(
            &mut state,
            &press,
            Rectangle::with_size(demo.size),
            mouse::Cursor::Available(point),
        )
        .unwrap();
    let _ = demo.update(action.into_inner().0.unwrap());
    // A new selection in an identically sized pane must not inherit the old drag.
    let _ = demo.update(Message::Remove);
    let _ = demo.update(Message::Reload);
    let movement = canvas::Event::Mouse(mouse::Event::CursorMoved {
        position: Point::new(250., 100.),
    });
    let action = demo
        .chart
        .program(Message::from)
        .update(
            &mut state,
            &movement,
            Rectangle::with_size(demo.size),
            mouse::Cursor::Available(point),
        )
        .unwrap();
    assert!(matches!(
        action.into_inner().0,
        Some(Message::PaneInteraction(MultiPaneInteraction::Cancel))
    ));
}
#[test]
fn explicit_display_scenarios_prepare_without_global_argument_state() {
    for candlesticks in [false, true] {
        for scenario in super::DisplayScenario::ALL {
            let mut demo =
                super::Demo::new_kind(super::DisplayConfig::for_scenario(scenario), candlesticks);
            demo.apply_scenario(std::time::Instant::now());
            assert!(demo.error.is_none(), "{scenario:?}: {:?}", demo.error);
            assert_eq!(demo.config.scenario, scenario);
            assert_eq!(
                demo.chart.frame().layout.available.width,
                f64::from(demo.size.width)
            );
            assert_eq!(
                demo.chart.frame().time.size.width,
                demo.chart.frame().plot.size.width
            );
            if scenario == super::DisplayScenario::Empty {
                assert!(!demo.has_series_geometry());
                let _ = demo.update(super::Message::Reload);
            }
            assert!(demo.has_series_geometry());
        }
    }
}

#[test]
fn native_price_axis_wheel_changes_only_the_hovered_scale() {
    use super::*;
    use iced::{Point, Rectangle, mouse, widget::canvas::Program};
    for kind in [
        SeriesKind::Line,
        SeriesKind::Candlesticks,
        SeriesKind::Histogram,
        SeriesKind::PriceVolume,
    ] {
        let mut demo = Demo::new_chart(DisplayConfig::default(), kind);
        let _ = demo.update(Message::Bounds(Size::new(800., 500.)));
        let mut state = ChartState::default();
        let count = demo.chart.model().panes().len();
        for pane in 0..count {
            let layout = demo
                .chart
                .multi_frame()
                .map_or(demo.chart.frame().layout, |f| f.layout.panes[pane]);
            let axis = layout.right_axis;
            let point = Point::new(
                (axis.origin.x + axis.size.width / 2.) as f32,
                (axis.origin.y + axis.size.height / 2.) as f32,
            );
            let before = demo.chart.model().panes()[pane]
                .right_price_scale()
                .price_range()
                .unwrap();
            let others = demo
                .chart
                .model()
                .panes()
                .iter()
                .map(|p| p.right_price_scale().price_range())
                .collect::<Vec<_>>();
            let time = (
                demo.chart.model().time_scale().bar_spacing(),
                demo.chart.model().time_scale().right_offset(),
            );
            let action = demo
                .chart
                .program(Message::from)
                .update(
                    &mut state,
                    &canvas::Event::Mouse(mouse::Event::WheelScrolled {
                        delta: mouse::ScrollDelta::Lines { x: 0., y: 1. },
                    }),
                    Rectangle::with_size(demo.size),
                    mouse::Cursor::Available(point),
                )
                .expect("wheel over the price axis must publish a scale action");
            let _ = demo.update(action.into_inner().0.unwrap());
            assert!(demo.error.is_none(), "{:?}", demo.error);
            let after = demo.chart.model().panes()[pane]
                .right_price_scale()
                .price_range()
                .unwrap();
            assert!(after.length() < before.length());
            assert!(
                !demo.chart.model().panes()[pane]
                    .right_price_scale()
                    .is_auto_scale()
            );
            assert_eq!(
                time,
                (
                    demo.chart.model().time_scale().bar_spacing(),
                    demo.chart.model().time_scale().right_offset()
                )
            );
            for (other, range) in others.into_iter().enumerate() {
                if other != pane {
                    assert_eq!(
                        demo.chart.model().panes()[other]
                            .right_price_scale()
                            .price_range(),
                        range
                    );
                }
            }
        }
    }
}

fn dispatch_axis_event(
    demo: &mut super::Demo,
    state: &mut super::ChartState,
    event: &iced::widget::canvas::Event,
    point: iced::Point,
) -> bool {
    use super::*;
    use iced::widget::canvas::Program;
    let action = demo.chart.program(Message::from).update(
        state,
        event,
        iced::Rectangle::with_size(demo.size),
        iced::mouse::Cursor::Available(point),
    );
    if let Some(message) = action.and_then(|a| a.into_inner().0) {
        let _ = demo.update(message);
        assert!(demo.error.is_none(), "{:?}", demo.error);
        true
    } else {
        false
    }
}
#[test]
fn native_price_axis_drag_reset_and_cancellation_preserve_time_and_panes() {
    use super::*;
    use iced::{Point, mouse};
    for kind in [
        SeriesKind::Line,
        SeriesKind::Candlesticks,
        SeriesKind::Histogram,
        SeriesKind::PriceVolume,
    ] {
        let mut demo = Demo::new_chart(DisplayConfig::default(), kind);
        let _ = demo.update(Message::Bounds(Size::new(800., 500.)));
        let _ = demo.update(Message::LeftAxis);
        let count = demo.chart.model().panes().len();
        for pane in 0..count {
            for side in [
                feather_charts::model::axis_snapshots::PriceAxisSide::Right,
                feather_charts::model::axis_snapshots::PriceAxisSide::Left,
            ] {
                let mut state = ChartState::default();
                let layout = demo
                    .chart
                    .multi_frame()
                    .map_or(demo.chart.frame().layout, |f| f.layout.panes[pane]);
                let axis = if side == feather_charts::model::axis_snapshots::PriceAxisSide::Right {
                    layout.right_axis
                } else {
                    layout.left_axis
                };
                if axis.size.width == 0.
                    || !demo.chart.axis_scaling()[pane][if side
                        == feather_charts::model::axis_snapshots::PriceAxisSide::Left
                    {
                        0
                    } else {
                        1
                    }]
                {
                    continue;
                }
                let scale = |demo: &Demo| {
                    if side == feather_charts::model::axis_snapshots::PriceAxisSide::Right {
                        demo.chart.model().panes()[pane]
                            .right_price_scale()
                            .price_range()
                    } else {
                        demo.chart.model().panes()[pane]
                            .left_price_scale()
                            .price_range()
                    }
                };
                let before = scale(&demo).unwrap();
                let time = (
                    demo.chart.model().time_scale().bar_spacing(),
                    demo.chart.model().time_scale().right_offset(),
                );
                let point = Point::new(
                    (axis.origin.x + axis.size.width / 2.) as f32,
                    (axis.origin.y + axis.size.height / 2.) as f32,
                );
                let press = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
                let release =
                    canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
                assert!(dispatch_axis_event(&mut demo, &mut state, &press, point));
                let outside = Point::new(layout.plot.origin.x as f32 + 100., point.y - 25.);
                let movement =
                    canvas::Event::Mouse(mouse::Event::CursorMoved { position: outside });
                assert!(dispatch_axis_event(
                    &mut demo, &mut state, &movement, outside
                ));
                assert!(scale(&demo).unwrap().length() < before.length());
                for _ in 0..5 {
                    assert!(!dispatch_axis_event(
                        &mut demo, &mut state, &movement, outside
                    ));
                }
                // Redraw events must still deliver bounds/animation while an axis owns the pointer.
                assert!(!dispatch_axis_event(
                    &mut demo,
                    &mut state,
                    &canvas::Event::Window(iced::window::Event::RedrawRequested(Instant::now())),
                    outside
                ));
                assert!(dispatch_axis_event(
                    &mut demo, &mut state, &release, outside
                ));
                assert!(demo.chart.active_price_axis().is_none());
                assert_eq!(
                    time,
                    (
                        demo.chart.model().time_scale().bar_spacing(),
                        demo.chart.model().time_scale().right_offset()
                    )
                );
                // Recognize consecutive native clicks before expensive frame preparation
                // so parallel profiling tests cannot turn them into slow single clicks.
                let mut input = feather_charts::ui::price_axis_input::PriceAxisInput::default();
                let layouts = demo.chart.multi_frame().map_or_else(
                    || vec![demo.chart.frame().layout],
                    |frame| frame.layout.panes.clone(),
                );
                let enabled = demo.chart.axis_scaling();
                let actions = [&press, &release, &press].map(|event| {
                    input
                        .update(event, Some(point), &layouts, &enabled)
                        .unwrap()
                });
                assert!(matches!(
                    actions[2].action,
                    feather_charts::ui::price_axis_input::PriceAxisAction::Reset
                ));
                for action in actions {
                    let _ = demo.update(Message::PriceAxis(action));
                    assert!(demo.error.is_none());
                }
                assert!((scale(&demo).unwrap().length() - before.length()).abs() < 1e-8);
                let mut state = ChartState::default();
                assert!(dispatch_axis_event(&mut demo, &mut state, &press, point));
                assert!(dispatch_axis_event(
                    &mut demo,
                    &mut state,
                    &canvas::Event::Window(iced::window::Event::Unfocused),
                    point
                ));
                assert!(demo.chart.active_price_axis().is_none());
                let range_after_focus = scale(&demo);
                dispatch_axis_event(&mut demo, &mut state, &movement, outside);
                assert_eq!(scale(&demo), range_after_focus);
                assert!(demo.chart.active_price_axis().is_none());
            }
        }
    }
}
#[test]
fn axis_crossing_does_not_steal_plot_drag_and_data_removal_ends_axis_drag() {
    use super::*;
    use iced::{Point, mouse};
    let mut demo = Demo::new_chart(DisplayConfig::default(), SeriesKind::PriceVolume);
    let _ = demo.update(Message::Bounds(Size::new(800., 500.)));
    let mut state = ChartState::default();
    let layout = demo.chart.multi_frame().unwrap().layout.panes[0];
    let plot = Point::new(layout.plot.origin.x as f32 + 100., 100.);
    let axis = Point::new((layout.right_axis.origin.x + 10.) as f32, 100.);
    let press = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
    let release = canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
    let before = demo.chart.model().panes()[0]
        .right_price_scale()
        .price_range();
    assert!(dispatch_axis_event(&mut demo, &mut state, &press, plot));
    assert!(dispatch_axis_event(
        &mut demo,
        &mut state,
        &canvas::Event::Mouse(mouse::Event::CursorMoved { position: axis }),
        axis
    ));
    assert!(!state.is_axis_dragging());
    assert_eq!(
        demo.chart.model().panes()[0]
            .right_price_scale()
            .price_range(),
        before
    );
    assert!(dispatch_axis_event(&mut demo, &mut state, &release, axis));
    assert!(dispatch_axis_event(&mut demo, &mut state, &press, axis));
    assert!(demo.chart.active_price_axis().is_some());
    let _ = demo.update(Message::Remove);
    assert!(demo.chart.active_price_axis().is_none());
    dispatch_axis_event(
        &mut demo,
        &mut state,
        &canvas::Event::Mouse(mouse::Event::CursorMoved { position: axis }),
        axis,
    );
    assert!(!state.is_axis_dragging());
    let _ = demo.update(Message::Reload);
    assert!(demo.error.is_none());
}

#[test]
fn axis_drag_redraw_does_not_route_plot_input_when_label_width_changes() {
    use super::*;
    use iced::{Point, mouse};
    let mut demo = Demo::new_chart(DisplayConfig::default(), SeriesKind::Line);
    let _ = demo.update(Message::Bounds(Size::new(800., 500.)));
    let mut state = ChartState::default();
    let plot = Point::new(200., 100.);
    dispatch_axis_event(
        &mut demo,
        &mut state,
        &canvas::Event::Mouse(mouse::Event::CursorMoved { position: plot }),
        plot,
    );
    let point = Point::new(
        (demo.chart.frame().layout.right_axis.origin.x + 10.) as f32,
        100.,
    );
    dispatch_axis_event(
        &mut demo,
        &mut state,
        &canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        point,
    );
    // Axis label measurement may grow the axis and change plot bounds mid-drag.
    let SeriesOptionsMap::Line(mut options) = demo.series_options() else {
        unreachable!()
    };
    if let PriceFormat::BuiltIn(format) = &mut options.common.price_format {
        format.precision = 6;
    }
    demo.chart
        .replace_series_options(demo.id, SeriesOptionsMap::Line(options))
        .unwrap();
    demo.prepare(Instant::now());
    assert!(!dispatch_axis_event(
        &mut demo,
        &mut state,
        &canvas::Event::Window(iced::window::Event::RedrawRequested(Instant::now())),
        plot
    ));
    assert!(state.is_axis_dragging());
    assert!(demo.chart.model().crosshair().position().is_none());
}

#[test]
fn native_plot_drag_pans_price_and_time_in_all_demos() {
    use super::*;
    use iced::{Point, mouse};
    for kind in [
        SeriesKind::Line,
        SeriesKind::Candlesticks,
        SeriesKind::Histogram,
        SeriesKind::PriceVolume,
    ] {
        let mut demo = Demo::new_chart(DisplayConfig::default(), kind);
        let _ = demo.update(Message::Bounds(Size::new(800., 500.)));
        for pane in 0..demo.chart.model().panes().len() {
            let mut state = ChartState::default();
            let layout = demo
                .chart
                .multi_frame()
                .map_or(demo.chart.frame().layout, |f| f.layout.panes[pane]);
            let point = Point::new(
                layout.plot.origin.x as f32 + 200.,
                layout.plot.origin.y as f32 + 70.,
            );
            let press = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
            assert!(dispatch_axis_event(&mut demo, &mut state, &press, point));
            let anchor = Point::new(point.x + 10., point.y + 10.);
            dispatch_axis_event(
                &mut demo,
                &mut state,
                &canvas::Event::Mouse(mouse::Event::CursorMoved { position: anchor }),
                anchor,
            );
            let before = demo.chart.model().panes()[pane]
                .right_price_scale()
                .price_range()
                .unwrap();
            let others = demo
                .chart
                .model()
                .panes()
                .iter()
                .map(|p| p.right_price_scale().price_range())
                .collect::<Vec<_>>();
            let offset = demo.chart.model().time_scale().right_offset();
            let moved = Point::new(anchor.x + 30., anchor.y + 30.);
            assert!(dispatch_axis_event(
                &mut demo,
                &mut state,
                &canvas::Event::Mouse(mouse::Event::CursorMoved { position: moved }),
                moved
            ));
            let after = demo.chart.model().panes()[pane]
                .right_price_scale()
                .price_range()
                .unwrap();
            assert_ne!(
                after, before,
                "a diagonal plot drag must pan the price range"
            );
            assert!((after.length() - before.length()).abs() < before.length() * 1e-10);
            assert_ne!(offset, demo.chart.model().time_scale().right_offset());
            assert!(
                !demo.chart.model().panes()[pane]
                    .right_price_scale()
                    .is_auto_scale()
            );
            for (other, range) in others.into_iter().enumerate() {
                if other != pane {
                    assert_eq!(
                        demo.chart.model().panes()[other]
                            .right_price_scale()
                            .price_range(),
                        range
                    );
                }
            }
            dispatch_axis_event(
                &mut demo,
                &mut state,
                &canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                moved,
            );
        }
    }
}

#[test]
fn vertical_plot_drag_preserves_time_horizontal_drag_preserves_autoscale_and_focus_ends_both() {
    use super::*;
    use iced::{Point, mouse};
    let mut demo = Demo::new_chart(DisplayConfig::default(), SeriesKind::PriceVolume);
    let _ = demo.update(Message::Bounds(Size::new(800., 500.)));
    let mut state = ChartState::default();
    let x = demo.chart.frame().layout.plot.origin.x as f32 + 200.;
    let press = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
    let release = canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
    let mut point = Point::new(x, 100.);
    dispatch_axis_event(&mut demo, &mut state, &press, point);
    let before = demo.chart.model().panes()[0]
        .right_price_scale()
        .price_range();
    let time = (
        demo.chart.model().time_scale().bar_spacing(),
        demo.chart.model().time_scale().right_offset(),
    );
    for y in [110., 130., 80.] {
        point = Point::new(x, y);
        dispatch_axis_event(
            &mut demo,
            &mut state,
            &canvas::Event::Mouse(mouse::Event::CursorMoved { position: point }),
            point,
        );
    }
    assert_ne!(
        demo.chart.model().panes()[0]
            .right_price_scale()
            .price_range(),
        before
    );
    assert_eq!(
        time,
        (
            demo.chart.model().time_scale().bar_spacing(),
            demo.chart.model().time_scale().right_offset()
        )
    );
    let stopped = demo.chart.model().panes()[0]
        .right_price_scale()
        .price_range();
    dispatch_axis_event(
        &mut demo,
        &mut state,
        &canvas::Event::Window(iced::window::Event::Unfocused),
        point,
    );
    assert!(!demo.chart.is_panning());
    point = Point::new(x + 20., 150.);
    dispatch_axis_event(
        &mut demo,
        &mut state,
        &canvas::Event::Mouse(mouse::Event::CursorMoved { position: point }),
        point,
    );
    assert_eq!(
        demo.chart.model().panes()[0]
            .right_price_scale()
            .price_range(),
        stopped
    );
    assert_eq!(
        time,
        (
            demo.chart.model().time_scale().bar_spacing(),
            demo.chart.model().time_scale().right_offset()
        )
    );
    demo.chart
        .reset_price_scale(0, PriceAxisSide::Right)
        .unwrap();
    point = Point::new(x, 100.);
    dispatch_axis_event(&mut demo, &mut state, &press, point);
    for dx in [10., 30., 50.] {
        point = Point::new(x + dx, 100.);
        dispatch_axis_event(
            &mut demo,
            &mut state,
            &canvas::Event::Mouse(mouse::Event::CursorMoved { position: point }),
            point,
        );
    }
    assert!(
        demo.chart.model().panes()[0]
            .right_price_scale()
            .is_auto_scale()
    );
    assert_ne!(demo.chart.model().time_scale().right_offset(), time.1);
    dispatch_axis_event(&mut demo, &mut state, &release, point);
    assert!(!demo.chart.is_panning());
}
#[test]
fn plot_pan_keeps_origin_pane_when_crossing_volume_and_ends_on_removal() {
    use super::*;
    use iced::{Point, mouse};
    let mut demo = Demo::new_chart(DisplayConfig::default(), SeriesKind::PriceVolume);
    let _ = demo.update(Message::Bounds(Size::new(800., 500.)));
    let mut state = ChartState::default();
    let x = demo.chart.frame().layout.plot.origin.x as f32 + 200.;
    let press = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
    dispatch_axis_event(&mut demo, &mut state, &press, Point::new(x, 100.));
    let bottom = demo.chart.model().panes()[1]
        .right_price_scale()
        .price_range();
    for y in [110., 130., 360.] {
        let point = Point::new(x, y);
        dispatch_axis_event(
            &mut demo,
            &mut state,
            &canvas::Event::Mouse(mouse::Event::CursorMoved { position: point }),
            point,
        );
    }
    assert_eq!(demo.chart.pan_pane().unwrap(), 0);
    assert_eq!(
        demo.chart.model().panes()[1]
            .right_price_scale()
            .price_range(),
        bottom
    );
    let _ = demo.update(Message::Remove);
    assert!(!demo.chart.is_panning());
    let _ = demo.update(Message::Reload);
    assert!(demo.error.is_none());
}
