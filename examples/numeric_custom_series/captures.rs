use super::*;
use iced::advanced::{
    graphics::geometry::Renderer as _,
    renderer::{Headless, Renderer as _},
};
pub fn run(directory: &std::path::Path, wgpu: bool) {
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some(if wgpu { "wgpu" } else { "tiny-skia" }),
    ))
    .unwrap();
    assert!(
        matches!(
            (&renderer, wgpu),
            (iced::Renderer::Primary(_), true) | (iced::Renderer::Secondary(_), false)
        ),
        "backend fallback"
    );
    capture::<OptionsChart>(&mut renderer, &directory.join("price"));
    capture::<YieldCurveChart>(&mut renderer, &directory.join("yield"));
}
fn capture<C: Host>(renderer: &mut iced::Renderer, dir: &std::path::Path) {
    std::fs::create_dir_all(dir).unwrap();
    for (w, h) in [(800., 500.), (300., 300.), (300., 90.)] {
        for ratio in [1., 1.25, 1.5, 2.] {
            for scene in ["sparse", "mixed", "updates", "dense", "empty"] {
                let mut app = App::<C>::new(false, scene == "dense");
                app.size = Size::new(w, h);
                app.ratio = ratio;
                if scene == "updates" {
                    app.action(&Message::History).unwrap();
                    app.action(&Message::Whitespace).unwrap();
                    app.action(&Message::Style).unwrap();
                }
                if scene == "empty" {
                    app.action(&Message::Empty).unwrap();
                    app.prepare(Instant::now());
                    app.action(&Message::Reset).unwrap();
                    app.prepare(Instant::now());
                    app.action(&Message::Empty).unwrap();
                }
                if scene == "mixed" {
                    app.action(&Message::Labels).unwrap();
                }
                let now = Instant::now();
                app.chart.prepare(app.size, ratio, now).unwrap();
                assert_eq!(!has_custom(app.chart.core().frame()), scene == "empty");
                let retained = app.chart.core().frame().clone();
                let size = app.size;
                let mut draw = |snapshot: &feather_charts::ui::chart_frame::IcedChartFrame| {
                    renderer.reset(iced::Rectangle::with_size(size));
                    let mut frame = iced::widget::canvas::Frame::new(renderer, size);
                    snapshot.draw(&mut frame);
                    renderer.draw_geometry(frame.into_geometry());
                    renderer.screenshot(
                        Size::new((w * ratio) as u32, (h * ratio) as u32),
                        ratio,
                        iced::Color::WHITE,
                    )
                };
                let rgba = draw(&retained);
                assert!(rgba == draw(&retained), "frozen draw changed");
                if scene != "empty" {
                    app.action(&Message::Latest).unwrap();
                    app.chart.prepare(app.size, ratio, now).unwrap();
                    assert!(
                        rgba == draw(&retained),
                        "old frame changed after data update"
                    );
                }
                if let Some(handle) = app.custom.take() {
                    app.chart.remove(handle.id()).unwrap();
                }
                app.chart.prepare(app.size, ratio, now).unwrap();
                assert!(rgba == draw(&retained), "old frame changed after removal");
                if scene != "empty" && retained.plot.size.height > 30. {
                    assert!(
                        rgba != draw(app.chart.core().frame()),
                        "custom interval pixels missing"
                    );
                }
                drop(app.chart);
                assert!(
                    rgba == draw(&retained),
                    "old frame changed after chart drop"
                );
                std::fs::write(dir.join(format!("{scene}-{w}x{h}-{ratio}.rgba")), rgba).unwrap();
            }
        }
    }
}
