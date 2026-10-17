use feather_charts::{
    api::*,
    model::{
        data_consumer::*,
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
    },
};
use iced::{
    Point, Rectangle, Size, mouse,
    widget::canvas::{Event, Program},
};
use std::time::Instant;

fn setup() -> (Chart, feather_charts::model::data_layer::SeriesId) {
    let mut c = Chart::default();
    c.add_pane();
    c.set_preserve_empty_pane(0, true).unwrap();
    let mut first = None;
    for pane in 0..2 {
        let id = c
            .add_series(
                SeriesOptionsMap::Line(Default::default()),
                pane,
                PriceScalePosition::Right,
            )
            .unwrap();
        first.get_or_insert(id);
        c.set_data(
            id,
            (0..120)
                .map(|i| {
                    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                        time: UtcTimestamp::new(i as f64 + 1.).into(),
                        value: 100. + (i % 10) as f64,
                        color: None,
                        custom_values: None,
                    }))
                })
                .collect(),
        )
        .unwrap();
    }
    c.fit_content();
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    (c, first.unwrap())
}
fn input(c: &mut Chart, state: &mut ChartState, event: Event, point: Point) {
    let message = c
        .program(std::convert::identity)
        .update(
            state,
            &event,
            Rectangle::with_size(Size::new(800., 500.)),
            mouse::Cursor::Available(point),
        )
        .and_then(|a| a.into_inner().0);
    if let Some(m) = message {
        c.update(m).unwrap();
    }
}

#[test]
fn structural_operations_cancel_real_canvas_plot_axis_and_divider_captures() {
    for operation in 0..4 {
        for gesture in 0..3 {
            let (mut c, id) = setup();
            let mut state = ChartState::default();
            let layout = &c.multi_frame().unwrap().layout;
            let point = match gesture {
                0 => Point::new(200., 100.),
                1 => Point::new(layout.panes[0].right_axis.origin.x as f32 + 5., 100.),
                _ => Point::new(200., layout.separators[0].origin.y as f32 + 1.),
            };
            input(
                &mut c,
                &mut state,
                Event::Mouse(mouse::Event::CursorMoved { position: point }),
                point,
            );
            input(
                &mut c,
                &mut state,
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                point,
            );
            let moved = Point::new(point.x + 20., point.y + 20.);
            input(
                &mut c,
                &mut state,
                Event::Mouse(mouse::Event::CursorMoved { position: moved }),
                moved,
            );
            if gesture == 0 {
                assert!(c.is_panning());
            }
            if gesture == 1 {
                assert!(c.active_price_axis().is_some());
            }
            match operation {
                0 => c.move_pane(0, 1),
                1 => c.swap_panes(0, 1),
                2 => c.move_series_to_pane(id, 1),
                _ => c.remove_pane(0),
            }
            .unwrap();
            assert!(!c.is_panning());
            assert!(!c.active_price_axis().is_some());
            c.prepare(Size::new(800., 500.), 1., Instant::now())
                .unwrap();
            let offset = c.model().time_scale().right_offset();
            let stretches: Vec<_> = c
                .model()
                .panes()
                .iter()
                .map(|p| p.stretch_factor())
                .collect();
            let moved = Point::new(moved.x + 30., moved.y + 30.);
            input(
                &mut c,
                &mut state,
                Event::Mouse(mouse::Event::CursorMoved { position: moved }),
                moved,
            );
            input(
                &mut c,
                &mut state,
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                moved,
            );
            assert_eq!(offset, c.model().time_scale().right_offset());
            assert_eq!(
                stretches,
                c.model()
                    .panes()
                    .iter()
                    .map(|p| p.stretch_factor())
                    .collect::<Vec<_>>()
            );
            assert!(!c.is_panning());
            assert!(!c.active_price_axis().is_some());
        }
    }
}

#[test]
fn rejected_and_identity_operations_preserve_active_drag() {
    let (mut c, id) = setup();
    let mut state = ChartState::default();
    let p = Point::new(200., 100.);
    input(
        &mut c,
        &mut state,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        p,
    );
    let moved = Point::new(220., 120.);
    input(
        &mut c,
        &mut state,
        Event::Mouse(mouse::Event::CursorMoved { position: moved }),
        moved,
    );
    assert!(c.is_panning());
    assert!(c.move_pane(0, 99).is_err());
    assert!(c.swap_panes(99, 0).is_err());
    assert!(c.remove_pane(99).is_err());
    c.move_pane(0, 0).unwrap();
    c.swap_panes(1, 1).unwrap();
    c.move_series_to_pane(id, 0).unwrap();
    assert!(c.is_panning());
    let before = c.model().time_scale().right_offset();
    let moved = Point::new(250., 120.);
    input(
        &mut c,
        &mut state,
        Event::Mouse(mouse::Event::CursorMoved { position: moved }),
        moved,
    );
    assert_ne!(before, c.model().time_scale().right_offset());
}

