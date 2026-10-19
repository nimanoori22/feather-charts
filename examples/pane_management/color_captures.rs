//! Configured color scenes on both actual native renderers.
use super::*;
use iced::advanced::renderer::Headless;
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
            let mut baseline = None;
            for scene in [
                "initial",
                "palette",
                "row",
                "extensions",
                "invalid",
                "p3",
                "recovered",
            ] {
                let mut app = App::<C>::new_colors(false).unwrap();
                app.size = Size::new(w, h);
                app.ratio = ratio;
                match scene {
                    "palette" => app.color_action(color_policy::Action::Palette).unwrap(),
                    "row" => app.color_action(color_policy::Action::Row).unwrap(),
                    "extensions" | "recovered" => {
                        app.color_action(color_policy::Action::Background).unwrap();
                        app.color_action(color_policy::Action::Extensions).unwrap();
                    }
                    "invalid" => app.color_action(color_policy::Action::Invalid).unwrap(),
                    "p3" => app.color_action(color_policy::Action::Target).unwrap(),
                    _ => {}
                }
                if scene == "recovered" {
                    app.action(Message::Swap).unwrap();
                    app.action(Message::LineNew).unwrap();
                    app.chart
                        .prepare(Size::new(300., 10.), ratio, Instant::now())
                        .unwrap();
                }
                app.chart.prepare(app.size, ratio, Instant::now()).unwrap();
                app.chart
                    .synthetic(app.line, Some(&C::key(10)), 4.5)
                    .unwrap();
                app.chart.prepare(app.size, ratio, Instant::now()).unwrap();
                let calls = app.colors.as_ref().unwrap().calls.clone();
                let before = calls.get();
                let pixels = metadata_captures::screen(&app.chart, app.size, ratio, renderer);
                assert!(
                    pixels == metadata_captures::screen(&app.chart, app.size, ratio, renderer),
                    "color replay changed"
                );
                if scene == "initial" {
                    baseline = Some(pixels.clone());
                }
                if matches!(scene, "invalid" | "p3") {
                    assert!(
                        Some(&pixels) == baseline.as_ref(),
                        "rejected options changed chart pixels"
                    );
                }
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
                app.action(Message::Reset).unwrap();
                drop(app);
                assert!(
                    original.rgba() == exporter.render(&shot, options).unwrap().rgba(),
                    "export must own colors after changing/dropping chart"
                );
                assert_eq!(
                    calls.get(),
                    before,
                    "rendering and retained export must not invoke parsers"
                );
                count += 1;
            }
        }
    }
    assert_eq!(count, 84);
    println!("CONFIGURED_COLORS_CAPTURES {domain} {count}");
}
