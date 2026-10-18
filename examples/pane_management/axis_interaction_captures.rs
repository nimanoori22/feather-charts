use super::*;
use iced::{
    advanced::{
        graphics::geometry::Renderer as _,
        renderer::{Headless, Renderer as _},
    },
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
    capture::<Chart>(dir, "calendar", &mut renderer, &mut exporter);
    capture::<OptionsChart>(dir, "strike", &mut renderer, &mut exporter);
    capture::<YieldCurveChart>(dir, "maturity", &mut renderer, &mut exporter);
    capture::<Chart<session_behavior::Sessions>>(dir, "sessions", &mut renderer, &mut exporter);
}
fn capture<C: Host>(
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
                "baseline",
                "all-off",
                "stretched",
                "compressed",
                "reset",
                "drag-off",
                "reset-off",
                "moved",
                "touch-reset",
            ] {
                let mut app = App::<C>::new(false);
                app.enable_axis().unwrap();
                app.size = Size::new(w, h);
                app.ratio = ratio;
                app.chart.prepare(app.size, ratio, Instant::now()).unwrap();
                let mut state = ChartState::default();
                match scene {
                    "baseline" => {}
                    "all-off" => app.axis_action(Action::AllOff).unwrap(),
                    "stretched" => drag(&mut app, &mut state, 0.6, 0.2),
                    "compressed" => drag(&mut app, &mut state, 0.6, 0.8),
                    "reset" | "touch-reset" => {
                        drag(&mut app, &mut state, 0.6, 0.2);
                        twice(&mut app, &mut state, scene == "touch-reset");
                        assert_eq!(app.chart.core().model().time_scale().bar_spacing(), 9.);
                        assert_eq!(app.chart.core().model().time_scale().right_offset(), 3.);
                    }
                    "drag-off" => {
                        app.axis_action(Action::TimeDrag).unwrap();
                        let before = app.chart.core().model().time_scale().bar_spacing();
                        drag(&mut app, &mut state, 0.6, 0.2);
                        assert_eq!(before, app.chart.core().model().time_scale().bar_spacing());
                    }
                    "reset-off" => {
                        drag(&mut app, &mut state, 0.6, 0.2);
                        app.axis_action(Action::TimeReset).unwrap();
                        let before = app.chart.core().model().time_scale().bar_spacing();
                        twice(&mut app, &mut state, false);
                        assert_eq!(before, app.chart.core().model().time_scale().bar_spacing());
                    }
                    "moved" => {
                        app.action(Message::Swap).unwrap();
                        app.action(Message::LineNew).unwrap();
                        app.chart.prepare(app.size, ratio, Instant::now()).unwrap();
                        drag(&mut app, &mut state, 0.6, 0.2);
                    }
                    _ => unreachable!(),
                }
                app.chart.prepare(app.size, ratio, Instant::now()).unwrap();
                // Populate crosshair/price labels through the public owner.
                let plot = app.chart.core().frame().layout.plot;
                if plot.size.height > 0. {
                    app.chart
                        .update(ChartMessage::Interaction(
                            feather_charts::ui::chart_input::ChartInteraction {
                                position: Some(iced::Point::new(
                                    (plot.size.width / 2.) as f32,
                                    (plot.size.height / 2.) as f32,
                                )),
                                navigation: None,
                                capture: false,
                            },
                        ))
                        .unwrap();
                }
                app.chart.prepare(app.size, ratio, Instant::now()).unwrap();
                let mut screen = |app: &App<C>| {
                    renderer.reset(iced::Rectangle::with_size(app.size));
                    let program = app
                        .chart
                        .core()
                        .program(std::convert::identity::<ChartMessage>);
                    for geometry in program.draw(
                        &ChartState::default(),
                        renderer,
                        &iced::Theme::Light,
                        iced::Rectangle::with_size(app.size),
                        iced::mouse::Cursor::Unavailable,
                    ) {
                        renderer.draw_geometry(geometry);
                    }
                    renderer.screenshot(
                        Size::new((w * ratio).ceil() as u32, (h * ratio).ceil() as u32),
                        ratio,
                        iced::Color::WHITE,
                    )
                };
                let pixels = screen(&app);
                assert!(pixels == screen(&app), "screen replay changed");
                std::fs::write(dir.join(format!("{scene}-{w}x{h}-{ratio}.rgba")), &pixels).unwrap();
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
                app.axis_action(Action::TimeDrag).unwrap();
                app.axis_action(Action::Defaults).unwrap();
                app.chart
                    .prepare(Size::new(500., 400.), ratio, Instant::now())
                    .unwrap();
                drop(app);
                let retained = exporter.render(&shot, options).unwrap();
                assert!(
                    original.rgba() == retained.rgba(),
                    "retained export changed"
                );
                if w == 800. && ratio == 1. {
                    export::save(&dir.join(scene), &original).unwrap();
                }
                count += 1;
            }
        }
    }
    assert_eq!(count, 108);
    println!("AXIS_INTERACTION_CAPTURES {domain} {count}");
}

