use feather_charts::{
    api::*,
    model::{
        data_consumer::*,
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
    },
};
use std::time::Instant;
fn row(time: f64, value: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: UtcTimestamp::new(time).into(),
        value,
        color: None,
        custom_values: None,
    }))
}
#[test]
fn capture_requires_successful_display_and_preserves_queued_events() {
    let mut c = Chart::default();
    assert!(matches!(
        c.screenshot_of_prepared(Default::default()),
        Err(ScreenshotError::NoPreparedFrame)
    ));
    c.set_event_interests(EventInterests {
        data_changed: true,
        horizontal_axis_size: true,
        ..Default::default()
    });
    let id = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(id, vec![row(1., 10.), row(2., 20.)]).unwrap();
    c.fit_content();
    let now = Instant::now();
    c.prepare(iced::Size::new(300., 200.), 1., now).unwrap();
    let shot = c.screenshot_of_prepared(Default::default()).unwrap();
    assert_eq!(shot.logical_size(), iced::Size::new(300., 200.));
    c.update_data(id, row(3., 30.), false).unwrap();
    c.prepare_screenshot(Default::default(), now).unwrap();
    let events = c.take_events();
    assert!(events.iter().any(|e| matches!(
        e,
        ChartEvent::DataChanged {
            scope: DataChangeScope::Update,
            ..
        }
    )));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ChartEvent::HorizontalAxisSizeChanged { .. }))
    );
    assert!(c.prepare(iced::Size::new(f32::NAN, 200.), 1., now).is_err());
    assert!(matches!(
        c.screenshot_of_prepared(Default::default()),
        Err(ScreenshotError::NoPreparedFrame)
    ));
    drop(c);
    assert_eq!(shot.logical_size(), iced::Size::new(300., 200.));
}

