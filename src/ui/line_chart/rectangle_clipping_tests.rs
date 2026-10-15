use super::CanvasRectangle;
use iced::{Color, Point, Rectangle, Size};

fn rectangle(x: f32, y: f32, width: f32, height: f32) -> CanvasRectangle {
    CanvasRectangle {
        origin: Point::new(x, y),
        size: Size::new(width, height),
        color: Color::from_rgba(0.2, 0.4, 0.6, 0.35),
    }
}

fn bounds() -> Rectangle {
    Rectangle::new(Point::new(10.25, 20.5), Size::new(100.5, 80.25))
}

#[test]
fn clips_each_boundary_at_fractional_nonzero_plot_origin_without_mutating_input() {
    for (name, input, origin, size) in [
        (
            "left",
            rectangle(5.25, 30.5, 20., 10.),
            Point::new(10.25, 30.5),
            Size::new(15., 10.),
        ),
        (
            "right",
            rectangle(100.25, 30.5, 20., 10.),
            Point::new(100.25, 30.5),
            Size::new(10.5, 10.),
        ),
        (
            "top",
            rectangle(30.25, 15.5, 10., 20.),
            Point::new(30.25, 20.5),
            Size::new(10., 15.),
        ),
        (
            "bottom",
            rectangle(30.25, 90.5, 10., 20.),
            Point::new(30.25, 90.5),
            Size::new(10., 10.25),
        ),
        (
            "top-left",
            rectangle(5.25, 15.5, 20., 20.),
            Point::new(10.25, 20.5),
            Size::new(15., 15.),
        ),
        (
            "bottom-right",
            rectangle(100.25, 90.5, 20., 20.),
            Point::new(100.25, 90.5),
            Size::new(10.5, 10.25),
        ),
    ] {
        let original = (input.origin, input.size, input.color);
        let clipped = input.clipped_to(bounds()).expect(name);
        assert_eq!(clipped.origin, origin, "{name}");
        assert_eq!(clipped.size, size, "{name}");
        assert_eq!(clipped.color, input.color, "{name}: preserve RGBA");
        assert_eq!(
            (input.origin, input.size, input.color),
            original,
            "{name}: input remains reusable"
        );
    }
}

#[test]
fn visible_rectangle_and_alpha_remain_unchanged() {
    let input = rectangle(25.25, 30.75, 8.5, 5.25);
    let clipped = input.clipped_to(bounds()).unwrap();
    assert_eq!(clipped.origin, input.origin);
    assert_eq!(clipped.size, input.size);
    assert_eq!(clipped.color, input.color);
    assert_eq!(clipped.color.a, 0.35);
}

#[test]
fn fully_outside_or_merely_touching_rectangles_have_no_visible_area() {
    for input in [
        rectangle(-100., 30., 10., 10.),
        rectangle(200., 30., 10., 10.),
        rectangle(30., -100., 10., 10.),
        rectangle(30., 200., 10., 10.),
        rectangle(-9.75, 30., 20., 10.),
        rectangle(110.75, 30., 10., 10.),
        rectangle(30., 10.5, 10., 10.),
        rectangle(30., 100.75, 10., 10.),
    ] {
        assert!(input.clipped_to(bounds()).is_none(), "{input:?}");
    }
}

#[test]
fn empty_rectangles_or_empty_clip_bounds_produce_no_drawing() {
    for input in [rectangle(30., 30., 0., 10.), rectangle(30., 30., 10., 0.)] {
        assert!(input.clipped_to(bounds()).is_none());
    }
    let input = rectangle(30., 30., 10., 10.);
    for clip in [
        Rectangle::new(Point::new(30., 30.), Size::new(0., 10.)),
        Rectangle::new(Point::new(30., 30.), Size::new(10., 0.)),
    ] {
        assert!(input.clipped_to(clip).is_none());
    }
}

#[test]
fn enormous_offscreen_baseline_is_reduced_to_visible_plot_area() {
    let input = rectangle(30.25, 30., 5., 1_000_000.);
    let clipped = input.clipped_to(bounds()).unwrap();
    assert_eq!(clipped.origin, Point::new(30.25, 30.));
    assert_eq!(clipped.size, Size::new(5., 70.75));
    assert_eq!(clipped.color, input.color);
    let covering = rectangle(-1_000., -1_000., 2_000., 2_000.)
        .clipped_to(bounds())
        .unwrap();
    assert_eq!(covering.origin, bounds().position());
    assert_eq!(covering.size, bounds().size());
}
