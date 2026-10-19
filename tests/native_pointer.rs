use feather_charts::api::*;
use iced::{Point, keyboard::Modifiers};

#[test]
fn supplied_pointer_snapshot_is_owned_and_optional() {
    let input = PointerInput {
        device: PointerDevice::Mouse,
        kind: PointerInputKind::Release,
        button: Some(iced::mouse::Button::Left),
        chart_position: Some(Point::new(30., 40.)),
        window_position: Some(Point::new(50., 70.)),
        modifiers: Some(Modifiers::CTRL | Modifiers::SHIFT),
    };
    let message = ChartMessage::Click {
        pane: 0,
        position: Point::new(10., 20.),
    };
    let (message, source) = message.with_pointer_input(input).into_pointer_parts();
    assert_eq!(source, Some(input));
    assert!(matches!(message, ChartMessage::Click { pane: 0, .. }));
    assert!(message.into_pointer_parts().1.is_none());
    assert!(
        ChartMessage::Tick(std::time::Instant::now())
            .with_pointer_input(input)
            .into_pointer_parts()
            .1
            .is_none()
    );
}

fn chart() -> Chart {
    use feather_charts::model::{
        data_consumer::*, horz_scale_behavior_time::types::UtcTimestamp, pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
    };
    let mut chart = Chart::default();
    let id = chart
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(
            id,
            (1..=5)
                .map(|i| {
                    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                        time: UtcTimestamp::new(i as f64).into(),
                        value: i as f64,
                        color: None,
                        custom_values: None,
                    }))
                })
                .collect(),
        )
        .unwrap();
    chart.fit_content();
    chart
        .prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    chart.set_event_interests(EventInterests {
        double_click: true,
        ..Default::default()
    });
    chart
}
fn dispatch(
    chart: &Chart,
    state: &mut ChartState,
    event: iced::Event,
    cursor: Option<Point>,
) -> Option<ChartMessage> {
    use iced::widget::canvas::Program;
    chart
        .program(std::convert::identity)
        .update(
            state,
            &event,
            iced::Rectangle::new(Point::new(20., 30.), iced::Size::new(800., 500.)),
            cursor.map_or(
                iced::mouse::Cursor::Unavailable,
                iced::mouse::Cursor::Available,
            ),
        )
        .and_then(|a| a.into_inner().0)
}
fn pointers(events: Vec<ChartEvent>) -> Vec<PointerEvent> {
    events
        .into_iter()
        .filter_map(|e| match e {
            ChartEvent::Click(p) | ChartEvent::DoubleClick(p) | ChartEvent::CrosshairMoved(p) => {
                Some(p)
            }
            _ => None,
        })
        .collect()
}
#[test]
fn widget_samples_coordinates_and_modifiers_before_dispatch() {
    let mut chart = chart();
    let mut state = ChartState::default();
    dispatch(
        &chart,
        &mut state,
        iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(
            Modifiers::CTRL | Modifiers::SHIFT,
        )),
        None,
    );
    let position = Point::new(240., 170.);
    let message = dispatch(
        &chart,
        &mut state,
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }),
        Some(Point::new(245., 177.)),
    )
    .unwrap();
    dispatch(
        &chart,
        &mut state,
        iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(Modifiers::NONE)),
        Some(position),
    );
    let events = pointers(chart.update(message).unwrap());
    let source = events.last().unwrap().source_event.expect("native source");
    assert_eq!(source.window_position, Some(position));
    assert_eq!(source.chart_position, Some(Point::new(220., 140.)));
    assert_eq!(source.modifiers, Some(Modifiers::CTRL | Modifiers::SHIFT));
    assert_eq!(source.kind, PointerInputKind::Move);
    assert_eq!(source.device, PointerDevice::Mouse);
    assert_eq!(source.button, None);
    let retained = events.last().unwrap().clone();
    drop(chart);
    assert_eq!(retained.source_event, Some(source));
}

