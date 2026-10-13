use super::*;
use iced::{
    mouse,
    widget::canvas::{Event, Program},
};

fn dispatch(
    app: &mut App,
    state: &mut feather_charts::api::ChartState,
    event: &Event,
    point: Point,
) {
    let plot = app.chart.frame().layout.plot;
    let message = app
        .chart
        .program(std::convert::identity)
        .update(
            state,
            event,
            Rectangle::new(Point::ORIGIN, app.size),
            mouse::Cursor::Available(Point::new(
                point.x + plot.origin.x as f32,
                point.y + plot.origin.y as f32,
            )),
        )
        .and_then(|a| a.into_inner().0);
    if let Some(message) = message {
        let _ = app.update(Message::Chart(message));
    }
}
fn press() -> Event {
    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
}
fn release() -> Event {
    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
}
fn movement() -> Event {
    Event::Mouse(mouse::Event::CursorMoved {
        position: Point::ORIGIN,
    })
}
fn start_point(app: &mut App) -> Point {
    let id = app.primitive_scene.as_ref().unwrap().region.unwrap();
    let c = app.chart.primitive_context(id).unwrap();
    Point::new(
        c.continuous_logical_to_coordinate(30.).unwrap() as f32,
        c.price_to_coordinate(114.).unwrap() as f32,
    )
}
pub(in super::super) fn setup_capture(app: &mut App, scenario: &str) {
    if scenario == "primitive-selected-narrow" {
        app.size = Size::new(400., 300.);
        app.chart.fit_content();
        app.prepare();
    }
    let mut state = Default::default();
    let original = app.primitive_scene.as_ref().unwrap().value();
    let mut p = start_point(app);
    dispatch(app, &mut state, &press(), p);
    if scenario == "primitive-selected" || scenario == "primitive-selected-narrow" {
        dispatch(app, &mut state, &release(), p);
    } else {
        if scenario == "primitive-resized" {
            dispatch(app, &mut state, &release(), p);
            let id = app.primitive_scene.as_ref().unwrap().region.unwrap();
            let value = app.primitive_scene.as_ref().unwrap().value();
            let c = app.chart.primitive_context(id).unwrap();
            p = Point::new(
                c.continuous_logical_to_coordinate(value.to).unwrap() as f32,
                c.price_to_coordinate(value.high).unwrap() as f32,
            );
            dispatch(app, &mut state, &press(), p);
        }
        dispatch(
            app,
            &mut state,
            &movement(),
            Point::new(p.x + 32., p.y + 18.),
        );
        if scenario == "primitive-cancelled" {
            dispatch(
                app,
                &mut state,
                &Event::Window(iced::window::Event::Unfocused),
                p,
            );
        }
    }
    assert!(app.primitive_scene.as_ref().unwrap().value().selected);
    let current = app.primitive_scene.as_ref().unwrap().value();
    match scenario {
        "primitive-moving" => assert!(current.from > original.from),
        "primitive-resized" => assert!(current.to > original.to && current.high < original.high),
        "primitive-cancelled" => assert_eq!(
            [current.from, current.to, current.low, current.high],
            [original.from, original.to, original.low, original.high]
        ),
        _ => {}
    }
}
pub(in super::super) fn verify_pixels(app: &App, rgba: &[u8], width: usize) {
    let value = app.primitive_scene.as_ref().unwrap().value();
    assert!(value.selected);
    let time = app.chart.model().time_scale();
    let x = |logical| {
        time.index_to_coordinate(TimePointIndex::new(0.)).value() + logical * time.bar_spacing()
    };
    let scale = app.chart.model().panes()[0].right_price_scale();
    let (y1, y2) = (
        scale.price_to_coordinate(value.low, 100.).value(),
        scale.price_to_coordinate(value.high, 100.).value(),
    );
    let rect = Rectangle::new(
        Point::new(x(value.from) as f32, y1.min(y2) as f32),
        Size::new((x(value.to) - x(value.from)) as f32, (y2 - y1).abs() as f32),
    );
    let plot = app.chart.frame().layout.plot;
    let height = rgba.len() / width / 4;
    let mut visible = 0;
    for (handle, p) in handles(rect) {
        if p.x < 5.
            || p.y < 5.
            || f64::from(p.x) >= plot.size.width - 5.
            || f64::from(p.y) >= plot.size.height - 5.
        {
            continue;
        }
        let hit = app
            .chart
            .primitive_hit(0, p)
            .expect("visible handle must be hittable");
        assert_eq!(hit.external_id, handle.name());
        let px = ((f64::from(p.x) + plot.origin.x) * f64::from(app.ratio)).round() as usize;
        let py = ((f64::from(p.y) + plot.origin.y) * f64::from(app.ratio)).round() as usize;
        assert!(px < width && py < height);
        let pixel = &rgba[(py * width + px) * 4..][..3];
        assert!(
            pixel.iter().all(|v| *v >= 250),
            "handle center must render white: {pixel:?}"
        );
        visible += 1;
    }
    assert!(
        visible >= 2,
        "capture must contain visible selection handles"
    );
}
pub(in super::super) fn profile() {
    for (rows, sources, live) in [
        (1200, 1, false),
        (20000, 1, false),
        (100000, 1, false),
        (20000, 1, true),
        (100000, 1, true),
        (1200, 16, true),
    ] {
        let mut app = App::new(SeriesType::Line);
        let data = || {
            (0..rows)
                .map(|i| {
                    sample_row(
                        SeriesType::Line,
                        Time::from(UtcTimestamp::new(1_700_000_000. + i as f64 * 60.)),
                        100. + (i as f64 * 0.1).sin() * 8.,
                    )
                })
                .collect()
        };
        app.chart.set_data(app.price, data()).unwrap();
        for _ in 1..sources {
            let id = app
                .chart
                .add_series(
                    sample_options(SeriesType::Line, true),
                    0,
                    PriceScalePosition::Right,
                )
                .unwrap();
            app.chart.set_data(id, data()).unwrap();
        }
        app.primitive_scene = Some(Scene::new(&mut app.chart, app.price));
        app.chart
            .set_logical_range(feather_charts::model::time_data::LogicalRange {
                from: 20.0.into(),
                to: 100.0.into(),
            });
        app.size = Size::new(800., 500.);
        app.ratio = 1.25;
        app.prepare();
        let mut state = Default::default();
        let p = start_point(&mut app);
        dispatch(&mut app, &mut state, &press(), p);
        let mut samples = vec![];
        for i in 0..120 {
            let now = Instant::now();
            if live {
                let time = Time::from(UtcTimestamp::new(1_700_000_000. + (rows - 1) as f64 * 60.));
                app.chart
                    .update_data(
                        app.price,
                        sample_row(SeriesType::Line, time, 100. + (i as f64 * 0.1).sin() * 8.),
                        false,
                    )
                    .unwrap();
            }
            let position = Point::new(
                p.x + 20. + (i as f32 * 0.15).sin() * 10.,
                p.y + 20. + (i as f32 * 0.12).cos() * 10.,
            );
            dispatch(&mut app, &mut state, &movement(), position);
            assert!(app.primitive_scene.as_ref().unwrap().edit.is_some());
            if i >= 20 {
                samples.push(now.elapsed().as_secs_f64() * 1000.);
            }
        }
        samples.sort_by(f64::total_cmp);
        println!(
            "primitive-edit rows={rows} sources={sources} live={live} median_ms={:.3} p95_ms={:.3}",
            samples[50], samples[95]
        );
        dispatch(&mut app, &mut state, &release(), p);
    }
}