#[derive(Debug)]
struct Top;
impl PrimitiveRenderer for Top {
    fn draw(&self, frame: &mut iced::widget::canvas::Frame) {
        frame.fill_rectangle(
            iced::Point::new(40., 40.),
            iced::Size::new(20., 20.),
            iced::Color::from_rgb(1., 0., 1.),
        );
    }
}
impl DrawingPrimitive for Top {
    fn prepare(&self, _: &PrimitiveContext<'_>) -> PreparedPrimitive {
        PreparedPrimitive {
            views: vec![PrimitiveView::new(PrimitiveLayer::Top, Top)],
            ..Default::default()
        }
    }
}
#[test]
fn real_software_layers_freshness_fit_alpha_and_retained_owner() {
    let mut exporter =
        iced::futures::executor::block_on(ChartExporter::new(ScreenshotBackend::Software)).unwrap();
    assert_eq!(exporter.backend(), ScreenshotBackend::Software);
    let mut c = Chart::default();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(id, vec![row(1., 10.), row(2., 20.), row(3., 15.)])
        .unwrap();
    c.add_series_primitive(id, Top).unwrap();
    c.fit_content();
    let now = Instant::now();
    c.prepare(iced::Size::new(300., 200.), 1., now).unwrap();
    use feather_charts::ui::chart_input::ChartInteraction;
    c.update(ChartMessage::Interaction(ChartInteraction {
        position: Some(iced::Point::new(100., 80.)),
        navigation: None,
        capture: false,
    }))
    .unwrap();
    c.prepare(iced::Size::new(300., 200.), 1., now).unwrap();
    let shots: Vec<_> = [(false, false), (false, true), (true, false), (true, true)]
        .into_iter()
        .map(|(top, cross)| {
            c.screenshot_of_prepared(ScreenshotLayers {
                include_top_layer: top,
                include_crosshair: cross,
            })
            .unwrap()
        })
        .collect();
    let images: Vec<_> = shots
        .iter()
        .map(|s| exporter.render(s, Default::default()).unwrap())
        .collect();
    assert_eq!(images[0].rgba(), images[1].rgba());
    assert_ne!(images[0].rgba(), images[2].rgba());
    assert_ne!(images[2].rgba(), images[3].rgba());
    println!("SCREENSHOT_REFERENCE [true,true,true]");
    c.update_data(id, row(2., 5.), true).unwrap();
    let stale = c.screenshot_of_prepared(Default::default()).unwrap();
    assert_eq!(
        exporter.render(&stale, Default::default()).unwrap().rgba(),
        images[0].rgba()
    );
    let fresh = c.prepare_screenshot(Default::default(), now).unwrap();
    assert_ne!(
        exporter.render(&fresh, Default::default()).unwrap().rgba(),
        images[0].rgba()
    );
    c.remove_series(id).unwrap();
    drop(c);
    assert_eq!(
        exporter
            .render(&shots[3], Default::default())
            .unwrap()
            .rgba(),
        images[3].rgba()
    );
    let fit = exporter
        .render(
            &shots[0],
            ScreenshotRasterOptions {
                output_size: Some(iced::Size::new(100, 100)),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(fit.size(), iced::Size::new(100, 100));
    assert_eq!(&fit.rgba()[..4], &[0, 0, 0, 0]);
    let alpha = exporter
        .render(
            &shots[0],
            ScreenshotRasterOptions {
                background: iced::Color::from_rgba(1., 0., 0., 0.5),
                output_size: Some(iced::Size::new(100, 100)),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(&alpha.rgba()[..4], &[255, 0, 0, 128]);
}

#[test]
fn all_frontends_capture_and_snapshot_only_preparation_keeps_display() {
    let now = Instant::now();
    let mut calendar = Chart::default();
    calendar
        .prepare(iced::Size::new(300., 200.), 1., now)
        .unwrap();
    calendar
        .prepare_snapshot(iced::Size::new(800., 500.), 2., now)
        .unwrap();
    assert_eq!(
        calendar
            .screenshot_of_prepared(Default::default())
            .unwrap()
            .logical_size(),
        iced::Size::new(300., 200.)
    );
    let mut strike = OptionsChart::new_options(Default::default()).unwrap();
    strike
        .prepare(iced::Size::new(300., 200.), 1., now)
        .unwrap();
    assert_eq!(
        strike
            .prepare_screenshot(Default::default(), now)
            .unwrap()
            .logical_size(),
        iced::Size::new(300., 200.)
    );
    let mut maturity = YieldCurveChart::new(Default::default()).unwrap();
    maturity
        .prepare(iced::Size::new(300., 200.), 1., now)
        .unwrap();
    assert_eq!(
        maturity
            .prepare_screenshot(Default::default(), now)
            .unwrap()
            .logical_size(),
        iced::Size::new(300., 200.)
    );
}

#[test]
fn fractional_ratio_uses_the_requested_scale_without_recentering() {
    use iced::advanced::{
        graphics::geometry::Renderer as _,
        renderer::{Headless, Renderer as _},
    };
    let now = Instant::now();
    let size = iced::Size::new(300., 90.);
    let ratio = 1.25;
    let mut c = Chart::default();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(id, vec![row(1., 10.), row(2., 20.), row(3., 15.)])
        .unwrap();
    c.fit_content();
    c.prepare(size, ratio, now).unwrap();
    let layers = ScreenshotLayers {
        include_top_layer: true,
        include_crosshair: true,
    };
    let shot = c.screenshot_of_prepared(layers).unwrap();
    let mut exporter =
        iced::futures::executor::block_on(ChartExporter::new(ScreenshotBackend::Software)).unwrap();
    let exported = exporter
        .render(
            &shot,
            ScreenshotRasterOptions {
                pixel_ratio: ratio,
                ..Default::default()
            },
        )
        .unwrap();
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some("tiny-skia"),
    ))
    .unwrap();
    renderer.reset(iced::Rectangle::with_size(size));
    let mut frame = iced::widget::canvas::Frame::new(&renderer, size);
    c.frame().draw(&mut frame);
    renderer.draw_geometry(frame.into_geometry());
    let mut expected =
        renderer.screenshot(iced::Size::new(375, 113), ratio, iced::Color::TRANSPARENT);
    for p in expected.as_chunks_mut::<4>().0 {
        let alpha = u32::from(p[3]);
        if alpha == 0 {
            p.fill(0);
        } else if alpha < 255 {
            for channel in &mut p[..3] {
                *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }
    assert!(
        exported.rgba() == expected,
        "fractional DPR shifted the captured layout"
    );
}
