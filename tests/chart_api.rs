use feather_charts::{
    api::{Chart, ChartError},
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{LineSeriesOptions, SeriesOptionsMap},
    },
};

fn row(time: f64, value: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: Time::from(UtcTimestamp::new(time)),
        value,
        color: None,
        custom_values: None,
    }))
}
fn add(chart: &mut Chart) -> feather_charts::model::data_layer::SeriesId {
    chart
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap()
}
#[test]
fn public_data_lifecycle_and_foreign_ids() {
    let mut chart = Chart::default();
    let id = add(&mut chart);
    chart.update_data(id, row(10., 100.), false).unwrap();
    chart.update_data(id, row(20., 120.), false).unwrap();
    chart.update_data(id, row(20., 125.), false).unwrap();
    assert_eq!(chart.model().series(id).unwrap().bars().rows().len(), 2);
    let mut other = Chart::default();
    let foreign = add(&mut other);
    assert!(matches!(
        chart.set_data(foreign, vec![row(1., 1.)]),
        Err(ChartError::UnknownSeries(_))
    ));
    chart.remove_series(id).unwrap();
    assert!(matches!(
        chart.update_data(id, row(30., 130.), false),
        Err(ChartError::UnknownSeries(_))
    ));
    assert!(chart.model().time_scale().is_empty());
}
#[test]
fn invalid_replacement_preserves_existing_data_and_future_updates() {
    let mut chart = Chart::default();
    let id = add(&mut chart);
    chart
        .set_data(id, vec![row(10., 100.), row(20., 120.)])
        .unwrap();
    for bad in [
        vec![row(20., 1.), row(10., 2.)],
        vec![row(10., 1.), row(10., 2.)],
        vec![row(30., f64::NAN)],
        vec![row(f64::NAN, 1.)],
    ] {
        assert!(chart.set_data(id, bad).is_err());
        assert_eq!(chart.model().series(id).unwrap().bars().rows().len(), 2);
    }
    assert!(chart.update_data(id, row(15., 115.), false).is_err());
    chart.update_data(id, row(10., 105.), true).unwrap();
    chart.update_data(id, row(30., 130.), false).unwrap();
    assert_eq!(chart.model().series(id).unwrap().bars().rows().len(), 3);
}
#[test]
fn composed_series_share_one_pane_and_empty_removal_leaves_no_geometry() {
    let mut chart = Chart::default();
    let first = add(&mut chart);
    let second = add(&mut chart);
    chart
        .set_data(first, vec![row(10., 100.), row(20., 120.)])
        .unwrap();
    chart
        .set_data(second, vec![row(10., 200.), row(20., 220.)])
        .unwrap();
    chart.fit_content();
    chart
        .prepare(iced::Size::new(800., 500.), 1.25, std::time::Instant::now())
        .unwrap();
    assert!(!chart.frame().plot.lines.is_empty());
    assert_eq!(chart.frame().plot.series_layers.len(), 1);
    assert!(!chart.frame().plot.series_layers[0].lines.is_empty());
    chart.remove_series(first).unwrap();
    chart.remove_series(second).unwrap();
    chart
        .prepare(iced::Size::new(800., 500.), 1.25, std::time::Instant::now())
        .unwrap();
    assert!(chart.frame().plot.lines.is_empty());
    assert!(chart.frame().plot.series_layers.is_empty());
}
#[test]
fn public_events_report_exact_rows_and_original_whitespace_time() {
    use feather_charts::{
        api::{ChartEvent, ChartMessage},
        model::data_consumer::WhitespaceData,
        ui::chart_input::ChartInteraction,
    };
    let mut chart = Chart::default();
    let first = add(&mut chart);
    let second = add(&mut chart);
    chart
        .set_data(
            first,
            vec![
                row(10., 100.),
                BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(WhitespaceData {
                    time: Time::from(UtcTimestamp::new(20.)),
                    custom_values: None,
                })),
                row(30., 130.),
            ],
        )
        .unwrap();
    chart
        .set_data(second, vec![row(10., 200.), row(30., 230.)])
        .unwrap();
    chart
        .prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    chart.fit_content();
    chart
        .prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    let point = iced::Point::new(
        chart
            .model()
            .time_scale()
            .index_to_coordinate(1.0.into())
            .value() as f32,
        100.,
    );
    let events = chart
        .update(ChartMessage::Click {
            pane: 0,
            position: point,
        })
        .unwrap();
    let click = events
        .iter()
        .find_map(|e| match e {
            ChartEvent::Click(p) => Some(p),
            _ => None,
        })
        .unwrap();
    assert_eq!(click.time, Some(Time::from(UtcTimestamp::new(20.))));
    assert!(
        click.series_data.is_empty(),
        "whitespace must not return a neighboring price row"
    );
    let point = iced::Point::new(
        chart
            .model()
            .time_scale()
            .index_to_coordinate(0.0.into())
            .value() as f32,
        100.,
    );
    let events = chart
        .update(ChartMessage::Interaction(ChartInteraction {
            position: Some(point),
            navigation: None,
            capture: false,
        }))
        .unwrap();
    let moved = events
        .iter()
        .find_map(|e| match e {
            ChartEvent::CrosshairMoved(p) => Some(p),
            _ => None,
        })
        .unwrap();
    assert_eq!(moved.series_data.len(), 2);
    assert!(moved.price.is_some());
    let events = chart
        .update(ChartMessage::Interaction(ChartInteraction {
            position: None,
            navigation: None,
            capture: false,
        }))
        .unwrap();
    let ChartEvent::CrosshairMoved(left) = &events[0] else {
        panic!("missing leave event")
    };
    assert!(left.point.is_none() && left.time.is_none() && left.series_data.is_empty());
}
#[test]
fn pane_lifecycle_and_mixed_geometry_follow_model_membership() {
    use feather_charts::model::{
        data_consumer::{HistogramData, HistogramDataItem},
        series_options::HistogramSeriesOptions,
    };
    let mut chart = Chart::default();
    let line = add(&mut chart);
    chart
        .set_data(line, vec![row(10., 100.), row(20., 120.)])
        .unwrap();
    let pane = chart.add_pane();
    chart.set_preserve_empty_pane(pane, true).unwrap();
    let volume = chart
        .add_series(
            SeriesOptionsMap::Histogram(HistogramSeriesOptions::default()),
            pane,
            PriceScalePosition::Right,
        )
        .unwrap();
    let volume_row = |time| {
        BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
            time: Time::from(UtcTimestamp::new(time)),
            value: 1000.,
            color: None,
            custom_values: None,
        }))
    };
    chart
        .set_data(volume, vec![volume_row(10.), volume_row(20.)])
        .unwrap();
    assert!(chart.set_data(line, vec![volume_row(30.)]).is_err());
    chart
        .prepare(iced::Size::new(800., 500.), 1.5, std::time::Instant::now())
        .unwrap();
    let frame = chart.multi_frame().unwrap();
    assert_eq!(
        frame.panes[0].plot.size.width,
        frame.panes[1].plot.size.width
    );
    assert!(!frame.panes[1].plot.candles.is_empty());
    chart.remove_series(volume).unwrap();
    chart
        .prepare(iced::Size::new(800., 500.), 1.5, std::time::Instant::now())
        .unwrap();
    assert_eq!(chart.model().panes().len(), 2);
    assert!(
        chart.multi_frame().unwrap().panes[1]
            .plot
            .candles
            .is_empty()
    );
    chart.set_preserve_empty_pane(pane, false).unwrap();
    let temporary = chart
        .add_series(
            SeriesOptionsMap::Histogram(HistogramSeriesOptions::default()),
            pane,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart.remove_series(temporary).unwrap();
    assert_eq!(chart.model().panes().len(), 1);
    chart
        .prepare(iced::Size::new(800., 500.), 1.5, std::time::Instant::now())
        .unwrap();
    assert!(!chart.frame().plot.lines.is_empty());
}
#[test]
fn canvas_emits_clicks_but_never_after_a_drag() {
    use feather_charts::api::{ChartMessage, ChartState};
    use iced::{
        Point, Rectangle, mouse,
        widget::canvas::{Event, Program},
    };
    let mut chart = Chart::default();
    let id = add(&mut chart);
    chart
        .set_data(id, vec![row(10., 100.), row(20., 120.)])
        .unwrap();
    let size = iced::Size::new(800., 500.);
    chart.prepare(size, 1., std::time::Instant::now()).unwrap();
    let mut state = ChartState::default();
    let mut run = |event, point| {
        let message = chart
            .program(std::convert::identity)
            .update(
                &mut state,
                &event,
                Rectangle::with_size(size),
                mouse::Cursor::Available(point),
            )
            .and_then(|a| a.into_inner().0);
        let clicked = matches!(message, Some(ChartMessage::Click { .. }));
        if let Some(message) = message {
            chart.update(message).unwrap();
        }
        clicked
    };
    assert!(!run(
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Point::new(200., 100.)
    ));
    assert!(run(
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        Point::new(200., 100.)
    ));
    assert!(!run(
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Point::new(200., 100.)
    ));
    assert!(!run(
        Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(220., 120.)
        }),
        Point::new(220., 120.)
    ));
    assert!(!run(
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        Point::new(220., 120.)
    ));
}
#[test]
fn malformed_public_messages_do_not_partially_resize_or_pan() {
    use feather_charts::{
        api::ChartMessage,
        ui::chart_input::{ChartInteraction, MultiPaneInteraction, Navigation},
    };
    let mut chart = Chart::default();
    let id = add(&mut chart);
    chart
        .set_data(id, vec![row(10., 100.), row(20., 120.)])
        .unwrap();
    chart.add_pane();
    chart
        .prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    let stretch = chart.model().panes()[0].stretch_factor();
    let offset = chart.model().time_scale().right_offset();
    for message in [
        ChartMessage::PaneInteraction(MultiPaneInteraction::Resize {
            top: 0,
            upper_stretch: 2.,
            lower_stretch: f64::NAN,
        }),
        ChartMessage::Interaction(ChartInteraction {
            position: None,
            navigation: Some(Navigation::Pan {
                start: Some(iced::Point::new(20., 20.)),
                position: iced::Point::new(f32::NAN, 30.),
            }),
            capture: true,
        }),
        ChartMessage::Click {
            pane: 20,
            position: iced::Point::new(20., 20.),
        },
        ChartMessage::Bounds(iced::Size::new(f32::NAN, 500.)),
    ] {
        assert!(matches!(
            chart.update(message),
            Err(ChartError::InvalidInteraction)
        ));
        assert_eq!(chart.model().panes()[0].stretch_factor(), stretch);
        assert_eq!(chart.model().time_scale().right_offset(), offset);
        assert!(!chart.is_panning());
    }
}
