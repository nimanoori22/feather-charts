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

#[test]
fn owned_transform_survives_scale_restoration_and_resize_in_every_mode() {
    use feather_charts::model::{
        axis_snapshots::PriceAxisSide,
        price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
    };
    for mode in [
        PriceScaleMode::Normal,
        PriceScaleMode::Logarithmic,
        PriceScaleMode::Percentage,
        PriceScaleMode::IndexedTo100,
    ] {
        for inverted in [false, true] {
            let (mut chart, id, _) = setup_edit();
            chart
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
            prepare(&mut chart);
            let c = chart.primitive_context(id).unwrap();
            let transform = c.coordinate_transform().unwrap();
            for price in [90., 100., 115.] {
                let y = c.price_to_coordinate(price).unwrap();
                assert!((transform.coordinate_to_price(y).unwrap() - price).abs() < 1e-8);
                assert!((transform.price_to_coordinate(price).unwrap() - y).abs() < 1e-8);
            }
            let x = transform.logical_to_coordinate(30.125).unwrap();
            let y = transform.price_to_coordinate(105.).unwrap();
            chart
                .prepare(Size::new(400., 300.), 2., Instant::now())
                .unwrap();
            assert!((transform.coordinate_to_logical(x).unwrap() - 30.125).abs() < 1e-8);
            assert!((transform.coordinate_to_price(y).unwrap() - 105.).abs() < 1e-8);
        }
    }
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

fn escape() -> Event {
    use iced::keyboard::{self, key};
    Event::Keyboard(keyboard::Event::KeyPressed {
        key: keyboard::Key::Named(key::Named::Escape),
        modified_key: keyboard::Key::Named(key::Named::Escape),
        physical_key: key::Physical::Code(key::Code::Escape),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::default(),
        text: None,
        repeat: false,
    })
}
#[test]
fn cancellation_releases_capture_and_navigation_resumes_without_stuck_cursor() {
    use feather_charts::api::{PrimitiveEditCancelReason as Cancel, PrimitiveEditPhase as Phase};
    for (event, reason) in [
        (escape(), Cancel::Escape),
        (
            Event::Window(iced::window::Event::Unfocused),
            Cancel::FocusLost,
        ),
        (Event::Mouse(mouse::Event::CursorLeft), Cancel::WindowLeft),
    ] {
        let (mut chart, _, _) = setup_edit();
        let mut state = ChartState::default();
        let p = Point::new(50., 50.);
        let begin = native(&chart, &mut state, &press(), p).unwrap();
        chart.update(begin).unwrap();
        let movement = native(&chart, &mut state, &moved(), Point::new(60., 60.)).unwrap();
        chart.update(movement).unwrap();
        let cancel = native(&chart, &mut state, &event, p).unwrap();
        assert!(
            matches!(&cancel, ChartMessage::PrimitiveEdit(e) if e.phase==Phase::Cancel(reason))
        );
        let events = chart.update(cancel).unwrap();
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, ChartEvent::PrimitiveEdit(_)))
                .count(),
            1
        );
        assert!(chart.model().panes()[0].right_price_scale().is_auto_scale());
        native(
            &chart,
            &mut state,
            &Event::Window(iced::window::Event::Focused),
            p,
        );
        let outside = Point::new(250., 250.);
        let msg = native(&chart, &mut state, &press(), outside).unwrap();
        chart.update(msg).unwrap();
        let msg = native(&chart, &mut state, &moved(), Point::new(280., 280.)).unwrap();
        chart.update(msg).unwrap();
        assert!(chart.is_panning());
    }
}
#[test]
fn external_updates_removal_and_resize_cancel_once_and_reject_old_preview_tokens() {
    use feather_charts::api::{PrimitiveEditCancelReason as Cancel, PrimitiveEditPhase as Phase};
    for mode in 0..5 {
        let (mut chart, id, detached) = setup_edit();
        let mut state = ChartState::default();
        let msg = native(&chart, &mut state, &press(), Point::new(50., 50.)).unwrap();
        let ChartMessage::PrimitiveEdit(begin) = &msg else {
            panic!()
        };
        let token = begin.id;
        chart.update(msg).unwrap();
        let reason = match mode {
            0 => {
                chart.request_primitive_update(id).unwrap();
                Cancel::ExternalChange
            }
            1 => {
                chart
                    .replace_primitive(
                        id,
                        Editable {
                            detached: detached.clone(),
                        },
                    )
                    .unwrap();
                Cancel::ExternalChange
            }
            2 => {
                chart.remove_primitive(id).unwrap();
                Cancel::OwnerRemoved
            }
            3 => {
                chart
                    .prepare(Size::new(400., 300.), 2., Instant::now())
                    .unwrap();
                Cancel::ViewportChanged
            }
            _ => {
                chart.set_right_offset(5.);
                prepare(&mut chart);
                Cancel::ViewportChanged
            }
        };
        let events = chart.update(ChartMessage::Tick(Instant::now())).unwrap();
        assert_eq!(
            events
                .iter()
                .filter(
                    |e| matches!(e,ChartEvent::PrimitiveEdit(e) if e.phase==Phase::Cancel(reason))
                )
                .count(),
            1
        );
        assert!(chart.preview_primitive_update(token).is_err());
        assert!(chart.model().panes()[0].right_price_scale().is_auto_scale());
        assert!(
            !chart
                .update(ChartMessage::Tick(Instant::now()))
                .unwrap()
                .iter()
                .any(|e| matches!(e, ChartEvent::PrimitiveEdit(_)))
        );
        assert!(!chart.is_animating());
    }
}
#[test]
fn wheel_is_suspended_during_edit_and_external_manual_policy_survives_cancellation() {
    use feather_charts::model::{
        axis_snapshots::PriceAxisSide, price_scale::PriceScaleOptionsPatch,
    };
    let (mut chart, _, _) = setup_edit();
    let mut state = ChartState::default();
    let p = Point::new(50., 50.);
    let msg = native(&chart, &mut state, &press(), p).unwrap();
    chart.update(msg).unwrap();
    let spacing = chart.model().time_scale().bar_spacing();
    assert!(
        native(
            &chart,
            &mut state,
            &Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Lines { x: 0., y: 4. }
            }),
            p
        )
        .is_none()
    );
    assert_eq!(chart.model().time_scale().bar_spacing(), spacing);
    chart
        .apply_price_axis_options(
            0,
            PriceAxisSide::Right,
            PriceScaleOptionsPatch {
                auto_scale: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
    chart.update(ChartMessage::Tick(Instant::now())).unwrap();
    assert!(!chart.model().panes()[0].right_price_scale().is_auto_scale());
}

#[test]
fn captured_cursor_clears_immediately_after_removal_and_manual_autoscale_policy_is_retained() {
    use feather_charts::model::{
        axis_snapshots::PriceAxisSide, price_scale::PriceScaleOptionsPatch,
    };
    let (mut chart, id, _) = setup_edit();
    chart
        .apply_price_axis_options(
            0,
            PriceAxisSide::Right,
            PriceScaleOptionsPatch {
                auto_scale: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
    prepare(&mut chart);
    let mut state = ChartState::default();
    let p = Point::new(50., 50.);
    let begin = native(&chart, &mut state, &press(), p).unwrap();
    chart.update(begin).unwrap();
    let plot = chart.frame().layout.plot;
    let cursor = mouse::Cursor::Available(Point::new(
        p.x + plot.origin.x as f32,
        p.y + plot.origin.y as f32,
    ));
    let bounds = iced::Rectangle::new(Point::ORIGIN, Size::new(640., 480.));
    assert_eq!(
        chart
            .program(std::convert::identity)
            .mouse_interaction(&state, bounds, cursor),
        mouse::Interaction::Grabbing
    );
    chart.remove_primitive(id).unwrap();
    prepare(&mut chart);
    assert_ne!(
        chart
            .program(std::convert::identity)
            .mouse_interaction(&state, bounds, cursor),
        mouse::Interaction::Grabbing
    );
    assert!(!chart.model().panes()[0].right_price_scale().is_auto_scale());
}

#[test]
fn owner_specific_transforms_cover_left_overlay_and_pane_attachments() {
    let (mut chart, _) = chart();
    for position in [
        PriceScalePosition::Left,
        PriceScalePosition::Overlay("drawing".into()),
    ] {
        let series = chart
            .add_series(SeriesOptionsMap::Line(Default::default()), 0, position)
            .unwrap();
        chart
            .set_data(
                series,
                (0..100)
                    .map(|i| {
                        BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                            time: Time::from(UtcTimestamp::new(1000. + i as f64 * 10.)),
                            value: 200. + (i % 10) as f64,
                            color: None,
                            custom_values: None,
                        }))
                    })
                    .collect(),
            )
            .unwrap();
        let id = chart
            .add_series_primitive(
                series,
                Editable {
                    detached: Rc::new(Cell::new(0)),
                },
            )
            .unwrap();
        prepare(&mut chart);
        let c = chart.primitive_context(id).unwrap();
        let t = c.coordinate_transform().unwrap();
        for price in [190., 200., 220.] {
            let y = c.price_to_coordinate(price).unwrap();
            assert!((t.coordinate_to_price(y).unwrap() - price).abs() < 1e-8);
        }
    }
    let id = chart
        .add_pane_primitive(
            0,
            Editable {
                detached: Rc::new(Cell::new(0)),
            },
        )
        .unwrap();
    let c = chart.primitive_context(id).unwrap();
    let t = c.coordinate_transform().unwrap();
    assert_eq!(t.coordinate_to_price(20.), None);
    assert_eq!(t.price_to_coordinate(100.), None);
    assert!(t.logical_to_coordinate(30.25).is_some());
}

