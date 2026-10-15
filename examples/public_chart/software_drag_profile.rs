//! Diagnostic only: compare unclipped and geometrically clipped rectangle raster work.
use super::*;
use iced::advanced::{
    graphics::geometry::Renderer as _,
    renderer::{Headless, Renderer as _},
};

pub(super) fn run() {
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some("tiny-skia"),
    ))
    .unwrap();
    println!("diagnostic_only=true size,variant,sample,geometry_ns,raster_ns");
    for size in [Size::new(800., 500.), Size::new(300., 500.)] {
        let mut app = App {
            size,
            ratio: 1.,
            ..App::new(SeriesType::Histogram)
        };
        for sample in 0..120 {
            if sample % 15 == 0 {
                let _ = app.update(Message::Append);
            }
            app.chart.prepare(size, 1., Instant::now()).unwrap();
            let original = app.chart.multi_frame().unwrap();
            let mut clipped = original.clone();
            for pane in &mut clipped.panes {
                let bounds = iced::Rectangle::with_size(pane.plot.size);
                let clip = |rect: &feather_charts::ui::line_chart::CanvasRectangle| {
                    let visible =
                        iced::Rectangle::new(rect.origin, rect.size).intersection(&bounds)?;
                    (visible.width > 0. && visible.height > 0.).then_some(
                        feather_charts::ui::line_chart::CanvasRectangle {
                            origin: visible.position(),
                            size: visible.size(),
                            color: rect.color,
                        },
                    )
                };
                pane.plot.candles = pane.plot.candles.iter().filter_map(clip).collect();
                for layer in &mut pane.plot.series_layers {
                    layer.candles = layer.candles.iter().filter_map(clip).collect();
                }
            }
            let mut pixels = None;
            // Alternate order; compare every pair, including warmups.
            let variants = if sample % 2 == 0 {
                [("original", original), ("clipped", &clipped)]
            } else {
                [("clipped", &clipped), ("original", original)]
            };
            for (variant, snapshot) in variants {
                renderer.reset(iced::Rectangle::with_size(size));
                let start = Instant::now();
                let mut canvas = iced::widget::canvas::Frame::new(&renderer, size);
                snapshot.draw(&mut canvas);
                renderer.draw_geometry(canvas.into_geometry());
                let geometry = start.elapsed();
                let start = Instant::now();
                let rgba = renderer.screenshot(
                    Size::new(size.width as u32, size.height as u32),
                    1.,
                    iced::Color::BLACK,
                );
                let raster = start.elapsed();
                if let Some(first) = &pixels {
                    assert!(first == &rgba, "clipping must preserve pixels");
                } else {
                    pixels = Some(rgba);
                }
                if sample >= 20 {
                    println!(
                        "{}x{},{variant},{sample},{},{}",
                        size.width,
                        size.height,
                        geometry.as_nanos(),
                        raster.as_nanos()
                    );
                }
            }
        }
    }
}

pub(super) fn check_cached(wgpu: bool) {
    use iced::widget::canvas::Program as _;
    let backend = if wgpu { "wgpu" } else { "tiny-skia" };
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some(backend),
    ))
    .unwrap();
    let mut pairs = 0;
    for kind in [
        SeriesType::Histogram,
        SeriesType::Line,
        SeriesType::Area,
        SeriesType::Baseline,
        SeriesType::Bar,
        SeriesType::Candlestick,
    ] {
        for size in [
            Size::new(941., 900.8),
            Size::new(420., 320.),
            Size::new(300., 180.),
            Size::new(300., 100.),
        ] {
            for ratio in [1., 1.25, 1.5, 2.] {
                let mut app = App {
                    size,
                    ratio,
                    ..App::new(kind)
                };
                let state = feather_charts::api::ChartState::default();
                for (step, message) in [
                    Message::Append,
                    Message::MovePrimitive,
                    Message::ToggleLast,
                    Message::Append,
                ]
                .into_iter()
                .enumerate()
                {
                    let _ = app.update(message);
                    app.chart.prepare(size, ratio, Instant::now()).unwrap();
                    let physical =
                        Size::new((size.width * ratio) as u32, (size.height * ratio) as u32);
                    renderer.reset(iced::Rectangle::with_size(size));
                    let mut raw = iced::widget::canvas::Frame::new(&renderer, size);
                    app.chart.multi_frame().unwrap().draw(&mut raw);
                    renderer.draw_geometry(raw.into_geometry());
                    let reference = renderer.screenshot(physical, ratio, iced::Color::BLACK);
                    renderer.reset(iced::Rectangle::with_size(size));
                    for geometry in app.chart.program(std::convert::identity).draw(
                        &state,
                        &renderer,
                        &iced::Theme::Dark,
                        iced::Rectangle::with_size(size),
                        iced::mouse::Cursor::Unavailable,
                    ) {
                        renderer.draw_geometry(geometry);
                    }
                    let cached = renderer.screenshot(physical, ratio, iced::Color::BLACK);
                    assert!(
                        reference == cached,
                        "cached geometry changed pixels: {kind:?} {size:?} ratio={ratio} step={step}"
                    );
                    pairs += 1;
                }
            }
        }
    }
    println!(
        "diagnostic_only=true backend={backend} cached_reference_pairs={pairs} all_identical=true"
    );
}
