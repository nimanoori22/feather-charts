use super::*;
use crate::model::{
    data_consumer::{LineData, LineDataItem},
    horz_scale_behavior_time::types::UtcTimestamp,
    pane::PriceScalePosition,
    series_options::SeriesOptionsMap,
};
use std::time::Duration;

#[test]
fn mouse_kinetic_navigation_uses_supplied_time_and_is_opt_in() {
    for enabled in [false, true] {
        let mut chart = Chart::default();
        let now = Instant::now();
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
                (0..120)
                    .map(|i| {
                        BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                            time: UtcTimestamp::new(i as f64 + 1.).into(),
                            value: 100. + (i as f64).sin(),
                            color: None,
                            custom_values: None,
                        }))
                    })
                    .collect(),
            )
            .unwrap();
        chart
            .prepare(iced::Size::new(640., 480.), 1.25, now)
            .unwrap();
        chart.set_kinetic_scroll_options(super::super::KineticScrollOptions {
            mouse: enabled,
            touch: true,
        });
        for (ms, start, x) in [
            (0, Some(iced::Point::new(200., 150.)), 200.),
            (20, None, 240.),
            (40, None, 280.),
        ] {
            chart
                .apply_interaction(
                    0,
                    ChartInteraction {
                        position: Some(iced::Point::new(x, 150.)),
                        navigation: Some(Navigation::Pan {
                            start,
                            position: iced::Point::new(x, 150.),
                        }),
                        capture: true,
                    },
                    now + Duration::from_millis(ms),
                )
                .unwrap();
        }
        chart
            .apply_interaction(
                0,
                ChartInteraction {
                    position: None,
                    navigation: Some(Navigation::EndScroll),
                    capture: true,
                },
                now + Duration::from_millis(50),
            )
            .unwrap();
        chart
            .prepare(
                iced::Size::new(640., 480.),
                1.25,
                now + Duration::from_millis(50),
            )
            .unwrap();
        assert_eq!(chart.is_animating(), enabled);
    }
}
