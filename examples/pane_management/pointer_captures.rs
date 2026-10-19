//! Actual Canvas input followed by strict native renderer and retained-export checks.
use super::*;
use iced::{
    Event, Point,
    advanced::renderer::Headless,
    keyboard::{Event as Key, Modifiers},
    mouse,
    widget::canvas::Program,
};
pub fn run(dir: &std::path::Path, wgpu: bool) {
    let backend = if wgpu { "wgpu" } else { "tiny-skia" };
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some(backend),
    ))
    .unwrap();
    assert_eq!(renderer.name(), backend, "renderer fallback");
    let mut exporter = iced::futures::executor::block_on(ChartExporter::new(if wgpu {
        ScreenshotBackend::Wgpu
    } else {
        ScreenshotBackend::Software
    }))
    .unwrap();
    capture::<Chart<HorzScaleBehaviorTime, Rc<metadata::Record>>>(
        dir,
        "calendar",
        &mut renderer,
        &mut exporter,
    );
    capture::<Chart<HorzScaleBehaviorPrice, Rc<metadata::Record>>>(
        dir,
        "strike",
        &mut renderer,
        &mut exporter,
    );
    capture::<YieldCurveChart<Rc<metadata::Record>>>(dir, "maturity", &mut renderer, &mut exporter);
    capture::<Chart<session_behavior::Sessions, Rc<metadata::Record>>>(
        dir,
        "sessions",
        &mut renderer,
        &mut exporter,
    );
}
fn send<C: Host>(app: &mut App<C>, state: &mut ChartState, event: Event, point: Point) {
    let message = app
        .chart
        .core()
        .program(std::convert::identity)
        .update(
            state,
            &event,
            iced::Rectangle::new(Point::new(20., 30.), app.size),
            mouse::Cursor::Available(point),
        )
        .and_then(|a| a.into_inner().0);
    if let Some(message) = message {
        let events = app.chart.update(message).unwrap();
        app.consume_events(events);
    }
}
fn click<C: Host>(app: &mut App<C>, state: &mut ChartState, point: Point) {
    for event in [
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
    ] {
        send(app, state, Event::Mouse(event), point);
    }
}
fn capture<C: Host<Metadata = Rc<metadata::Record>>>(
    dir: &std::path::Path,
    domain: &str,
    renderer: &mut iced::Renderer,
    exporter: &mut ChartExporter,
) {
    let dir = dir.join(domain);
    std::fs::create_dir_all(&dir).unwrap();
    let mut count = 0;
    for (w, h) in [(800., 500.), (300., 300.), (300., 90.)] {
        for ratio in [1., 1.25, 1.5, 2.] {
            for scene in [
                "hover",
                "pinned",
                "cleared",
                "programmatic",
                "moved",
                "recovered",
            ] {
                let mut app = App::<C>::new(false);
                app.enable_pointer();
                app.size = Size::new(w, h);
                app.ratio = ratio;
                app.prepare(Instant::now());
                if scene == "moved" {
                    app.action(Message::Swap).unwrap();
                    app.action(Message::LineNew).unwrap();
                    app.prepare(Instant::now());
                }
                if scene == "recovered" {
                    app.chart
                        .prepare(Size::new(300., 10.), ratio, Instant::now())
                        .unwrap();
                    app.prepare(Instant::now());
                }
                let pane = app.chart.core().series_pane(app.line).unwrap();
                let plot = app.chart.core().multi_frame().unwrap().layout.panes[pane].plot;
                assert!(plot.size.width > 0. && plot.size.height > 0.);
                let local = Point::new(
                    plot.origin.x as f32 + plot.size.width as f32 * 0.75,
                    plot.origin.y as f32 + plot.size.height as f32 * 0.5,
                );
                let position = Point::new(local.x + 20., local.y + 30.);
                let mut state = ChartState::default();
                send(
                    &mut app,
                    &mut state,
                    Event::Mouse(mouse::Event::CursorMoved { position }),
                    position,
                );
                let source = app
                    .pointer
                    .as_ref()
                    .unwrap()
                    .last
                    .expect("native hover source");
                assert_eq!(
                    source.chart_position,
                    Some(Point::new(position.x - 20., position.y - 30.))
                );
                assert_eq!(source.window_position, Some(position));
                assert_eq!(source.modifiers, None);
                let initial = metadata_captures::screen(&app.chart, app.size, ratio, renderer);
                if matches!(scene, "pinned" | "cleared") {
                    send(
                        &mut app,
                        &mut state,
                        Event::Keyboard(Key::ModifiersChanged(Modifiers::CTRL)),
                        position,
                    );
                    click(&mut app, &mut state, position);
                    let pin = app
                        .pointer
                        .as_ref()
                        .unwrap()
                        .pinned
                        .as_ref()
                        .expect("Ctrl click must pin");
                    assert_eq!(pin.pane, Some(pane));
                    assert_eq!(pin.source_event.unwrap().modifiers, Some(Modifiers::CTRL));
                    if scene == "cleared" {
                        send(
                            &mut app,
                            &mut state,
                            Event::Keyboard(Key::ModifiersChanged(Modifiers::SHIFT)),
                            position,
                        );
                        click(&mut app, &mut state, position);
                        assert!(
                            app.pointer.as_ref().unwrap().pinned.is_none(),
                            "rapid Shift click must clear even when recognized as double-click"
                        );
                    }
                }
                if scene == "programmatic" {
                    app.action(Message::PointerProgrammatic).unwrap();
                    app.prepare(Instant::now());
                    assert!(app.pointer.as_ref().unwrap().last.is_none());
                }
                let pixels = metadata_captures::screen(&app.chart, app.size, ratio, renderer);
                assert!(
                    pixels == metadata_captures::screen(&app.chart, app.size, ratio, renderer),
                    "replay changed"
                );
                if matches!(scene, "pinned" | "cleared") {
                    assert!(
                        pixels == initial,
                        "modifier-only action changed chart pixels"
                    );
                }
                std::fs::write(dir.join(format!("{scene}-{w}x{h}-{ratio}.rgba")), pixels).unwrap();
                let shot = app
                    .chart
                    .core()
                    .screenshot_of_prepared(ScreenshotLayers {
                        include_top_layer: true,
                        include_crosshair: true,
                    })
                    .unwrap();
                let options = ScreenshotRasterOptions {
                    pixel_ratio: ratio,
                    ..Default::default()
                };
                let original = exporter.render(&shot, options).unwrap();
                app.action(Message::Reset).unwrap();
                drop(app);
                drop(state);
                assert!(
                    original.rgba() == exporter.render(&shot, options).unwrap().rgba(),
                    "retained export changed"
                );
                count += 1;
            }
        }
    }
    assert_eq!(count, 72);
    println!("NATIVE_POINTER_CAPTURES {domain} {count}");
}
