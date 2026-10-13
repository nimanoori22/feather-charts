use feather_charts::{
    api::{
        Chart, DrawingPrimitive, PreparedPrimitive, PrimitiveContext, PrimitiveHitCandidate,
        PrimitiveLayer, PrimitiveRenderer, PrimitiveView,
    },
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        data_layer::SeriesId,
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
    },
};
use iced::{Point, Size};
use std::{cell::Cell, rc::Rc, time::Instant};

#[derive(Debug)]
struct Editable {
    detached: Rc<Cell<usize>>,
}
impl DrawingPrimitive for Editable {
    fn prepare(&self, _: &PrimitiveContext<'_>) -> PreparedPrimitive {
        PreparedPrimitive {
            views: vec![PrimitiveView::new(PrimitiveLayer::Top, EditHit)],
            ..Default::default()
        }
    }
    fn detached(&self) {
        self.detached.set(self.detached.get() + 1);
    }
}
#[derive(Debug)]
struct EditHit;
impl PrimitiveRenderer for EditHit {
    fn draw(&self, _: &mut iced::widget::canvas::Frame) {}
    fn hit_test(&self, p: Point) -> Option<PrimitiveHitCandidate> {
        (p.x >= 20. && p.x <= 200. && p.y >= 20. && p.y <= 200.).then(|| {
            let mut hit = PrimitiveHitCandidate::new("move");
            hit.layer = PrimitiveLayer::Top;
            hit.editable = true;
            hit.cursor = Some(iced::mouse::Interaction::Grab);
            hit
        })
    }
}
fn chart() -> (Chart, SeriesId) {
    let mut chart = Chart::default();
    let series = chart
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(
            series,
            (0..100)
                .map(|i| {
                    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                        time: Time::from(UtcTimestamp::new(1000. + i as f64 * 10.)),
                        value: 100. + (i % 10) as f64,
                        color: None,
                        custom_values: None,
                    }))
                })
                .collect(),
        )
        .unwrap();
    chart.fit_content();
    prepare(&mut chart);
    (chart, series)
}
fn prepare(chart: &mut Chart) {
    chart
        .prepare(Size::new(640., 480.), 1.25, Instant::now())
        .unwrap();
}

#[test]
fn continuous_coordinates_round_trip_fractional_and_offscreen_bounds() {
    let (mut chart, series) = chart();
    let detached = Rc::new(Cell::new(0));
    let id = chart
        .add_series_primitive(series, Editable { detached })
        .unwrap();
    let c = chart.primitive_context(id).unwrap();
    for logical in [-20.125, 0., 10.25, 99.75, 1000.] {
        let x = c.continuous_logical_to_coordinate(logical).unwrap();
        assert!((c.coordinate_to_logical(x).unwrap() - logical).abs() < 1e-9);
    }
    for price in [88., 100., 110., 140.] {
        let y = c.price_to_coordinate(price).unwrap();
        assert!((c.coordinate_to_price(y).unwrap() - price).abs() < 1e-9);
    }
    assert_eq!(c.logical_to_coordinate(10.25), Some(0.));
    assert_eq!(c.coordinate_to_logical(f64::NAN), None);
    assert_eq!(c.coordinate_to_price(f64::INFINITY), None);
    assert_eq!(c.continuous_logical_to_coordinate(f64::INFINITY), None);
}

#[test]
fn update_notifications_preserve_attachment_lifetime_and_validate_owner() {
    let (mut chart, series) = chart();
    let detached = Rc::new(Cell::new(0));
    let id = chart
        .add_series_primitive(
            series,
            Editable {
                detached: detached.clone(),
            },
        )
        .unwrap();
    for _ in 0..4 {
        chart.request_primitive_update(id).unwrap();
        prepare(&mut chart);
    }
    assert_eq!(detached.get(), 0);
    chart.remove_primitive(id).unwrap();
    assert_eq!(detached.get(), 1);
    assert!(chart.request_primitive_update(id).is_err());
    assert!(chart.primitive_context(id).is_err());
}

