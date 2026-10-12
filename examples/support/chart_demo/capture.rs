use super::*;

/// Real Iced backend output at explicit physical sizes; no window-scale spoofing.
/// Captures are PPM plus JSON metadata, and can be converted to PNG externally.
pub(super) fn capture_display(
    directory: &std::path::Path,
    wgpu: bool,
    candlesticks: bool,
    histogram: bool,
    volume: bool,
) {
    use iced::advanced::{
        graphics::geometry::Renderer as _,
        renderer::{Headless, Renderer as _},
    };
    let backend = if wgpu { "wgpu" } else { "tiny-skia" };
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some(backend),
    ))
    .expect("requested Iced headless backend unavailable");
    std::fs::create_dir_all(directory).unwrap();
    let styles = if candlesticks && std::env::args().any(|a| a == "--capture-styles") {
        8
    } else {
        1
    };
    for style_index in 0..styles {
        for scenario in DisplayScenario::ALL {
            if style_index > 0 && scenario != DisplayScenario::Default {
                continue;
            }
            for ratio in [1., 1.25, 1.5, 2.] {
                let mut demo = Demo::new_chart(
                    DisplayConfig::for_scenario(scenario),
                    if volume {
                        SeriesKind::PriceVolume
                    } else if histogram {
                        SeriesKind::Histogram
                    } else if candlesticks {
                        SeriesKind::Candlesticks
                    } else {
                        SeriesKind::Line
                    },
                );
                demo.scale = ratio;
                if style_index > 0 {
                    demo.kind = if style_index & 1 == 0 {
                        LineType::Simple
                    } else {
                        LineType::WithSteps
                    };
                    demo.style = if style_index & 2 == 0 {
                        LineStyle::Solid
                    } else {
                        LineStyle::Dashed
                    };
                    demo.markers = style_index & 4 != 0;
                    demo.apply_style();
                }
                demo.apply_scenario(Instant::now());
                if std::env::args().any(|arg| arg == "--capture-interactions") && demo.registered {
                    let point = iced::Point::new(
                        demo.frame.plot.size.width * 0.5,
                        demo.frame.plot.size.height * 0.4,
                    );
                    let _ = demo.update(Message::Interaction(ChartInteraction {
                        position: Some(point),
                        navigation: Some(Navigation::Wheel {
                            x: f64::from(point.x),
                            zoom: 0.32,
                            scroll: 0.,
                        }),
                        capture: true,
                    }));
                    assert!(!demo.frame.plot.crosshair.is_empty());
                    assert_eq!(
                        demo.frame.plot.crosshair_markers.is_empty(),
                        (candlesticks || histogram) && !demo.left_registered
                    );
                }
                assert!(demo.error.is_none(), "{:?}", demo.error);
                let before = format!("{:?}", demo.frame);
                let offset = demo.model.time_scale().right_offset();
                let active = demo.controller.is_active();
                let size = Size::new(
                    (demo.size.width * ratio).round() as u32,
                    (demo.size.height * ratio).round() as u32,
                );
                let mut last = None;
                for _ in 0..2 {
                    renderer.reset(iced::Rectangle::with_size(demo.size));
                    let mut frame = Frame::new(&renderer, demo.size);
                    if let Some(multi) = &demo.multi_frame {
                        multi.draw(&mut frame);
                    } else {
                        demo.frame.draw(&mut frame);
                    }
                    renderer.draw_geometry(frame.into_geometry());
                    let rgba =
                        renderer.screenshot(size, ratio, iced::Color::from_rgb8(255, 0, 255));
                    assert_eq!(rgba.len(), size.width as usize * size.height as usize * 4);
                    if let Some(previous) = &last {
                        assert_eq!(previous, &rgba, "drawing must be repeatable");
                    }
                    last = Some(rgba);
                }
                assert_eq!(before, format!("{:?}", demo.frame));
                assert_eq!(offset, demo.model.time_scale().right_offset());
                assert_eq!(active, demo.controller.is_active());
                let name = if style_index == 0 {
                    format!("{}-{backend}-{ratio}", scenario.name())
                } else {
                    format!("style-{style_index}-{backend}-{ratio}")
                };
                let rgba = last.unwrap();
                if demo.has_series_geometry() {
                    let painted = rgba.as_chunks::<4>().0.iter().any(|p| {
                        if candlesticks {
                            (p[1] > 50 && f64::from(p[1]) > f64::from(p[0]) * 1.7)
                                || (p[0] > 50 && f64::from(p[0]) > f64::from(p[1]) * 1.4)
                        } else {
                            p[2] > 180 && p[1] > 80 && p[0] < 80
                        }
                    });
                    assert!(painted, "prepared series must remain visible in {name}");
                }
                // Magenta is the offscreen clear color, never a chart color. An
                // exposed interior pixel indicates a hole between region clips.
                for pixel in rgba.as_chunks::<4>().0 {
                    assert!(
                        !(pixel[0] > pixel[1].saturating_add(30)
                            && pixel[2] > pixel[1].saturating_add(30)),
                        "unpainted chart seam in {name}: {pixel:?}"
                    );
                }
                let mut ppm = format!("P6\n{} {}\n255\n", size.width, size.height).into_bytes();
                for pixel in rgba.as_chunks::<4>().0 {
                    ppm.extend_from_slice(&pixel[..3]);
                }
                std::fs::write(directory.join(format!("{name}.ppm")), ppm).unwrap();
                std::fs::write(directory.join(format!("{name}.json")), format!(
                "{{\"scenario\":\"{}\",\"backend\":\"{}\",\"font\":\"sans-serif\",\"fontSize\":12,\"logicalWidth\":{},\"logicalHeight\":{},\"physicalWidth\":{},\"physicalHeight\":{},\"scaleFactor\":{},\"plotWidth\":{},\"plotHeight\":{},\"leftWidth\":{},\"rightWidth\":{},\"timeHeight\":{},\"passes\":{}}}\n",
                scenario.name(), renderer.name(), demo.size.width, demo.size.height, size.width, size.height,
                ratio, demo.frame.layout.plot.size.width, demo.frame.layout.plot.size.height,
                demo.frame.layout.left_axis.size.width, demo.frame.layout.right_axis.size.width,
                demo.frame.layout.time_axis.size.height, demo.frame.passes)).unwrap();
            }
        }
    }
    println!(
        "{} {backend} captures and metadata written to {}",
        28 + (styles - 1) * 4,
        directory.display()
    );
}
