//! Frozen locale scenes on both actual native renderers.
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
            let mut english = None;
            for (scene, locale) in [
                ("english", "en-US"),
                ("french", "fr-FR"),
                ("german", "de-DE"),
                ("arabic", "ar-EG"),
                ("latin", "ar-EG-u-nu-latn"),
                ("extended", "ar-EG-u-nu-arabext"),
                ("german-arabic", "de-DE-u-nu-arab"),
                ("fallback", "bad_locale"),
                ("custom", "ar-EG"),
                ("recovered", "ar-EG"),
            ] {
                let mut app = App::<C>::new(false);
                app.enable_locale().unwrap();
                app.size = Size::new(w, h);
                app.ratio = ratio;
                app.locale_action(locale::Action::Set(locale));
                app.locale_action(locale::Action::Date);
                if scene == "custom" {
                    app.locale_action(locale::Action::Callback);
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
                    .synthetic(app.line, Some(&C::locale_key(10)), 4.5)
                    .unwrap();
                app.chart.prepare(app.size, ratio, Instant::now()).unwrap();
                let pixels = metadata_captures::screen(&app.chart, app.size, ratio, renderer);
                assert!(
                    pixels == metadata_captures::screen(&app.chart, app.size, ratio, renderer),
                    "locale replay changed"
                );
                if scene == "english" {
                    english = Some(pixels.clone());
                }
                if domain == "calendar" && scene == "arabic" {
                    assert!(
                        Some(&pixels) != english.as_ref(),
                        "Arabic labels must change actual chart pixels"
                    );
                }
                if scene == "fallback" {
                    assert!(
                        Some(&pixels) == english.as_ref(),
                        "unknown locale must match English pixels"
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
                app.locale_action(locale::Action::Set("en-US"));
                app.action(Message::Reset).unwrap();
                drop(app);
                assert!(
                    original.rgba() == exporter.render(&shot, options).unwrap().rgba(),
                    "locale export must own labels after changing/dropping chart"
                );
                count += 1;
            }
        }
    }
    assert_eq!(count, 120);
    println!("TIME_LOCALE_CAPTURES {domain} {count}");
}
