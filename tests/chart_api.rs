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
fn incremental_whitespace_transitions_keep_other_rows_and_shared_indexes() {
    use feather_charts::model::time_scale_host::TimeScaleHost;
    use feather_charts::model::{data_consumer::WhitespaceData, plot_data::MutablePlotRow};
    let mut chart = Chart::default();
    let id = add(&mut chart);
    let other = add(&mut chart);
    chart
        .set_data(id, vec![row(10., 100.), row(20., 120.), row(30., 130.)])
        .unwrap();
    chart
        .set_data(other, vec![row(10., 200.), row(20., 220.), row(30., 230.)])
        .unwrap();
    chart
        .update_data(
            id,
            BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(WhitespaceData {
                time: Time::from(UtcTimestamp::new(20.)),
                custom_values: None,
            })),
            true,
        )
        .unwrap();
    let indices = |chart: &Chart, id| {
        chart
            .model()
            .series(id)
            .unwrap()
            .bars()
            .rows()
            .iter()
            .map(MutablePlotRow::index)
            .collect::<Vec<_>>()
    };
    assert_eq!(indices(&chart, id), vec![0.0.into(), 2.0.into()]);
    assert_eq!(
        indices(&chart, other),
        vec![0.0.into(), 1.0.into(), 2.0.into()]
    );
    assert_eq!(chart.model().fulfilled_time_indices().len(), 3);
    chart.update_data(id, row(20., 125.), true).unwrap();
    assert_eq!(
        indices(&chart, id),
        vec![0.0.into(), 1.0.into(), 2.0.into()]
    );
    assert_eq!(
        chart.model().series(id).unwrap().bars().rows()[0]
            .values()
            .unwrap()[3],
        100.
    );
    assert_eq!(
        chart.model().series(id).unwrap().bars().rows()[2]
            .values()
            .unwrap()[3],
        130.
    );
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

#[test]
fn native_wheel_detent_changes_time_spacing_by_ten_percent() {
    use feather_charts::api::ChartState;
    use iced::{
        Point, Rectangle, mouse,
        widget::canvas::{Event, Program},
    };
    let mut chart = Chart::default();
    let id = add(&mut chart);
    chart
        .set_data(id, (0..100).map(|i| row(i as f64, 100.)).collect())
        .unwrap();
    let size = iced::Size::new(800., 500.);
    chart.prepare(size, 1., std::time::Instant::now()).unwrap();
    let mut state = ChartState::default();
    for (delta, factor) in [
        (mouse::ScrollDelta::Lines { x: 0., y: 1. }, 1.1),
        (mouse::ScrollDelta::Lines { x: 0., y: -1. }, 0.9),
        (mouse::ScrollDelta::Pixels { x: 0., y: 5. }, 1.005),
    ] {
        let before = chart.model().time_scale().bar_spacing();
        let message = chart
            .program(std::convert::identity)
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::WheelScrolled { delta }),
                Rectangle::with_size(size),
                mouse::Cursor::Available(Point::new(200., 100.)),
            )
            .and_then(|a| a.into_inner().0)
            .unwrap();
        chart.update(message).unwrap();
        assert!((chart.model().time_scale().bar_spacing() / before - factor).abs() < 1e-9);
    }
}

#[test]
fn price_line_lifecycle_validation_and_series_cleanup() {
    use feather_charts::api::PriceLineOptions;
    let mut chart = Chart::default();
    let series = add(&mut chart);
    let line = chart
        .add_price_line(series, PriceLineOptions::new(100.))
        .unwrap();
    let mut options = chart.price_line_options(line).unwrap();
    options.price = f64::NAN;
    assert!(chart.replace_price_line_options(line, options).is_err());
    assert_eq!(chart.price_line_options(line).unwrap().price, 100.);
    let mut other = Chart::default();
    assert!(other.remove_price_line(line).is_err());
    chart.set_data(series, vec![]).unwrap();
    assert_eq!(chart.price_line_options(line).unwrap().price, 100.);
    chart.remove_series(series).unwrap();
    assert!(chart.price_line_options(line).is_err());
    assert!(chart.remove_price_line(line).is_err());
}

