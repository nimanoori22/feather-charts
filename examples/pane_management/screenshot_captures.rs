use super::*;
pub fn run(dir: &std::path::Path, wgpu: bool) {
    let mut exporter = iced::futures::executor::block_on(ChartExporter::new(if wgpu {
        ScreenshotBackend::Wgpu
    } else {
        ScreenshotBackend::Software
    }))
    .unwrap();
    capture::<Chart>(dir.join("calendar"), &mut exporter);
    capture::<OptionsChart>(dir.join("price"), &mut exporter);
    capture::<YieldCurveChart>(dir.join("yield"), &mut exporter);
}
fn capture<C: Host>(dir: std::path::PathBuf, exporter: &mut ChartExporter) {
    std::fs::create_dir_all(&dir).unwrap();
    for (w, h) in [(800., 500.), (300., 300.), (300., 90.)] {
        for ratio in [1., 1.25, 1.5, 2.] {
            for scene in ["base", "crosshair", "transferred", "hidden-axis"] {
                let mut app = App::<C>::new(false);
                app.size = Size::new(w, h);
                app.ratio = ratio;
                let now = Instant::now();
                if scene == "transferred" {
                    app.action(Message::LineNext).unwrap();
                    app.action(Message::CustomToLine).unwrap();
                }
                if scene == "hidden-axis" {
                    app.chart.axis_visible(false);
                }
                app.chart.prepare(app.size, ratio, now).unwrap();
                if scene == "crosshair" {
                    use feather_charts::ui::chart_input::ChartInteraction;
                    app.chart
                        .update(ChartMessage::Interaction(ChartInteraction {
                            position: Some(iced::Point::new(w * 0.5, h * 0.25)),
                            navigation: None,
                            capture: false,
                        }))
                        .unwrap();
                    app.chart.prepare(app.size, ratio, now).unwrap();
                }
                let mut images = vec![];
                for (top, cross) in [(false, false), (false, true), (true, false), (true, true)] {
                    let screenshot = app
                        .chart
                        .core()
                        .screenshot_of_prepared(ScreenshotLayers {
                            include_top_layer: top,
                            include_crosshair: cross,
                        })
                        .unwrap();
                    let options = ScreenshotRasterOptions {
                        pixel_ratio: ratio,
                        ..Default::default()
                    };
                    let image = exporter.render(&screenshot, options).unwrap();
                    assert_eq!(
                        image.rgba(),
                        exporter.render(&screenshot, options).unwrap().rgba(),
                        "unstable export"
                    );
                    let stem = format!("{w}x{h}-{ratio}-{scene}-top{top}-cross{cross}");
                    export::save(&dir.join(stem), &image).unwrap();
                    images.push(image);
                }
                assert_eq!(
                    images[0].rgba(),
                    images[1].rgba(),
                    "crosshair must not force top canvas"
                );
                let retained = app
                    .chart
                    .core()
                    .screenshot_of_prepared(ScreenshotLayers {
                        include_top_layer: true,
                        include_crosshair: true,
                    })
                    .unwrap();
                app.action(Message::Swap).unwrap();
                app.chart.prepare(app.size, ratio, now).unwrap();
                drop(app);
                assert_eq!(
                    images[3].rgba(),
                    exporter
                        .render(
                            &retained,
                            ScreenshotRasterOptions {
                                pixel_ratio: ratio,
                                ..Default::default()
                            }
                        )
                        .unwrap()
                        .rgba()
                );
            }
        }
    }
    // Verify backend alpha and top-to-bottom padding with a non-square chart.
    let mut app = App::<C>::new(false);
    app.chart
        .prepare(Size::new(300., 90.), 1., Instant::now())
        .unwrap();
    let shot = app
        .chart
        .fresh_screenshot(Default::default(), Instant::now())
        .unwrap();
    let image = exporter
        .render(
            &shot,
            ScreenshotRasterOptions {
                output_size: Some(Size::new(100, 100)),
                background: iced::Color::from_rgba(1., 0., 0., 0.5),
                ..Default::default()
            },
        )
        .unwrap();
    println!("{:?} alpha {:?}", exporter.backend(), &image.rgba()[..4]);
    assert_eq!(&image.rgba()[..4], &[255, 0, 0, 128]);
    let colored = exporter
        .render(
            &shot,
            ScreenshotRasterOptions {
                output_size: Some(Size::new(100, 100)),
                background: iced::Color::from_rgba(0.25, 0.5, 0.75, 0.5),
                ..Default::default()
            },
        )
        .unwrap();
    for (actual, expected) in colored.rgba()[..4].iter().zip([64u8, 128, 191, 128]) {
        assert!(
            actual.abs_diff(expected) <= 3,
            "RGBA/alpha normalization {:?}",
            &colored.rgba()[..4]
        );
    }
    export::save(&dir.join("alpha-fit"), &image).unwrap();
}