#[test]
fn invalid_edit_input_and_empty_data_cannot_leave_an_active_session() {
    let (mut chart, id, _) = setup_edit();
    let mut state = ChartState::default();
    let begin = native(&chart, &mut state, &press(), Point::new(50., 50.)).unwrap();
    let ChartMessage::PrimitiveEdit(mut invalid) = begin.clone() else {
        panic!()
    };
    invalid.position.x = f32::NAN;
    assert!(chart.update(ChartMessage::PrimitiveEdit(invalid)).is_err());
    assert!(chart.model().panes()[0].right_price_scale().is_auto_scale());
    let ChartMessage::PrimitiveEdit(event) = &begin else {
        panic!()
    };
    let token = event.id;
    chart.update(begin).unwrap();
    let series = match chart.primitive_owner(id).unwrap() {
        feather_charts::api::PrimitiveOwner::Series(s) => s,
        _ => panic!(),
    };
    chart.set_data(series, vec![]).unwrap();
    prepare(&mut chart);
    assert!(chart.preview_primitive_update(token).is_err());
    assert!(
        chart
            .primitive_context(id)
            .unwrap()
            .coordinate_transform()
            .is_none()
    );
    let events = chart.update(ChartMessage::Tick(Instant::now())).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, ChartEvent::PrimitiveEdit(_)))
            .count(),
        1
    );
}

