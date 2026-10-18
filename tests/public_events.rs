use feather_charts::{
    api::*,
    model::{
        data_consumer::*,
        data_layer::SeriesId,
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
    },
};

fn row(time: f64, value: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: UtcTimestamp::new(time).into(),
        value,
        color: None,
        custom_values: None,
    }))
}
fn add(c: &mut Chart) -> SeriesId {
    c.add_series(
        SeriesOptionsMap::Line(Default::default()),
        0,
        PriceScalePosition::Right,
    )
    .unwrap()
}
fn scopes(c: &mut Chart, id: SeriesId) -> Vec<&'static str> {
    c.take_events()
        .into_iter()
        .map(|e| match e {
            ChartEvent::DataChanged { series, scope } => {
                assert_eq!(series, id);
                match scope {
                    DataChangeScope::Full => "full",
                    DataChangeScope::Update => "update",
                }
            }
            e => panic!("unexpected {e:?}"),
        })
        .collect()
}
#[test]
fn source_data_sequence() {
    let mut c = Chart::default();
    let id = add(&mut c);
    c.set_event_interests(EventInterests {
        data_changed: true,
        ..Default::default()
    });
    let rows = vec![row(1., 10.), row(2., 20.), row(3., 30.)];
    c.set_data(id, rows.clone()).unwrap();
    c.set_data(id, rows.clone()).unwrap();
    c.update_data(id, row(3., 30.), false).unwrap();
    c.update_data(id, row(1., 12.), true).unwrap();
    c.pop_data(id, 0).unwrap();
    c.pop_data(id, 1).unwrap();
    c.set_data(id, vec![]).unwrap();
    c.set_data(id, vec![]).unwrap();
    c.pop_data(id, 1).unwrap();
    assert!(c.set_data(id, vec![row(2., 1.), row(1., 2.)]).is_err());
    c.set_data(id, rows).unwrap();
    let result = scopes(&mut c, id);
    assert_eq!(
        result,
        [
            "full", "full", "update", "update", "update", "full", "full", "full"
        ]
    );
    println!("EVENT_REFERENCE {:?}", result);
    assert!(c.take_events().is_empty());
}
#[test]
fn interests_errors_and_draining_keep_committed_events() {
    let mut c = Chart::default();
    let id = add(&mut c);
    c.set_data(id, vec![row(1., 10.)]).unwrap();
    assert!(c.take_events().is_empty());
    c.set_event_interests(EventInterests {
        data_changed: true,
        ..Default::default()
    });
    c.update_data(id, row(1., 11.), false).unwrap();
    assert!(
        c.update(ChartMessage::Click {
            pane: 99,
            position: iced::Point::ORIGIN
        })
        .is_err()
    );
    c.set_event_interests(Default::default());
    assert_eq!(scopes(&mut c, id), ["update"]);
    c.update_data(id, row(1., 12.), false).unwrap();
    assert!(c.take_events().is_empty());
}

#[test]
fn ranges_preserve_strict_causes_and_original_items() {
    use feather_charts::model::time_data::LogicalRange;
    let mut c = Chart::default();
    let id = add(&mut c);
    c.set_data(id, vec![row(1., 10.), row(2., 20.), row(3., 30.)])
        .unwrap();
    c.fit_content();
    let now = std::time::Instant::now();
    c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
    c.set_event_interests(EventInterests {
        visible_logical_range: true,
        visible_horizontal_range: true,
        ..Default::default()
    });
    c.set_logical_range(LogicalRange {
        from: 0.1.into(),
        to: 2.1.into(),
    });
    c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
    let events = c.take_events();
    assert_eq!(events.len(), 2, "{events:?}");
    match &events[0] {
        ChartEvent::VisibleHorizontalRangeChanged(Some(r)) => assert_eq!(
            r,
            &HorizontalRange {
                from: UtcTimestamp::new(1.).into(),
                to: UtcTimestamp::new(3.).into()
            }
        ),
        e => panic!("{e:?}"),
    }
    assert!(matches!(
        events[1],
        ChartEvent::VisibleLogicalRangeChanged(Some(_))
    ));
    c.set_logical_range(LogicalRange {
        from: 0.2.into(),
        to: 2.2.into(),
    });
    c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
    assert!(matches!(
        c.take_events().as_slice(),
        [ChartEvent::VisibleLogicalRangeChanged(Some(_))]
    ));
    c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
    assert!(c.take_events().is_empty());
    c.set_data(id, vec![]).unwrap();
    c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
    assert!(matches!(
        c.take_events().as_slice(),
        [
            ChartEvent::VisibleHorizontalRangeChanged(None),
            ChartEvent::VisibleLogicalRangeChanged(None)
        ]
    ));
}