use feather_charts::api::{ChartEvent, ChartMessage, ChartState, PrimitiveEditPhase};
use iced::{
    mouse,
    widget::canvas::{Event, Program},
};
fn native(chart: &Chart, state: &mut ChartState, event: &Event, p: Point) -> Option<ChartMessage> {
    let plot = chart.frame().layout.plot;
    chart
        .program(std::convert::identity)
        .update(
            state,
            event,
            iced::Rectangle::new(Point::ORIGIN, Size::new(640., 480.)),
            mouse::Cursor::Available(Point::new(
                p.x + plot.origin.x as f32,
                p.y + plot.origin.y as f32,
            )),
        )
        .and_then(|a| a.into_inner().0)
}
fn press() -> Event {
    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
}
fn moved() -> Event {
    Event::Mouse(mouse::Event::CursorMoved {
        position: Point::ORIGIN,
    })
}
fn release() -> Event {
    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
}
fn setup_edit() -> (Chart, feather_charts::api::PrimitiveId, Rc<Cell<usize>>) {
    let (mut chart, series) = chart();
    let detached = Rc::new(Cell::new(0));
    let id = chart
        .add_series_primitive(
            series,
            Editable {
                detached: detached.clone(),
            },
        )
        .unwrap();
    prepare(&mut chart);
    (chart, id, detached)
}

#[test]
fn editable_drag_captures_before_pan_and_commits_once_without_lifecycle_churn() {
    let (mut chart, _, detached) = setup_edit();
    let mut state = ChartState::default();
    let p = Point::new(50., 50.);
    let range = chart.model().panes()[0].right_price_scale().price_range();
    let offset = chart.model().time_scale().right_offset();
    let message = native(&chart, &mut state, &press(), p).unwrap();
    let ChartMessage::PrimitiveEdit(begin) = &message else {
        panic!("edit must beat pan")
    };
    assert_eq!(begin.phase, PrimitiveEditPhase::Begin);
    let token = begin.id;
    let events = chart.update(message).unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ChartEvent::PrimitiveEdit(_)))
    );
    assert!(!chart.model().panes()[0].right_price_scale().is_auto_scale());
    assert!(!chart.is_panning());
    assert!(native(&chart, &mut state, &moved(), Point::new(52., 51.)).is_none());
    let message = native(&chart, &mut state, &moved(), Point::new(65., 70.)).unwrap();
    assert!(
        matches!(&message, ChartMessage::PrimitiveEdit(e) if e.phase == PrimitiveEditPhase::Update)
    );
    chart.update(message).unwrap();
    chart.preview_primitive_update(token).unwrap();
    prepare(&mut chart);
    assert_eq!(
        chart.model().panes()[0].right_price_scale().price_range(),
        range
    );
    assert_eq!(chart.model().time_scale().right_offset(), offset);
    assert!(native(&chart, &mut state, &moved(), Point::new(65., 70.)).is_none());
    // Outside the plot, but still inside the window: capture keeps the owner.
    let message = native(&chart, &mut state, &release(), Point::new(-10., 70.)).unwrap();
    assert!(
        matches!(&message, ChartMessage::PrimitiveEdit(e) if e.phase == PrimitiveEditPhase::Commit)
    );
    let events = chart.update(message.clone()).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, ChartEvent::PrimitiveEdit(_)))
            .count(),
        1
    );
    assert!(!events.iter().any(|e| matches!(e, ChartEvent::Click(_))));
    assert!(chart.model().panes()[0].right_price_scale().is_auto_scale());
    assert!(chart.preview_primitive_update(token).is_err());
    assert!(
        !chart
            .update(message)
            .unwrap()
            .iter()
            .any(|e| matches!(e, ChartEvent::PrimitiveEdit(_)))
    );
    assert_eq!(detached.get(), 0);
}
