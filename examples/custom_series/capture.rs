use super::{App, Message};
use iced::advanced::{
    graphics::geometry::Renderer as _,
    renderer::{Headless, Renderer as _},
};
use iced::{Color, Size, widget::canvas::Frame};
use std::{io::Write, path::Path, time::Instant};

pub fn run(directory: &str, wgpu: bool) {
    std::fs::create_dir_all(directory).unwrap();
    let backend = if wgpu { "wgpu" } else { "tiny-skia" };
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some(backend),
    ))
    .unwrap();
    let states = [
        "normal",
        "hover",
        "append",
        "replace",
        "historical",
        "whitespace",
        "style",
        "empty",
        "reload",
        "removed",
        "readded",
        "narrow",
        "zoom",
        "inverted",
        "log",
        "percentage",
        "indexed",
        "hidden",
        "left",
        "multi-pane",
        "manual",
        "pan",
        "markers",
        "touch-pan",
        "touch-pinch",
        "touch-tracking",
        "touch-loss",
        "touch-inertia",
        "touch-narrow",
        "touch-multi-pane",
    ];
    let mut count = 0;
    for ratio in [1., 1.25, 1.5, 2.] {
        for state in states {
            if std::env::args().any(|a| a == "--conflation")
                && ![
                    "normal", "append", "replace", "narrow", "inverted", "manual",
                ]
                .contains(&state)
            {
                continue;
            }

            let mut app = App {
                size: if matches!(state, "narrow" | "touch-narrow") {
                    Size::new(420., 320.)
                } else {
                    Size::new(900., 600.)
                },
                ratio,
                ..Default::default()
            };
            app.chart.prepare(app.size, ratio, Instant::now()).unwrap();
            match state {
                "left" | "multi-pane" | "touch-multi-pane" => {
                    app.apply(Message::Remove);
                    let pane = if matches!(state, "multi-pane" | "touch-multi-pane") {
                        app.chart.add_pane()
                    } else {
                        0
                    };
                    let side = if state == "left" {
                        feather_charts::model::pane::PriceScalePosition::Left
                    } else {
                        feather_charts::model::pane::PriceScalePosition::Right
                    };
                    let handle = app
                        .chart
                        .add_custom_series(
                            super::View::default(),
                            feather_charts::model::series_options::CustomSeriesOptions::default(),
                            pane,
                            side,
                        )
                        .unwrap();
                    app.chart
                        .set_custom_data(&handle, (0..120).map(super::sample).collect())
                        .unwrap();
                    app.custom = Some(handle);
                }
                "pan" => app.chart.set_right_offset(-30.),
                "manual" => {
                    use feather_charts::ui::chart_input::{ChartInteraction, Navigation};
                    for (start, position) in [
                        (
                            Some(iced::Point::new(300., 200.)),
                            iced::Point::new(300., 200.),
                        ),
                        (None, iced::Point::new(340., 240.)),
                    ] {
                        app.chart
                            .update(feather_charts::api::ChartMessage::Interaction(
                                ChartInteraction {
                                    position: Some(position),
                                    navigation: Some(Navigation::Pan { start, position }),
                                    capture: true,
                                },
                            ))
                            .unwrap();
                    }
                    assert!(
                        !app.chart.model().panes()[0]
                            .right_price_scale()
                            .options()
                            .auto_scale
                    );
                }
                "markers" => {
                    let feather_charts::model::icustom_series::CustomSeriesDataItem::Data(data) =
                        super::sample(60)
                    else {
                        unreachable!()
                    };
                    app.chart
                        .add_series_markers(
                            app.custom.as_ref().unwrap().id(),
                            vec![feather_charts::api::SeriesMarker::new(
                                data.time,
                                feather_charts::api::MarkerPosition::AboveBar,
                                feather_charts::api::MarkerShape::Circle,
                                "yellow",
                            )],
                            Default::default(),
                        )
                        .unwrap();
                }
                "append" => app.apply(Message::Append),
                "replace" => app.apply(Message::Replace),
                "historical" => app.apply(Message::Historical),
                "whitespace" => app.apply(Message::Whitespace),
                "style" => app.apply(Message::Style),
                "empty" => app.apply(Message::Clear),
                "reload" => {
                    app.apply(Message::Clear);
                    app.apply(Message::Reload);
                }
                "removed" => app.apply(Message::Remove),
                "readded" => {
                    app.apply(Message::Remove);
                    app.apply(Message::Reload);
                }
                "zoom" => {
                    app.chart
                        .set_logical_range(feather_charts::model::time_data::LogicalRange {
                            from: 35.0.into(),
                            to: 65.0.into(),
                        })
                }
                "inverted" | "log" | "percentage" | "indexed" => {
                    use feather_charts::model::price_scale::{
                        PriceScaleMode, PriceScaleOptionsPatch,
                    };
                    app.chart
                        .apply_price_axis_options(
                            0,
                            feather_charts::model::axis_snapshots::PriceAxisSide::Right,
                            PriceScaleOptionsPatch {
                                invert_scale: Some(state == "inverted"),
                                mode: Some(match state {
                                    "log" => PriceScaleMode::Logarithmic,
                                    "percentage" => PriceScaleMode::Percentage,
                                    "indexed" => PriceScaleMode::IndexedTo100,
                                    _ => PriceScaleMode::Normal,
                                }),
                                ..Default::default()
                            },
                        )
                        .unwrap();
                }
                "hidden" => {
                    let mut options =
                        feather_charts::model::series_options::CustomSeriesOptions::default();
                    options.common.visible = false;
                    app.chart
                        .replace_custom_series_options(app.custom.as_ref().unwrap(), options)
                        .unwrap();
                }
                _ => {}
            }
            app.chart.prepare(app.size, ratio, Instant::now()).unwrap();
            if state.starts_with("touch-") {
                touch(&mut app, state);
            }
            if state == "hover" {
                app.chart
                    .update(feather_charts::api::ChartMessage::Click {
                        pane: 0,
                        position: iced::Point::new(400., 240.),
                    })
                    .unwrap();
                app.chart.prepare(app.size, ratio, Instant::now()).unwrap();
            }
            let mut frame = Frame::new(&renderer, app.size);
            if let Some(multi) = app.chart.multi_frame() {
                multi.draw(&mut frame);
            } else {
                app.chart.frame().draw(&mut frame);
            }
            renderer.reset(iced::Rectangle::with_size(app.size));
            renderer.draw_geometry(frame.into_geometry());
            let physical = Size::new(
                (app.size.width * ratio) as u32,
                (app.size.height * ratio) as u32,
            );
            if state == "normal" && std::env::args().any(|a| a == "--conflation") {
                assert!(app.chart.model().time_scale().conflation_factor() > 1);
            }
            let bytes = renderer.screenshot(physical, ratio, Color::BLACK);
            assert_eq!(
                bytes,
                renderer.screenshot(physical, ratio, Color::BLACK),
                "{state}: unstable retained draw"
            );
            assert!(
                bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .any(|pixel| pixel[0] != pixel[1] || pixel[1] != pixel[2]),
                "{state}: scene is blank"
            );
            let has_boxes = bytes
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[..3] == [233, 30, 99]);
            assert_eq!(
                has_boxes,
                !matches!(state, "empty" | "removed" | "hidden"),
                "{state}: custom boxes missing or stale"
            );
            let mut file =
                std::fs::File::create(Path::new(directory).join(format!("{state}-{ratio}.ppm")))
                    .unwrap();
            write!(file, "P6\n{} {}\n255\n", physical.width, physical.height).unwrap();
            let mut rgb = Vec::with_capacity(bytes.len() / 4 * 3);
            for pixel in bytes.as_chunks::<4>().0 {
                rgb.extend_from_slice(&pixel[..3]);
            }
            file.write_all(&rgb).unwrap();
            count += 1;
        }
    }
    println!("{} custom-series captures passed on {backend}", count);
}

