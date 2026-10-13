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