fn point<C: Host>(app: &App<C>, fraction: f64) -> iced::Point {
    let r = app.chart.core().frame().layout.time_axis;
    iced::Point::new(
        (r.origin.x + r.size.width * fraction) as f32,
        (r.origin.y + r.size.height * 0.5) as f32,
    )
}
fn send<C: Host>(
    app: &mut App<C>,
    state: &mut ChartState,
    event: iced::widget::canvas::Event,
    p: Option<iced::Point>,
) {
    let action = app.chart.core().program(std::convert::identity).update(
        state,
        &event,
        iced::Rectangle::with_size(app.size),
        p.map_or(
            iced::mouse::Cursor::Unavailable,
            iced::mouse::Cursor::Available,
        ),
    );
    if let Some(message) = action.and_then(|a| a.into_inner().0) {
        app.chart.update(message).unwrap();
    }
    // Chart::update already prepares, matching the normal live demo event path.
}
fn drag<C: Host>(app: &mut App<C>, state: &mut ChartState, from: f64, to: f64) {
    use iced::{
        mouse::{Button, Event as M},
        widget::canvas::Event,
    };
    let start = point(app, from);
    send(
        app,
        state,
        Event::Mouse(M::ButtonPressed(Button::Left)),
        Some(start),
    );
    let end = point(app, to);
    send(
        app,
        state,
        Event::Mouse(M::CursorMoved { position: end }),
        Some(end),
    );
    send(
        app,
        state,
        Event::Mouse(M::ButtonReleased(Button::Left)),
        Some(end),
    );
}
fn twice<C: Host>(app: &mut App<C>, state: &mut ChartState, touch: bool) {
    use iced::{
        mouse::{Button, Event as M},
        touch::{Event as T, Finger},
        widget::canvas::Event,
    };
    for _ in 0..2 {
        let p = point(app, 0.6);
        if touch {
            send(
                app,
                state,
                Event::Touch(T::FingerPressed {
                    id: Finger(1),
                    position: p,
                }),
                None,
            );
            send(
                app,
                state,
                Event::Touch(T::FingerLifted {
                    id: Finger(1),
                    position: p,
                }),
                None,
            );
        } else {
            send(
                app,
                state,
                Event::Mouse(M::ButtonPressed(Button::Left)),
                Some(p),
            );
            send(
                app,
                state,
                Event::Mouse(M::ButtonReleased(Button::Left)),
                Some(p),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn narrow_fractional_touch_reset_uses_real_canvas_owner() {
        let mut app = App::<Chart>::new(false);
        app.enable_axis().unwrap();
        app.size = Size::new(300., 300.);
        app.ratio = 1.25;
        app.chart
            .prepare(app.size, app.ratio, Instant::now())
            .unwrap();
        let mut state = ChartState::default();
        drag(&mut app, &mut state, 0.6, 0.2);
        twice(&mut app, &mut state, true);
        assert_eq!(app.chart.core().model().time_scale().bar_spacing(), 9.);
    }
}
