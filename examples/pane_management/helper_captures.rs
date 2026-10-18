//! Renderer evidence for the public-helper consumer and linked chart.
use super::*;
use iced::advanced::{
    graphics::geometry::Renderer as _,
    renderer::{Headless, Renderer as _},
};
use iced::widget::canvas::Program;
pub fn run(dir: &std::path::Path, wgpu: bool) {
    let backend = if wgpu { "wgpu" } else { "tiny-skia" };
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some(backend),
    ))
    .unwrap();
    assert_eq!(renderer.name(), backend, "renderer fallback");
    let mut exporter = iced::futures::executor::block_on(ChartExporter::new(if wgpu {
        ScreenshotBackend::Wgpu
    } else {
        ScreenshotBackend::Software
    }))
    .unwrap();
    capture::<Chart>(dir, "calendar", &mut renderer, &mut exporter);
    capture::<OptionsChart>(dir, "strike", &mut renderer, &mut exporter);
    capture::<YieldCurveChart>(dir, "maturity", &mut renderer, &mut exporter);
    capture::<Chart<session_behavior::Sessions>>(dir, "sessions", &mut renderer, &mut exporter);
}
fn screen<C: Host>(chart: &C, size: Size, ratio: f32, renderer: &mut iced::Renderer) -> Vec<u8> {
    renderer.reset(iced::Rectangle::with_size(size));
    for geometry in chart
        .core()
        .program(std::convert::identity::<ChartMessage>)
        .draw(
            &ChartState::default(),
            renderer,
            &iced::Theme::Light,
            iced::Rectangle::with_size(size),
            iced::mouse::Cursor::Unavailable,
        )
    {
        renderer.draw_geometry(geometry);
    }
    renderer.screenshot(
        Size::new(
            (size.width * ratio).ceil() as u32,
            (size.height * ratio).ceil() as u32,
        ),
        ratio,
        iced::Color::WHITE,
    )
}
fn capture<C: Host>(
    dir: &std::path::Path,
    domain: &str,
    renderer: &mut iced::Renderer,
    exporter: &mut ChartExporter,
) {
    let dir = dir.join(domain);
    std::fs::create_dir_all(&dir).unwrap();
    let mut count = 0;
    for (w, h) in [(800., 500.), (300., 300.), (300., 90.)] {
        for ratio in [1., 1.25, 2.] {
            for scene in [
                "range", "reset", "custom", "moved", "synced", "empty", "outside",
            ] {
                let mut app = App::<C>::new(false);
                app.enable_helpers().unwrap();
                app.size = Size::new(w, h);
                app.ratio = ratio;
                app.helpers.as_mut().unwrap().size = app.size;
                let now = Instant::now();
                app.chart.prepare(app.size, ratio, now).unwrap();
                app.helpers
                    .as_mut()
                    .unwrap()
                    .peer
                    .prepare(app.size, ratio, now)
                    .unwrap();
                match scene {
                    "range" => app.helper_action(helpers::Action::Range).unwrap(),
                    "reset" => {
                        app.helper_action(helpers::Action::Range).unwrap();
                        app.chart.prepare(app.size, ratio, now).unwrap();
                        app.helper_action(helpers::Action::Reset).unwrap();
                    }
                    "custom" => {
                        app.selected_custom = true;
                        app.helper_action(helpers::Action::Range).unwrap();
                        app.helper_action(helpers::Action::Lines).unwrap();
                    }
                    "moved" => {
                        app.action(Message::Swap).unwrap();
                        app.action(Message::LineNew).unwrap();
                        app.chart
                            .assign_scale(app.line, PriceScalePosition::Left)
                            .unwrap();
                        app.chart.prepare(app.size, ratio, now).unwrap();
                        app.helper_action(helpers::Action::Inspect).unwrap();
                    }
                    "synced" => {
                        app.chart.fit();
                        app.chart.prepare(app.size, ratio, now).unwrap();
                        let x = app
                            .chart
                            .core()
                            .horizontal_to_coordinate(&C::key(12))
                            .unwrap()
                            .unwrap();
                        let events = app
                            .chart
                            .update(ChartMessage::Interaction(
                                feather_charts::ui::chart_input::ChartInteraction {
                                    position: Some(iced::Point::new(
                                        x as f32,
                                        (app.chart.core().frame().layout.plot.size.height * 0.5)
                                            as f32,
                                    )),
                                    navigation: None,
                                    capture: false,
                                },
                            ))
                            .unwrap();
                        app.consume_events(events);
                        assert!(app.helpers.as_ref().unwrap().deliveries > 0);
                        assert!(
                            app.helpers
                                .as_ref()
                                .unwrap()
                                .peer
                                .core()
                                .model()
                                .crosshair()
                                .origin()
                                .is_some()
                        );
                    }
                    "outside" => {
                        app.helper_action(helpers::Action::Range).unwrap();
                        app.chart.prepare(app.size, ratio, now).unwrap();
                        app.chart
                            .synthetic(app.line, Some(&C::key(23)), 20.)
                            .unwrap();
                        if app.chart.core().frame().layout.plot.size.height > 0. {
                            let expected = app
                                .chart
                                .core()
                                .horizontal_to_index(&C::key(18), true)
                                .unwrap()
                                .unwrap();
                            assert_eq!(
                                app.chart
                                    .core()
                                    .model()
                                    .crosshair()
                                    .position()
                                    .unwrap()
                                    .index,
                                expected
                            );
                        }
                    }
                    "empty" => {
                        app.chart.raw_data(app.line, vec![]).unwrap();
                        app.chart.custom_data(&app.custom, vec![]).unwrap();
                    }
                    _ => unreachable!(),
                }
                app.chart.prepare(app.size, ratio, now).unwrap();
                let pixels = screen(&app.chart, app.size, ratio, renderer);
                assert!(
                    pixels == screen(&app.chart, app.size, ratio, renderer),
                    "screen replay changed"
                );
                std::fs::write(dir.join(format!("{scene}-{w}x{h}-{ratio}.rgba")), pixels).unwrap();
                if scene == "synced" {
                    let panel = app.helpers.as_ref().unwrap();
                    let pixels = screen(&panel.peer, panel.size, ratio, renderer);
                    assert!(
                        pixels == screen(&panel.peer, panel.size, ratio, renderer),
                        "linked screen replay changed"
                    );
                    std::fs::write(dir.join(format!("peer-{w}x{h}-{ratio}.rgba")), pixels).unwrap();
                }
                let shot = app
                    .chart
                    .core()
                    .screenshot_of_prepared(ScreenshotLayers {
                        include_top_layer: true,
                        include_crosshair: true,
                    })
                    .unwrap();
                let options = ScreenshotRasterOptions {
                    pixel_ratio: ratio,
                    ..Default::default()
                };
                let original = exporter.render(&shot, options).unwrap();
                app.helper_action(helpers::Action::Reset).unwrap();
                app.chart
                    .prepare(Size::new(500., 400.), ratio, now)
                    .unwrap();
                drop(app);
                assert!(
                    original.rgba() == exporter.render(&shot, options).unwrap().rgba(),
                    "retained export changed"
                );
                if w == 800. && ratio == 1. {
                    export::save(&dir.join(scene), &original).unwrap();
                }
                count += 1;
            }
        }
    }
    assert_eq!(count, 63);
    println!("PUBLIC_HELPER_CAPTURES {domain} {count} plus 9 linked-chart replays");
}
