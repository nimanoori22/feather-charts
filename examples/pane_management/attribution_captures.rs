use super::*;
use iced::{
    advanced::{
        graphics::geometry::Renderer as _,
        renderer::{Headless, Renderer as _},
    },
    widget::canvas::Program,
};
pub fn run(dir: &std::path::Path, wgpu: bool) {
    let name = if wgpu { "wgpu" } else { "tiny-skia" };
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some(name),
    ))
    .unwrap();
    assert_eq!(renderer.name(), name, "renderer fallback");
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
    mode: &str,
    renderer: &mut iced::Renderer,
    exporter: &mut ChartExporter,
) {
    let dir = dir.join(mode);
    std::fs::create_dir_all(&dir).unwrap();
    let mut images = 0;
    for (w, h) in [(800., 500.), (300., 300.), (300., 90.)] {
        for ratio in [1., 1.25, 1.5, 2.] {
            for scene in [
                "light", "dark", "mismatch", "moved", "removed", "empty", "hidden",
            ] {
                let mut app = App::<C>::new(false);
                app.enable_attribution().unwrap();
                app.size = Size::new(w, h);
                app.ratio = ratio;
                match scene {
                    "dark" => {
                        app.attribution_action(Action::Background).unwrap();
                        app.attribution_action(Action::Text).unwrap();
                    }
                    "mismatch" => app.attribution_action(Action::Background).unwrap(),
                    "moved" => app.action(Message::Swap).unwrap(),
                    "removed" => app.action(Message::Remove).unwrap(),
                    "hidden" => app.attribution_action(Action::Visible).unwrap(),
                    "empty" => {
                        let ids = app
                            .chart
                            .core()
                            .series_ids()
                            .filter(|id| *id != app.custom.id())
                            .collect::<Vec<_>>();
                        for id in ids {
                            app.chart.raw_data(id, vec![]).unwrap();
                        }
                        app.chart.custom_data(&app.custom, vec![]).unwrap();
                    }
                    _ => {}
                }
                app.prepare(Instant::now());
                let size = app.size;
                let mut screen = |app: &App<C>| {
                    renderer.reset(iced::Rectangle::with_size(size));
                    let program = app
                        .chart
                        .core()
                        .program(std::convert::identity::<ChartMessage>);
                    for geometry in program.draw(
                        &ChartState::default(),
                        renderer,
                        &iced::Theme::Light,
                        iced::Rectangle::with_size(size),
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
                let normal = screen(&app);
                assert!(normal == screen(&app), "screen replay changed");
                let file = dir.join(format!("{scene}-{w}x{h}-{ratio}.rgba"));
                std::fs::write(file, &normal).unwrap();
                images += 1;
                let mut snapshots = vec![];
                for top in [false, true] {
                    for crosshair in [false, true] {
                        let shot = app
                            .chart
                            .core()
                            .screenshot_of_prepared(ScreenshotLayers {
                                include_top_layer: top,
                                include_crosshair: crosshair,
                            })
                            .unwrap();
                        let image = exporter
                            .render(
                                &shot,
                                ScreenshotRasterOptions {
                                    pixel_ratio: ratio,
                                    ..Default::default()
                                },
                            )
                            .unwrap();
                        snapshots.push((shot, image));
                    }
                }
                if scene != "hidden" {
                    app.attribution_action(Action::Visible).unwrap();
                    app.prepare(Instant::now());
                }
                let hidden = screen(&app);
                if h >= 300. {
                    assert_eq!(
                        normal == hidden,
                        scene == "hidden",
                        "on-screen visibility: {mode}/{scene}"
                    );
                }
                for (index, (top, crosshair)) in [false, true]
                    .into_iter()
                    .flat_map(|t| [false, true].map(|c| (t, c)))
                    .enumerate()
                {
                    let shot = app
                        .chart
                        .core()
                        .screenshot_of_prepared(ScreenshotLayers {
                            include_top_layer: top,
                            include_crosshair: crosshair,
                        })
                        .unwrap();
                    let image = exporter
                        .render(
                            &shot,
                            ScreenshotRasterOptions {
                                pixel_ratio: ratio,
                                ..Default::default()
                            },
                        )
                        .unwrap();
                    if snapshots[index].1.rgba() != image.rgba() {
                        export::save(&dir.join("export-before"), &snapshots[index].1).unwrap();
                        export::save(&dir.join("export-after"), &image).unwrap();
                        eprintln!(
                            "Export mismatch {mode}/{scene} {w}x{h} DPR {ratio} top={top} crosshair={crosshair}"
                        );
                    }
                    assert!(
                        snapshots[index].1.rgba() == image.rgba(),
                        "visibility toggle changed exported pixels: {mode}/{scene}"
                    );
                }
                drop(app);
                for (shot, image) in snapshots {
                    assert!(
                        image.rgba()
                            == exporter
                                .render(
                                    &shot,
                                    ScreenshotRasterOptions {
                                        pixel_ratio: ratio,
                                        ..Default::default()
                                    }
                                )
                                .unwrap()
                                .rgba(),
                        "retained export changed"
                    );
                }
            }
        }
    }
    assert_eq!(images, 84);
    println!("ATTRIBUTION_CAPTURES {mode} {images}");
}
