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
            demo.model.zoom_time(
                200.0.into(),
                if (continuous && i < 70) || (!continuous && i % 2 == 0) {
                    0.32
                } else {
                    -0.32 / 1.032
                },
            );
            demo.model
                .set_crosshair_position(
                    0,
                    Some(feather_charts::renderers::line_renderer::Point { x: 200., y: 200. }),
                )
                .unwrap();
            let model = start.elapsed();
            let input = ChartFrameInput {
                series: Some(ID),
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
            demo.frame = IcedChartFrame::from_snapshot(snapshot, &demo.fonts).unwrap();
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
                bounds.x + demo.frame.layout.plot.origin.x as f32 + 200.,
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
                    if let Some(action) = (Chart {
                        snapshot: &demo.frame,
                        multi: None,
                        stretches: vec![],
                        animate: false,
                        enabled: true,
                    })
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
            assert!(demo.model.crosshair().position().is_some());
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
            let width_before = demo.frame.layout.plot.size.width;
            let retained = format!("{:?}", demo.frame);
            let point = Point::new(demo.frame.layout.plot.origin.x as f32 + 200., 100.);
            let bounds = Rectangle::with_size(demo.size);
            let event = canvas::Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Lines { x: 0., y: 1. },
            });
            let before = demo.model.time_scale().bar_spacing();
            let message = Chart {
                snapshot: &demo.frame,
                multi: None,
                stretches: vec![],
                animate: false,
                enabled: true,
            }
            .update(&mut state, &event, bounds, mouse::Cursor::Available(point))
            .unwrap()
            .into_inner()
            .0
            .unwrap();
            let _ = demo.update(message);
            assert!(demo.error.is_none());
            assert!(demo.model.time_scale().bar_spacing() > before);
            assert!(!demo.frame.plot.crosshair.is_empty());
            assert_eq!(
                demo.frame.plot.crosshair_markers.is_empty(),
                candlesticks && !demo.left_registered
            );
            let axes = demo.frame.plot.axes.as_ref().unwrap();
            assert!(axes.right.crosshair_label.is_some());
            assert!(axes.time.crosshair_label.is_some());
            assert_eq!(demo.frame.layout.plot.size.width, width_before);
            assert_ne!(format!("{:?}", demo.frame), retained);
            let _ = demo.update(Message::Animate);
            let _ = demo.update(Message::Interaction(ChartInteraction {
                position: Some(Point::new(200., 100.)),
                navigation: Some(Navigation::Scroll {
                    start: Some(200.),
                    x: 200.,
                }),
                capture: true,
            }));
            assert!(!demo.controller.is_active());
            let offset = demo.model.time_scale().right_offset();
            let _ = demo.update(Message::Interaction(ChartInteraction {
                position: None,
                navigation: Some(Navigation::Scroll {
                    start: None,
                    x: 250.,
                }),
                capture: true,
            }));
            assert_ne!(demo.model.time_scale().right_offset(), offset);
            let leave = canvas::Event::Window(iced::window::Event::Unfocused);
            let message = Chart {
                snapshot: &demo.frame,
                multi: None,
                stretches: vec![],
                animate: false,
                enabled: true,
            }
            .update(&mut state, &leave, bounds, mouse::Cursor::Unavailable)
            .unwrap()
            .into_inner()
            .0
            .unwrap();
            let _ = demo.update(message);
            assert!(demo.frame.plot.crosshair.is_empty());
            assert!(
                demo.frame
                    .plot
                    .axes
                    .as_ref()
                    .unwrap()
                    .time
                    .crosshair_label
                    .is_none()
            );
            let _ = demo.update(Message::Remove);
            assert!(demo.frame.plot.crosshair_markers.is_empty());
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
        let layout = demo.multi_frame.as_ref().unwrap().layout.panes[pane].plot;
        let point = Point::new(layout.origin.x as f32 + 200., layout.origin.y as f32 + 50.);
        let movement = canvas::Event::Mouse(mouse::Event::CursorMoved { position: point });
        let wheel = canvas::Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0., y: 1. },
        });
        for event in [&movement, &wheel] {
            let action = Chart {
                snapshot: &demo.frame,
                multi: demo.multi_frame.as_ref(),
                stretches: demo
                    .model
                    .panes()
                    .iter()
                    .map(|p| p.stretch_factor())
                    .collect(),
                animate: false,
                enabled: true,
            }
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
        assert_eq!(demo.model.crosshair().position().unwrap().pane, pane);
        let mut published = 0;
        for _ in 0..10 {
            if let Some(action) = (Chart {
                snapshot: &demo.frame,
                multi: demo.multi_frame.as_ref(),
                stretches: demo
                    .model
                    .panes()
                    .iter()
                    .map(|p| p.stretch_factor())
                    .collect(),
                animate: false,
                enabled: true,
            })
            .update(
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
        let frame = demo.multi_frame.as_ref().unwrap();
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
        demo.multi_frame.as_ref().unwrap().layout.separators[0]
            .origin
            .y as f32,
    );
    let press = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
    let movement = canvas::Event::Mouse(mouse::Event::CursorMoved { position: point });
    let spacing = demo.model.time_scale().bar_spacing();
    for (event, cursor) in [
        (&press, point),
        (&movement, Point::new(200., point.y - 50.)),
    ] {
        let action = Chart {
            snapshot: &demo.frame,
            multi: demo.multi_frame.as_ref(),
            stretches: demo
                .model
                .panes()
                .iter()
                .map(|p| p.stretch_factor())
                .collect(),
            animate: false,
            enabled: true,
        }
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
    assert_eq!(spacing, demo.model.time_scale().bar_spacing());
    assert!(demo.model.panes()[0].stretch_factor() < 2.);
    let _ = demo.update(Message::Append);
    let _ = demo.update(Message::Replace);
    let frame = demo.multi_frame.as_ref().unwrap();
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
    let action = Chart {
        snapshot: &demo.frame,
        multi: demo.multi_frame.as_ref(),
        stretches: vec![2., 1.],
        animate: false,
        enabled: true,
    }
    .update(
        &mut state,
        &press,
        Rectangle::with_size(demo.size),
        mouse::Cursor::Available(point),
    )
    .unwrap();
    let _ = demo.update(action.into_inner().0.unwrap());
    // A new selection in an identically sized pane must not inherit the old drag.
    demo.multi_frame.as_mut().unwrap().series[0] = Some(SeriesId::new(100));
    let movement = canvas::Event::Mouse(mouse::Event::CursorMoved {
        position: Point::new(250., 100.),
    });
    let action = Chart {
        snapshot: &demo.frame,
        multi: demo.multi_frame.as_ref(),
        stretches: vec![2., 1.],
        animate: false,
        enabled: true,
    }
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
                demo.frame.layout.available.width,
                f64::from(demo.size.width)
            );
            assert_eq!(demo.frame.time.size.width, demo.frame.plot.size.width);
            if scenario == super::DisplayScenario::Empty {
                assert!(!demo.has_series_geometry());
                let _ = demo.update(super::Message::Reload);
            }
            assert!(demo.has_series_geometry());
        }
    }
}