fn send(
    chart: &mut Chart,
    state: &mut ChartState,
    event: iced::Event,
    cursor: Option<Point>,
) -> Vec<ChartEvent> {
    dispatch(chart, state, event, cursor).map_or_else(Vec::new, |m| chart.update(m).unwrap())
}
fn click(chart: &mut Chart, state: &mut ChartState, position: Point) -> Vec<ChartEvent> {
    send(
        chart,
        state,
        iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        Some(position),
    );
    send(
        chart,
        state,
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
            iced::mouse::Button::Left,
        )),
        Some(position),
    )
}
#[test]
fn clicks_double_clicks_focus_and_programmatic_events_have_scoped_sources() {
    let mut c = chart();
    let mut s = ChartState::default();
    let p = Point::new(240., 170.);
    let first = pointers(click(&mut c, &mut s, p));
    let first = first.iter().find_map(|p| p.source_event).unwrap();
    assert_eq!(first.modifiers, None);
    assert_eq!(first.kind, PointerInputKind::Release);
    assert_eq!(first.button, Some(iced::mouse::Button::Left));
    send(
        &mut c,
        &mut s,
        iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(
            Modifiers::ALT | Modifiers::LOGO,
        )),
        Some(p),
    );
    let events = click(&mut c, &mut s, p);
    let double = events
        .iter()
        .find_map(|e| {
            if let ChartEvent::DoubleClick(p) = e {
                Some(p)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(
        double.source_event.unwrap().modifiers,
        Some(Modifiers::ALT | Modifiers::LOGO)
    );
    send(
        &mut c,
        &mut s,
        iced::Event::Window(iced::window::Event::Unfocused),
        None,
    );
    send(
        &mut c,
        &mut s,
        iced::Event::Window(iced::window::Event::Focused),
        Some(p),
    );
    assert!(
        pointers(click(&mut c, &mut s, p)).iter().all(|p| p
            .source_event
            .unwrap()
            .modifiers
            .is_none())
    );
    let events = c
        .update(ChartMessage::Click {
            pane: 0,
            position: Point::new(210., 140.),
        })
        .unwrap();
    assert!(pointers(events).iter().all(|p| p.source_event.is_none()));
    let mut invalid = first;
    invalid.chart_position = Some(Point::new(f32::NAN, 0.));
    let before = c.model().crosshair().origin();
    assert!(
        c.update(
            ChartMessage::Click {
                pane: 0,
                position: Point::new(80., 80.)
            }
            .with_pointer_input(invalid)
        )
        .is_err()
    );
    assert_eq!(c.model().crosshair().origin(), before);
    assert!(c.take_events().is_empty());
}
#[test]
fn touch_tap_hold_and_synthetic_mouse_suppression_keep_original_source() {
    use iced::{
        Event,
        touch::{Event as Touch, Finger},
    };
    let mut c = chart();
    let mut s = ChartState::default();
    let p = Point::new(240., 170.);
    let id = Finger(47);
    send(
        &mut c,
        &mut s,
        Event::Keyboard(iced::keyboard::Event::ModifiersChanged(Modifiers::SHIFT)),
        None,
    );
    send(
        &mut c,
        &mut s,
        Event::Touch(Touch::FingerPressed { id, position: p }),
        None,
    );
    let events = send(
        &mut c,
        &mut s,
        Event::Touch(Touch::FingerLifted { id, position: p }),
        None,
    );
    let tap = events
        .iter()
        .find_map(|e| {
            if let ChartEvent::Click(p) = e {
                Some(p)
            } else {
                None
            }
        })
        .unwrap();
    let source = tap.source_event.unwrap();
    assert_eq!(source.device, PointerDevice::Touch(id));
    assert_eq!(source.kind, PointerInputKind::Release);
    assert_eq!(source.button, None);
    assert_eq!(source.window_position, Some(p));
    assert_eq!(source.modifiers, Some(Modifiers::SHIFT));
    assert!(
        click(&mut c, &mut s, p).is_empty(),
        "synthesized mouse sequence must remain suppressed"
    );
    send(
        &mut c,
        &mut s,
        Event::Touch(Touch::FingerPressed { id, position: p }),
        None,
    );
    send(
        &mut c,
        &mut s,
        Event::Keyboard(iced::keyboard::Event::ModifiersChanged(Modifiers::CTRL)),
        None,
    );
    let held = send(
        &mut c,
        &mut s,
        Event::Window(iced::window::Event::RedrawRequested(
            std::time::Instant::now() + std::time::Duration::from_millis(300),
        )),
        None,
    );
    let source = pointers(held)
        .into_iter()
        .find_map(|p| p.source_event)
        .expect("long-press source");
    assert_eq!(source.kind, PointerInputKind::Press);
    assert_eq!(source.modifiers, Some(Modifiers::SHIFT));
    assert_eq!(source.device, PointerDevice::Touch(id));
    send(
        &mut c,
        &mut s,
        Event::Window(iced::window::Event::Unfocused),
        None,
    );
    let cleared = pointers(
        c.update(ChartMessage::Click {
            pane: 0,
            position: Point::new(180., 90.),
        })
        .unwrap(),
    );
    assert!(cleared.iter().all(|p| p.source_event.is_none()));
}

#[test]
fn keyboard_snapshots_refresh_modifiers_and_redraw_never_fabricates_source() {
    use iced::{
        Event,
        keyboard::{self, Key, Location, key},
    };
    let mut c = chart();
    let mut s = ChartState::default();
    let p = Point::new(240., 170.);
    let keyboard = keyboard::Event::KeyPressed {
        key: Key::Character("a".into()),
        modified_key: Key::Character("A".into()),
        physical_key: key::Physical::Code(key::Code::KeyA),
        location: Location::Standard,
        modifiers: Modifiers::SHIFT,
        text: None,
        repeat: false,
    };
    send(&mut c, &mut s, Event::Keyboard(keyboard), None);
    assert!(
        pointers(click(&mut c, &mut s, p))
            .iter()
            .all(|p| p.source_event.unwrap().modifiers == Some(Modifiers::SHIFT))
    );
    send(
        &mut c,
        &mut s,
        Event::Keyboard(keyboard::Event::KeyReleased {
            key: Key::Character("a".into()),
            modified_key: Key::Character("a".into()),
            physical_key: key::Physical::Code(key::Code::KeyA),
            location: Location::Standard,
            modifiers: Modifiers::NONE,
        }),
        Some(p),
    );
    assert!(
        pointers(click(&mut c, &mut s, p))
            .iter()
            .all(|p| p.source_event.unwrap().modifiers == Some(Modifiers::NONE))
    );
    if let Some(message) = dispatch(
        &c,
        &mut s,
        Event::Window(iced::window::Event::RedrawRequested(
            std::time::Instant::now(),
        )),
        Some(Point::new(280., 200.)),
    ) {
        assert!(message.into_pointer_parts().1.is_none());
    }
    let programmatic =
        ChartMessage::Interaction(feather_charts::ui::chart_input::ChartInteraction {
            position: Some(Point::new(350., 200.)),
            navigation: None,
            capture: false,
        });
    let events = pointers(c.update(programmatic).unwrap());
    assert!(!events.is_empty());
    assert!(events.iter().all(|p| p.source_event.is_none()));
}
