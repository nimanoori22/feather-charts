//! Native regression for raster/vector insertion order, including cached geometry.
use iced::{
    Color, Point, Rectangle, Size,
    advanced::{
        graphics::geometry::Renderer as _,
        renderer::{Headless, Renderer as _},
    },
    widget::canvas::{Cache, Frame},
};
fn main() {
    let backend = if std::env::args().any(|v| v == "--wgpu") {
        "wgpu"
    } else {
        "tiny-skia"
    };
    let mut r = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some(backend),
    ))
    .unwrap();
    let size = Size::new(32., 32.);
    let cache = Cache::new();
    for cached in [false, true] {
        for present in [true, false, true] {
            cache.clear();
            r.reset(Rectangle::with_size(size));
            let draw =
                |f: &mut Frame| {
                    f.fill_rectangle(Point::ORIGIN, size, Color::WHITE);
                    if present {
                        f.draw_image(
                            Rectangle::new(Point::new(4., 4.), Size::new(24., 24.)),
                            iced::advanced::image::Image::new(
                                iced::widget::image::Handle::from_rgba(1, 1, vec![255, 0, 0, 255]),
                            ),
                        );
                    }
                    f.fill_rectangle(
                        Point::new(8., 8.),
                        Size::new(16., 16.),
                        Color::from_rgb8(0, 255, 0),
                    );
                    if present {
                        f.draw_image(
                            Rectangle::new(Point::new(12., 12.), Size::new(8., 8.)),
                            iced::advanced::image::Image::new(
                                iced::widget::image::Handle::from_rgba(1, 1, vec![0, 0, 255, 255]),
                            ),
                        );
                    }
                };
            let geometry = if cached {
                cache.draw(&r, size, draw)
            } else {
                let mut f = Frame::new(&r, size);
                draw(&mut f);
                f.into_geometry()
            };
            r.draw_geometry(geometry);
            let pixels = r.screenshot(Size::new(32, 32), 1., Color::BLACK);
            assert_eq!(
                &pixels[(10 * 32 + 10) * 4..(10 * 32 + 10) * 4 + 4],
                &[0, 255, 0, 255],
                "{backend} cached={cached}: image must precede later vector fill"
            );
            assert_eq!(
                &pixels[(16 * 32 + 16) * 4..(16 * 32 + 16) * 4 + 4],
                if present {
                    &[0, 0, 255, 255]
                } else {
                    &[0, 255, 0, 255]
                },
                "{backend}: second image must follow intervening vector"
            );
            let corner = &pixels[(6 * 32 + 6) * 4..(6 * 32 + 6) * 4 + 4];
            assert_eq!(
                corner,
                if present {
                    &[255, 0, 0, 255]
                } else {
                    &[255, 255, 255, 255]
                },
                "{backend}: cached image removal/restoration"
            );
        }
    }
    // Intrinsic alpha and opacity multiply before source-over blending on white.
    for opacity in [0_f32, 0.5, 1.] {
        r.reset(Rectangle::with_size(size));
        let mut f = Frame::new(&r, size);
        f.fill_rectangle(Point::ORIGIN, size, Color::WHITE);
        f.with_clip(
            Rectangle::new(Point::new(4., 4.), Size::new(24., 24.)),
            |f| {
                f.draw_image(
                    Rectangle::with_size(size),
                    iced::advanced::image::Image::new(iced::widget::image::Handle::from_rgba(
                        1,
                        1,
                        vec![0, 0, 0, 128],
                    ))
                    .opacity(opacity),
                );
            },
        );
        r.draw_geometry(f.into_geometry());
        let pixels = r.screenshot(Size::new(32, 32), 1., Color::BLACK);
        assert_eq!(
            &pixels[..4],
            &[255, 255, 255, 255],
            "image leaked outside pane clip"
        );
        let expected = (255. - 128. * opacity).round() as u8;
        let pixel = &pixels[(16 * 32 + 16) * 4..(16 * 32 + 16) * 4 + 4];
        for channel in &pixel[..3] {
            assert!(
                u8::abs_diff(*channel, expected) <= 1,
                "{backend}: intrinsic alpha times opacity: {pixel:?}, expected {expected}"
            );
        }
    }
    r.reset(Rectangle::with_size(size));
    let mut f = Frame::new(&r, size);
    f.fill_text(iced::widget::canvas::Text {
        content: "M".into(),
        position: Point::new(4., 4.),
        size: 24_f32.into(),
        color: Color::BLACK,
        align_y: iced::alignment::Vertical::Top,
        ..Default::default()
    });
    f.draw_image(
        Rectangle::with_size(size),
        iced::advanced::image::Image::new(iced::widget::image::Handle::from_rgba(
            1,
            1,
            vec![255, 0, 0, 255],
        )),
    );
    r.draw_geometry(f.into_geometry());
    let pixels = r.screenshot(Size::new(32, 32), 1., Color::BLACK);
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[0] < 64 && p[1] == 0 && p[2] == 0),
        "canvas text must retain its overlay semantics"
    );
    println!(
        "{backend}: raster/vector insertion order, placement, alpha, clip and text overlay pass"
    );
}
