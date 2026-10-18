use super::*;
use iced::{
    advanced::{
        graphics::geometry::Renderer as _,
        renderer::{Headless, Renderer as _},
    },
    widget::canvas::Program,
};
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
        for ratio in [1., 1.25, 1.5, 2.] {
            for scene in [
                "left",
                "right",
                "overlay",
                "shared",
                "shared-leave",
                "empty",
                "moved",
                "options",
            ] {
                let mut app = App::<C>::new(false);
                app.enable_assignment().unwrap();
                app.size = Size::new(w, h);
                app.ratio = ratio;
                match scene {
                    "left" => app.assignment_action(Action::Left).unwrap(),
                    "right" => app.assignment_action(Action::Right).unwrap(),
                    "overlay" => app.assignment_action(Action::Shared).unwrap(),
                    "empty" => app.assignment_action(Action::Empty).unwrap(),
                    "shared" | "shared-leave" => {
                        app.action(Message::CustomToLine).unwrap();
                        app.assignment_action(Action::Shared).unwrap();
                        app.assignment_action(Action::Select).unwrap();
                        app.assignment_action(Action::Shared).unwrap();
                        app.assignment_action(Action::Freeze).unwrap();
                        if scene == "shared-leave" {
                            app.assignment_action(Action::Other).unwrap();
                        }
                    }
                    "moved" => {
                        app.assignment_action(Action::Shared).unwrap();
                        app.action(Message::LineNew).unwrap();
                        app.show_assignment_axes().unwrap();
                    }
                    "options" => {
                        app.assignment_action(Action::Method).unwrap();
                        app.assignment_action(Action::Left).unwrap();
                        app.assignment_action(Action::Select).unwrap();
                        app.assignment_action(Action::Other).unwrap();
                    }
                    _ => unreachable!(),
                }
                let line_id = app.chart.core().series_price_scale(app.line).unwrap();
                assert_eq!(
                    app.chart
                        .core()
                        .model()
                        .series(app.line)
                        .unwrap()
                        .common_options()
                        .price_scale_id
                        .as_deref(),
                    Some(line_id.id())
                );
                assert_eq!(
                    app.custom.price_scale_id().as_deref(),
                    Some(
                        app.chart
                            .core()
                            .series_price_scale(app.custom.id())
                            .unwrap()
                            .id()
                    )
                );
                app.chart.prepare(app.size, ratio, Instant::now()).unwrap();
                // Populate crosshair/price labels through the public owner.
                let plot = app.chart.core().frame().layout.plot;
                if plot.size.height > 0. {
                    app.chart
                        .update(ChartMessage::Interaction(
                            feather_charts::ui::chart_input::ChartInteraction {
                                position: Some(iced::Point::new(
                                    (plot.size.width / 2.) as f32,
                                    (plot.size.height / 2.) as f32,
                                )),
                                navigation: None,
                                capture: false,
                            },
                        ))
                        .unwrap();
                }
                app.chart.prepare(app.size, ratio, Instant::now()).unwrap();
                let mut screen = |app: &App<C>| {
                    renderer.reset(iced::Rectangle::with_size(app.size));
                    let program = app
                        .chart
                        .core()
                        .program(std::convert::identity::<ChartMessage>);
                    for geometry in program.draw(
                        &ChartState::default(),
                        renderer,
                        &iced::Theme::Light,
                        iced::Rectangle::with_size(app.size),
                        iced::mouse::Cursor::Unavailable,
                    ) {
                        renderer.draw_geometry(geometry);
                    }
                    renderer.screenshot(
                        Size::new((w * ratio).ceil() as u32, (h * ratio).ceil() as u32),
                        ratio,
                        iced::Color::WHITE,
                    )
                };
                let pixels = screen(&app);
                assert!(pixels == screen(&app), "screen replay changed");
                std::fs::write(dir.join(format!("{scene}-{w}x{h}-{ratio}.rgba")), &pixels).unwrap();
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
                app.chart
                    .assign_scale(app.line, PriceScalePosition::Overlay("retained-new".into()))
                    .unwrap();
                app.chart
                    .prepare(Size::new(500., 400.), ratio, Instant::now())
                    .unwrap();
                drop(app);
                let retained = exporter.render(&shot, options).unwrap();
                assert!(
                    original.rgba() == retained.rgba(),
                    "retained export changed"
                );
                if w == 800. && ratio == 1. {
                    export::save(&dir.join(scene), &original).unwrap();
                }
                count += 1;
            }
        }
    }
    assert_eq!(count, 96);
    println!("SCALE_ASSIGNMENT_CAPTURES {domain} {count}");
}