fn touch(app: &mut App, scenario: &str) {
    use feather_charts::api::{ChartMessage, ChartState};
    use iced::{
        Point, Rectangle, mouse, touch,
        widget::canvas::{Event, Program},
    };
    use std::time::Duration;
    let mut state = ChartState::default();
    let now = Instant::now();
    let pane = usize::from(scenario == "touch-multi-pane");
    let plot = app
        .chart
        .multi_frame()
        .map_or(app.chart.frame().layout.plot, |multi| {
            multi.layout.panes[pane].plot
        });
    let position = Point::new(
        plot.origin.x as f32 + plot.size.width as f32 * 0.4,
        plot.origin.y as f32 + plot.size.height as f32 * 0.4,
    );
    let mut deliver = |event: Event, ms| {
        if let Some(mut message) = app
            .chart
            .program(std::convert::identity)
            .update(
                &mut state,
                &event,
                Rectangle::with_size(app.size),
                mouse::Cursor::Unavailable,
            )
            .and_then(|a| a.into_inner().0)
        {
            if let ChartMessage::Touch(input) = &mut message {
                input.now = now + Duration::from_millis(ms);
            }
            app.chart.update(message).unwrap();
        }
    };
    let id = touch::Finger(1);
    deliver(
        Event::Touch(touch::Event::FingerPressed { id, position }),
        0,
    );
    match scenario {
        "touch-pinch" => {
            let second = Point::new(position.x + 60., position.y);
            deliver(
                Event::Touch(touch::Event::FingerPressed {
                    id: touch::Finger(2),
                    position: second,
                }),
                10,
            );
            deliver(
                Event::Touch(touch::Event::FingerMoved {
                    id: touch::Finger(2),
                    position: Point::new(second.x + 60., second.y),
                }),
                30,
            );
        }
        "touch-tracking" => {
            deliver(
                Event::Window(iced::window::Event::RedrawRequested(
                    now + Duration::from_millis(241),
                )),
                241,
            );
            deliver(
                Event::Touch(touch::Event::FingerMoved {
                    id,
                    position: Point::new(position.x + 30., position.y + 20.),
                }),
                260,
            );
            deliver(
                Event::Touch(touch::Event::FingerLifted { id, position }),
                280,
            );
            assert!(app.chart.model().crosshair().position().is_some());
        }
        _ => {
            for (ms, dx) in [(10, 10.), (30, 40.), (50, 80.), (70, 120.)] {
                deliver(
                    Event::Touch(touch::Event::FingerMoved {
                        id,
                        position: Point::new(position.x + dx, position.y + dx * 0.15),
                    }),
                    ms,
                );
            }
            if scenario == "touch-loss" {
                deliver(Event::Touch(touch::Event::FingerLost { id, position }), 80);
                assert!(!app.chart.is_animating());
                assert!(!app.chart.is_panning());
            } else if scenario == "touch-inertia" {
                deliver(
                    Event::Touch(touch::Event::FingerLifted {
                        id,
                        position: Point::new(position.x + 120., position.y + 18.),
                    }),
                    80,
                );
                assert!(app.chart.is_animating());
                app.chart
                    .update(ChartMessage::Tick(now + Duration::from_millis(180)))
                    .unwrap();
            } else {
                assert!(app.chart.is_panning());
                assert_eq!(app.chart.pan_pane(), Some(pane));
            }
        }
    }
}
