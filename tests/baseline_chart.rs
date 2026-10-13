use feather_charts::{
    api::Chart,
    model::{
        data_consumer::{BaselineData, BaselineDataItem, BuiltInSeriesDataItem},
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{BaseValuePrice, BaseValueType, BaselineSeriesOptions, SeriesOptionsMap},
    },
};
fn row(i: f64, v: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(BaselineData {
        time: Time::from(UtcTimestamp::new(i)),
        value: v,
        top_fill_color1: None,
        top_fill_color2: None,
        bottom_fill_color1: None,
        bottom_fill_color2: None,
        top_line_color: None,
        bottom_line_color: None,
        custom_values: None,
    }))
}
fn options(base: f64) -> BaselineSeriesOptions {
    let mut o = BaselineSeriesOptions::default();
    o.style.base_value = BaseValueType::Price(BaseValuePrice { price: base });
    o
}
fn prepare(chart: &mut Chart) {
    chart
        .prepare(iced::Size::new(800., 500.), 1.25, std::time::Instant::now())
        .unwrap();
}
#[test]
fn baseline_crossings_options_validation_and_lifecycle() {
    let mut c = Chart::default();
    let id = c
        .add_series(
            SeriesOptionsMap::Baseline(options(105.)),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(id, vec![row(0., 100.), row(1., 110.), row(2., 105.)])
        .unwrap();
    c.fit_content();
    prepare(&mut c);
    assert!(!c.frame().plot.fills.is_empty());
    assert!(!c.frame().plot.gradient_lines.is_empty());
    assert!(!c.frame().plot.price_lines.is_empty());
    let gradient = &c.frame().plot.fills[0].gradient;
    assert_eq!(
        gradient.stops[1].unwrap().offset,
        gradient.stops[2].unwrap().offset
    );
    assert_ne!(
        gradient.stops[1].unwrap().color,
        gradient.stops[2].unwrap().color
    );
    let mut o = options(105.);
    o.style.line_visible = false;
    o.style.point_markers_visible = true;
    c.replace_series_options(id, SeriesOptionsMap::Baseline(o))
        .unwrap();
    prepare(&mut c);
    assert!(c.frame().plot.gradient_lines.is_empty());
    assert!(!c.frame().plot.gradient_markers.is_empty());
    assert!(!c.frame().plot.fills.is_empty());
    let mut invalid = options(f64::NAN);
    assert!(
        c.replace_series_options(id, SeriesOptionsMap::Baseline(invalid))
            .is_err()
    );
    invalid = options(105.);
    invalid.style.top_line_color = "invalid".into();
    assert!(
        c.replace_series_options(id, SeriesOptionsMap::Baseline(invalid))
            .is_err()
    );
    let mut bad = row(3., 120.);
    if let BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(r)) = &mut bad {
        r.bottom_fill_color1 = Some("invalid".into());
    }
    assert!(c.set_data(id, vec![bad]).is_err());
    assert_eq!(c.model().series(id).unwrap().bars().rows().len(), 3);
    c.update_data(id, row(3., 115.), false).unwrap();
    c.update_data(id, row(0., 95.), true).unwrap();
    c.replace_series_options(id, SeriesOptionsMap::Baseline(options(1000.)))
        .unwrap();
    c.fit_content();
    prepare(&mut c);
    assert!(!c.frame().plot.fills.is_empty());
    let mut hidden = options(105.);
    hidden.common.visible = false;
    c.replace_series_options(id, SeriesOptionsMap::Baseline(hidden))
        .unwrap();
    prepare(&mut c);
    assert!(c.frame().plot.fills.is_empty());
    assert!(c.frame().plot.price_lines.is_empty());
    c.replace_series_options(id, SeriesOptionsMap::Baseline(options(105.)))
        .unwrap();
    c.set_data(id, vec![]).unwrap();
    prepare(&mut c);
    assert!(c.frame().plot.fills.is_empty());
    c.set_data(id, vec![row(0., 105.)]).unwrap();
    let mut o = options(105.);
    o.style.relative_gradient = true;
    c.replace_series_options(id, SeriesOptionsMap::Baseline(o))
        .unwrap();
    c.fit_content();
    prepare(&mut c);
    c.remove_series(id).unwrap();
}

