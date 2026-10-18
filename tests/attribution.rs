use feather_charts::{api::*, model::chart_model::ChartModelOptions};
#[test]
fn construction_and_runtime_visibility_readback_do_not_change_layout() {
    let mut options = ChartModelOptions::default();
    options.layout.attribution_logo = true;
    let mut chart = Chart::new(options);
    assert!(chart.runtime_options().model.layout.attribution_logo);
    chart
        .prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    let before = chart.frame().layout;
    chart
        .apply_chart_options(ChartOptionsPatch {
            model: ChartModelOptionsPatch {
                layout: Some(LayoutOptionsPatch {
                    attribution_logo: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    chart
        .prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    assert!(!chart.runtime_options().model.layout.attribution_logo);
    assert_eq!(chart.frame().layout.plot, before.plot);
}

use iced::{
    Point, Rectangle, Size, mouse,
    widget::canvas::{Event, Program},
};
fn prepared() -> Chart {
    let mut chart = Chart::default();
    chart
        .apply_chart_options(ChartOptionsPatch {
            model: ChartModelOptionsPatch {
                layout: Some(LayoutOptionsPatch {
                    attribution_logo: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    chart
        .prepare(Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    chart
}
fn send(chart: &mut Chart, state: &mut ChartState, event: Event, point: Point) -> Vec<ChartEvent> {
    let message = chart
        .program(std::convert::identity)
        .update(
            state,
            &event,
            Rectangle::with_size(Size::new(800., 500.)),
            mouse::Cursor::Available(point),
        )
        .and_then(|a| a.into_inner().0);
    message.map_or_else(Vec::new, |m| chart.update(m).unwrap())
}
fn logo_point(chart: &Chart) -> Point {
    let plot = chart.frame().layout.plot;
    Point::new(
        plot.origin.x as f32 + 20.,
        (plot.origin.y + plot.size.height) as f32 - 20.,
    )
}
#[test]
fn logo_click_is_a_default_host_action_even_on_an_empty_chart() {
    let mut chart = prepared();
    let p = logo_point(&chart);
    let mut state = ChartState::default();
    for _ in 0..2 {
        assert!(
            send(
                &mut chart,
                &mut state,
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                p
            )
            .is_empty()
        );
        let events = send(
            &mut chart,
            &mut state,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            p,
        );
        assert!(
            matches!(events.as_slice(),[ChartEvent::AttributionActivated{url}] if *url=="https://www.tradingview.com/")
        );
        assert!(!chart.is_panning() && chart.active_price_axis().is_none());
    }
}

fn visibility(chart: &mut Chart, visible: bool, color: Option<&str>) {
    chart
        .apply_chart_options(ChartOptionsPatch {
            model: ChartModelOptionsPatch {
                layout: Some(LayoutOptionsPatch {
                    attribution_logo: Some(visible),
                    text_color: color.map(Into::into),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    chart
        .prepare(Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
}
fn down(chart: &mut Chart, state: &mut ChartState, p: Point) {
    send(
        chart,
        state,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        p,
    );
}
fn up(chart: &mut Chart, state: &mut ChartState, p: Point) -> Vec<ChartEvent> {
    send(
        chart,
        state,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        p,
    )
}
#[test]
fn drag_outside_focus_escape_and_visibility_changes_cancel_logo_clicks() {
    for cause in [
        "drag", "outside", "focus", "escape", "hidden", "theme", "resize", "ratio", "pane",
    ] {
        let mut c = prepared();
        let p = logo_point(&c);
        let mut state = ChartState::default();
        down(&mut c, &mut state, p);
        let outside = Point::new(p.x + 60., p.y);
        match cause {
            "drag" | "outside" => {
                send(
                    &mut c,
                    &mut state,
                    Event::Mouse(mouse::Event::CursorMoved { position: outside }),
                    outside,
                );
            }
            "focus" => {
                send(
                    &mut c,
                    &mut state,
                    Event::Window(iced::window::Event::Unfocused),
                    p,
                );
            }
            "escape" => {
                send(
                    &mut c,
                    &mut state,
                    Event::Keyboard(iced::keyboard::Event::KeyPressed {
                        key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                        modified_key: iced::keyboard::Key::Named(
                            iced::keyboard::key::Named::Escape,
                        ),
                        physical_key: iced::keyboard::key::Physical::Unidentified(
                            iced::keyboard::key::NativeCode::Unidentified,
                        ),
                        location: iced::keyboard::Location::Standard,
                        modifiers: iced::keyboard::Modifiers::default(),
                        text: None,
                        repeat: false,
                    }),
                    p,
                );
            }
            "hidden" => visibility(&mut c, false, None),
            "theme" => visibility(&mut c, true, Some("#ffffff")),
            "resize" => {
                c.prepare(Size::new(300., 300.), 1., std::time::Instant::now())
                    .unwrap();
            }
            "ratio" => {
                c.prepare(Size::new(800., 500.), 2., std::time::Instant::now())
                    .unwrap();
            }
            "pane" => {
                c.add_pane();
                c.prepare(Size::new(800., 500.), 1., std::time::Instant::now())
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let events = up(
            &mut c,
            &mut state,
            if cause == "outside" { outside } else { p },
        );
        assert!(
            !events.iter().any(|e| matches!(
                e,
                ChartEvent::AttributionActivated { .. }
                    | ChartEvent::Click(_)
                    | ChartEvent::DoubleClick(_)
                    | ChartEvent::PrimitiveEdit(_)
            )),
            "{cause}: {events:?}"
        );
    }
}
#[test]
fn touch_tap_survives_redraw_but_drag_multitouch_and_synthetic_mouse_do_not_activate() {
    use iced::touch::{Event as FingerEvent, Finger};
    for cause in ["tap", "move", "second", "lost"] {
        let mut c = prepared();
        let p = logo_point(&c);
        let mut state = ChartState::default();
        send(
            &mut c,
            &mut state,
            Event::Touch(FingerEvent::FingerPressed {
                id: Finger(1),
                position: p,
            }),
            Point::ORIGIN,
        );
        send(
            &mut c,
            &mut state,
            Event::Window(iced::window::Event::RedrawRequested(
                std::time::Instant::now(),
            )),
            Point::ORIGIN,
        );
        match cause {
            "move" => {
                send(
                    &mut c,
                    &mut state,
                    Event::Touch(FingerEvent::FingerMoved {
                        id: Finger(1),
                        position: Point::new(p.x + 20., p.y),
                    }),
                    Point::ORIGIN,
                );
            }
            "second" => {
                send(
                    &mut c,
                    &mut state,
                    Event::Touch(FingerEvent::FingerPressed {
                        id: Finger(2),
                        position: p,
                    }),
                    Point::ORIGIN,
                );
            }
            "lost" => {
                send(
                    &mut c,
                    &mut state,
                    Event::Touch(FingerEvent::FingerLost {
                        id: Finger(1),
                        position: p,
                    }),
                    Point::ORIGIN,
                );
            }
            _ => {}
        }
        let events = send(
            &mut c,
            &mut state,
            Event::Touch(FingerEvent::FingerLifted {
                id: Finger(1),
                position: p,
            }),
            Point::ORIGIN,
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, ChartEvent::AttributionActivated { .. }))
                .count(),
            usize::from(cause == "tap"),
            "{cause}"
        );
        down(&mut c, &mut state, p);
        assert!(
            up(&mut c, &mut state, p).is_empty(),
            "synthetic mouse must not activate"
        );
    }
}
#[test]
fn last_pane_theme_threshold_and_tiny_clip_are_source_compatible() {
    let mut c = prepared();
    c.add_pane();
    c.prepare(Size::new(800., 500.), 1.25, std::time::Instant::now())
        .unwrap();
    let first = c
        .multi_frame()
        .unwrap()
        .attribution
        .as_ref()
        .unwrap()
        .clone();
    let plot = c.multi_frame().unwrap().panes.last().unwrap().layout.plot;
    assert_eq!(first.bounds().size(), Size::new(35., 19.));
    assert_eq!(first.bounds().x, plot.origin.x as f32 + 10.);
    assert_eq!(
        first.bounds().y,
        (plot.origin.y + plot.size.height) as f32 - 29.
    );
    assert!(!first.dark());
    c.swap_panes(0, 1).unwrap();
    c.prepare(Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    assert_ne!(
        first.owner(),
        c.multi_frame()
            .unwrap()
            .attribution
            .as_ref()
            .unwrap()
            .owner()
    );
    for (color, dark) in [
        ("#9f9f9f", false),
        ("#a0a0a0", true),
        ("#a1a1a1", true),
        ("#ffffff", true),
    ] {
        visibility(&mut c, true, Some(color));
        assert_eq!(
            c.multi_frame()
                .unwrap()
                .attribution
                .as_ref()
                .unwrap()
                .dark(),
            dark
        );
    }
    c.prepare(Size::new(35., 40.), 1.5, std::time::Instant::now())
        .unwrap();
    if let Some(logo) = &c.multi_frame().unwrap().attribution {
        assert!(logo.clip().intersection(&logo.hit_bounds()).is_some());
    }
    drop(c);
    assert!(!first.dark());
}
#[test]
fn all_screenshot_flags_exclude_enabled_logo_and_snapshots_survive_drop() {
    let mut exporter =
        iced::futures::executor::block_on(ChartExporter::new(ScreenshotBackend::Software)).unwrap();
    let mut c = prepared();
    let mut enabled = vec![];
    for top in [false, true] {
        for crosshair in [false, true] {
            let shot = c
                .screenshot_of_prepared(ScreenshotLayers {
                    include_top_layer: top,
                    include_crosshair: crosshair,
                })
                .unwrap();
            enabled.push((
                shot.clone(),
                exporter.render(&shot, Default::default()).unwrap(),
            ));
        }
    }
    visibility(&mut c, false, None);
    for (index, (top, crosshair)) in [false, true]
        .into_iter()
        .flat_map(|t| [false, true].map(|c| (t, c)))
        .enumerate()
    {
        let shot = c
            .screenshot_of_prepared(ScreenshotLayers {
                include_top_layer: top,
                include_crosshair: crosshair,
            })
            .unwrap();
        assert_eq!(
            enabled[index].1.rgba(),
            exporter.render(&shot, Default::default()).unwrap().rgba()
        );
    }
    drop(c);
    for (shot, image) in enabled {
        assert_eq!(
            image.rgba(),
            exporter.render(&shot, Default::default()).unwrap().rgba()
        );
    }
}

#[test]
fn screen_pixels_change_only_in_the_logo_and_native_source_policy_matches() {
    use iced::advanced::{
        graphics::geometry::Renderer as _,
        renderer::{Headless, Renderer as _},
    };
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some("tiny-skia"),
    ))
    .unwrap();
    let mut c = prepared();
    let draw = |c: &Chart, renderer: &mut iced::Renderer| {
        renderer.reset(Rectangle::with_size(Size::new(800., 500.)));
        for geometry in c.program(std::convert::identity::<ChartMessage>).draw(
            &ChartState::default(),
            renderer,
            &iced::Theme::Light,
            Rectangle::with_size(Size::new(800., 500.)),
            mouse::Cursor::Unavailable,
        ) {
            renderer.draw_geometry(geometry);
        }
        renderer.screenshot(Size::new(800, 500), 1., iced::Color::WHITE)
    };
    let enabled = draw(&c, &mut renderer);
    let p = logo_point(&c);
    visibility(&mut c, false, None);
    let disabled = draw(&c, &mut renderer);
    let mut differences = 0;
    for (i, (a, b)) in enabled
        .as_chunks::<4>()
        .0
        .iter()
        .zip(disabled.as_chunks::<4>().0.iter())
        .enumerate()
    {
        if a != b {
            differences += 1;
            let x = (i % 800) as f32;
            let y = (i / 800) as f32;
            assert!(
                (p.x - 10. ..p.x + 25.).contains(&x) && (p.y - 9. ..p.y + 10.).contains(&y),
                "pixel {x},{y} escaped logo"
            );
        }
    }
    assert!(differences > 100, "missing on-screen logo");
    c.add_pane();
    visibility(&mut c, true, None);
    let logo = c.multi_frame().unwrap().attribution.as_ref().unwrap();
    let plot = c.multi_frame().unwrap().panes.last().unwrap().layout.plot;
    let (width, height, left, bottom, dark) = (
        logo.bounds().width,
        logo.bounds().height,
        logo.bounds().x - plot.origin.x as f32,
        plot.origin.y as f32 + plot.size.height as f32 - logo.bounds().y - logo.bounds().height,
        logo.dark(),
    );
    let mut themes = vec![];
    for color in ["#a0a0a0", "#a1a1a1", "#ffffff"] {
        visibility(&mut c, true, Some(color));
        themes.push(
            c.multi_frame()
                .unwrap()
                .attribution
                .as_ref()
                .unwrap()
                .dark(),
        );
    }
    println!(
        "ATTRIBUTION_REFERENCE [{width},{height},{left},{bottom},{dark},{},{},{}]",
        themes[0], themes[1], themes[2]
    );
}

fn data_chart() -> Chart {
    use feather_charts::model::{
        data_consumer::*, horz_scale_behavior_time::types::UtcTimestamp, pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
    };
    let mut c = prepared();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(
        id,
        (0..100)
            .map(|i| {
                BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                    time: UtcTimestamp::new(i as f64 + 1.).into(),
                    value: 100. + (i as f64 * 0.2).sin(),
                    color: None,
                    custom_values: None,
                }))
            })
            .collect(),
    )
    .unwrap();
    c.fit_content();
    c.prepare(Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    c
}
#[test]
fn active_plot_and_price_axis_drags_keep_ownership_when_crossing_logo() {
    for axis in [false, true] {
        let mut c = data_chart();
        let mut state = ChartState::default();
        let p = logo_point(&c);
        send(
            &mut c,
            &mut state,
            Event::Window(iced::window::Event::RedrawRequested(
                std::time::Instant::now(),
            )),
            p,
        );
        let start = if axis {
            let r = c.frame().layout.right_axis;
            Point::new(r.origin.x as f32 + 8., 100.)
        } else {
            Point::new(250., 150.)
        };
        down(&mut c, &mut state, start);
        let events = send(
            &mut c,
            &mut state,
            Event::Mouse(mouse::Event::CursorMoved { position: p }),
            p,
        );
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, ChartEvent::AttributionActivated { .. }))
        );
        assert!(
            !up(&mut c, &mut state, p)
                .iter()
                .any(|e| matches!(e, ChartEvent::AttributionActivated { .. }))
        );
        assert!(!c.is_panning());
        assert!(c.active_price_axis().is_none());
    }
}
#[derive(Debug)]
struct Cover;
impl DrawingPrimitive for Cover {
    fn prepare(&self, _: &PrimitiveContext<'_>) -> PreparedPrimitive {
        PreparedPrimitive {
            views: vec![PrimitiveView::new(PrimitiveLayer::Top, Self)],
            ..Default::default()
        }
    }
}
impl PrimitiveRenderer for Cover {
    fn draw(&self, _: &mut iced::widget::canvas::Frame) {}
    fn hit_test(&self, _: Point) -> Option<PrimitiveHitCandidate> {
        let mut hit = PrimitiveHitCandidate::new("cover");
        hit.layer = PrimitiveLayer::Top;
        hit.editable = true;
        Some(hit)
    }
}
#[test]
fn logo_wins_over_overlapping_primitive_but_active_edit_is_not_stolen() {
    let mut c = data_chart();
    let id = c.series_ids().next().unwrap();
    c.add_series_primitive(id, Cover).unwrap();
    c.prepare(Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    let mut state = ChartState::default();
    let p = logo_point(&c);
    down(&mut c, &mut state, p);
    assert!(matches!(
        up(&mut c, &mut state, p).as_slice(),
        [ChartEvent::AttributionActivated { .. }]
    ));
    let start = Point::new(250., 150.);
    let begin = send(
        &mut c,
        &mut state,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        start,
    );
    assert!(
        begin
            .iter()
            .any(|e| matches!(e, ChartEvent::PrimitiveEdit(_)))
    );
    let moved = send(
        &mut c,
        &mut state,
        Event::Mouse(mouse::Event::CursorMoved { position: p }),
        p,
    );
    assert!(
        !moved
            .iter()
            .any(|e| matches!(e, ChartEvent::AttributionActivated { .. }))
    );
    let release = up(&mut c, &mut state, p);
    assert!(
        release
            .iter()
            .any(|e| matches!(e, ChartEvent::PrimitiveEdit(_)))
    );
    assert!(
        !release
            .iter()
            .any(|e| matches!(e, ChartEvent::AttributionActivated { .. }))
    );
}

#[test]
fn hovering_logo_clears_underlying_plot_and_redraw_does_not_restore_it() {
    let mut c = data_chart();
    let mut state = ChartState::default();
    let outside = Point::new(250., 150.);
    send(
        &mut c,
        &mut state,
        Event::Mouse(mouse::Event::CursorMoved { position: outside }),
        outside,
    );
    assert!(c.model().crosshair().position().is_some());
    let p = logo_point(&c);
    let events = send(
        &mut c,
        &mut state,
        Event::Mouse(mouse::Event::CursorMoved { position: p }),
        p,
    );
    assert!(c.model().crosshair().position().is_none());
    assert!(!events.iter().any(|e| matches!(
        e,
        ChartEvent::AttributionActivated { .. } | ChartEvent::Click(_)
    )));
    send(
        &mut c,
        &mut state,
        Event::Window(iced::window::Event::RedrawRequested(
            std::time::Instant::now(),
        )),
        p,
    );
    assert!(c.model().crosshair().position().is_none());
}

#[test]
fn logo_hover_keeps_last_price_animation_ticks_scheduled() {
    use feather_charts::model::series_options::{
        LastPriceAnimationMode, LineSeriesOptions, SeriesOptionsMap,
    };
    let mut c = data_chart();
    let id = c.series_ids().next().unwrap();
    let mut options = LineSeriesOptions::default();
    options.style.last_price_animation = LastPriceAnimationMode::Continuous;
    c.replace_series_options(id, SeriesOptionsMap::Line(options))
        .unwrap();
    c.prepare(Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    assert!(c.is_animating());
    let p = logo_point(&c);
    let mut state = ChartState::default();
    send(
        &mut c,
        &mut state,
        Event::Mouse(mouse::Event::CursorMoved { position: p }),
        p,
    );
    let action = c
        .program(std::convert::identity::<ChartMessage>)
        .update(
            &mut state,
            &Event::Window(iced::window::Event::RedrawRequested(
                std::time::Instant::now(),
            )),
            Rectangle::with_size(Size::new(800., 500.)),
            mouse::Cursor::Available(p),
        )
        .expect("animation must keep scheduling over logo");
    assert!(matches!(action.into_inner().0, Some(ChartMessage::Tick(_))));
}
