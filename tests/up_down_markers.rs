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