#[test]
fn baseline_scale_projection_and_decoration_colors_follow_source() {
    use feather_charts::{
        api::ChartMessage,
        model::{
            axis_snapshots::PriceAxisSide,
            price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
        },
        ui::chart_input::ChartInteraction,
    };
    let mut c = Chart::default();
    let id = c
        .add_series(
            SeriesOptionsMap::Baseline(options(105.)),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let mut last = row(2., 110.);
    if let BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(r)) = &mut last {
        r.top_line_color = Some("blue".into());
        r.bottom_line_color = Some("yellow".into());
    }
    c.set_data(id, vec![row(0., 100.), row(1., 105.), last])
        .unwrap();
    c.fit_content();
    for mode in [
        PriceScaleMode::Normal,
        PriceScaleMode::Logarithmic,
        PriceScaleMode::Percentage,
        PriceScaleMode::IndexedTo100,
    ] {
        for inverted in [false, true] {
            c.apply_price_axis_options(
                0,
                PriceAxisSide::Right,
                PriceScaleOptionsPatch {
                    mode: Some(mode),
                    invert_scale: Some(inverted),
                    ..Default::default()
                },
            )
            .unwrap();
            prepare(&mut c);
            let scale = c.model().panes()[0].right_price_scale();
            let y = scale.price_to_coordinate(105., 100.).value();
            let height = c.frame().plot.size.height;
            let offset = c.frame().plot.fills[0].gradient.stops[1].unwrap().offset;
            assert!((offset - (y / f64::from(height)).clamp(0., 1.) as f32).abs() < 1e-5);
            let labels = &c.frame().plot.axes.as_ref().unwrap().right.labels;
            assert_eq!(labels[0].axis.background, "rgb(38, 166, 154)");
            let x = c
                .model()
                .time_scale()
                .index_to_coordinate(2.0.into())
                .value() as f32;
            c.update(ChartMessage::Interaction(ChartInteraction {
                position: None,
                navigation: None,
                capture: false,
            }))
            .unwrap();
            assert!(
                !c.update(ChartMessage::Interaction(ChartInteraction {
                    position: Some(iced::Point::new(x, 100.)),
                    navigation: None,
                    capture: false
                }))
                .unwrap()
                .is_empty()
            );
            assert!(!c.frame().plot.crosshair_markers.is_empty());
        }
    }
    c.apply_price_axis_options(
        0,
        PriceAxisSide::Right,
        PriceScaleOptionsPatch {
            mode: Some(PriceScaleMode::Normal),
            invert_scale: Some(false),
            ..Default::default()
        },
    )
    .unwrap();
    prepare(&mut c);
    let before = c.model().panes()[0].right_price_scale().price_range();
    c.replace_series_options(id, SeriesOptionsMap::Baseline(options(1000.)))
        .unwrap();
    prepare(&mut c);
    assert_eq!(
        c.model().panes()[0].right_price_scale().price_range(),
        before,
        "Baseline base is not forced into autoscale"
    );
    assert_eq!(
        c.frame().plot.axes.as_ref().unwrap().right.labels[0]
            .axis
            .background,
        "rgb(239, 83, 80)"
    );
    use feather_charts::ui::price_axis_input::{PriceAxisAction, PriceAxisInteraction};
    c.replace_series_options(id, SeriesOptionsMap::Baseline(options(105.)))
        .unwrap();
    prepare(&mut c);
    let before = c.model().panes()[0].right_price_scale().price_range();
    c.update(ChartMessage::PriceAxis(PriceAxisInteraction {
        pane: 0,
        side: PriceAxisSide::Right,
        action: PriceAxisAction::Wheel { delta: 1. },
    }))
    .unwrap();
    let scale = c.model().panes()[0].right_price_scale();
    assert!(!scale.options().auto_scale);
    assert_ne!(scale.price_range(), before);
    let expected = (scale.price_to_coordinate(105., 100.).value()
        / f64::from(c.frame().plot.size.height))
    .clamp(0., 1.) as f32;
    assert!((c.frame().plot.fills[0].gradient.stops[1].unwrap().offset - expected).abs() < 1e-5);
}

