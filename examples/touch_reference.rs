//! Native Canvas recognition fixtures for the upstream synchronous source oracle.
use feather_charts::{
    api::{Chart, ChartMessage, ChartState, TouchAction, TouchNavigationOptions},
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        horz_scale_behavior_time::types::UtcTimestamp,
        pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
    },
};
use iced::{
    Point, Rectangle, Size, mouse, touch,
    widget::canvas::{Event, Program},
};
use std::time::Instant;

fn send(chart: &Chart, state: &mut ChartState, event: touch::Event) -> Option<TouchAction> {
    chart
        .program(std::convert::identity)
        .update(
            state,
            &Event::Touch(event),
            Rectangle::with_size(Size::new(640., 480.)),
            mouse::Cursor::Unavailable,
        )
        .and_then(|action| action.into_inner().0)
        .and_then(|message| {
            if let ChartMessage::Touch(input) = message {
                Some(input.action)
            } else {
                None
            }
        })
}
fn main() {
    let mut chart = Chart::default();
    let series = chart
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(
            series,
            (0..120)
                .map(|index| {
                    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                        time: UtcTimestamp::new(index as f64 + 1.).into(),
                        value: 100. + (index as f64).sin(),
                        color: None,
                        custom_values: None,
                    }))
                })
                .collect(),
        )
        .unwrap();
    chart
        .prepare(Size::new(640., 480.), 1.25, Instant::now())
        .unwrap();
    let origin = chart.frame().layout.plot.origin;
    let point = |x, y| Point::new(x + origin.x as f32, y + origin.y as f32);
    let mut cases = Vec::new();
    for horizontal in [false, true] {
        for vertical in [false, true] {
            chart.set_touch_navigation_options(TouchNavigationOptions {
                horizontal_drag: horizontal,
                vertical_drag: vertical,
                ..Default::default()
            });
            for (dx, dy) in [
                (1., 1.),
                (4., 0.),
                (5., 0.),
                (4., 1.),
                (10., 4.),
                (10., 5.),
                (-10., -5.),
                (0., 5.),
                (2., 10.),
                (30., -5.),
            ] {
                let mut state = ChartState::default();
                let id = touch::Finger(1);
                send(
                    &chart,
                    &mut state,
                    touch::Event::FingerPressed {
                        id,
                        position: point(200., 150.),
                    },
                );
                let pan = matches!(
                    send(
                        &chart,
                        &mut state,
                        touch::Event::FingerMoved {
                            id,
                            position: point(200. + dx, 150. + dy)
                        }
                    ),
                    Some(TouchAction::Pan { .. })
                );
                cases.push(format!("{{\"kind\":\"direction\",\"horizontal\":{horizontal},\"vertical\":{vertical},\"dx\":{dx},\"dy\":{dy},\"pan\":{pan}}}"));
            }
        }
    }
    for first in [Point::new(100., 100.), Point::new(170.5, 190.25)] {
        for second in [Point::new(200., 100.), Point::new(300.5, 230.75)] {
            for delta in [
                Point::new(50., 0.),
                Point::new(-20., 30.),
                Point::new(0., -40.),
            ] {
                let mut state = ChartState::default();
                send(
                    &chart,
                    &mut state,
                    touch::Event::FingerPressed {
                        id: touch::Finger(1),
                        position: point(first.x, first.y),
                    },
                );
                send(
                    &chart,
                    &mut state,
                    touch::Event::FingerPressed {
                        id: touch::Finger(2),
                        position: point(second.x, second.y),
                    },
                );
                let Some(TouchAction::Pinch { anchor, zoom }) = send(
                    &chart,
                    &mut state,
                    touch::Event::FingerMoved {
                        id: touch::Finger(2),
                        position: point(second.x + delta.x, second.y + delta.y),
                    },
                ) else {
                    panic!("pinch fixture");
                };
                cases.push(format!("{{\"kind\":\"pinch\",\"first\":[{},{}],\"second\":[{},{}],\"delta\":[{},{}],\"anchor\":[{},{}],\"zoom\":{zoom}}}",first.x,first.y,second.x,second.y,delta.x,delta.y,anchor.x,anchor.y));
            }
        }
    }
    println!("[{}]", cases.join(","));
}
