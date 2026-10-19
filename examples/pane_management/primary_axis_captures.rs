//! Actual renderer evidence for runtime primary-axis selection.
use super::*;
use iced::advanced::{
    graphics::geometry::Renderer as _,
    renderer::{Headless, Renderer as _},
};
use iced::widget::canvas::Program;
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
fn screen<C: Host>(chart: &C, size: Size, ratio: f32, renderer: &mut iced::Renderer) -> Vec<u8> {
    renderer.reset(iced::Rectangle::with_size(size));
    for geometry in chart
        .core()
        .program(std::convert::identity::<ChartMessage>)
        .draw(
            &ChartState::default(),
            renderer,
            &iced::Theme::Light,
            iced::Rectangle::with_size(size),
            iced::mouse::Cursor::Unavailable,
        )
    {
        renderer.draw_geometry(geometry);
    }
    renderer.screenshot(
        Size::new(
            (size.width * ratio).ceil() as u32,
            (size.height * ratio).ceil() as u32,
        ),
        ratio,
        iced::Color::WHITE,
    )
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
            for scene in ["right", "left", "hidden", "moved", "recovered", "rejected"] {
                let mut app = App::<C>::with_configuration(false, false, false, true);
                app.size = Size::new(w, h);
                app.ratio = ratio;
                let now = Instant::now();
                app.chart.prepare(app.size, ratio, now).unwrap();
                app.configuration_action(configuration::Action::CompareAxes)
                    .unwrap();
                app.configuration_action(if scene == "right" {
                    configuration::Action::PrimaryRight
                } else {
                    configuration::Action::PrimaryLeft
                })
                .unwrap();
                match scene {
                    "hidden" => app
                        .chart
                        .scale_patch(
                            0,
                            &PriceScalePosition::Left,
                            feather_charts::model::price_scale::PriceScaleOptionsPatch {
                                visible: Some(false),
                                ..Default::default()
                            },
                        )
                        .unwrap(),
                    "moved" => app.action(Message::Swap).unwrap(),
                    "recovered" => app.chart.prepare(Size::new(300., 1.), ratio, now).unwrap(),
                    "rejected" => {
                        let mut p = ChartOptionsPatch::default();
                        p.model.default_visible_price_scale = Some(DefaultVisiblePriceScale::Right);
                        p.model.layout = Some(LayoutOptionsPatch {
                            font_size: Some(f64::NAN),
                            ..Default::default()
                        });
                        assert!(app.chart.configure(p).is_err());
                        assert_eq!(
                            app.chart
                                .core()
                                .runtime_options()
                                .model
                                .pane
                                .default_visible_price_scale,
                            DefaultVisiblePriceScale::Left
                        );
                    }
                    _ => {}
                }
                app.chart.prepare(app.size, ratio, now).unwrap();
                let pane = app.chart.core().series_pane(app.line).unwrap();
                let p = &app.chart.core().model().panes()[pane];
                let expected = if scene == "right" || scene == "hidden" {
                    "right"
                } else {
                    "left"
                };
                assert_eq!(p.default_price_scale().id(), expected);
                let frame = app
                    .chart
                    .core()
                    .multi_frame()
                    .map_or_else(|| app.chart.core().frame(), |f| &f.panes[pane]);
                let axes = frame.plot.axes.as_ref().unwrap();
                assert_eq!(
                    p.grid().pane_view().renderer().data().unwrap().price_marks,
                    if expected == "left" {
                        axes.left.marks.clone()
                    } else {
                        axes.right.marks.clone()
                    }
                );
                let pixels = screen(&app.chart, app.size, ratio, renderer);
                assert_eq!(
                    pixels,
                    screen(&app.chart, app.size, ratio, renderer),
                    "screen replay changed"
                );
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
                app.configuration_action(configuration::Action::PrimaryRight)
                    .unwrap();
                app.action(Message::Reset).unwrap();
                app.chart
                    .prepare(Size::new(500., 400.), ratio, now)
                    .unwrap();
                drop(app);
                assert_eq!(
                    original.rgba(),
                    exporter.render(&shot, options).unwrap().rgba(),
                    "retained export changed"
                );
                if w == 800. && ratio == 1. {
                    export::save(&dir.join(scene), &original).unwrap();
                }
                count += 1;
            }
        }
    }
    assert_eq!(count, 72);
    println!("PRIMARY_AXIS_CAPTURES {domain} {count}");
}
