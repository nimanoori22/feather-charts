#[test]
fn profile_zoom_frame_stages() {
    profile_zoom(false);
}
#[test]
fn profile_candle_zoom_frame_stages() {
    profile_zoom(true);
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
