//! Example-owned box geometry uses the existing primitive edit router.
use super::*;
use iced::{
    Color, Point, Rectangle,
    widget::canvas::{Frame, Path, Stroke},
};
use std::cell::RefCell;
#[derive(Clone, Copy, Debug, PartialEq)]
struct Bounds {
    from: f64,
    to: f64,
    low: f64,
    high: f64,
}
#[derive(Debug)]
struct Source(Rc<RefCell<Bounds>>);
pub(super) struct Scene {
    id: PrimitiveId,
    value: Rc<RefCell<Bounds>>,
    edit: Option<(PrimitiveEditId, Bounds, PrimitiveCoordinateTransform, bool)>,
}
impl Scene {
    pub fn new(chart: &mut OptionsChart, series: SeriesId) -> Self {
        let value = Rc::new(RefCell::new(Bounds {
            from: 3.,
            to: 7.,
            low: 4.,
            high: 4.5,
        }));
        let id = chart
            .add_series_primitive(series, Source(value.clone()))
            .unwrap();
        Self {
            id,
            value,
            edit: None,
        }
    }
    pub fn edit(
        &mut self,
        chart: &mut OptionsChart,
        event: &PrimitiveEditEvent,
    ) -> Result<(), ChartError> {
        if event.hit.primitive != self.id {
            return Ok(());
        }
        match event.phase {
            PrimitiveEditPhase::Begin => {
                if let Some(transform) = chart.primitive_context(self.id)?.coordinate_transform() {
                    self.edit = Some((
                        event.id,
                        *self.value.borrow(),
                        transform,
                        event.hit.external_id == "resize",
                    ));
                }
            }
            PrimitiveEditPhase::Update | PrimitiveEditPhase::Commit => {
                if let Some((id, original, t, resize)) = self.edit
                    && id == event.id
                {
                    let dx = f64::from(event.position.x - event.start.x);
                    let dy = f64::from(event.position.y - event.start.y);
                    if let Some([x1, y1, x2, y2]) = pixels(original, t) {
                        let left = if resize { x1 } else { x1 + dx };
                        let right = (x2 + dx).max(left + 8.);
                        if let Some(((from, to), (low, high))) = t
                            .coordinate_to_logical(left)
                            .zip(t.coordinate_to_logical(right))
                            .zip(
                                t.coordinate_to_price(if resize { y1 } else { y1 + dy })
                                    .zip(t.coordinate_to_price(if resize { y2 } else { y2 + dy })),
                            )
                        {
                            *self.value.borrow_mut() = Bounds {
                                from,
                                to,
                                low: low.min(high),
                                high: low.max(high),
                            };
                        }
                    }
                }
                if event.phase == PrimitiveEditPhase::Commit {
                    self.edit = None;
                    chart.request_primitive_update(self.id)?;
                } else {
                    chart.preview_primitive_update(event.id)?;
                }
            }
            PrimitiveEditPhase::Cancel(reason) => {
                if let Some((id, original, _, _)) = self.edit.take()
                    && id == event.id
                    && reason.restores_bounds()
                {
                    *self.value.borrow_mut() = original;
                    chart.request_primitive_update(self.id)?;
                }
            }
        }
        Ok(())
    }
}
fn pixels(b: Bounds, t: PrimitiveCoordinateTransform) -> Option<[f64; 4]> {
    Some([
        t.logical_to_coordinate(b.from)?,
        t.price_to_coordinate(b.low)?,
        t.logical_to_coordinate(b.to)?,
        t.price_to_coordinate(b.high)?,
    ])
}
impl DrawingPrimitive<HorzScaleBehaviorPrice> for Source {
    fn prepare(&self, c: &PrimitiveContext<'_, HorzScaleBehaviorPrice>) -> PreparedPrimitive {
        let Some([x1, y1, x2, y2]) = c
            .coordinate_transform()
            .and_then(|t| pixels(*self.0.borrow(), t))
        else {
            return Default::default();
        };
        let rect = Rectangle::new(
            Point::new(x1.min(x2) as f32, y1.min(y2) as f32),
            Size::new((x2 - x1).abs() as f32, (y2 - y1).abs() as f32),
        );
        PreparedPrimitive {
            views: vec![PrimitiveView::new(PrimitiveLayer::Top, Drawing(rect))],
            ..Default::default()
        }
    }
}
#[derive(Debug)]
struct Drawing(Rectangle);
impl PrimitiveRenderer for Drawing {
    fn draw(&self, f: &mut Frame) {
        let p = Path::rectangle(self.0.position(), self.0.size());
        f.fill(&p, Color::from_rgba(0.6, 0.15, 0.7, 0.08));
        f.stroke(
            &p,
            Stroke::default()
                .with_color(Color::from_rgb(0.6, 0.15, 0.7))
                .with_width(1.),
        );
    }
    fn hit_test(&self, p: Point) -> Option<PrimitiveHitCandidate> {
        let resize = (p.x - self.0.x - self.0.width).abs() <= 7.
            && p.y >= self.0.y
            && p.y <= self.0.y + self.0.height;
        if !resize && !self.0.contains(p) {
            return None;
        }
        let mut h = PrimitiveHitCandidate::new(if resize { "resize" } else { "move" });
        h.layer = PrimitiveLayer::Top;
        h.editable = true;
        h.cursor = Some(if resize {
            iced::mouse::Interaction::ResizingHorizontally
        } else {
            iced::mouse::Interaction::Grab
        });
        Some(h)
    }
}
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
