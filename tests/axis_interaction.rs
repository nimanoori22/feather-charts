use feather_charts::{
    api::*,
    model::{
        data_consumer::*, horz_scale_behavior_time::types::UtcTimestamp, pane::PriceScalePosition,
        series_options::SeriesOptionsMap, time_scale_options::HorzScaleOptionsPatch,
    },
};
use iced::{
    Point, Rectangle, Size, mouse,
    widget::canvas::{Event, Program},
};
use std::time::Instant;

fn chart() -> Chart {
    let mut chart = Chart::default();
    chart
        .apply_chart_options(ChartOptionsPatch {
            model: ChartModelOptionsPatch {
                time_scale: Some(HorzScaleOptionsPatch {
                    bar_spacing: Some(9.),
                    right_offset: Some(3.),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
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
            (1..=250)
                .map(|i| {
                    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                        time: UtcTimestamp::new(i as f64).into(),
                        value: 100. + (i % 10) as f64,
                        color: None,
                        custom_values: None,
                    }))
                })
                .collect(),
        )
        .unwrap();
    prepare(&mut chart);
    chart
}
fn prepare(c: &mut Chart) {
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
}
fn input(c: &mut Chart, state: &mut ChartState, event: Event, point: Option<Point>) {
    let message = c
        .program(std::convert::identity)
        .update(
            state,
            &event,
            Rectangle::with_size(Size::new(800., 500.)),
            point.map_or(mouse::Cursor::Unavailable, mouse::Cursor::Available),
        )
        .and_then(|a| a.into_inner().0);
    if let Some(m) = message {
        c.update(m).unwrap();
    }
}
fn press(c: &mut Chart, s: &mut ChartState, p: Point) {
    input(
        c,
        s,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Some(p),
    );
}
fn release(c: &mut Chart, s: &mut ChartState, p: Option<Point>) {
    input(
        c,
        s,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        p,
    );
}
fn movement(c: &mut Chart, s: &mut ChartState, p: Point) {
    input(
        c,
        s,
        Event::Mouse(mouse::Event::CursorMoved { position: p }),
        Some(p),
    );
}
fn axis(c: &Chart, fraction: f64) -> Point {
    let r = c.frame().layout.time_axis;
    Point::new(
        (r.origin.x + r.size.width * fraction) as f32,
        (r.origin.y + r.size.height * 0.5) as f32,
    )
}
#[test]
fn canvas_horizontal_axis_drag_scales_about_right_edge_and_releases_outside() {
    let mut c = chart();
    let mut s = ChartState::default();
    let start = axis(&c, 0.6);
    let end = axis(&c, 0.4);
    let before = c.model().time_scale().bar_spacing();
    let offset = c.model().time_scale().right_offset();
    press(&mut c, &mut s, start);
    movement(&mut c, &mut s, end);
    let width = c.model().time_scale().width();
    let origin = c.frame().layout.time_axis.origin.x;
    let expected =
        before * (width - (f64::from(end.x) - origin)) / (width - (f64::from(start.x) - origin));
    assert!(
        (c.model().time_scale().bar_spacing() - expected).abs() < 1e-8,
        "axis drag did not change spacing: {} expected {expected}",
        c.model().time_scale().bar_spacing()
    );
    assert_eq!(c.model().time_scale().right_offset(), offset);
    release(&mut c, &mut s, None);
    let spacing = c.model().time_scale().bar_spacing();
    movement(&mut c, &mut s, start);
    assert_eq!(c.model().time_scale().bar_spacing(), spacing);
}
#[test]
fn canvas_axis_double_click_restores_configured_spacing_and_offset() {
    let mut c = chart();
    let mut s = ChartState::default();
    c.set_logical_range(feather_charts::model::time_data::LogicalRange {
        from: 200.0.into(),
        to: 240.0.into(),
    });
    c.set_right_offset(8.);
    prepare(&mut c);
    let p = axis(&c, 0.6);
    for _ in 0..2 {
        press(&mut c, &mut s, p);
        release(&mut c, &mut s, Some(p));
    }
    prepare(&mut c);
    assert_eq!(c.model().time_scale().bar_spacing(), 9.);
    assert_eq!(c.model().time_scale().right_offset(), 3.);
}

#[test]
fn stale_axis_captures_end_on_focus_resize_dpr_and_pane_changes() {
    for reason in 0..7 {
        let mut c = chart();
        let mut s = ChartState::default();
        c.add_pane();
        c.set_preserve_empty_pane(1, true).unwrap();
        prepare(&mut c);
        let start = axis(&c, 0.6);
        let moved = axis(&c, 0.4);
        press(&mut c, &mut s, start);
        movement(&mut c, &mut s, moved);
        assert!(s.is_axis_dragging());
        match reason {
            0 => input(
                &mut c,
                &mut s,
                Event::Window(iced::window::Event::Unfocused),
                None,
            ),
            1 => input(&mut c, &mut s, Event::Mouse(mouse::Event::CursorLeft), None),
            2 => {
                c.prepare(Size::new(600., 300.), 1., Instant::now())
                    .unwrap();
            }
            3 => {
                c.prepare(Size::new(800., 500.), 1.5, Instant::now())
                    .unwrap();
            }
            4 => c.swap_panes(0, 1).unwrap(),
            5 => c.remove_pane(1).unwrap(),
            _ => c.cancel_interaction(),
        }
        let before = c.model().time_scale().bar_spacing();
        movement(&mut c, &mut s, start);
        release(&mut c, &mut s, None);
        assert_eq!(
            before,
            c.model().time_scale().bar_spacing(),
            "reason {reason}"
        );
        assert!(!s.is_axis_dragging(), "reason {reason}");
    }
}

#[test]
fn empty_hidden_and_side_stubs_do_not_scale_or_capture_the_plot() {
    for reason in 0..3 {
        let mut c = chart();
        let mut s = ChartState::default();
        let p = axis(&c, 0.6);
        if reason == 0 {
            let id = c.series_ids().next().unwrap();
            c.set_data(id, vec![]).unwrap();
        } else if reason == 1 {
            c.apply_chart_options(ChartOptionsPatch {
                model: ChartModelOptionsPatch {
                    time_scale: Some(HorzScaleOptionsPatch {
                        visible: Some(false),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();
        }
        prepare(&mut c);
        let p = if reason == 2 {
            Point::new(799., p.y)
        } else {
            p
        };
        let before = c.model().time_scale().bar_spacing();
        press(&mut c, &mut s, p);
        movement(&mut c, &mut s, Point::new(p.x - 100., p.y));
        release(&mut c, &mut s, None);
        assert_eq!(before, c.model().time_scale().bar_spacing());
        assert!(!s.is_axis_dragging());
    }
}

#[test]
fn reset_retains_pixel_offset_policy_and_runtime_defaults() {
    let mut c = chart();
    let mut s = ChartState::default();
    c.apply_chart_options(ChartOptionsPatch {
        model: ChartModelOptionsPatch {
            time_scale: Some(HorzScaleOptionsPatch {
                bar_spacing: Some(12.),
                right_offset_pixels: Some(Some(60.)),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
    c.set_logical_range(feather_charts::model::time_data::LogicalRange {
        from: 200.0.into(),
        to: 240.0.into(),
    });
    prepare(&mut c);
    let p = axis(&c, 0.6);
    for _ in 0..2 {
        press(&mut c, &mut s, p);
        release(&mut c, &mut s, Some(p));
    }
    prepare(&mut c);
    assert_eq!(c.model().time_scale().bar_spacing(), 12.);
    assert_eq!(c.model().time_scale().right_offset(), 5.);
    assert_eq!(
        c.runtime_options().model.time_scale.right_offset_pixels,
        Some(60.)
    );
}
