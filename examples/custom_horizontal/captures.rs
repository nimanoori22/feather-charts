use super::*;
fn suffix(path: &std::path::Path, extension: &str) -> std::path::PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(extension);
    name.into()
}
pub fn save(path: &std::path::Path, image: &ScreenshotImage) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(suffix(path, ".rgba"), image.rgba())?;
    std::fs::write(
        suffix(path, ".json"),
        format!(
            "{{\"width\":{},\"height\":{},\"format\":\"straight-alpha sRGB RGBA8\"}}\n",
            image.size().width,
            image.size().height
        ),
    )
}
pub fn run(dir: &std::path::Path, backend: ScreenshotBackend) {
    let mut exporter =
        iced::futures::executor::block_on(ChartExporter::new(backend)).expect("requested backend");
    assert_eq!(exporter.backend(), backend);
    let mut count = 0;
    for (w, h) in [(800., 500.), (300., 300.), (300., 90.)] {
        for ratio in [1., 1.25, 1.5, 2.] {
            for scene in ["initial", "updates", "labels", "panes", "empty"] {
                let mut app = App::new(false, None);
                app.size = Size::new(w, h);
                app.ratio = ratio;
                let actions: &[Message] = match scene {
                    "updates" => &[
                        Message::Latest,
                        Message::History,
                        Message::Pop,
                        Message::Append,
                    ],
                    "labels" => &[Message::Labels],
                    "panes" => &[Message::Transfer, Message::Swap],
                    "empty" => &[Message::Empty],
                    _ => &[],
                };
                for action in actions {
                    app.action(action).unwrap();
                }
                app.prepare(Instant::now()).unwrap();
                for top in [false, true] {
                    let shot = app
                        .chart
                        .screenshot_of_prepared(ScreenshotLayers {
                            include_top_layer: top,
                            include_crosshair: false,
                        })
                        .unwrap();
                    let options = ScreenshotRasterOptions {
                        pixel_ratio: ratio,
                        ..Default::default()
                    };
                    let image = exporter.render(&shot, options).unwrap();
                    assert_eq!(
                        image.size(),
                        Size::new((w * ratio).ceil() as u32, (h * ratio).ceil() as u32)
                    );
                    assert_eq!(
                        image.rgba(),
                        exporter.render(&shot, options).unwrap().rgba()
                    );
                    save(
                        &dir.join(format!("{scene}-{w}x{h}-{ratio}-top{top}")),
                        &image,
                    )
                    .unwrap();
                    count += 1;
                }
                let retained = app
                    .chart
                    .screenshot_of_prepared(ScreenshotLayers {
                        include_top_layer: true,
                        include_crosshair: true,
                    })
                    .unwrap();
                let options = ScreenshotRasterOptions {
                    pixel_ratio: ratio,
                    ..Default::default()
                };
                let before = exporter.render(&retained, options).unwrap();
                app.action(&Message::Reset).unwrap();
                app.action(&Message::Labels).unwrap();
                app.prepare(Instant::now()).unwrap();
                drop(app);
                assert_eq!(
                    before.rgba(),
                    exporter.render(&retained, options).unwrap().rgba(),
                    "snapshot changed after owner mutation/drop"
                );
            }
        }
    }
    assert_eq!(count, 120);
    println!("CUSTOM_HORIZONTAL_CAPTURES {{\"backend\":\"{backend:?}\",\"images\":{count}}}");
}
