//! Box constraints belong to this example, not the chart's gesture router.
use super::*;
use feather_charts::api::{PrimitiveEditId, PrimitiveHitTester};

const HIT_RADIUS: f32 = 7.;
const MIN_EXTENT: f64 = 8.;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Handle {
    Move,
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
impl Handle {
    fn name(self) -> &'static str {
        match self {
            Self::Move => "move",
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::TopLeft => "top-left",
            Self::TopRight => "top-right",
            Self::BottomLeft => "bottom-left",
            Self::BottomRight => "bottom-right",
        }
    }
    fn parse(name: &str) -> Self {
        match name {
            "left" => Self::Left,
            "right" => Self::Right,
            "top" => Self::Top,
            "bottom" => Self::Bottom,
            "top-left" => Self::TopLeft,
            "top-right" => Self::TopRight,
            "bottom-left" => Self::BottomLeft,
            "bottom-right" => Self::BottomRight,
            _ => Self::Move,
        }
    }
    fn cursor(self) -> iced::mouse::Interaction {
        use iced::mouse::Interaction::*;
        match self {
            Self::Move => Grab,
            Self::Left | Self::Right => ResizingHorizontally,
            Self::Top | Self::Bottom => ResizingVertically,
            Self::TopLeft | Self::BottomRight => ResizingDiagonallyDown,
            Self::TopRight | Self::BottomLeft => ResizingDiagonallyUp,
        }
    }
}
fn handles(r: Rectangle) -> [(Handle, Point); 8] {
    let (left, right, top, bottom) = (r.x, r.x + r.width, r.y, r.y + r.height);
    let (x, y) = (left + r.width / 2., top + r.height / 2.);
    [
        (Handle::TopLeft, Point::new(left, top)),
        (Handle::TopRight, Point::new(right, top)),
        (Handle::BottomLeft, Point::new(left, bottom)),
        (Handle::BottomRight, Point::new(right, bottom)),
        (Handle::Left, Point::new(left, y)),
        (Handle::Right, Point::new(right, y)),
        (Handle::Top, Point::new(x, top)),
        (Handle::Bottom, Point::new(x, bottom)),
    ]
}
#[derive(Debug)]
pub(super) struct SelectionDrawing {
    pub rect: Rectangle,
}
impl PrimitiveHitTester for SelectionDrawing {
    fn hit_test(&self, p: Point) -> Option<PrimitiveHitCandidate> {
        let r = self.rect;
        let target = handles(r)
            .into_iter()
            .filter_map(|(h, center)| {
                let d = p.distance(center);
                (d <= HIT_RADIUS).then_some((h, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .or_else(|| {
                let edges = [
                    (
                        Handle::Left,
                        (p.x - r.x).abs(),
                        p.y >= r.y && p.y <= r.y + r.height,
                    ),
                    (
                        Handle::Right,
                        (p.x - r.x - r.width).abs(),
                        p.y >= r.y && p.y <= r.y + r.height,
                    ),
                    (
                        Handle::Top,
                        (p.y - r.y).abs(),
                        p.x >= r.x && p.x <= r.x + r.width,
                    ),
                    (
                        Handle::Bottom,
                        (p.y - r.y - r.height).abs(),
                        p.x >= r.x && p.x <= r.x + r.width,
                    ),
                ];
                edges
                    .into_iter()
                    .filter(|(_, d, eligible)| *eligible && *d <= HIT_RADIUS)
                    .map(|(h, d, _)| (h, d))
                    .min_by(|a, b| a.1.total_cmp(&b.1))
            })
            .or_else(|| r.contains(p).then_some((Handle::Move, 0.)))?;
        let mut hit = PrimitiveHitCandidate::new(target.0.name());
        hit.editable = true;
        hit.layer = PrimitiveLayer::Top;
        hit.priority = if target.0 == Handle::Move {
            HitTestPriority::Range
        } else {
            HitTestPriority::Point
        };
        hit.distance = f64::from(target.1);
        hit.cursor = Some(target.0.cursor());
        Some(hit)
    }
}
impl PrimitiveRenderer for SelectionDrawing {
    fn draw(&self, frame: &mut Frame) {
        let stroke = Stroke::default()
            .with_width(2.)
            .with_color(Color::from_rgb8(156, 39, 176));
        frame.stroke(
            &Path::rectangle(self.rect.position(), self.rect.size()),
            stroke,
        );
        for (_, p) in handles(self.rect) {
            let path = Path::rectangle(Point::new(p.x - 4., p.y - 4.), Size::new(8., 8.));
            frame.fill(&path, Color::WHITE);
            frame.stroke(&path, stroke);
        }
    }
}
pub(super) struct BoxEdit {
    pub id: PrimitiveEditId,
    pub original: Region,
    transform: PrimitiveCoordinateTransform,
    handle: Handle,
    start: Point,
    bounds: [f64; 4],
}
impl BoxEdit {
    pub fn new(
        event: &PrimitiveEditEvent,
        original: Region,
        transform: PrimitiveCoordinateTransform,
    ) -> Option<Self> {
        let x1 = transform.logical_to_coordinate(original.from)?;
        let x2 = transform.logical_to_coordinate(original.to)?;
        let y1 = transform.price_to_coordinate(original.low)?;
        let y2 = transform.price_to_coordinate(original.high)?;
        Some(Self {
            id: event.id,
            original,
            transform,
            handle: Handle::parse(&event.hit.external_id),
            start: event.start,
            bounds: [x1.min(x2), x1.max(x2), y1.min(y2), y1.max(y2)],
        })
    }
    pub fn preview(&self, position: Point) -> Option<Region> {
        if !position.x.is_finite() || !position.y.is_finite() {
            return None;
        }
        if position == self.start {
            return Some(self.original.clone());
        }
        let [mut left, mut right, mut top, mut bottom] = self.bounds;
        let dx = f64::from(position.x - self.start.x);
        let dy = f64::from(position.y - self.start.y);
        use Handle::*;
        if self.handle == Move {
            left += dx;
            right += dx;
            top += dy;
            bottom += dy;
        }
        if matches!(self.handle, Left | TopLeft | BottomLeft) {
            left = (left + dx).min(right - MIN_EXTENT);
        }
        if matches!(self.handle, Right | TopRight | BottomRight) {
            right = (right + dx).max(left + MIN_EXTENT);
        }
        if matches!(self.handle, Top | TopLeft | TopRight) {
            top = (top + dy).min(bottom - MIN_EXTENT);
        }
        if matches!(self.handle, Bottom | BottomLeft | BottomRight) {
            bottom = (bottom + dy).max(top + MIN_EXTENT);
        }
        let from = self.transform.coordinate_to_logical(left)?;
        let to = self.transform.coordinate_to_logical(right)?;
        let a = self.transform.coordinate_to_price(top)?;
        let b = self.transform.coordinate_to_price(bottom)?;
        let mut value = self.original.clone();
        value.from = from;
        value.to = to;
        value.low = a.min(b);
        value.high = a.max(b);
        (value.from < value.to && value.low < value.high).then_some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use feather_charts::model::{
        axis_snapshots::PriceAxisSide,
        price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
    };
    use iced::{
        mouse,
        widget::canvas::{Event, Program},
    };
    fn setup(
        mode: PriceScaleMode,
        inverted: bool,
    ) -> (Region, PrimitiveCoordinateTransform, PrimitiveEditEvent) {
        let mut app = App::new(SeriesType::Line);
        app.primitive_scene = Some(Scene::new(&mut app.chart, app.price));
        app.chart
            .apply_price_axis_options(
                0,
                PriceAxisSide::Right,
                PriceScaleOptionsPatch {
                    mode: Some(mode),
                    invert_scale: Some(inverted),
                    ..Default::default()
                },
            )
            .unwrap();
        app.size = Size::new(800., 500.);
        app.prepare();
        let id = app.primitive_scene.as_ref().unwrap().region.unwrap();
        let mut value = app.primitive_scene.as_ref().unwrap().value();
        value.selected = true;
        let transform = app
            .chart
            .primitive_context(id)
            .unwrap()
            .coordinate_transform()
            .unwrap();
        let p = Point::new(
            transform.logical_to_coordinate(30.).unwrap() as f32,
            transform.price_to_coordinate(114.).unwrap() as f32,
        );
        let plot = app.chart.frame().layout.plot;
        let message = app
            .chart
            .program(std::convert::identity)
            .update(
                &mut Default::default(),
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Rectangle::new(Point::ORIGIN, app.size),
                mouse::Cursor::Available(Point::new(
                    p.x + plot.origin.x as f32,
                    p.y + plot.origin.y as f32,
                )),
            )
            .unwrap()
            .into_inner()
            .0
            .unwrap();
        let ChartMessage::PrimitiveEdit(event) = message else {
            panic!("expected editable region")
        };
        (value, transform, event)
    }
    fn bounds(value: &Region, t: PrimitiveCoordinateTransform) -> [f64; 4] {
        let x1 = t.logical_to_coordinate(value.from).unwrap();
        let x2 = t.logical_to_coordinate(value.to).unwrap();
        let y1 = t.price_to_coordinate(value.low).unwrap();
        let y2 = t.price_to_coordinate(value.high).unwrap();
        [x1.min(x2), x1.max(x2), y1.min(y2), y1.max(y2)]
    }
    #[test]
    fn every_handle_changes_only_its_edges_in_all_price_modes_and_inversions() {
        use Handle::*;
        for mode in [
            PriceScaleMode::Normal,
            PriceScaleMode::Logarithmic,
            PriceScaleMode::Percentage,
            PriceScaleMode::IndexedTo100,
        ] {
            for inverted in [false, true] {
                let (value, t, mut event) = setup(mode, inverted);
                let original = bounds(&value, t);
                for h in [
                    Left,
                    Right,
                    Top,
                    Bottom,
                    TopLeft,
                    TopRight,
                    BottomLeft,
                    BottomRight,
                ] {
                    event.hit.external_id = h.name().into();
                    let edit = BoxEdit::new(&event, value.clone(), t).unwrap();
                    let p = Point::new(event.start.x + 17., event.start.y + 13.);
                    let result = edit.preview(p).unwrap();
                    let actual = bounds(&result, t);
                    let dx = f64::from(p.x - event.start.x);
                    let dy = f64::from(p.y - event.start.y);
                    let expected = [
                        original[0]
                            + if matches!(h, Left | TopLeft | BottomLeft) {
                                dx
                            } else {
                                0.
                            },
                        original[1]
                            + if matches!(h, Right | TopRight | BottomRight) {
                                dx
                            } else {
                                0.
                            },
                        original[2]
                            + if matches!(h, Top | TopLeft | TopRight) {
                                dy
                            } else {
                                0.
                            },
                        original[3]
                            + if matches!(h, Bottom | BottomLeft | BottomRight) {
                                dy
                            } else {
                                0.
                            },
                    ];
                    for (a, b) in actual.into_iter().zip(expected) {
                        assert!(
                            (a - b).abs() < 1e-7,
                            "{mode:?} {inverted} {h:?}: {actual:?} vs {expected:?}"
                        );
                    }
                    // Returning to the start must not accumulate previous previews.
                    assert_eq!(bounds(&edit.preview(event.start).unwrap(), t), original);
                }
            }
        }
    }
    #[test]
    fn crossing_edges_clamps_without_changing_handle_and_nonfinite_preview_is_rejected() {
        let (value, t, mut event) = setup(PriceScaleMode::Normal, false);
        for (h, delta) in [
            (Handle::Left, Point::new(10000., 0.)),
            (Handle::Right, Point::new(-10000., 0.)),
            (Handle::Top, Point::new(0., 10000.)),
            (Handle::Bottom, Point::new(0., -10000.)),
            (Handle::TopLeft, Point::new(10000., 10000.)),
        ] {
            event.hit.external_id = h.name().into();
            let edit = BoxEdit::new(&event, value.clone(), t).unwrap();
            let preview = edit
                .preview(Point::new(event.start.x + delta.x, event.start.y + delta.y))
                .unwrap();
            let [left, right, top, bottom] = bounds(&preview, t);
            assert!(right - left >= MIN_EXTENT - 1e-7 && bottom - top >= MIN_EXTENT - 1e-7);
            assert_eq!(edit.handle, h);
            assert!(edit.preview(Point::new(f32::NAN, f32::INFINITY)).is_none());
        }
    }
    #[test]
    fn selected_handles_and_edges_have_directional_cursors_and_interior_moves() {
        let rect = Rectangle::new(Point::new(10., 20.), Size::new(200., 100.));
        let drawing = SelectionDrawing { rect };
        for (h, p) in handles(rect) {
            let hit = PrimitiveHitTester::hit_test(&drawing, p).unwrap();
            assert_eq!(hit.external_id, h.name());
            assert_eq!(hit.cursor, Some(h.cursor()));
            assert!(hit.editable);
        }
        assert_eq!(
            PrimitiveHitTester::hit_test(&drawing, Point::new(10., 45.))
                .unwrap()
                .external_id,
            "left"
        );
        assert_eq!(
            PrimitiveHitTester::hit_test(&drawing, Point::new(100., 60.))
                .unwrap()
                .external_id,
            "move"
        );
        assert!(PrimitiveHitTester::hit_test(&drawing, Point::new(-10., 60.)).is_none());
    }
}
