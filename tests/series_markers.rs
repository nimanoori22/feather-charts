use feather_charts::{
    api::{Chart, MarkerPosition, MarkerShape, SeriesMarker, SeriesMarkersOptions},
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{LineSeriesOptions, SeriesOptionsMap},
    },
};
fn time(t: f64) -> Time {
    Time::from(UtcTimestamp::new(t))
}
fn marker(t: f64) -> SeriesMarker<Time> {
    SeriesMarker::new(
        time(t),
        MarkerPosition::AboveBar,
        MarkerShape::Circle,
        "#f00",
    )
}
fn chart() -> (Chart, feather_charts::model::data_layer::SeriesId) {
    let mut c = Chart::default();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(
        id,
        (0..10)
            .map(|i| {
                BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                    time: time(i as f64 * 10.),
                    value: 100. + i as f64,
                    color: None,
                    custom_values: None,
                }))
            })
            .collect(),
    )
    .unwrap();
    c.fit_content();
    (c, id)
}
#[test]
fn independent_marker_attachments_validate_replace_clear_and_detach() {
    let (mut c, series) = chart();
    let a = c
        .add_series_markers(
            series,
            vec![marker(0.), marker(0.)],
            SeriesMarkersOptions::default(),
        )
        .unwrap();
    let b = c
        .add_series_markers(series, vec![marker(10.)], SeriesMarkersOptions::default())
        .unwrap();
    let mut invalid = marker(20.);
    invalid.color = "invalid".into();
    assert!(c.set_series_markers(a, vec![invalid]).is_err());
    assert!(
        c.set_series_markers(a, vec![marker(10.), marker(0.)])
            .is_err()
    );
    assert_eq!(c.series_markers(a).unwrap().len(), 2);
    c.set_series_markers(a, vec![]).unwrap();
    assert_eq!(c.series_markers(b).unwrap().len(), 1);
    c.remove_series_markers(a).unwrap();
    assert!(c.series_markers(a).is_err());
    assert!(c.remove_series_markers(a).is_err());
    c.remove_series(series).unwrap();
    assert!(c.series_markers(b).is_err());
}
fn prepare(c: &mut Chart) {
    c.prepare(iced::Size::new(800., 500.), 1.25, std::time::Instant::now())
        .unwrap();
}
#[test]
fn marker_margins_add_to_configured_scale_padding_and_clear_restores_it() {
    let (mut c, series) = chart();
    prepare(&mut c);
    let scale = c.model().panes()[0].right_price_scale();
    let configured = scale.options().scale_margins;
    let before = scale.internal_height();
    let collection = c
        .add_series_markers(series, vec![marker(20.)], SeriesMarkersOptions::default())
        .unwrap();
    prepare(&mut c);
    let scale = c.model().panes()[0].right_price_scale();
    assert_eq!(scale.options().scale_margins, configured);
    assert!(scale.internal_height() < before);
    c.set_series_markers(collection, vec![]).unwrap();
    prepare(&mut c);
    assert_eq!(
        c.model().panes()[0].right_price_scale().internal_height(),
        before
    );
}
#[test]
fn public_frames_place_stack_and_clear_markers_without_modifying_retained_frames() {
    let (mut c, series) = chart();
    let id = c
        .add_series_markers(
            series,
            vec![marker(25.), marker(25.)],
            SeriesMarkersOptions::default(),
        )
        .unwrap();
    prepare(&mut c);
    let retained = c.frame().clone();
    let items = &retained.plot.annotations[0].items;
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].index.value(), 3.); // shared-time nearest lookup selects time 30
    assert_eq!(items[0].center.x, items[1].center.x);
    assert!(items[1].center.y < items[0].center.y);
    c.set_series_markers(id, vec![]).unwrap();
    prepare(&mut c);
    assert!(c.frame().plot.annotations.is_empty());
    assert_eq!(retained.plot.annotations[0].items.len(), 2);
}
#[test]
fn public_click_and_hover_identify_text_and_clear_stale_hits() {
    use feather_charts::{
        api::{ChartEvent, ChartMessage},
        ui::chart_input::ChartInteraction,
    };
    let (mut c, series) = chart();
    let mut m = marker(40.);
    m.size = 0.;
    m.text = Some("A long trade annotation".into());
    m.id = Some("buy".into());
    let id = c
        .add_series_markers(series, vec![m], SeriesMarkersOptions::default())
        .unwrap();
    prepare(&mut c);
    let t = c.frame().plot.annotations[0].items[0].text.clone().unwrap();
    let p = iced::Point::new((t.x + 2.) as f32, t.y as f32);
    let events = c
        .update(ChartMessage::Click {
            pane: 0,
            position: p,
        })
        .unwrap();
    let hit = events
        .iter()
        .find_map(|e| match e {
            ChartEvent::Click(p) => p.marker.as_ref(),
            _ => None,
        })
        .unwrap();
    assert_eq!(hit.collection, id);
    assert_eq!(hit.series, series);
    assert_eq!(hit.external_id.as_deref(), Some("buy"));
    c.set_series_markers(id, vec![]).unwrap();
    let events = c
        .update(ChartMessage::Tick(std::time::Instant::now()))
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ChartEvent::CrosshairMoved(p) if p.marker.is_none()))
    );
    let events = c
        .update(ChartMessage::Interaction(ChartInteraction {
            position: None,
            navigation: None,
            capture: false,
        }))
        .unwrap();
    assert!(events.iter().all(|e| match e {
        ChartEvent::Click(p) | ChartEvent::CrosshairMoved(p) => p.marker.is_none(),
        ChartEvent::AttributionActivated { .. }
        | ChartEvent::PrimitiveEdit(_)
        | ChartEvent::DataChanged { .. }
        | ChartEvent::DoubleClick(_)
        | ChartEvent::VisibleLogicalRangeChanged(_)
        | ChartEvent::VisibleHorizontalRangeChanged(_)
        | ChartEvent::HorizontalAxisSizeChanged { .. } => false,
    }));
}
#[test]
fn marker_projection_tracks_shared_time_reindexing_history_whitespace_and_reload() {
    use feather_charts::model::{
        data_consumer::WhitespaceData, series_options::SeriesOptionsCommon,
    };
    let (mut c, series) = chart();
    let id = c
        .add_series_markers(
            series,
            vec![marker(-1.), marker(25.), marker(1000.)],
            SeriesMarkersOptions::default(),
        )
        .unwrap();
    prepare(&mut c);
    let indices = c.frame().plot.annotations[0]
        .items
        .iter()
        .map(|m| m.index.value())
        .collect::<Vec<_>>();
    assert_eq!(indices, vec![0., 3., 9.]);
    let other = c
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            0,
            PriceScalePosition::Left,
        )
        .unwrap();
    c.set_data(
        other,
        vec![BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(
            WhitespaceData {
                time: time(25.),
                custom_values: None,
            },
        ))],
    )
    .unwrap();
    prepare(&mut c);
    let item = &c.frame().plot.annotations[0].items[1];
    assert_eq!(item.index.value(), 2.); // exact shared time 25 then nearest-left series row 20
    let before = item.center.y;
    c.update_data(
        series,
        BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time: time(20.),
            value: 105.,
            color: None,
            custom_values: None,
        })),
        true,
    )
    .unwrap();
    prepare(&mut c);
    assert_ne!(c.frame().plot.annotations[0].items[1].center.y, before);
    c.replace_series_options(
        series,
        SeriesOptionsMap::Line(LineSeriesOptions {
            common: SeriesOptionsCommon {
                visible: false,
                ..LineSeriesOptions::default().common
            },
            ..LineSeriesOptions::default()
        }),
    )
    .unwrap();
    prepare(&mut c);
    assert!(c.frame().plot.annotations.is_empty());
    c.replace_series_options(series, SeriesOptionsMap::Line(LineSeriesOptions::default()))
        .unwrap();
    c.set_data(series, vec![]).unwrap();
    prepare(&mut c);
    assert!(c.frame().plot.annotations.is_empty());
    assert_eq!(c.series_markers(id).unwrap().len(), 3);
    c.set_data(
        series,
        vec![BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time: time(20.),
            value: 100.,
            color: None,
            custom_values: None,
        }))],
    )
    .unwrap();
    c.fit_content();
    prepare(&mut c);
    assert!(
        !c.frame().plot.annotations.is_empty(),
        "range {:?}, scale {:?}, rows {:?}",
        c.model().time_scale().base_index(),
        c.model().panes()[0].right_price_scale().price_range(),
        c.model().series(series).unwrap().bars().indices()
    );
}
#[test]
fn overlapping_marker_hits_use_reverse_series_order_then_first_attachment_and_item() {
    use feather_charts::api::{ChartEvent, ChartMessage, MarkerZOrder};
    let (mut c, first) = chart();
    let second = c
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let data = (0..10)
        .map(|i| {
            BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                time: time(i as f64 * 10.),
                value: 100. + i as f64,
                color: None,
                custom_values: None,
            }))
        })
        .collect();
    c.set_data(second, data).unwrap();
    let mut m = marker(40.);
    m.position = MarkerPosition::InBar;
    let first_attachment = c
        .add_series_markers(
            first,
            vec![m.clone()],
            SeriesMarkersOptions {
                auto_scale: false,
                z_order: MarkerZOrder::Top,
            },
        )
        .unwrap();
    let second_attachment = c
        .add_series_markers(
            second,
            vec![m.clone(), m.clone()],
            SeriesMarkersOptions::default(),
        )
        .unwrap();
    c.add_series_markers(
        second,
        vec![m],
        SeriesMarkersOptions {
            auto_scale: true,
            z_order: MarkerZOrder::Top,
        },
    )
    .unwrap();
    prepare(&mut c);
    let center = c.frame().plot.annotations[0].items[0].center;
    let p = iced::Point::new(center.x as f32, center.y as f32);
    let hit = c.marker_hit(0, p).unwrap();
    assert_eq!(hit.collection, second_attachment);
    assert_eq!(hit.index, 0);
    let events = c
        .update(ChartMessage::Click {
            pane: 0,
            position: p,
        })
        .unwrap();
    assert!(events.iter().any(
        |e| matches!(e,ChartEvent::Click(p) if p.marker.as_ref().is_some_and(|h|h.series==second))
    ));
    c.remove_series(second).unwrap();
    prepare(&mut c);
    assert_eq!(c.marker_hit(0, p).unwrap().collection, first_attachment);
}
#[test]
fn markers_follow_supported_scale_modes_and_manual_navigation_at_fractional_ratios() {
    use feather_charts::model::{
        axis_snapshots::PriceAxisSide,
        price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
    };
    let (mut c, series) = chart();
    let positions = [
        MarkerPosition::AboveBar,
        MarkerPosition::BelowBar,
        MarkerPosition::InBar,
        MarkerPosition::AtPriceTop(103.),
        MarkerPosition::AtPriceBottom(103.),
        MarkerPosition::AtPriceMiddle(103.),
    ];
    let id = c
        .add_series_markers(
            series,
            positions
                .iter()
                .map(|p| {
                    let mut m = marker(40.);
                    m.position = *p;
                    m
                })
                .collect(),
            SeriesMarkersOptions::default(),
        )
        .unwrap();
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
            for ratio in [1., 1.25, 1.5, 2.] {
                c.prepare(
                    iced::Size::new(800., 500.),
                    ratio,
                    std::time::Instant::now(),
                )
                .unwrap();
                let collection = &c.frame().plot.annotations[0];
                assert_eq!(collection.id, id);
                assert_eq!(collection.items.len(), 6);
                let scale = c.model().panes()[0].right_price_scale();
                let y = scale.price_to_coordinate(103., 100.).value();
                assert!((collection.items[5].center.y - y).abs() < 1e-8);
                assert!(collection.items[3].center.y < y && collection.items[4].center.y > y);
            }
        }
    }
    c.apply_price_axis_options(
        0,
        PriceAxisSide::Right,
        PriceScaleOptionsPatch {
            mode: Some(PriceScaleMode::Normal),
            auto_scale: Some(false),
            ..Default::default()
        },
    )
    .unwrap();
    let before = c.model().panes()[0].right_price_scale().price_range();
    c.replace_series_marker_options(
        id,
        SeriesMarkersOptions {
            auto_scale: false,
            ..Default::default()
        },
    )
    .unwrap();
    prepare(&mut c);
    assert_eq!(
        c.model().panes()[0].right_price_scale().price_range(),
        before
    );
}
#[test]
fn annotations_render_and_hit_on_all_six_families_in_independent_panes() {
    use feather_charts::model::{data_consumer::*, series_options::*};
    let mut c = Chart::default();
    for (pane, kind) in [
        SeriesType::Line,
        SeriesType::Area,
        SeriesType::Bar,
        SeriesType::Baseline,
        SeriesType::Candlestick,
        SeriesType::Histogram,
    ]
    .into_iter()
    .enumerate()
    {
        if pane > 0 {
            c.add_pane();
        }
        let options = match kind {
            SeriesType::Line => SeriesOptionsMap::Line(LineSeriesOptions::default()),
            SeriesType::Area => SeriesOptionsMap::Area(AreaSeriesOptions::default()),
            SeriesType::Bar => SeriesOptionsMap::Bar(BarSeriesOptions::default()),
            SeriesType::Baseline => SeriesOptionsMap::Baseline(BaselineSeriesOptions::default()),
            SeriesType::Candlestick => {
                SeriesOptionsMap::Candlestick(CandlestickSeriesOptions::default())
            }
            SeriesType::Histogram => SeriesOptionsMap::Histogram(HistogramSeriesOptions::default()),
            _ => unreachable!(),
        };
        let series = c
            .add_series(options, pane, PriceScalePosition::Right)
            .unwrap();
        let data = (0..10)
            .map(|i| {
                let time = time(i as f64 * 10.);
                let v = 100. + i as f64;
                match kind {
                    SeriesType::Line => BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                        time,
                        value: v,
                        color: None,
                        custom_values: None,
                    })),
                    SeriesType::Area => BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
                        time,
                        value: v,
                        line_color: None,
                        top_color: None,
                        bottom_color: None,
                        custom_values: None,
                    })),
                    SeriesType::Baseline => {
                        BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(BaselineData {
                            time,
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
                    SeriesType::Bar => BuiltInSeriesDataItem::Bar(BarDataItem::Data(BarData {
                        time,
                        open: v - 1.,
                        high: v + 2.,
                        low: v - 2.,
                        close: v,
                        color: None,
                        custom_values: None,
                    })),
                    SeriesType::Candlestick => BuiltInSeriesDataItem::Candlestick(
                        CandlestickDataItem::Data(CandlestickData {
                            time,
                            open: v - 1.,
                            high: v + 2.,
                            low: v - 2.,
                            close: v,
                            color: None,
                            border_color: None,
                            wick_color: None,
                            custom_values: None,
                        }),
                    ),
                    SeriesType::Histogram => {
                        BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
                            time,
                            value: v,
                            color: None,
                            custom_values: None,
                        }))
                    }
                    _ => unreachable!(),
                }
            })
            .collect();
        c.set_data(series, data).unwrap();
        let mut m = marker(40.);
        m.position = MarkerPosition::InBar;
        m.color = "rgba(255, 0, 0, 0.5)".into();
        m.text = Some(String::new());
        c.add_series_markers(
            series,
            vec![m],
            SeriesMarkersOptions {
                auto_scale: false,
                ..Default::default()
            },
        )
        .unwrap();
    }
    c.fit_content();
    c.prepare(iced::Size::new(800., 1600.), 1.5, std::time::Instant::now())
        .unwrap();
    let frame = c.multi_frame().unwrap();
    assert_eq!(frame.panes.len(), 6);
    for (pane, p) in frame.panes.iter().enumerate() {
        assert!(!p.plot.annotation_geometry[0].paths.is_empty());
        assert!(
            p.plot.annotation_geometry[0]
                .paths
                .iter()
                .all(|(_, color)| color.a == 0.5)
        );
        let m = &p.plot.annotations[0].items[0];
        assert!(m.text.is_none());
        assert_eq!(
            c.marker_hit(pane, iced::Point::new(m.center.x as f32, m.center.y as f32))
                .unwrap()
                .series,
            m.hit.series
        );
    }
}
#[test]
fn native_canvas_clicks_markers_and_redraw_reconciles_stationary_cursor() {
    use feather_charts::api::{ChartEvent, ChartMessage, ChartState};
    use iced::{
        Point, Rectangle, mouse,
        widget::canvas::{Event, Program},
    };
    let (mut c, series) = chart();
    let id = c
        .add_series_markers(series, vec![marker(40.)], SeriesMarkersOptions::default())
        .unwrap();
    prepare(&mut c);
    let m = c.frame().plot.annotations[0].items[0].center;
    let plot = c.frame().layout.plot;
    let p = Point::new((m.x + plot.origin.x) as f32, (m.y + plot.origin.y) as f32);
    let size = iced::Size::new(800., 500.);
    let mut state = ChartState::default();
    let mut events = vec![];
    for event in [
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ] {
        let message = c
            .program(std::convert::identity)
            .update(
                &mut state,
                &event,
                Rectangle::with_size(size),
                mouse::Cursor::Available(p),
            )
            .and_then(|a| a.into_inner().0);
        if let Some(message) = message {
            events.extend(c.update(message).unwrap());
        }
    }
    assert!(events.iter().any(
        |e| matches!(e,ChartEvent::Click(p) if p.marker.as_ref().is_some_and(|h|h.collection==id))
    ));
    c.set_series_markers(id, vec![]).unwrap();
    prepare(&mut c);
    assert!(c.is_animating());
    let message = c
        .program(std::convert::identity)
        .update(
            &mut state,
            &Event::Window(iced::window::Event::RedrawRequested(
                std::time::Instant::now(),
            )),
            Rectangle::with_size(size),
            mouse::Cursor::Available(p),
        )
        .and_then(|a| a.into_inner().0)
        .expect("stationary hit change must request an update");
    let events = c.update(message).unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e,ChartEvent::CrosshairMoved(p) if p.marker.is_none()))
    );
    assert!(!c.is_animating());
    assert!(
        c.update(ChartMessage::Tick(std::time::Instant::now()))
            .unwrap()
            .is_empty()
    );
}
#[test]
fn equal_total_margin_changes_reproject_other_collections_and_disabled_margins_restore() {
    let (mut c, series) = chart();
    let mut in_bar = marker(40.);
    in_bar.position = MarkerPosition::InBar;
    let first = c
        .add_series_markers(
            series,
            vec![in_bar],
            SeriesMarkersOptions {
                auto_scale: false,
                ..Default::default()
            },
        )
        .unwrap();
    let second = c
        .add_series_markers(series, vec![marker(40.)], SeriesMarkersOptions::default())
        .unwrap();
    prepare(&mut c);
    let before = c.frame().plot.annotations[0].items[0].center.y;
    let height = c.model().panes()[0].right_price_scale().internal_height();
    let mut below = marker(40.);
    below.position = MarkerPosition::BelowBar;
    c.set_series_markers(second, vec![below]).unwrap();
    prepare(&mut c);
    assert_eq!(c.frame().plot.annotations[0].id, first);
    assert_eq!(
        c.model().panes()[0].right_price_scale().internal_height(),
        height
    );
    assert_ne!(c.frame().plot.annotations[0].items[0].center.y, before);
    c.replace_series_marker_options(
        second,
        SeriesMarkersOptions {
            auto_scale: false,
            ..Default::default()
        },
    )
    .unwrap();
    prepare(&mut c);
    assert!(c.model().panes()[0].right_price_scale().internal_height() > height);
}
