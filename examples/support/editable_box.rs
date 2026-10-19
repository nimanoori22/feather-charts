// Shared example-owned box geometry uses the existing primitive edit router.
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
pub(super) struct Source(Rc<RefCell<Bounds>>);
pub(super) struct Scene {
    id: PrimitiveId,
    value: Rc<RefCell<Bounds>>,
    edit: Option<(PrimitiveEditId, Bounds, PrimitiveCoordinateTransform, bool)>,
}
impl Scene {
    pub fn new<C: RegionHost>(chart: &mut C, series: SeriesId, span: std::ops::Range<f64>) -> Self {
        let value = Rc::new(RefCell::new(Bounds {
            from: span.start,
            to: span.end,
            low: 4.,
            high: 4.5,
        }));
        let id = chart.add_box(series, Source(value.clone())).unwrap();
        Self {
            id,
            value,
            edit: None,
        }
    }
    pub fn edit<C: RegionHost>(
        &mut self,
        chart: &mut C,
        event: &PrimitiveEditEvent,
    ) -> Result<(), ChartError> {
        if event.hit.primitive != self.id {
            return Ok(());
        }
        match event.phase {
            PrimitiveEditPhase::Begin => {
                if let Some(transform) = chart.box_transform(self.id)? {
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
                    chart.box_update(self.id)?;
                } else {
                    chart.box_preview(event.id)?;
                }
            }
            PrimitiveEditPhase::Cancel(reason) => {
                if let Some((id, original, _, _)) = self.edit.take()
                    && id == event.id
                    && reason.restores_bounds()
                {
                    *self.value.borrow_mut() = original;
                    chart.box_update(self.id)?;
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
impl<B: ChartBehavior, M: Clone + 'static> DrawingPrimitive<B, M> for Source {
    fn prepare(&self, c: &PrimitiveContext<'_, B, M>) -> PreparedPrimitive {
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

pub(super) trait RegionHost {
    fn add_box(&mut self, series: SeriesId, source: Source) -> Result<PrimitiveId, ChartError>;
    fn box_transform(
        &mut self,
        id: PrimitiveId,
    ) -> Result<Option<PrimitiveCoordinateTransform>, ChartError>;
    fn box_update(&mut self, id: PrimitiveId) -> Result<(), ChartError>;
    fn box_preview(&mut self, id: PrimitiveEditId) -> Result<(), ChartError>;
}
macro_rules! region_methods {
    () => {
        fn add_box(&mut self, series: SeriesId, source: Source) -> Result<PrimitiveId, ChartError> {
            self.add_series_primitive(series, source)
        }
        fn box_transform(
            &mut self,
            id: PrimitiveId,
        ) -> Result<Option<PrimitiveCoordinateTransform>, ChartError> {
            Ok(self.primitive_context(id)?.coordinate_transform())
        }
        fn box_update(&mut self, id: PrimitiveId) -> Result<(), ChartError> {
            self.request_primitive_update(id)
        }
        fn box_preview(&mut self, id: PrimitiveEditId) -> Result<(), ChartError> {
            self.preview_primitive_update(id)
        }
    };
}
impl<B: ChartBehavior, M: Clone + 'static> RegionHost for Chart<B, M> {
    region_methods!();
}
impl<M: Clone + 'static> RegionHost for YieldCurveChart<M> {
    region_methods!();
}
