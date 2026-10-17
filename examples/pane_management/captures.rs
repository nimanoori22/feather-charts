use super::*;
use iced::advanced::{
    graphics::geometry::Renderer as _,
    renderer::{Headless, Renderer as _},
};
pub fn run(dir: &std::path::Path, wgpu: bool) {
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
    capture::<Chart>(&mut renderer, &dir.join("calendar"));
    capture::<OptionsChart>(&mut renderer, &dir.join("price"));
    capture::<YieldCurveChart>(&mut renderer, &dir.join("yield"));
}
fn capture<C: Host>(renderer: &mut iced::Renderer, dir: &std::path::Path) {
    std::fs::create_dir_all(dir).unwrap();
    for (w, h) in [(800., 500.), (300., 300.), (300., 90.)] {
        for ratio in [1., 1.25, 1.5, 2.] {
            for scene in [
                "initial",
                "move",
                "swap",
                "builtin-transfer",
                "custom-transfer",
                "remove",
            ] {
                let mut app = App::<C>::new(false);
                app.size = Size::new(w, h);
                app.ratio = ratio;
                let action = match scene {
                    "move" => Some(Message::Down),
                    "swap" => Some(Message::Swap),
                    "builtin-transfer" => Some(Message::LineNext),
                    "custom-transfer" => Some(Message::CustomToLine),
                    "remove" => Some(Message::Remove),
                    _ => None,
                };
                if let Some(action) = action {
                    app.action(action).unwrap();
                }
                let now = Instant::now();
                app.chart.prepare(app.size, ratio, now).unwrap();
                let retained = app.chart.core().multi_frame().unwrap().clone();
                let size = app.size;
                let mut draw =
                    |snapshot: &feather_charts::ui::multi_pane_frame::IcedMultiPaneChartFrame| {
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
                let pixels = draw(&retained);
                assert!(pixels == draw(&retained), "unstable snapshot");
                app.action(Message::Swap).unwrap();
                app.chart.prepare(size, ratio, now).unwrap();
                assert!(pixels == draw(&retained), "snapshot changed after swap");
                app.action(Message::LineNew).unwrap();
                app.chart.prepare(size, ratio, now).unwrap();
                assert!(pixels == draw(&retained), "snapshot changed after transfer");
                app.action(Message::Remove).unwrap();
                app.chart.prepare(size, ratio, now).unwrap();
                assert!(pixels == draw(&retained), "snapshot changed after removal");
                drop(app);
                assert!(pixels == draw(&retained), "snapshot changed after drop");
                std::fs::write(dir.join(format!("{scene}-{w}x{h}-{ratio}.rgba")), pixels).unwrap();
            }
        }
    }
}