#[test]
fn new_series_compose_owned_frames_and_preserve_exact_event_rows() {
    use feather_charts::{
        api::{ChartEvent, ChartMessage},
        model::{
            data_consumer::{AreaData, AreaDataItem, BarData, BarDataItem},
            series_options::{AreaSeriesOptions, BarSeriesOptions},
        },
        ui::chart_input::ChartInteraction,
    };
    let mut c = Chart::default();
    let baseline = c
        .add_series(
            SeriesOptionsMap::Baseline(options(105.)),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let area = c
        .add_series(
            SeriesOptionsMap::Area(AreaSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let bar = c
        .add_series(
            SeriesOptionsMap::Bar(BarSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let mut expected = vec![];
    for id in [baseline, area, bar] {
        let rows = (0..3)
            .map(|i| {
                let time = Time::from(UtcTimestamp::new(i as f64));
                let value = 100. + i as f64 * 5.;
                if id == baseline {
                    let mut r = row(i as f64, value);
                    if let BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(r)) = &mut r {
                        r.top_line_color = Some("blue".into());
                        r.custom_values = Some(());
                    }
                    r
                } else if id == area {
                    BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
                        time,
                        value,
                        line_color: Some("yellow".into()),
                        top_color: None,
                        bottom_color: None,
                        custom_values: Some(()),
                    }))
                } else {
                    BuiltInSeriesDataItem::Bar(BarDataItem::Data(BarData {
                        time,
                        open: value - 1.,
                        high: value + 2.,
                        low: value - 2.,
                        close: value,
                        color: Some("red".into()),
                        custom_values: Some(()),
                    }))
                }
            })
            .collect::<Vec<_>>();
        expected.push((id, rows[1].clone()));
        c.set_data(id, rows).unwrap();
    }
    c.fit_content();
    prepare(&mut c);
    let retained = c.frame().plot.clone();
    assert_eq!(retained.series_layers.len(), 2);
    assert!(!retained.gradient_lines.is_empty());
    assert!(!retained.series_layers[0].fills.is_empty());
    assert_eq!(retained.series_layers[1].candles.len(), 9);
    let x = c
        .model()
        .time_scale()
        .index_to_coordinate(1.0.into())
        .value() as f32;
    let events = c
        .update(ChartMessage::Interaction(ChartInteraction {
            position: Some(iced::Point::new(x, 100.)),
            navigation: None,
            capture: false,
        }))
        .unwrap();
    let event = events
        .iter()
        .find_map(|e| {
            if let ChartEvent::CrosshairMoved(e) = e {
                Some(e)
            } else {
                None
            }
        })
        .unwrap();
    let clicked = c
        .update(ChartMessage::Click {
            pane: 0,
            position: iced::Point::new(x, 100.),
        })
        .unwrap();
    let clicked = clicked
        .iter()
        .find_map(|e| {
            if let ChartEvent::Click(e) = e {
                Some(e)
            } else {
                None
            }
        })
        .unwrap();
    for (id, row) in expected {
        assert_eq!(
            clicked
                .series_data
                .iter()
                .find(|(i, _)| *i == id)
                .map(|(_, r)| r),
            Some(&row)
        );
        assert_eq!(
            event
                .series_data
                .iter()
                .find(|(i, _)| *i == id)
                .map(|(_, r)| r),
            Some(&row)
        );
    }
    for id in [baseline, area, bar] {
        c.set_data(id, vec![]).unwrap();
    }
    prepare(&mut c);
    assert!(c.frame().plot.fills.is_empty());
    assert!(
        c.frame()
            .plot
            .series_layers
            .iter()
            .all(|l| l.fills.is_empty() && l.candles.is_empty())
    );
    assert!(!retained.fills.is_empty());
    assert_eq!(retained.series_layers[1].candles.len(), 9);
}

#[test]
fn whitespace_has_no_geometry_or_nearest_row_event_payload() {
    use feather_charts::{
        api::{ChartEvent, ChartMessage},
        model::{
            data_consumer::{AreaData, AreaDataItem, BarData, BarDataItem, WhitespaceData},
            series_options::{AreaSeriesOptions, BarSeriesOptions},
        },
    };
    let mut c = Chart::default();
    let base = c
        .add_series(
            SeriesOptionsMap::Baseline(options(105.)),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let area = c
        .add_series(
            SeriesOptionsMap::Area(AreaSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let bar = c
        .add_series(
            SeriesOptionsMap::Bar(BarSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let time = |i| Time::from(UtcTimestamp::new(i));
    let white = WhitespaceData {
        time: time(1.),
        custom_values: None,
    };
    c.set_data(
        base,
        vec![
            row(0., 100.),
            BuiltInSeriesDataItem::Baseline(BaselineDataItem::Whitespace(white.clone())),
            row(2., 110.),
        ],
    )
    .unwrap();
    let area_row = |i, v| {
        BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
            time: time(i),
            value: v,
            line_color: None,
            top_color: None,
            bottom_color: None,
            custom_values: None,
        }))
    };
    let bar_row = |i, v| {
        BuiltInSeriesDataItem::Bar(BarDataItem::Data(BarData {
            time: time(i),
            open: v - 1.,
            high: v + 2.,
            low: v - 2.,
            close: v,
            color: None,
            custom_values: None,
        }))
    };
    c.set_data(
        area,
        vec![
            area_row(0., 100.),
            BuiltInSeriesDataItem::Area(AreaDataItem::Whitespace(white.clone())),
            area_row(2., 110.),
        ],
    )
    .unwrap();
    c.set_data(
        bar,
        vec![
            bar_row(0., 100.),
            BuiltInSeriesDataItem::Bar(BarDataItem::Whitespace(white)),
            bar_row(2., 110.),
        ],
    )
    .unwrap();
    c.fit_content();
    prepare(&mut c);
    assert_eq!(c.frame().plot.series_layers[1].candles.len(), 6);
    for (index, count) in [(1., 0), (2., 3)] {
        let x = c
            .model()
            .time_scale()
            .index_to_coordinate(index.into())
            .value() as f32;
        let events = c
            .update(ChartMessage::Click {
                pane: 0,
                position: iced::Point::new(x, 100.),
            })
            .unwrap();
        let e = events
            .iter()
            .find_map(|e| {
                if let ChartEvent::Click(e) = e {
                    Some(e)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(e.series_data.len(), count);
    }
}
