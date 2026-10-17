use super::*;
use iced::advanced::{
    graphics::geometry::Renderer as _,
    renderer::{Headless, Renderer as _},
};
pub(super) fn run(directory: &std::path::Path, wgpu: bool) {
    let backend = if wgpu { "wgpu" } else { "tiny-skia" };
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some(backend),
    ))
    .unwrap();
    assert!(
        matches!(
            (&renderer, wgpu),
            (iced::Renderer::Primary(_), true) | (iced::Renderer::Secondary(_), false)
        ),
        "backend fallback"
    );
    std::fs::create_dir_all(directory).unwrap();
    for (width, height) in [(800., 500.), (300., 300.), (300., 90.)] {
        for ratio in [1., 1.25, 1.5, 2.] {
            for scene in ["line", "two-curves", "domain", "markers", "empty"] {
                let mut app = App::new(false);
                app.size = Size::new(width, height);
                app.ratio = ratio;
                let now = Instant::now();
                match scene {
                    "line" => {
                        app.chart.remove_series(app.area).unwrap();
                    }
                    "domain" => {
                        app.action(&Message::Domain, now).unwrap();
                        app.action(&Message::Formatter, now).unwrap();
                    }
                    "markers" => {
                        app.action(&Message::Manual, now).unwrap();
                    }
                    "empty" => {
                        app.prepare(now);
                        app.action(&Message::Empty, now).unwrap();
                        app.prepare(now);
                        app.action(&Message::Reset, now).unwrap();
                        app.prepare(now);
                        assert!(
                            !app.chart.frame().plot.lines.is_empty(),
                            "empty recovery failed"
                        );
                        app.action(&Message::Empty, now).unwrap();
                    }
                    _ => {}
                }
                app.prepare(now);
                let retained = app.chart.frame().clone();
                assert_eq!(
                    retained.plot.lines.is_empty(),
                    scene == "empty",
                    "curve visibility: {scene}"
                );
                let mut draw = |snapshot: &feather_charts::ui::chart_frame::IcedChartFrame| {
                    renderer.reset(iced::Rectangle::with_size(app.size));
                    let mut frame = iced::widget::canvas::Frame::new(&renderer, app.size);
                    snapshot.draw(&mut frame);
                    renderer.draw_geometry(frame.into_geometry());
                    renderer.screenshot(
                        Size::new((width * ratio) as u32, (height * ratio) as u32),
                        ratio,
                        iced::Color::BLACK,
                    )
                };
                let rgba = draw(&retained);
                assert!(rgba == draw(&retained), "frozen frame pixels changed");
                if scene == "markers" {
                    app.chart.clear_up_down_markers(app.markers).unwrap();
                    app.chart.prepare(app.size, ratio, now).unwrap();
                    if retained.plot.size.height > 30. {
                        assert!(rgba != draw(app.chart.frame()), "numeric markers invisible");
                    }
                }
                app.chart.remove_series(app.line).unwrap();
                app.chart.prepare(app.size, ratio, now).unwrap();
                assert!(
                    rgba == draw(&retained),
                    "old frame changed after owner removal"
                );
                assert!(rgba.iter().any(|v| *v != 0), "empty screenshot");
                std::fs::write(
                    directory.join(format!("{scene}-{width}x{height}-{ratio}.rgba")),
                    rgba,
                )
                .unwrap();
            }
        }
    }
}