#[test]
fn price_lines_and_last_labels_use_different_source_selection() {
    use feather_charts::api::PriceLineOptions;
    use feather_charts::model::time_data::LogicalRange;
    let mut chart = Chart::default();
    let series = add(&mut chart);
    chart
        .set_data(
            series,
            (0..20).map(|i| row(i as f64, 100. + i as f64)).collect(),
        )
        .unwrap();
    chart
        .prepare(iced::Size::new(800., 500.), 1.25, std::time::Instant::now())
        .unwrap();
    chart.set_logical_range(LogicalRange {
        from: 0.0.into(),
        to: 9.0.into(),
    });
    let line = chart
        .add_price_line(series, PriceLineOptions::new(110.))
        .unwrap();
    chart
        .prepare(iced::Size::new(800., 500.), 1.25, std::time::Instant::now())
        .unwrap();
    let axes = chart.frame().plot.axes.as_ref().unwrap();
    assert_eq!(axes.right.labels[0].axis.text, "109.00");
    assert_eq!(axes.right.labels[1].axis.text, "110.00");
    assert_eq!(chart.frame().plot.price_lines.len(), 1);
    let range = chart.model().panes()[0]
        .right_price_scale()
        .price_range()
        .unwrap();
    let mut options = chart.price_line_options(line).unwrap();
    options.price = 100_000.;
    chart.replace_price_line_options(line, options).unwrap();
    chart
        .prepare(iced::Size::new(800., 500.), 1.25, std::time::Instant::now())
        .unwrap();
    assert_eq!(
        chart.model().panes()[0]
            .right_price_scale()
            .price_range()
            .unwrap(),
        range
    );
    chart.set_data(series, vec![]).unwrap();
    chart
        .prepare(iced::Size::new(800., 500.), 1.25, std::time::Instant::now())
        .unwrap();
    assert!(chart.frame().plot.price_lines.is_empty());
    assert!(
        chart
            .frame()
            .plot
            .axes
            .as_ref()
            .unwrap()
            .right
            .labels
            .is_empty()
    );
}

#[test]
fn custom_price_labels_visibility_live_updates_and_pane_isolation() {
    use feather_charts::api::PriceLineOptions;
    let mut chart = Chart::default();
    let first = add(&mut chart);
    chart
        .set_data(first, vec![row(0., 100.), row(1., 110.)])
        .unwrap();
    let pane = chart.add_pane();
    let second = chart
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            pane,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(second, vec![row(0., 1.), row(1., 2.)])
        .unwrap();
    let mut options = PriceLineOptions::new(1.5);
    options.title = "Volume level".into();
    options.line_visible = false;
    options.axis_label_color = "white".into();
    options.axis_label_text_color = "black".into();
    let line = chart.add_price_line(second, options.clone()).unwrap();
    let prepare = |chart: &mut Chart| {
        chart
            .prepare(iced::Size::new(800., 500.), 1.5, std::time::Instant::now())
            .unwrap()
    };
    prepare(&mut chart);
    let frames = &chart.multi_frame().unwrap().panes;
    assert_eq!(frames[0].plot.price_lines.len(), 1);
    assert_eq!(frames[1].plot.price_lines.len(), 1);
    let labels = &frames[1].plot.axes.as_ref().unwrap().right.labels;
    assert_eq!(labels.len(), 2);
    assert_eq!(labels[1].pane_text, "Volume level");
    assert_eq!(labels[1].axis.text, "1.50");
    assert_eq!(labels[1].axis.color, "black");
    let old = labels[1].axis.coordinate;
    chart.update_data(second, row(1., 3.), false).unwrap();
    prepare(&mut chart);
    let labels = &chart.multi_frame().unwrap().panes[1]
        .plot
        .axes
        .as_ref()
        .unwrap()
        .right
        .labels;
    assert_eq!(labels[0].axis.text, "3.00");
    assert_ne!(labels[1].axis.coordinate, old);
    options.color = "not a color".into();
    assert!(
        chart
            .replace_price_line_options(line, options.clone())
            .is_err()
    );
    assert_eq!(chart.price_line_options(line).unwrap().color, "#FF0000");
    options.color = "red".into();
    options.line_visible = true;
    options.axis_label_visible = false;
    chart.replace_price_line_options(line, options).unwrap();
    prepare(&mut chart);
    let frame = &chart.multi_frame().unwrap().panes[1];
    assert_eq!(frame.plot.price_lines.len(), 2);
    assert_eq!(frame.plot.axes.as_ref().unwrap().right.labels.len(), 1);
    chart.remove_price_line(line).unwrap();
    prepare(&mut chart);
    assert_eq!(
        chart.multi_frame().unwrap().panes[1].plot.price_lines.len(),
        1
    );
    let mut hidden = LineSeriesOptions::default();
    hidden.common.visible = false;
    chart
        .replace_series_options(second, SeriesOptionsMap::Line(hidden))
        .unwrap();
    prepare(&mut chart);
    let frame = &chart.multi_frame().unwrap().panes[1];
    assert!(frame.plot.price_lines.is_empty());
    assert!(frame.plot.axes.as_ref().unwrap().right.labels.is_empty());
}