fn sample(kind: usize, i: usize, value: f64) -> BuiltInSeriesDataItem<Time> {
    use feather_charts::model::data_consumer::*;
    let time = UtcTimestamp::new(1_700_000_000. + i as f64 * 60.).into();
    match kind {
        0 => BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time,
            value,
            color: None,
            custom_values: None,
        })),
        1 => BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(CandlestickData {
            time,
            open: value - 1.,
            high: value + 2.,
            low: value - 2.,
            close: value,
            color: None,
            border_color: None,
            wick_color: None,
            custom_values: None,
        })),
        2 => BuiltInSeriesDataItem::Bar(BarDataItem::Data(BarData {
            time,
            open: value - 1.,
            high: value + 2.,
            low: value - 2.,
            close: value,
            color: None,
            custom_values: None,
        })),
        3 => BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
            time,
            value,
            color: None,
            custom_values: None,
        })),
        4 => BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
            time,
            value,
            line_color: None,
            top_color: None,
            bottom_color: None,
            custom_values: None,
        })),
        5 => BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(BaselineData {
            time,
            value,
            top_fill_color1: None,
            top_fill_color2: None,
            top_line_color: None,
            bottom_fill_color1: None,
            bottom_fill_color2: None,
            bottom_line_color: None,
            custom_values: None,
        })),
        _ => unreachable!(),
    }
}
fn options(kind: usize) -> SeriesOptionsMap {
    match kind {
        0 => SeriesOptionsMap::Line(Default::default()),
        1 => SeriesOptionsMap::Candlestick(Default::default()),
        2 => SeriesOptionsMap::Bar(Default::default()),
        3 => SeriesOptionsMap::Histogram(Default::default()),
        4 => SeriesOptionsMap::Area(Default::default()),
        5 => SeriesOptionsMap::Baseline(Default::default()),
        _ => unreachable!(),
    }
}

#[test]
fn every_builtin_family_keeps_canonical_storage_and_prepares_on_its_new_scale() {
    for kind in 0..6 {
        for side in [
            PriceScalePosition::Left,
            PriceScalePosition::Right,
            PriceScalePosition::Overlay("study".into()),
        ] {
            let mut c = Chart::default();
            c.add_pane();
            c.set_preserve_empty_pane(0, true).unwrap();
            let id = c.add_series(options(kind), 0, side).unwrap();
            c.set_data(
                id,
                (0..1000)
                    .map(|i| sample(kind, i, 100. + (i % 10) as f64))
                    .collect(),
            )
            .unwrap();
            c.fit_content();
            c.prepare(Size::new(800., 500.), 1., Instant::now())
                .unwrap();
            let rows = c.model().series(id).unwrap().bars().rows().as_ptr();
            c.move_series_to_pane(id, 1).unwrap();
            assert_eq!(c.model().series(id).unwrap().bars().rows().as_ptr(), rows);
            c.prepare(Size::new(300., 90.), 1.5, Instant::now())
                .unwrap();
            c.prepare(Size::new(800., 500.), 1., Instant::now())
                .unwrap();
            c.update_data(id, sample(kind, 999, 104.), false).unwrap();
            c.update_data(id, sample(kind, 20, 103.), true).unwrap();
            c.remove_pane(1).unwrap();
            assert!(c.series_ids().next().is_none());
            assert!(c.model().time_scale().is_empty());
        }
    }
}

#[test]
fn transfer_keeps_update_marker_deadlines_and_idle_expiration() {
    use std::time::Duration;
    let (mut c, id) = setup();
    let markers = c
        .add_up_down_markers(
            id,
            UpDownMarkersOptions {
                update_visibility_duration: Duration::from_secs(5),
                ..Default::default()
            },
        )
        .unwrap();
    c.set_up_down_data(markers, vec![sample(0, 0, 100.)])
        .unwrap();
    let now = Instant::now();
    c.update_up_down_data(markers, sample(0, 0, 101.), false, now)
        .unwrap();
    let deadline = c.next_up_down_expiration().unwrap();
    c.move_series_to_pane(id, 1).unwrap();
    c.swap_panes(0, 1).unwrap();
    assert_eq!(c.next_up_down_expiration(), Some(deadline));
    assert_eq!(c.up_down_markers(markers, now).unwrap().len(), 1);
    c.update(ChartMessage::Tick(deadline)).unwrap();
    assert!(c.up_down_markers(markers, deadline).unwrap().is_empty());
}

#[test]
fn pane_changes_stop_touch_and_kinetic_state() {
    use std::time::Duration;
    for operation in 0..4 {
        let (mut c, id) = setup();
        let now = Instant::now();
        for (ms, start, x) in [
            (0, Some(Point::new(200., 100.)), 200.),
            (20, None, 240.),
            (40, None, 280.),
        ] {
            c.update(ChartMessage::Touch(TouchInteraction {
                pane: 0,
                now: now + Duration::from_millis(ms),
                action: TouchAction::Pan {
                    start,
                    position: Point::new(x, 100.),
                },
            }))
            .unwrap();
        }
        c.update(ChartMessage::Touch(TouchInteraction {
            pane: 0,
            now: now + Duration::from_millis(50),
            action: TouchAction::End,
        }))
        .unwrap();
        c.prepare(Size::new(800., 500.), 1., now + Duration::from_millis(50))
            .unwrap();
        assert!(c.is_animating());
        match operation {
            0 => c.move_pane(0, 1),
            1 => c.swap_panes(0, 1),
            2 => c.move_series_to_pane(id, 1),
            _ => c.remove_pane(0),
        }
        .unwrap();
        c.prepare(Size::new(800., 500.), 1., now + Duration::from_millis(100))
            .unwrap();
        assert!(!c.is_animating());
        let before = c.model().time_scale().right_offset();
        c.update(ChartMessage::Tick(now + Duration::from_secs(1)))
            .unwrap();
        assert_eq!(before, c.model().time_scale().right_offset());
    }
}
