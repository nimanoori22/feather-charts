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
                "series",
                "global",
                "percentage",
                "indexed",
                "moved",
                "cleared",
            ] {
                let mut app = App::<C>::new(false);
                app.formatting = Some(Default::default());
                app.size = Size::new(w, h);
                app.ratio = ratio;
                app.formatting_action(Action::Clear).unwrap();
                app.formatting_action(Action::SeriesBatch).unwrap();
                if scene != "series" {
                    for action in [
                        Action::Price,
                        Action::PriceTicks,
                        Action::Percent,
                        Action::PercentTicks,
                        Action::Horizontal,
                    ] {
                        app.formatting_action(action).unwrap();
                    }
                }
                if matches!(scene, "percentage" | "indexed") {
                    for _ in 0..if scene == "percentage" { 2 } else { 3 } {
                        app.formatting_action(Action::Mode).unwrap();
                    }
                }
                if scene == "moved" {
                    app.action(Message::Swap).unwrap();
                    app.action(Message::LineNew).unwrap();
                }
                if scene == "cleared" {
                    app.formatting_action(Action::Clear).unwrap();
                }
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
                app.formatting_action(Action::Clear).unwrap();
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
    assert_eq!(count, 72);
    println!("FORMATTING_CAPTURES {domain} {count}");
}
