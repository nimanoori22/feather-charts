use super::*;
use iced::advanced::{layout::Limits, renderer::Headless, widget::Tree};

#[test]
fn hover_status_keeps_chart_bounds_stable_in_narrow_windows() {
    let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(16.),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");
    for width in [640., 480., 360.] {
        let mut app = App::new(SeriesType::Candlestick);
        app.chart
            .add_series_markers(
                app.price,
                sample_markers(false),
                SeriesMarkersOptions::default(),
            )
            .unwrap();
        let window = Size::new(width, 480.);
        let mut tree = Tree::new(app.view().as_widget());
        let mut expected = None;
        for hovered_marker in [false, true, false, true, false] {
            let bounds = {
                let mut view = app.view();
                tree.diff(view.as_widget());
                let layout = view.as_widget_mut().layout(
                    &mut tree,
                    &renderer,
                    &Limits::new(Size::ZERO, window),
                );
                layout.children().last().unwrap().bounds()
            };
            if let Some(previous) = expected {
                assert_eq!(
                    bounds, previous,
                    "hover resized the chart at window width {width}"
                );
            } else {
                expected = Some(bounds);
            }
            let _ = app.update(Message::Chart(ChartMessage::Bounds(bounds.size())));
            let position = if hovered_marker {
                let m = &app.chart.frame().plot.annotations[0].items[0];
                iced::Point::new(m.center.x as f32, m.center.y as f32)
            } else {
                iced::Point::new(1., 1.)
            };
            let _ = app.update(Message::Chart(ChartMessage::Interaction(
                feather_charts::ui::chart_input::ChartInteraction {
                    position: Some(position),
                    navigation: None,
                    capture: true,
                },
            )));
            assert_eq!(app.chart.marker_hit(0, position).is_some(), hovered_marker);
            assert_eq!(app.status.contains("marker Some("), hovered_marker);
            assert_eq!(app.status.contains("target Some("), hovered_marker);
        }
    }
}

#[test]
fn primitive_controls_and_hover_keep_narrow_chart_bounds_stable() {
    let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(16.),
        Some("tiny-skia"),
    ))
    .unwrap();
    for width in [640., 480., 360.] {
        let mut app = App::new(SeriesType::Candlestick);
        app.primitive_scene = Some(primitive_demo::Scene::new(&mut app.chart, app.price));
        let mut tree = Tree::new(app.view().as_widget());
        let mut expected = None;
        for action in [
            Message::PrimitiveLayer,
            Message::MovePrimitive,
            Message::TogglePrimitive,
            Message::TogglePrimitive,
        ] {
            let _ = app.update(action);
            for status in [
                "No selection",
                "primitive Some(PrimitiveHit { owner: Series, external_id: region boundary, cursor: Pointer })",
            ] {
                app.status = status.into();
                let mut view = app.view();
                tree.diff(view.as_widget());
                let layout = view.as_widget_mut().layout(
                    &mut tree,
                    &renderer,
                    &Limits::new(Size::ZERO, Size::new(width, 480.)),
                );
                let bounds = layout.children().last().unwrap().bounds();
                if let Some(expected) = expected {
                    assert_eq!(
                        bounds, expected,
                        "primitive status changed plot bounds at {width}"
                    );
                } else {
                    expected = Some(bounds);
                }
            }
        }
    }
}
