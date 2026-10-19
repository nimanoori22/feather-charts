//! Typed application records are retained by data, never built-in geometry.
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
            let mut baseline = None;
            for scene in [
                "initial",
                "latest",
                "history",
                "cleared",
                "retained",
                "restored",
                "moved",
                "removed",
                "recovered",
            ] {
                let mut app = App::<C>::new(false);
                app.metadata = Some(Default::default());
                app.size = Size::new(w, h);
                app.ratio = ratio;
                let now = Instant::now();
                app.chart.prepare(app.size, ratio, now).unwrap();
                let weak = {
                    let rows = app.chart.core().series_data(app.line).unwrap();
                    let BuiltInSeriesDataItem::Line(LineDataItem::Data(row)) = rows.last().unwrap()
                    else {
                        panic!()
                    };
                    Rc::downgrade(row.custom_values.as_ref().unwrap())
                };
                match scene {
                    "initial" => {}
                    "latest" => app.metadata_action(metadata::Action::Latest).unwrap(),
                    "history" => app.metadata_action(metadata::Action::History).unwrap(),
                    "cleared" => app
                        .metadata_action(metadata::Action::ClearMetadata)
                        .unwrap(),
                    "retained" | "removed" => {
                        app.metadata_action(metadata::Action::Retain).unwrap();
                        if scene == "retained" {
                            app.metadata_action(metadata::Action::ClearData).unwrap();
                        } else {
                            let pane = app.chart.core().series_pane(app.line).unwrap();
                            app.chart.remove_pane(pane).unwrap();
                        }
                        app.metadata_action(metadata::Action::InspectRetained)
                            .unwrap();
                        assert_eq!(app.metadata.as_ref().unwrap().retained.len(), 24);
                    }
                    "restored" => {
                        app.metadata_action(metadata::Action::Retain).unwrap();
                        app.metadata_action(metadata::Action::Latest).unwrap();
                        app.metadata_action(metadata::Action::Restore).unwrap();
                    }
                    "moved" => {
                        app.action(Message::Swap).unwrap();
                        app.action(Message::LineNew).unwrap();
                        app.metadata_action(metadata::Action::History).unwrap();
                    }
                    "recovered" => {
                        app.chart.prepare(Size::new(300., 10.), ratio, now).unwrap();
                        app.metadata_action(metadata::Action::Latest).unwrap();
                    }
                    _ => unreachable!(),
                }
                app.chart.prepare(app.size, ratio, now).unwrap();
                let pixels = screen(&app.chart, app.size, ratio, renderer);
                assert!(
                    pixels == screen(&app.chart, app.size, ratio, renderer),
                    "screen replay changed"
                );
                if scene == "initial" {
                    baseline = Some(pixels.clone());
                }
                if matches!(scene, "latest" | "history" | "cleared" | "restored") {
                    assert!(
                        baseline.as_ref().unwrap() == &pixels,
                        "metadata-only replacement changed pixels"
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
                app.chart
                    .prepare(Size::new(500., 400.), ratio, now)
                    .unwrap();
                drop(app);
                assert!(
                    weak.upgrade().is_none(),
                    "built-in snapshot retained application payload"
                );
                assert!(
                    original.rgba() == exporter.render(&shot, options).unwrap().rgba(),
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
    println!("APPLICATION_METADATA_CAPTURES {domain} {count}");
}