#[test]
fn axis_size_is_installed_logical_geometry_and_hidden_zero() {
    use feather_charts::model::time_scale_options::HorzScaleOptionsPatch;
    let mut c = Chart::default();
    let id = add(&mut c);
    c.set_data(id, vec![row(1., 1.), row(2., 2.)]).unwrap();
    c.set_event_interests(EventInterests {
        horizontal_axis_size: true,
        ..Default::default()
    });
    let now = std::time::Instant::now();
    let size = iced::Size::new(800., 500.);
    c.prepare_snapshot(size, 1., now).unwrap();
    assert!(c.take_events().is_empty());
    c.prepare(size, 1., now).unwrap();
    let first = c.take_events();
    assert!(
        matches!(first.as_slice(),[ChartEvent::HorizontalAxisSizeChanged{width,height}] if *width>0. && *width<800. && *height>0.)
    );
    c.prepare(size, 1., now).unwrap();
    assert!(c.take_events().is_empty());
    c.apply_time_scale_options(HorzScaleOptionsPatch {
        visible: Some(false),
        ..Default::default()
    });
    c.prepare(size, 1., now).unwrap();
    assert!(matches!(
        c.take_events().as_slice(),
        [ChartEvent::HorizontalAxisSizeChanged {
            width: 0.,
            height: 0.
        }]
    ));
    c.apply_time_scale_options(HorzScaleOptionsPatch {
        visible: Some(true),
        ..Default::default()
    });
    c.prepare(size, 1., now).unwrap();
    let axis = c.frame().layout.time_axis.size;
    assert!(
        matches!(c.take_events().as_slice(),[ChartEvent::HorizontalAxisSizeChanged{width,height}] if *width==axis.width && *height==axis.height)
    );
    c.update_data(id, row(2., 3.), false).unwrap();
    assert!(c.prepare(size, 0., now).is_err());
    assert!(c.take_events().is_empty());
}

#[path = "numeric_custom_series/support.rs"]
mod support;

