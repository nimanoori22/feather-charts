use feather_charts::{
    api::{Chart, MarkerSign, UpDownMarker, UpDownMarkersOptions},
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        horz_scale_behavior_time::types::UtcTimestamp,
        pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
    },
};
use iced::{
    Size,
    advanced::{
        graphics::geometry::Renderer as _,
        renderer::{Headless, Renderer as _},
    },
};
use std::time::Instant;
fn chart() -> Chart {
    let mut c = Chart::default();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(
        id,
        (0..32)
            .map(|i| {
                BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                    time: UtcTimestamp::new(1_700_000_000. + i as f64 * 60.).into(),
                    value: 100. + (i % 7) as f64,
                    color: None,
                    custom_values: None,
                }))
            })
            .collect(),
    )
    .unwrap();
    c.fit_content();
    c
}
fn prepare(c: &mut Chart) {
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
}
fn pixels(frame: &feather_charts::ui::chart_frame::IcedChartFrame) -> Vec<u8> {
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some("tiny-skia"),
    ))
    .unwrap();
    renderer.reset(iced::Rectangle::with_size(Size::new(800., 500.)));
    let mut canvas = iced::widget::canvas::Frame::new(&renderer, Size::new(800., 500.));
    frame.draw(&mut canvas);
    renderer.draw_geometry(canvas.into_geometry());
    renderer.screenshot(Size::new(800, 500), 1., iced::Color::BLACK)
}
#[test]
fn manual_markers_draw_visible_pixels() {
    let mut c = chart();
    prepare(&mut c);
    let before = pixels(c.frame());
    let series = c.series_ids().next().unwrap();
    let id = c
        .add_up_down_markers(series, UpDownMarkersOptions::default())
        .unwrap();
    c.set_up_down_markers(
        id,
        vec![UpDownMarker {
            time: UtcTimestamp::new(1_700_000_000. + 15. * 60.).into(),
            value: 103.,
            sign: MarkerSign::Positive,
        }],
    )
    .unwrap();
    prepare(&mut c);
    assert!(
        before != pixels(c.frame()),
        "attached up/down markers must draw visible output"
    );
}

fn row(
    i: usize,
    value: f64,
) -> BuiltInSeriesDataItem<feather_charts::model::horz_scale_behavior_time::types::Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: UtcTimestamp::new(1_700_000_000. + i as f64 * 60.).into(),
        value,
        color: None,
        custom_values: None,
    }))
}
#[test]
fn managed_updates_compare_to_the_seeded_baseline() {
    let mut c = chart();
    let series = c.series_ids().next().unwrap();
    let id = c.add_up_down_markers(series, Default::default()).unwrap();
    let now = Instant::now();
    c.set_up_down_data(id, vec![row(0, 10.), row(1, 20.)])
        .unwrap();
    c.update_up_down_data(id, row(0, 12.), true, now).unwrap();
    let markers = c.up_down_markers(id, now).unwrap();
    assert_eq!(markers.len(), 1, "managed update must create a marker");
    assert_eq!(markers[0].sign, MarkerSign::Positive);
    c.update_up_down_data(id, row(0, 11.), true, now).unwrap();
    assert_eq!(
        c.up_down_markers(id, now).unwrap()[0].sign,
        MarkerSign::Positive
    );
}