#[test]
fn visible_last_line_and_collision_alignment_preserve_price_coordinates() {
    use feather_charts::{
        api::PriceLineOptions,
        model::{
            axis_snapshots::PriceAxisSide, price_scale::PriceScaleOptionsPatch,
            series_options::PriceLineSource, time_data::LogicalRange,
        },
    };
    let mut chart = Chart::default();
    let series = add(&mut chart);
    chart
        .set_data(
            series,
            (0..20).map(|i| row(i as f64, 100. + i as f64)).collect(),
        )
        .unwrap();
    chart.set_logical_range(LogicalRange {
        from: 0.0.into(),
        to: 9.0.into(),
    });
    let mut options = LineSeriesOptions::default();
    options.common.price_line_source = PriceLineSource::LastVisible;
    chart
        .replace_series_options(series, SeriesOptionsMap::Line(options))
        .unwrap();
    let mut levels = vec![];
    for price in [108.99, 109., 109.01] {
        let mut options = PriceLineOptions::new(price);
        options.title = "Cluster".into();
        levels.push(chart.add_price_line(series, options).unwrap());
    }
    let prepare = |chart: &mut Chart| {
        chart
            .prepare(iced::Size::new(800., 500.), 1.25, std::time::Instant::now())
            .unwrap()
    };
    prepare(&mut chart);
    let axes = chart.frame().plot.axes.as_ref().unwrap();
    let labels = &axes.right.labels;
    assert_eq!(chart.frame().plot.price_lines.len(), 4);
    assert!(
        labels
            .iter()
            .any(|l| (l.axis.coordinate - l.render_coordinate).abs() > 1.)
    );
    let coordinates = labels.iter().map(|l| l.axis.coordinate).collect::<Vec<_>>();
    let lines = format!("{:?}", chart.frame().plot.price_lines);
    chart
        .apply_price_axis_options(
            0,
            PriceAxisSide::Right,
            PriceScaleOptionsPatch {
                align_labels: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
    prepare(&mut chart);
    let labels = &chart.frame().plot.axes.as_ref().unwrap().right.labels;
    assert_eq!(
        labels.iter().map(|l| l.axis.coordinate).collect::<Vec<_>>(),
        coordinates
    );
    assert!(
        labels
            .iter()
            .all(|l| l.axis.coordinate == l.render_coordinate)
    );
    assert_eq!(format!("{:?}", chart.frame().plot.price_lines), lines);
    let retained = format!("{:?}", chart.frame());
    prepare(&mut chart);
    assert_eq!(format!("{:?}", chart.frame()), retained);
}

#[test]
fn last_value_scale_modes_display_modes_and_color_precedence() {
    use feather_charts::model::{
        axis_snapshots::PriceAxisSide,
        price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
        series_options::PriceAxisLastValueMode,
    };
    let mut chart = Chart::default();
    let series = add(&mut chart);
    let mut last = row(1., 110.);
    if let BuiltInSeriesDataItem::Line(LineDataItem::Data(data)) = &mut last {
        data.color = Some("red".into());
    }
    chart.set_data(series, vec![row(0., 100.), last]).unwrap();
    chart.fit_content();
    for mode in [
        PriceScaleMode::Normal,
        PriceScaleMode::Logarithmic,
        PriceScaleMode::Percentage,
        PriceScaleMode::IndexedTo100,
    ] {
        chart
            .apply_price_axis_options(
                0,
                PriceAxisSide::Right,
                PriceScaleOptionsPatch {
                    mode: Some(mode),
                    ..Default::default()
                },
            )
            .unwrap();
        for both in [false, true] {
            let mut options = LineSeriesOptions::default();
            options.common.title = "ALPHA".into();
            options.common.series_last_value_mode =
                both.then_some(PriceAxisLastValueMode::LastPriceAndPercentageValue);
            if both {
                options.common.price_line_color = "blue".into();
            }
            chart
                .replace_series_options(series, SeriesOptionsMap::Line(options))
                .unwrap();
            chart
                .prepare(iced::Size::new(800., 500.), 1.25, std::time::Instant::now())
                .unwrap();
            let label = &chart.frame().plot.axes.as_ref().unwrap().right.labels[0];
            assert_eq!(
                label.axis.text,
                if mode == PriceScaleMode::Percentage {
                    "10.00%"
                } else {
                    "110.00"
                }
            );
            assert_eq!(
                label.pane_text,
                if !both {
                    "ALPHA"
                } else if mode == PriceScaleMode::Percentage {
                    "ALPHA 110.00"
                } else {
                    "ALPHA 10.00%"
                }
            );
            assert_eq!(
                label.axis.background,
                if both {
                    "rgb(0, 0, 255)"
                } else {
                    "rgb(255, 0, 0)"
                }
            );
            assert!(label.axis.coordinate.is_finite());
        }
    }
}
