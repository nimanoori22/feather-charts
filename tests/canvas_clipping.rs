//! Raster-level regression for pane clips through the actual software backend.
use iced::{
    Color, Point, Rectangle, Size,
    advanced::{
        graphics::geometry::Renderer as _,
        renderer::{Headless, Renderer as _},
    },
    widget::canvas::{self, Frame, Path, Stroke},
};

fn render(scale: f32, draw: impl Fn(&mut Frame)) -> Vec<u8> {
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some("tiny-skia"),
    ))
    .unwrap();
    let size = Size::new(100., 100.);
    renderer.reset(Rectangle::with_size(size));
    // Exercise retained geometry as well as the backend's clip merge.
    let cache = canvas::Cache::new();
    let geometry = cache.draw(&renderer, size, draw);
    renderer.draw_geometry(geometry);
    renderer.screenshot(
        Size::new((100. * scale) as u32, (100. * scale) as u32),
        scale,
        Color::WHITE,
    )
}
fn assert_clipped(pixels: &[u8], scale: f32, clip: Rectangle) {
    let width = (100. * scale) as usize;
    let mut colored = 0;
    let mut escaped = 0;
    for (i, p) in pixels.as_chunks::<4>().0.iter().enumerate() {
        if p[0] < 230 || p[1] < 230 || p[2] < 230 {
            colored += 1;
            let x = (i % width) as f32 + 0.5;
            let y = (i / width) as f32 + 0.5;
            // Edge coverage may touch one pixel at fractional scale factors.
            if x < clip.x * scale - 1.
                || y < clip.y * scale - 1.
                || x > (clip.x + clip.width) * scale + 1.
                || y > (clip.y + clip.height) * scale + 1.
            {
                escaped += 1;
            }
        }
    }
    assert!(colored > 0, "test must actually draw");
    assert_eq!(escaped, 0, "drawing escaped its pane clip");
}
#[test]
fn fills_and_wide_strokes_obey_offset_fractional_pane_clips() {
    let clip = Rectangle::new(Point::new(30.25, 20.5), Size::new(30.5, 40.25));
    for scale in [1., 1.25, 2.] {
        for stroke in [false, true] {
            let pixels = render(scale, |f| {
                f.with_clip(clip, |f| {
                    f.translate(iced::Vector::new(clip.x, clip.y));
                    if stroke {
                        f.stroke(
                            &Path::line(Point::new(10., -20.), Point::new(10., 80.)),
                            Stroke::default().with_width(12.).with_color(Color::BLACK),
                        );
                    } else {
                        f.fill_rectangle(
                            Point::new(-20., -20.),
                            Size::new(80., 100.),
                            Color::BLACK,
                        );
                    }
                })
            });
            assert_clipped(&pixels, scale, clip);
        }
    }
}
#[test]
fn nested_clips_intersect_instead_of_replacing_the_parent() {
    let parent = Rectangle::new(Point::new(30., 30.), Size::new(30., 30.));
    let child = Rectangle::new(Point::new(10., 10.), Size::new(80., 80.));
    let pixels = render(1., |f| {
        f.with_clip(parent, |f| {
            f.with_clip(child, |f| {
                f.fill_rectangle(Point::ORIGIN, Size::new(100., 100.), Color::BLACK)
            })
        })
    });
    assert_clipped(&pixels, 1., parent);
}
#[test]
fn disjoint_and_empty_nested_clips_draw_nothing() {
    for child in [
        Rectangle::new(Point::new(70., 70.), Size::new(10., 10.)),
        Rectangle::new(Point::new(25., 25.), Size::ZERO),
    ] {
        let pixels = render(1., |f| {
            f.with_clip(
                Rectangle::new(Point::new(20., 20.), Size::new(20., 20.)),
                |f| {
                    f.with_clip(child, |f| {
                        f.fill_rectangle(Point::ORIGIN, Size::new(100., 100.), Color::BLACK)
                    })
                },
            )
        });
        assert!(
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| *p == [255, 255, 255, 255])
        );
    }
}
#[test]
fn cached_text_and_images_obey_nested_pane_clips() {
    let clip = Rectangle::new(Point::new(30., 20.), Size::new(20., 20.));
    for image in [false, true] {
        let pixels = render(1., |f| {
            f.with_clip(clip, |f| {
                f.with_clip(Rectangle::with_size(Size::new(100., 100.)), |f| {
                    if image {
                        f.draw_image(
                            Rectangle::with_size(Size::new(100., 100.)),
                            &iced::widget::image::Handle::from_rgba(1, 1, vec![0, 0, 0, 255]),
                        );
                    } else {
                        f.fill_text(canvas::Text {
                            content: "MMMM".into(),
                            position: Point::new(5., 5.),
                            size: iced::Pixels(50.),
                            color: Color::BLACK,
                            ..Default::default()
                        });
                    }
                })
            })
        });
        assert_clipped(&pixels, 1., clip);
    }
}
#[test]
fn sibling_clips_keep_insertion_order_and_do_not_leak_masks() {
    let pixels = render(1., |f| {
        for (x, color) in [(10., Color::BLACK), (60., Color::from_rgb(1., 0., 0.))] {
            f.with_clip(
                Rectangle::new(Point::new(x, 20.), Size::new(20., 20.)),
                |f| f.fill_rectangle(Point::ORIGIN, Size::new(100., 100.), color),
            );
        }
        f.fill_rectangle(
            Point::new(40., 20.),
            Size::new(10., 20.),
            Color::from_rgb(0., 1., 0.),
        );
    });
    let pixel = |x: usize, y: usize| &pixels[(y * 100 + x) * 4..(y * 100 + x) * 4 + 4];
    assert_eq!(pixel(15, 25), [0, 0, 0, 255]);
    assert_eq!(pixel(65, 25), [255, 0, 0, 255]);
    assert_eq!(pixel(45, 25), [0, 255, 0, 255]);
    assert_eq!(pixel(90, 25), [255, 255, 255, 255]);
}