#[test]
fn automatic_markers_expire_at_the_exact_deadline() {
    use std::time::Duration;
    let mut c = chart();
    let series = c.series_ids().next().unwrap();
    let id = c.add_up_down_markers(series, Default::default()).unwrap();
    let now = Instant::now();
    c.set_up_down_data(id, vec![row(0, 10.)]).unwrap();
    c.update_up_down_data(id, row(0, 12.), false, now).unwrap();
    assert_eq!(
        c.next_up_down_expiration(),
        Some(now + Duration::from_secs(5))
    );
    assert_eq!(
        c.up_down_markers(id, now + Duration::from_millis(4999))
            .unwrap()
            .len(),
        1
    );
    assert!(
        c.up_down_markers(id, now + Duration::from_secs(5))
            .unwrap()
            .is_empty(),
        "due markers must expire without sleeps"
    );
}
#[test]
fn idle_widget_requests_one_deadline_then_publishes_tick_and_stops() {
    use feather_charts::api::{ChartMessage, ChartState};
    use iced::{
        Rectangle, mouse,
        widget::canvas::{Event, Program},
        window::{Event as WindowEvent, RedrawRequest},
    };
    use std::time::Duration;
    let mut c = chart();
    let series = c.series_ids().next().unwrap();
    let id = c.add_up_down_markers(series, Default::default()).unwrap();
    let now = Instant::now();
    c.set_up_down_data(id, vec![row(0, 10.), row(1, 20.)])
        .unwrap();
    c.update_up_down_data(id, row(0, 12.), true, now).unwrap();
    c.fit_content();
    c.prepare(Size::new(800., 500.), 1., now).unwrap();
    assert!(
        !c.is_animating(),
        "waiting for expiry must not enable frame polling"
    );
    let mut state = ChartState::default();
    let bounds = Rectangle::with_size(Size::new(800., 500.));
    let action = c
        .program(std::convert::identity)
        .update(
            &mut state,
            &Event::Window(WindowEvent::RedrawRequested(now)),
            bounds,
            mouse::Cursor::Unavailable,
        )
        .expect("idle marker needs a scheduled redraw")
        .into_inner();
    assert!(action.0.is_none());
    let due = now + Duration::from_secs(5);
    assert_eq!(action.1, RedrawRequest::At(due));
    let message = c
        .program(std::convert::identity)
        .update(
            &mut state,
            &Event::Window(WindowEvent::RedrawRequested(due)),
            bounds,
            mouse::Cursor::Unavailable,
        )
        .unwrap()
        .into_inner()
        .0;
    assert!(matches!(message,Some(ChartMessage::Tick(t)) if t==due));
    c.update(message.unwrap()).unwrap();
    assert!(c.up_down_markers(id, due).unwrap().is_empty());
    assert!(c.next_up_down_expiration().is_none());
    let after = c.program(std::convert::identity).update(
        &mut state,
        &Event::Window(WindowEvent::RedrawRequested(
            due + Duration::from_millis(16),
        )),
        bounds,
        mouse::Cursor::Unavailable,
    );
    if let Some(action) = after {
        let (message, request, _) = action.into_inner();
        assert_eq!(request, RedrawRequest::Wait);
        assert!(
            matches!(&message,Some(ChartMessage::Interaction(input)) if matches!(input.navigation,Some(feather_charts::ui::chart_input::Navigation::EndScroll))),
            "only existing navigation settlement may follow expiry"
        );
        c.update(message.unwrap()).unwrap();
    }
    assert!(
        c.program(std::convert::identity)
            .update(
                &mut state,
                &Event::Window(WindowEvent::RedrawRequested(
                    due + Duration::from_millis(32)
                )),
                bounds,
                mouse::Cursor::Unavailable
            )
            .is_none(),
        "expiry must settle without an idle redraw loop"
    );
}
#[derive(Debug)]
struct EmptyPrimitive;
impl feather_charts::api::DrawingPrimitive for EmptyPrimitive {
    fn prepare(
        &self,
        _: &feather_charts::api::PrimitiveContext<'_>,
    ) -> feather_charts::api::PreparedPrimitive {
        Default::default()
    }
}
#[test]
fn replacing_the_generic_attachment_invalidates_the_controller_id() {
    let mut c = chart();
    let series = c.series_ids().next().unwrap();
    let id = c.add_up_down_markers(series, Default::default()).unwrap();
    c.replace_primitive(id.primitive_id(), EmptyPrimitive)
        .unwrap();
    assert!(
        c.remove_up_down_markers(id).is_err(),
        "stale controller must not remove the replacement primitive"
    );
    assert!(c.primitive_owner(id.primitive_id()).is_ok());
}
#[test]
fn zero_negative_and_rejected_data_updates_preserve_the_native_contract() {
    use std::time::Duration;
    let mut c = chart();
    let series = c.series_ids().next().unwrap();
    let id = c.add_up_down_markers(series, Default::default()).unwrap();
    let now = Instant::now();
    c.set_up_down_data(id, vec![row(0, -10.), row(1, 0.), row(2, 10.)])
        .unwrap();
    for (i, value) in [(0, -11.), (1, 0.), (2, 12.)] {
        c.update_up_down_data(id, row(i, value), true, now).unwrap();
    }
    let before = c.up_down_markers(id, now).unwrap();
    assert_eq!(
        before.iter().map(|m| m.sign).collect::<Vec<_>>(),
        vec![
            MarkerSign::Negative,
            MarkerSign::Neutral,
            MarkerSign::Positive
        ]
    );
    let deadline = c.next_up_down_expiration();
    assert!(
        c.update_up_down_data(id, row(0, f64::NAN), true, now)
            .is_err()
    );
    assert!(c.update_up_down_data(id, row(9, 1.), true, now).is_err());
    assert!(
        c.set_up_down_data(id, vec![row(1, 999.), row(0, 999.)])
            .is_err()
    );
    assert_eq!(c.up_down_markers(id, now).unwrap(), before);
    assert_eq!(c.next_up_down_expiration(), deadline);
    c.replace_up_down_marker_options(
        id,
        UpDownMarkersOptions {
            update_visibility_duration: Duration::MAX,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(c.update_up_down_data(id, row(0, 1.), true, now).is_err());
    assert_eq!(c.up_down_markers(id, now).unwrap(), before);
    assert_eq!(c.next_up_down_expiration(), deadline);
    c.replace_up_down_marker_options(id, Default::default())
        .unwrap();
    c.update_up_down_data(id, row(1, 1.), true, now).unwrap();
    assert_eq!(
        c.up_down_markers(id, now).unwrap().last().unwrap().sign,
        MarkerSign::Positive
    );
}
#[test]
fn manual_and_existing_lifetimes_are_independent_of_new_options() {
    use std::time::Duration;
    let mut c = chart();
    let series = c.series_ids().next().unwrap();
    let id = c.add_up_down_markers(series, Default::default()).unwrap();
    let now = Instant::now();
    c.set_up_down_data(id, vec![row(0, 10.)]).unwrap();
    c.update_up_down_data(id, row(0, 11.), false, now).unwrap();
    c.replace_up_down_marker_options(
        id,
        UpDownMarkersOptions {
            update_visibility_duration: Duration::ZERO,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        c.next_up_down_expiration(),
        Some(now + Duration::from_secs(5))
    );
    c.update_up_down_data(id, row(0, 12.), false, now).unwrap();
    assert!(c.up_down_markers(id, now).unwrap().is_empty());
    c.set_up_down_markers(
        id,
        vec![UpDownMarker {
            time: UtcTimestamp::new(1_700_000_000.).into(),
            value: 10.,
            sign: MarkerSign::Neutral,
        }],
    )
    .unwrap();
    assert_eq!(
        c.up_down_markers(id, now + Duration::from_secs(100))
            .unwrap()
            .len(),
        1
    );
    assert!(c.next_up_down_expiration().is_none());
}
#[test]
fn color_replacement_updates_marker_pixels_and_old_frames_stay_frozen() {
    let mut c = chart();
    let series = c.series_ids().next().unwrap();
    let id = c.add_up_down_markers(series, Default::default()).unwrap();
    c.set_up_down_markers(
        id,
        vec![UpDownMarker {
            time: UtcTimestamp::new(1_700_000_000. + 15. * 60.).into(),
            value: 103.,
            sign: MarkerSign::Positive,
        }],
    )
    .unwrap();
    prepare(&mut c);
    let retained = c.frame().clone();
    let first = pixels(&retained);
    c.replace_up_down_marker_options(
        id,
        UpDownMarkersOptions {
            positive_color: "purple".into(),
            ..Default::default()
        },
    )
    .unwrap();
    prepare(&mut c);
    let changed = pixels(c.frame());
    assert!(changed != first);
    assert!(pixels(&retained) == first);
    assert!(
        c.replace_up_down_marker_options(
            id,
            UpDownMarkersOptions {
                negative_color: "invalid".into(),
                ..Default::default()
            }
        )
        .is_err()
    );
    prepare(&mut c);
    assert!(pixels(c.frame()) == changed);
    assert!(
        c.set_up_down_markers(
            id,
            vec![UpDownMarker {
                time: UtcTimestamp::new(f64::NAN).into(),
                value: 103.,
                sign: MarkerSign::Positive
            }]
        )
        .is_err()
    );
    assert_eq!(c.up_down_markers(id, Instant::now()).unwrap().len(), 1);
    c.remove_up_down_markers(id).unwrap();
    prepare(&mut c);
    assert!(pixels(c.frame()) != changed);
    assert!(pixels(&retained) == first);
}
#[test]
fn missing_times_hidden_series_and_unsupported_kinds_are_safe() {
    use feather_charts::model::series_options::{AreaSeriesOptions, LineSeriesOptions};
    let mut c = chart();
    let series = c.series_ids().next().unwrap();
    let id = c.add_up_down_markers(series, Default::default()).unwrap();
    prepare(&mut c);
    let baseline = pixels(c.frame());
    c.set_up_down_markers(
        id,
        vec![UpDownMarker {
            time: UtcTimestamp::new(1_600_000_000.).into(),
            value: 103.,
            sign: MarkerSign::Positive,
        }],
    )
    .unwrap();
    prepare(&mut c);
    assert!(baseline == pixels(c.frame()));
    let mut hidden = LineSeriesOptions::default();
    hidden.common.visible = false;
    c.replace_series_options(series, SeriesOptionsMap::Line(hidden))
        .unwrap();
    c.set_up_down_markers(
        id,
        vec![UpDownMarker {
            time: UtcTimestamp::new(1_700_000_000. + 15. * 60.).into(),
            value: 103.,
            sign: MarkerSign::Positive,
        }],
    )
    .unwrap();
    prepare(&mut c);
    let hidden_pixels = pixels(c.frame());
    c.clear_up_down_markers(id).unwrap();
    prepare(&mut c);
    assert!(pixels(c.frame()) == hidden_pixels);
    let wrong = c
        .add_series(
            SeriesOptionsMap::Candlestick(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    assert!(c.add_up_down_markers(wrong, Default::default()).is_err());
    let area = c
        .add_series(
            SeriesOptionsMap::Area(AreaSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let area_id = c.add_up_down_markers(area, Default::default()).unwrap();
    c.remove_up_down_markers(area_id).unwrap();
}
#[test]
fn pane_index_shifts_and_series_removal_cancel_all_controller_work() {
    let mut c = chart();
    let first = c.series_ids().next().unwrap();
    let pane = c.add_pane();
    let second = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            pane,
            PriceScalePosition::Right,
        )
        .unwrap();
    let now = Instant::now();
    let one = c.add_up_down_markers(first, Default::default()).unwrap();
    let two = c.add_up_down_markers(second, Default::default()).unwrap();
    for id in [one, two] {
        c.set_up_down_data(id, vec![row(0, 10.), row(1, 20.)])
            .unwrap();
        c.update_up_down_data(id, row(1, 21.), false, now).unwrap();
    }
    let owner = c.primitive_owner(two.primitive_id()).unwrap();
    c.remove_series(first).unwrap();
    prepare(&mut c);
    assert_eq!(c.primitive_owner(two.primitive_id()).unwrap(), owner);
    assert_eq!(c.up_down_markers(two, now).unwrap().len(), 1);
    assert!(c.up_down_markers(one, now).is_err());
    c.set_preserve_empty_pane(0, true).unwrap();
    c.remove_series(second).unwrap();
    assert!(c.up_down_markers(two, now).is_err());
    assert!(c.next_up_down_expiration().is_none());
}
#[test]
fn pulse_fast_path_keeps_marker_geometry_until_expiration_is_due() {
    use feather_charts::{
        api::ChartMessage,
        model::series_options::{LastPriceAnimationMode, LineSeriesOptions},
    };
    use std::{rc::Rc, time::Duration};
    let mut c = chart();
    let series = c.series_ids().next().unwrap();
    let mut opts = LineSeriesOptions::default();
    opts.style.last_price_animation = LastPriceAnimationMode::Continuous;
    c.replace_series_options(series, SeriesOptionsMap::Line(opts))
        .unwrap();
    let id = c.add_up_down_markers(series, Default::default()).unwrap();
    let now = Instant::now();
    c.set_up_down_data(id, vec![row(0, 10.), row(1, 20.)])
        .unwrap();
    c.update_up_down_data(id, row(1, 21.), false, now).unwrap();
    c.fit_content();
    c.prepare(Size::new(800., 500.), 1., now).unwrap();
    // Match the existing animation contract: settle initial axis work before measuring pulse-only ticks.
    c.update(ChartMessage::Tick(now)).unwrap();
    let before = c
        .frame()
        .plot
        .primitives
        .iter()
        .find(|a| a.id == id.primitive_id())
        .unwrap()
        .output
        .views[0]
        .renderer
        .clone();
    for i in 1..100 {
        c.update(ChartMessage::Tick(now + Duration::from_millis(i * 16)))
            .unwrap();
        let after = &c
            .frame()
            .plot
            .primitives
            .iter()
            .find(|a| a.id == id.primitive_id())
            .unwrap()
            .output
            .views[0]
            .renderer;
        assert!(Rc::ptr_eq(&before, after));
    }
    c.update(ChartMessage::Tick(now + Duration::from_secs(5)))
        .unwrap();
    assert!(
        c.up_down_markers(id, now + Duration::from_secs(5))
            .unwrap()
            .is_empty()
    );
    assert!(
        c.frame()
            .plot
            .primitives
            .iter()
            .find(|a| a.id == id.primitive_id())
            .unwrap()
            .output
            .views
            .is_empty()
    );
}

#[test]
fn historical_conflation_rejection_preserves_markers_and_deadline() {
    let mut c = chart();
    let series = c.series_ids().next().unwrap();
    let id = c.add_up_down_markers(series, Default::default()).unwrap();
    c.set_up_down_data(id, vec![row(0, 10.), row(1, 20.)])
        .unwrap();
    let now = Instant::now();
    c.update_up_down_data(id, row(1, 21.), false, now).unwrap();
    let before = c.up_down_markers(id, now).unwrap();
    let deadline = c.next_up_down_expiration();
    c.apply_time_scale_options(
        feather_charts::model::time_scale_options::HorzScaleOptionsPatch {
            enable_conflation: Some(true),
            min_bar_spacing: Some(0.001),
            bar_spacing: Some(0.125),
            ..Default::default()
        },
    );
    assert!(c.update_up_down_data(id, row(0, 11.), true, now).is_err());
    assert_eq!(c.up_down_markers(id, now).unwrap(), before);
    assert_eq!(c.next_up_down_expiration(), deadline);
}