#[test]
fn custom_events_are_metadata_only_and_public_owners_survive_frontend_lifecycles() {
    use support::*;
    macro_rules! exercise {
        ($c:expr,$time:expr,$item:ty,$internal:ty) => {{
            let mut c=$c; let time=$time;
            c.set_event_interests(EventInterests::ALL);
            let view=View::<$item,$internal>::default();let destroyed=view.destroyed.clone();
            let h=c.add_custom_series(view,Default::default(),0,PriceScalePosition::Right).unwrap();
            let clones=std::rc::Rc::new(std::cell::Cell::new(0));
            let mut data=datum(time(1.),10.);
            if let feather_charts::model::icustom_series::CustomSeriesDataItem::Data(d)=&mut data {d.clones=Some(clones.clone());}
            c.set_custom_data(&h,vec![data]).unwrap();let after_commit=clones.get();
            let events=c.take_events();assert_eq!(clones.get(),after_commit,"draining must not clone data");
            assert_eq!(events.iter().filter(|e|matches!(e,ChartEvent::DataChanged{series,scope:DataChangeScope::Full} if *series==h.id())).count(),1);
            c.update_custom_data(&h,datum(time(1.),11.),false).unwrap();
            c.pop_custom_data(&h,1).unwrap();c.pop_custom_data(&h,1).unwrap();
            c.set_custom_data(&h,vec![]).unwrap();
            let events=c.take_events();
            let scopes:Vec<_>=events.iter().filter_map(|e|match e {ChartEvent::DataChanged{series,scope}=>{assert_eq!(*series,h.id());Some(*scope)},_=>None}).collect();
            assert_eq!(scopes,[DataChangeScope::Update,DataChangeScope::Update,DataChangeScope::Full]);
            assert!(c.set_custom_data(&h,vec![datum(time(2.),2.),datum(time(1.),1.)]).is_err());
            c.prepare(iced::Size::new(300.,90.),1.5,std::time::Instant::now()).unwrap();
            assert!(c.take_events().iter().all(|e|!matches!(e,ChartEvent::DataChanged{..})),"private scaffold is not public data");
            c.set_custom_data(&h,vec![datum(time(2.),20.)]).unwrap();
            let pane=c.add_pane();c.move_series_to_pane(h.id(),pane).unwrap();
            c.remove_series(h.id()).unwrap();assert_eq!(destroyed.get(),1);
            let owned=c.take_events();drop(c);
            assert!(owned.iter().any(|e|matches!(e,ChartEvent::DataChanged{series,..} if *series==h.id())));
        }}
    }
    exercise!(
        Chart::default(),
        |v| Time::from(UtcTimestamp::new(v)),
        Time,
        feather_charts::model::horz_scale_behavior_time::types::TimePoint
    );
    exercise!(
        OptionsChart::new_options(Default::default()).unwrap(),
        |v| HorizontalPrice::new(v).unwrap(),
        HorizontalPrice,
        HorizontalPrice
    );
    exercise!(
        YieldCurveChart::new(Default::default()).unwrap(),
        |v| Maturity::from_months(v).unwrap(),
        Maturity,
        Maturity
    );
}

#[test]
fn wrappers_reactions_and_sustained_feed_do_not_duplicate_or_recurse() {
    let mut c = Chart::default();
    let id = add(&mut c);
    let markers = c.add_up_down_markers(id, Default::default()).unwrap();
    c.set_event_interests(EventInterests {
        data_changed: true,
        ..Default::default()
    });
    c.set_up_down_data(markers, vec![row(1., 1.)]).unwrap();
    assert_eq!(scopes(&mut c, id), ["full"]);
    for i in 1..=1000 {
        c.update_up_down_data(markers, row(1., i as f64), false, std::time::Instant::now())
            .unwrap();
        assert_eq!(scopes(&mut c, id), ["update"]);
    }
    c.update_data(id, row(2., 2.), false).unwrap();
    let batch = c.take_events();
    for _ in batch {
        c.update_data(id, row(2., 3.), false).unwrap();
    }
    assert_eq!(scopes(&mut c, id), ["update"]);
    assert!(c.take_events().is_empty());
    c.set_event_interests(Default::default());
    for i in 0..1000 {
        c.update_data(id, row(2., i as f64), false).unwrap();
    }
    assert!(c.take_events().is_empty());
}

#[test]
fn update_and_animation_fast_return_consume_the_same_pending_stream() {
    let mut c = Chart::default();
    let id = add(&mut c);
    let now = std::time::Instant::now();
    c.set_data(id, vec![row(1., 1.), row(2., 2.)]).unwrap();
    c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
    c.set_event_interests(EventInterests {
        data_changed: true,
        ..Default::default()
    });
    c.update_data(id, row(2., 3.), false).unwrap();
    c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
    let events = c.update(ChartMessage::Tick(now)).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, ChartEvent::DataChanged { .. }))
            .count(),
        1
    );
    assert!(c.take_events().is_empty());
    assert!(c.update(ChartMessage::Tick(now)).unwrap().is_empty());
}

