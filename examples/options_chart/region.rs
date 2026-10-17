include!("../support/editable_box.rs");
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn box_hit_distinguishes_moving_from_right_edge_resizing() {
        let d = Drawing(Rectangle::new(Point::new(10., 10.), Size::new(100., 50.)));
        assert_eq!(
            d.hit_test(Point::new(30., 30.)).unwrap().external_id,
            "move"
        );
        assert_eq!(
            d.hit_test(Point::new(110., 30.)).unwrap().external_id,
            "resize"
        );
        assert!(d.hit_test(Point::new(200., 30.)).is_none());
    }
}

#[cfg(test)]
mod edit_tests {
    use super::*;
    use iced::{
        mouse,
        widget::canvas::{Event, Program},
    };
    #[test]
    fn numeric_box_edit_router_moves_resizes_and_restores_on_escape() {
        for resize in [false, true] {
            let mut app = App::new(false);
            app.size = Size::new(800., 500.);
            app.prepare(Instant::now());
            let original = *app.region.value.borrow();
            let t = app
                .chart
                .primitive_context(app.region.id)
                .unwrap()
                .coordinate_transform()
                .unwrap();
            let x = t
                .logical_to_coordinate(if resize {
                    original.to
                } else {
                    (original.from + original.to) / 2.
                })
                .unwrap() as f32;
            let y = t
                .price_to_coordinate((original.low + original.high) / 2.)
                .unwrap() as f32;
            let plot = app.chart.frame().layout.plot;
            let message = app
                .chart
                .program(std::convert::identity)
                .update(
                    &mut Default::default(),
                    &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    Rectangle::new(Point::ORIGIN, app.size),
                    mouse::Cursor::Available(Point::new(
                        x + plot.origin.x as f32,
                        y + plot.origin.y as f32,
                    )),
                )
                .unwrap()
                .into_inner()
                .0
                .unwrap();
            let ChartMessage::PrimitiveEdit(mut event) = message.clone() else {
                panic!("expected box capture, got {message:?}; x={x}, y={y}, plot={plot:?}");
            };
            let _ = app.update(Message::Chart(message));
            event.phase = PrimitiveEditPhase::Update;
            event.position.x += 20.;
            event.position.y += 10.;
            let _ = app.update(Message::Chart(ChartMessage::PrimitiveEdit(event.clone())));
            let changed = *app.region.value.borrow();
            assert!(changed.to > original.to);
            if resize {
                assert_eq!(changed.from, original.from);
                assert_eq!(changed.low, original.low);
            } else {
                assert!(changed.from > original.from);
                assert_ne!(changed.low, original.low);
            }
            event.phase = PrimitiveEditPhase::Cancel(PrimitiveEditCancelReason::Escape);
            let _ = app.update(Message::Chart(ChartMessage::PrimitiveEdit(event)));
            assert_eq!(*app.region.value.borrow(), original);
        }
    }
}