#[derive(Debug)]
struct Growing(Rc<Cell<f64>>);
impl DrawingPrimitive for Growing {
    fn prepare(&self, c: &PrimitiveContext<'_>) -> PreparedPrimitive {
        PreparedPrimitive {
            views: vec![PrimitiveView::new(PrimitiveLayer::Top, EditHit)],
            price_labels: vec![feather_charts::api::PrimitiveAxisLabel::new(
                c.price_to_coordinate(self.0.get()).unwrap(),
                format!("Drawing value {:.0}", self.0.get()),
                "purple",
                "white",
            )],
            ..Default::default()
        }
    }
    fn autoscale_info(
        &self,
        _: feather_charts::model::time_data::TimePointIndex,
        _: feather_charts::model::time_data::TimePointIndex,
    ) -> Option<feather_charts::model::autoscale_info_impl::AutoscaleInfo> {
        Some(feather_charts::model::autoscale_info_impl::AutoscaleInfo {
            price_range: Some(feather_charts::model::price_range_impl::PriceRange {
                min_value: 90.,
                max_value: self.0.get(),
            }),
            margins: None,
        })
    }
}
#[test]
fn widening_labels_and_autoscale_contributions_keep_viewport_fixed_until_commit() {
    let (mut chart, series) = chart();
    let high = Rc::new(Cell::new(115.));
    chart
        .add_series_primitive(series, Growing(high.clone()))
        .unwrap();
    prepare(&mut chart);
    let layout = chart.frame().layout;
    let range = chart.model().panes()[0].right_price_scale().price_range();
    let mut state = ChartState::default();
    let p = Point::new(50., 50.);
    let begin = native(&chart, &mut state, &press(), p).unwrap();
    let ChartMessage::PrimitiveEdit(event) = &begin else {
        panic!()
    };
    let token = event.id;
    chart.update(begin).unwrap();
    high.set(1000000.);
    chart.preview_primitive_update(token).unwrap();
    prepare(&mut chart);
    assert_eq!(chart.frame().layout, layout);
    assert_eq!(
        chart.model().panes()[0].right_price_scale().price_range(),
        range
    );
    let commit = native(&chart, &mut state, &release(), p).unwrap();
    chart.update(commit).unwrap();
    prepare(&mut chart);
    assert!(chart.frame().layout.right_axis.size.width > layout.right_axis.size.width);
    assert!(
        chart.model().panes()[0]
            .right_price_scale()
            .price_range()
            .unwrap()
            .max_value()
            >= 1000000.
    );
}

#[test]
fn editing_hides_the_crosshair_and_focus_loss_does_not_leave_stale_hover_labels() {
    use feather_charts::ui::chart_input::ChartInteraction;
    let (mut chart, _, _) = setup_edit();
    let p = Point::new(50., 50.);
    chart
        .update(ChartMessage::Interaction(ChartInteraction {
            position: Some(p),
            navigation: None,
            capture: false,
        }))
        .unwrap();
    assert!(chart.model().crosshair().position().is_some());
    let mut state = ChartState::default();
    let begin = native(&chart, &mut state, &press(), p).unwrap();
    chart.update(begin).unwrap();
    prepare(&mut chart);
    assert!(chart.model().crosshair().position().is_none());
    let cancel = native(
        &chart,
        &mut state,
        &Event::Window(iced::window::Event::Unfocused),
        p,
    )
    .unwrap();
    chart.update(cancel).unwrap();
    prepare(&mut chart);
    assert!(chart.model().crosshair().position().is_none());
}