#[test]
fn plot_double_click_reuses_pointer_payload_without_moving_crosshair() {
    use iced::{
        Point, Rectangle, mouse,
        widget::canvas::{Event, Program},
    };
    let mut c = Chart::default();
    let id = add(&mut c);
    c.set_data(id, vec![row(1., 1.), row(2., 2.)]).unwrap();
    let size = iced::Size::new(800., 500.);
    c.prepare(size, 1., std::time::Instant::now()).unwrap();
    c.set_event_interests(EventInterests {
        double_click: true,
        ..Default::default()
    });
    c.fit_content();
    c.prepare(size, 1., std::time::Instant::now()).unwrap();
    let mut state = ChartState::default();
    let mut kinds = vec![];
    for _ in 0..3 {
        for button in [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            let message = c
                .program(std::convert::identity)
                .update(
                    &mut state,
                    &Event::Mouse(button),
                    Rectangle::with_size(size),
                    mouse::Cursor::Available(Point::new(200., 100.)),
                )
                .and_then(|a| a.into_inner().0);
            if let Some(m) = message {
                for event in c.update(m).unwrap() {
                    match event {
                        ChartEvent::Click(p) => {
                            assert_eq!(p.pane, Some(0));
                            kinds.push("click");
                        }
                        ChartEvent::DoubleClick(p) => {
                            assert_eq!(p.pane, Some(0));
                            assert!(!p.series_data.is_empty());
                            kinds.push("double");
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    assert_eq!(kinds, ["click", "double", "click"]);
    let before = c.model().crosshair().origin();
    let events = c
        .update(ChartMessage::DoubleClick {
            pane: 0,
            position: Point::new(300., 200.),
        })
        .unwrap();
    assert_eq!(c.model().crosshair().origin(), before);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ChartEvent::DoubleClick(_)))
    );
}

#[test]
fn source_range_sequence() {
    use feather_charts::model::time_data::LogicalRange;
    let mut c = Chart::default();
    let id = add(&mut c);
    let now = std::time::Instant::now();
    let size = iced::Size::new(800., 500.);
    c.set_data(id, vec![row(1., 10.), row(2., 20.), row(3., 30.)])
        .unwrap();
    c.fit_content();
    c.prepare(size, 1., now).unwrap();
    c.set_event_interests(EventInterests {
        visible_logical_range: true,
        visible_horizontal_range: true,
        ..Default::default()
    });
    let mut result = vec![];
    for (from, to) in [(0., 2.), (0.1, 2.1), (0.2, 2.2), (1., 3.)] {
        c.set_logical_range(LogicalRange {
            from: from.into(),
            to: to.into(),
        });
        c.prepare(size, 1., now).unwrap();
        for e in c.take_events() {
            let (kind, from, to) = match e {
                ChartEvent::VisibleLogicalRangeChanged(Some(r)) => {
                    ("logical", r.from.value(), r.to.value())
                }
                ChartEvent::VisibleHorizontalRangeChanged(Some(r)) => {
                    let value = |t| match t {
                        Time::UtcTimestamp(t) => t.seconds(),
                        _ => panic!("timestamp"),
                    };
                    ("horizontal", value(r.from), value(r.to))
                }
                e => panic!("unexpected {e:?}"),
            };
            result.push(format!("[\"{kind}\",{{\"from\":{from},\"to\":{to}}}]"));
        }
    }
    println!("RANGES_REFERENCE [{}]", result.join(","));
    assert_eq!(result.len(), 5);
}

#[test]
fn calendar_range_endpoints_retain_original_strings() {
    let mut c = Chart::default();
    let id = add(&mut c);
    let dates = [Time::from("2024-01-02"), Time::from("2024-01-05")];
    let rows = dates
        .iter()
        .enumerate()
        .map(|(i, time)| {
            let mut item = row(i as f64 + 1., i as f64 + 10.);
            *item.time_mut() = time.clone();
            item
        })
        .collect();
    c.set_data(id, rows).unwrap();
    c.set_event_interests(EventInterests {
        visible_horizontal_range: true,
        ..Default::default()
    });
    c.fit_content();
    c.prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    let ranges: Vec<_> = c
        .take_events()
        .into_iter()
        .filter_map(|e| match e {
            ChartEvent::VisibleHorizontalRangeChanged(Some(r)) => Some(r),
            _ => None,
        })
        .collect();
    assert!(!ranges.is_empty());
    for range in ranges {
        assert_eq!(range.from, dates[0]);
        assert_eq!(range.to, dates[1]);
    }
}

#[test]
fn cancelled_native_sequences_never_pair_with_the_next_plot_click() {
    use iced::{
        Point, Rectangle, mouse,
        widget::canvas::{Event, Program},
    };
    fn send(c: &mut Chart, state: &mut ChartState, event: Event, p: Point) -> Vec<ChartEvent> {
        let message = c
            .program(std::convert::identity)
            .update(
                state,
                &event,
                Rectangle::with_size(iced::Size::new(800., 500.)),
                mouse::Cursor::Available(p),
            )
            .and_then(|a| a.into_inner().0);
        message.map_or_else(Vec::new, |m| c.update(m).unwrap())
    }
    fn click(c: &mut Chart, state: &mut ChartState, p: Point) -> Vec<ChartEvent> {
        send(
            c,
            state,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            p,
        );
        send(
            c,
            state,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            p,
        )
    }
    let p = Point::new(200., 100.);
    for cause in [
        "drag", "leave", "focus", "revision", "outside", "axis", "divider",
    ] {
        let mut c = Chart::default();
        let id = add(&mut c);
        c.set_data(id, vec![row(1., 1.), row(2., 2.)]).unwrap();
        c.fit_content();
        if cause == "divider" {
            c.add_pane();
        }
        c.prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
            .unwrap();
        c.set_event_interests(EventInterests {
            double_click: true,
            ..Default::default()
        });
        let mut state = ChartState::default();
        assert!(
            click(&mut c, &mut state, p)
                .iter()
                .any(|e| matches!(e, ChartEvent::Click(_)))
        );
        match cause {
            "drag" => {
                send(
                    &mut c,
                    &mut state,
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    p,
                );
                let moved = Point::new(230., 120.);
                send(
                    &mut c,
                    &mut state,
                    Event::Mouse(mouse::Event::CursorMoved { position: moved }),
                    moved,
                );
                send(
                    &mut c,
                    &mut state,
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                    moved,
                );
            }
            "leave" => {
                send(
                    &mut c,
                    &mut state,
                    Event::Mouse(mouse::Event::CursorLeft),
                    p,
                );
                send(
                    &mut c,
                    &mut state,
                    Event::Mouse(mouse::Event::CursorEntered),
                    p,
                );
            }
            "focus" => {
                send(
                    &mut c,
                    &mut state,
                    Event::Window(iced::window::Event::Unfocused),
                    p,
                );
                send(
                    &mut c,
                    &mut state,
                    Event::Window(iced::window::Event::Focused),
                    p,
                );
            }
            "revision" => {
                c.add_pane();
                c.prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
                    .unwrap();
                send(
                    &mut c,
                    &mut state,
                    Event::Window(iced::window::Event::RedrawRequested(
                        std::time::Instant::now(),
                    )),
                    p,
                );
            }
            "outside" => {
                send(
                    &mut c,
                    &mut state,
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    p,
                );
                send(
                    &mut c,
                    &mut state,
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                    Point::new(-100., 100.),
                );
            }
            "divider" => {
                let y = c.multi_frame().unwrap().layout.separators[0].origin.y as f32 - 2.;
                let events = click(&mut c, &mut state, Point::new(200., y));
                assert!(
                    !events
                        .iter()
                        .any(|e| matches!(e, ChartEvent::Click(_) | ChartEvent::DoubleClick(_))),
                    "divider cannot click the plot"
                );
            }
            "axis" => {
                let events = click(&mut c, &mut state, Point::new(795., 100.));
                assert!(
                    !events
                        .iter()
                        .any(|e| matches!(e, ChartEvent::Click(_) | ChartEvent::DoubleClick(_)))
                );
            }
            _ => unreachable!(),
        }
        let events = click(&mut c, &mut state, p);
        assert!(
            events.iter().any(|e| matches!(e, ChartEvent::Click(_))),
            "{cause}: {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, ChartEvent::DoubleClick(_))),
            "{cause}"
        );
    }
}
