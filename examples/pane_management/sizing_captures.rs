//! Actual renderer evidence for persistent pane sizing.
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
        for ratio in [1., 1.25, 1.5, 2.] {
            for scene in [
                "set",
                "minimum",
                "maximum",
                "restore",
                "moved",
                "removed",
                "recovered",
            ] {
                let mut app = App::<C>::new(false);
                app.sizing = Some(Default::default());
                app.size = Size::new(w, h);
                app.ratio = ratio;
                let now = Instant::now();
                app.chart.prepare(app.size, ratio, now).unwrap();
                app.sizing_action(sizing::Action::Next).unwrap();
                app.sizing_action(sizing::Action::Save).unwrap();
                let saved = app.sizing.as_ref().unwrap().saved.clone();
                app.sizing_action(sizing::Action::Large).unwrap();
                app.chart.prepare(app.size, ratio, now).unwrap();
                match scene {
                    "set" => {}
                    "minimum" => app.sizing_action(sizing::Action::Minimum).unwrap(),
                    "maximum" => app.sizing_action(sizing::Action::Maximum).unwrap(),
                    "restore" => {
                        app.sizing_action(sizing::Action::Restore).unwrap();
                        assert_eq!(
                            app.chart
                                .core()
                                .model()
                                .panes()
                                .iter()
                                .map(|p| p.stretch_factor())
                                .collect::<Vec<_>>(),
                            saved
                        );
                    }
                    "moved" => app.action(Message::Swap).unwrap(),
                    "removed" => app.action(Message::Remove).unwrap(),
                    "recovered" => {
                        app.chart.prepare(Size::new(300., 10.), ratio, now).unwrap();
                        let factors = app
                            .chart
                            .core()
                            .model()
                            .panes()
                            .iter()
                            .map(|p| p.stretch_factor())
                            .collect::<Vec<_>>();
                        app.sizing_action(sizing::Action::Medium).unwrap();
                        assert_eq!(
                            app.chart
                                .core()
                                .model()
                                .panes()
                                .iter()
                                .map(|p| p.stretch_factor())
                                .collect::<Vec<_>>(),
                            factors
                        );
                    }
                    _ => unreachable!(),
                }
                app.chart.prepare(app.size, ratio, now).unwrap();
                assert!(
                    app.chart
                        .core()
                        .model()
                        .panes()
                        .iter()
                        .all(|p| p.height().is_finite() && p.stretch_factor() > 0.)
                );
                let pixels = screen(&app.chart, app.size, ratio, renderer);
                assert!(
                    pixels == screen(&app.chart, app.size, ratio, renderer),
                    "screen replay changed"
                );
                std::fs::write(dir.join(format!("{scene}-{w}x{h}-{ratio}.rgba")), pixels).unwrap();
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
                app.sizing_action(sizing::Action::Restore).unwrap();
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
    assert_eq!(count, 84);
    println!("PANE_SIZING_CAPTURES {domain} {count}");
}
