use super::*;
use iced::advanced::{
    graphics::geometry::Renderer as _,
    renderer::{Headless, Renderer as _},
};
pub fn run(dir: &std::path::Path, wgpu: bool, ordered: bool, data: bool, config: bool) {
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some(if wgpu { "wgpu" } else { "tiny-skia" }),
    ))
    .unwrap();
    assert!(
        matches!(
            (&renderer, wgpu),
            (iced::Renderer::Primary(_), true) | (iced::Renderer::Secondary(_), false)
        ),
        "backend fallback"
    );
    capture::<Chart>(&mut renderer, &dir.join("calendar"), ordered, data, config);
    capture::<OptionsChart>(&mut renderer, &dir.join("price"), ordered, data, config);
    capture::<YieldCurveChart>(&mut renderer, &dir.join("yield"), ordered, data, config);
}
fn capture<C: Host>(
    renderer: &mut iced::Renderer,
    dir: &std::path::Path,
    ordered: bool,
    data: bool,
    config: bool,
) {
    std::fs::create_dir_all(dir).unwrap();
    for (w, h) in [(800., 500.), (300., 300.), (300., 90.)] {
        for ratio in [1., 1.25, 1.5, 2.] {
            let scenes: &[&str] = if config {
                &[
                    "automatic",
                    "manual",
                    "live-manual",
                    "restored",
                    "appearance",
                    "log-manual",
                ]
            } else if data {
                &["initial", "partial-pop", "empty", "append-after-pop"]
            } else if ordered {
                &["initial", "reordered", "hovered", "transferred-reordered"]
            } else {
                &[
                    "initial",
                    "move",
                    "swap",
                    "builtin-transfer",
                    "custom-transfer",
                    "remove",
                ]
            };
            for &scene in scenes {
                let mut app = App::<C>::with_configuration(false, ordered, data, config);
                app.size = Size::new(w, h);
                app.ratio = ratio;
                let action = match scene {
                    "reordered" => Some(Message::ToFront),
                    "move" => Some(Message::Down),
                    "swap" => Some(Message::Swap),
                    "builtin-transfer" => Some(Message::LineNext),
                    "custom-transfer" => Some(Message::CustomToLine),
                    "remove" => Some(Message::Remove),
                    _ => None,
                };
                if let Some(action) = action {
                    app.action(action).unwrap();
                }
                if scene == "transferred-reordered" {
                    app.action(Message::LineNext).unwrap();
                    app.action(Message::CustomToLine).unwrap();
                    app.action(Message::ToFront).unwrap();
                }
                if data && scene != "initial" {
                    let count = if scene == "empty" { usize::MAX } else { 3 };
                    app.action(Message::Pop(count)).unwrap();
                    app.action(Message::SelectSeries).unwrap();
                    app.action(Message::Pop(count)).unwrap();
                    if scene == "append-after-pop" {
                        app.action(Message::Append).unwrap();
                    }
                    app.action(Message::SelectSeries).unwrap();
                    if scene == "append-after-pop" {
                        app.action(Message::Append).unwrap();
                    }
                }
                if config {
                    use configuration::Action;
                    if scene == "log-manual" {
                        app.configuration_action(Action::Mode).unwrap();
                    }
                    if matches!(scene, "manual" | "live-manual" | "restored" | "log-manual") {
                        app.configuration_action(Action::Manual).unwrap();
                    }
                    if matches!(scene, "live-manual" | "restored") {
                        app.live = true;
                        app.action(Message::Tick(Instant::now())).unwrap();
                        app.live = false;
                    }
                    if scene == "restored" {
                        app.configuration_action(Action::Auto).unwrap();
                    }
                    if scene == "appearance" {
                        app.configuration_action(Action::Appearance).unwrap();
                        app.configuration_action(Action::Grid).unwrap();
                    }
                }
                let now = Instant::now();
                app.chart.prepare(app.size, ratio, now).unwrap();
                if config {
                    let auto = app
                        .chart
                        .core()
                        .price_scale_options(0, &C::side())
                        .unwrap()
                        .auto_scale;
                    assert_eq!(
                        auto,
                        matches!(scene, "automatic" | "restored" | "appearance")
                    );
                    if matches!(scene, "manual" | "live-manual") {
                        assert_eq!(
                            app.chart.core().visible_price_range(0, &C::side()).unwrap(),
                            Some(PriceRange {
                                min_value: 3.5,
                                max_value: 6.5
                            })
                        );
                    }
                    if scene == "appearance" {
                        assert!(app.chart.core().frame().plot.background_gradient.is_some());
                    }
                }
                if scene == "hovered" {
                    use feather_charts::ui::{
                        chart_input::ChartInteraction, hit_testing::HitGeometry,
                    };
                    let point = app.chart.core().multi_frame().unwrap().panes[0]
                        .plot
                        .hit_series
                        .iter()
                        .find_map(|h| {
                            if h.series != app.line {
                                return None;
                            }
                            if let HitGeometry::Line { data, .. } = &h.geometry {
                                data.items
                                    .get(15)
                                    .map(|p| iced::Point::new(p.x as f32, p.y as f32))
                            } else {
                                None
                            }
                        })
                        .expect("visible line point");
                    app.chart
                        .update(ChartMessage::Interaction(ChartInteraction {
                            position: Some(point),
                            navigation: None,
                            capture: false,
                        }))
                        .unwrap();
                    app.chart.prepare(app.size, ratio, now).unwrap();
                    assert!(
                        app.chart.core().multi_frame().unwrap().panes[0]
                            .plot
                            .hovered_series
                            .is_some(),
                        "hover scene must select a source"
                    );
                }
                let retained = app.chart.core().multi_frame().unwrap().clone();
                let size = app.size;
                let mut draw =
                    |snapshot: &feather_charts::ui::multi_pane_frame::IcedMultiPaneChartFrame| {
                        renderer.reset(iced::Rectangle::with_size(size));
                        let mut frame = iced::widget::canvas::Frame::new(renderer, size);
                        snapshot.draw(&mut frame);
                        renderer.draw_geometry(frame.into_geometry());
                        renderer.screenshot(
                            Size::new((w * ratio) as u32, (h * ratio) as u32),
                            ratio,
                            iced::Color::WHITE,
                        )
                    };
                let pixels = draw(&retained);
                assert!(pixels == draw(&retained), "unstable snapshot");
                if data {
                    app.action(Message::Pop(usize::MAX)).unwrap();
                    app.chart.prepare(size, ratio, now).unwrap();
                    assert!(pixels == draw(&retained), "snapshot changed after pop");
                }
                if ordered {
                    app.action(Message::ToFront).unwrap();
                    app.chart.prepare(size, ratio, now).unwrap();
                    assert!(
                        pixels == draw(&retained),
                        "snapshot changed after reordering"
                    );
                }
                if config {
                    app.configuration_action(configuration::Action::Appearance)
                        .unwrap();
                    app.chart.prepare(size, ratio, now).unwrap();
                    assert!(
                        pixels == draw(&retained),
                        "snapshot changed after configuration"
                    );
                }
                app.action(Message::Swap).unwrap();
                app.chart.prepare(size, ratio, now).unwrap();
                assert!(pixels == draw(&retained), "snapshot changed after swap");
                app.action(Message::LineNew).unwrap();
                app.chart.prepare(size, ratio, now).unwrap();
                assert!(pixels == draw(&retained), "snapshot changed after transfer");
                app.action(Message::Remove).unwrap();
                app.chart.prepare(size, ratio, now).unwrap();
                assert!(pixels == draw(&retained), "snapshot changed after removal");
                drop(app);
                assert!(pixels == draw(&retained), "snapshot changed after drop");
                std::fs::write(dir.join(format!("{scene}-{w}x{h}-{ratio}.rgba")), pixels).unwrap();
            }
        }
    }
}
