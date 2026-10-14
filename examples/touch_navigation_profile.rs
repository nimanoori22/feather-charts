//! CPU timing of native Canvas inertia ticks, including public Chart preparation.
use feather_charts::{
    api::{Chart, ChartMessage, ChartState},
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        horz_scale_behavior_time::types::UtcTimestamp,
        pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
        time_data::LogicalRange,
    },
};
use iced::{
    Point, Rectangle, Size, mouse, touch,
    widget::canvas::{Event, Program},
};
use std::time::{Duration, Instant};
fn deliver(chart: &mut Chart, state: &mut ChartState, event: Event, now: Instant) {
    if let Some(mut message) = chart
        .program(std::convert::identity)
        .update(
            state,
            &event,
            Rectangle::with_size(Size::new(800., 500.)),
            mouse::Cursor::Unavailable,
        )
        .and_then(|a| a.into_inner().0)
    {
        if let ChartMessage::Touch(input) = &mut message {
            input.now = now;
        }
        chart.update(message).unwrap();
    }
}
fn flick(chart: &mut Chart, state: &mut ChartState, now: Instant) {
    chart.set_logical_range(LogicalRange {
        from: 20.0.into(),
        to: 100.0.into(),
    });
    chart.prepare(Size::new(800., 500.), 1.25, now).unwrap();
    // Settle the explicit-viewport epoch before starting a fresh finger gesture.
    deliver(
        chart,
        state,
        Event::Window(iced::window::Event::RedrawRequested(now)),
        now,
    );
    let origin = chart.frame().layout.plot.origin;
    let id = touch::Finger(1);
    let point = |x| Point::new(origin.x as f32 + x, origin.y as f32 + 150.);
    deliver(
        chart,
        state,
        Event::Touch(touch::Event::FingerPressed {
            id,
            position: point(200.),
        }),
        now,
    );
    for (ms, x) in [(10, 210.), (30, 240.), (50, 280.), (70, 320.)] {
        deliver(
            chart,
            state,
            Event::Touch(touch::Event::FingerMoved {
                id,
                position: point(x),
            }),
            now + Duration::from_millis(ms),
        );
    }
    deliver(
        chart,
        state,
        Event::Touch(touch::Event::FingerLifted {
            id,
            position: point(320.),
        }),
        now + Duration::from_millis(80),
    );
    assert!(chart.is_animating());
}
fn main() {
    for (count, sources) in [
        (1_200, 1),
        (20_000, 1),
        (100_000, 1),
        (1_200, 16),
        (100_000, 16),
    ] {
        let mut chart = Chart::default();
        for source in 0..sources {
            let id = chart
                .add_series(
                    SeriesOptionsMap::Line(Default::default()),
                    0,
                    PriceScalePosition::Right,
                )
                .unwrap();
            chart
                .set_data(
                    id,
                    (0..count)
                        .map(|index| {
                            BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                                time: UtcTimestamp::new(index as f64 + 1.).into(),
                                value: 100.
                                    + source as f64
                                    + if index + 1 == count {
                                        0.
                                    } else {
                                        (index as f64 * 0.05).sin()
                                    },
                                color: None,
                                custom_values: None,
                            }))
                        })
                        .collect(),
                )
                .unwrap();
        }
        let mut state = ChartState::default();
        let mut now = Instant::now();
        flick(&mut chart, &mut state, now);
        now += Duration::from_millis(80);
        let mut samples = Vec::new();
        for index in 0..120 {
            if !chart.is_animating() {
                flick(&mut chart, &mut state, now);
                now += Duration::from_millis(80);
            }
            now += Duration::from_millis(16);
            let started = Instant::now();
            deliver(
                &mut chart,
                &mut state,
                Event::Window(iced::window::Event::RedrawRequested(now)),
                now,
            );
            let elapsed = started.elapsed().as_secs_f64() * 1_000.;
            if index >= 20 {
                samples.push(elapsed);
            }
        }
        samples.sort_by(f64::total_cmp);
        println!(
            "rows={count} sources={sources} native_inertia_tick median_ms={:.6} p95_ms={:.6} warmups=20 samples=100",
            samples[50], samples[95]
        );
    }
}
