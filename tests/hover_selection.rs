use feather_charts::{
    api::*,
    model::{
        data_consumer::*,
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::*,
    },
};
fn chart() -> (Chart, feather_charts::model::data_layer::SeriesId) {
    let mut c = Chart::default();
    let mut o = LineSeriesOptions::default();
    o.common.price_line_visible = false;
    let id = c
        .add_series(SeriesOptionsMap::Line(o), 0, PriceScalePosition::Right)
        .unwrap();
    c.set_data(
        id,
        (0..10)
            .map(|i| {
                BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                    time: Time::from(UtcTimestamp::new(1000. + i as f64 * 10.)),
                    value: 100. + i as f64,
                    color: None,
                    custom_values: None,
                }))
            })
            .collect(),
    )
    .unwrap();
    c.fit_content();
    c.prepare(iced::Size::new(640., 480.), 1.25, std::time::Instant::now())
        .unwrap();
    (c, id)
}
#[test]
fn settled_line_hit_is_owned_and_public_click_reports_it() {
    let (mut c, id) = chart();
    let stroke = &c.frame().plot.lines[0];
    let a = match stroke.path[0] {
        feather_charts::renderers::line_renderer::PathOperation::MoveTo(p) => p,
        _ => panic!("move"),
    };
    let p = iced::Point::new(a.x as f32, a.y as f32);
    let hit = c.hit_test(0, p).unwrap();
    assert_eq!(hit.series, id);
    assert!(matches!(
        hit.target,
        HitTarget::SeriesLine | HitTarget::SeriesPoint
    ));
    let events = c
        .update(ChartMessage::Click {
            pane: 0,
            position: p,
        })
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e,ChartEvent::Click(data) if data.hit.as_ref()==Some(&hit)))
    );
    c.set_data(id, vec![]).unwrap();
    c.prepare(iced::Size::new(640., 480.), 1.25, std::time::Instant::now())
        .unwrap();
    assert!(c.hit_test(0, p).is_none());
    assert_eq!(hit.series, id);
}

fn options(kind: SeriesType) -> SeriesOptionsMap {
    macro_rules! make {
        ($variant:ident,$ty:ident) => {{
            let mut o = $ty::default();
            o.common.price_line_visible = false;
            SeriesOptionsMap::$variant(o)
        }};
    }
    match kind {
        SeriesType::Line => make!(Line, LineSeriesOptions),
        SeriesType::Area => make!(Area, AreaSeriesOptions),
        SeriesType::Baseline => make!(Baseline, BaselineSeriesOptions),
        SeriesType::Bar => make!(Bar, BarSeriesOptions),
        SeriesType::Candlestick => make!(Candlestick, CandlestickSeriesOptions),
        SeriesType::Histogram => make!(Histogram, HistogramSeriesOptions),
        _ => unreachable!(),
    }
}
fn row(kind: SeriesType, i: usize) -> BuiltInSeriesDataItem<Time> {
    let time = Time::from(UtcTimestamp::new(1000. + i as f64 * 10.));
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
        SeriesType::Candlestick => {
            BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(CandlestickData {
                time,
                open: v - 1.,
                high: v + 2.,
                low: v - 2.,
                close: v,
                color: None,
                border_color: None,
                wick_color: None,
                custom_values: None,
            }))
        }
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
}
fn position(
    c: &Chart,
    id: feather_charts::model::data_layer::SeriesId,
    index: f64,
    price: f64,
) -> iced::Point {
    let x = c
        .model()
        .time_scale()
        .index_to_coordinate(index.into())
        .value();
    let pane = c.model().pane_for_series(id).unwrap();
    let scale = c.model().panes()[pane].right_price_scale();
    let first = 100.;
    iced::Point::new(
        x as f32,
        scale.price_to_coordinate(price, first).value() as f32,
    )
}
#[test]
fn every_family_hits_its_source_geometry_at_fractional_ratios() {
    for kind in [
        SeriesType::Line,
        SeriesType::Area,
        SeriesType::Baseline,
        SeriesType::Bar,
        SeriesType::Candlestick,
        SeriesType::Histogram,
    ] {
        let mut c = Chart::default();
        let id = c
            .add_series(options(kind), 0, PriceScalePosition::Right)
            .unwrap();
        c.set_data(id, (0..10).map(|i| row(kind, i)).collect())
            .unwrap();
        c.fit_content();
        for ratio in [1., 1.25, 1.5, 2.] {
            c.prepare(
                iced::Size::new(640., 480.),
                ratio,
                std::time::Instant::now(),
            )
            .unwrap();
            let hit = c
                .hit_test(0, position(&c, id, 3., 103.))
                .expect("visible series hit");
            assert_eq!(hit.series, id);
            if matches!(
                kind,
                SeriesType::Bar | SeriesType::Candlestick | SeriesType::Histogram
            ) {
                assert_eq!(hit.target, HitTarget::SeriesRange);
            }
        }
    }
}

#[test]
fn custom_price_line_identity_visibility_and_lifecycle() {
    let (mut c, id) = chart();
    let mut o = PriceLineOptions::new(104.);
    o.external_id = Some("level".into());
    o.line_width = feather_charts::renderers::draw_line::LineWidth::Two;
    let line = c.add_price_line(id, o.clone()).unwrap();
    c.prepare(iced::Size::new(640., 480.), 1., std::time::Instant::now())
        .unwrap();
    let p = position(&c, id, 1., 104.);
    let hit = c.hit_test(0, p).expect("price-line hit");
    assert_eq!(
        hit.target,
        HitTarget::PriceLine {
            line: PriceLineKind::Custom(line),
            external_id: Some("level".into())
        }
    );
    o.line_visible = false;
    c.replace_price_line_options(line, o).unwrap();
    c.prepare(iced::Size::new(640., 480.), 1., std::time::Instant::now())
        .unwrap();
    assert!(c.hit_test(0, p).is_none());
    c.remove_price_line(line).unwrap();
    assert!(c.price_line_options(line).is_err());
}

#[test]
fn marker_foreground_beats_own_price_line_but_higher_source_wins() {
    let (mut c, id) = chart();
    let line = c.add_price_line(id, PriceLineOptions::new(104.)).unwrap();
    let mut m = SeriesMarker::new(
        Time::from(UtcTimestamp::new(1040.)),
        MarkerPosition::AtPriceMiddle(104.),
        MarkerShape::Circle,
        "red",
    );
    m.size = 3.;
    m.id = Some("marker".into());
    let collection = c
        .add_series_markers(
            id,
            vec![m],
            SeriesMarkersOptions {
                auto_scale: false,
                z_order: MarkerZOrder::Top,
            },
        )
        .unwrap();
    c.prepare(iced::Size::new(640., 480.), 1., std::time::Instant::now())
        .unwrap();
    let p = position(&c, id, 4., 104.);
    assert!(matches!(
        c.hit_test(0, p).unwrap().target,
        HitTarget::Marker(_)
    ));
    let upper = c
        .add_series(options(SeriesType::Line), 0, PriceScalePosition::Right)
        .unwrap();
    c.set_data(upper, (0..10).map(|i| row(SeriesType::Line, i)).collect())
        .unwrap();
    c.prepare(iced::Size::new(640., 480.), 1., std::time::Instant::now())
        .unwrap();
    assert_eq!(c.hit_test(0, p).unwrap().series, upper);
    assert_eq!(c.marker_hit(0, p).unwrap().collection, collection);
    let events = c
        .update(ChartMessage::Click {
            pane: 0,
            position: p,
        })
        .unwrap();
    assert!(events.iter().any(|e|matches!(e,ChartEvent::Click(d) if d.hit.as_ref().is_some_and(|h|h.series==upper) && d.marker.is_none())));
    c.remove_series(upper).unwrap();
    c.remove_series_markers(collection).unwrap();
    c.prepare(iced::Size::new(640., 480.), 1., std::time::Instant::now())
        .unwrap();
    assert_eq!(
        c.hit_test(0, p).unwrap().target,
        HitTarget::PriceLine {
            line: PriceLineKind::Custom(line),
            external_id: None
        }
    );
}
#[test]
fn stationary_pointer_reconciles_price_line_removal_once() {
    let (mut c, id) = chart();
    let line = c.add_price_line(id, PriceLineOptions::new(104.)).unwrap();
    c.prepare(iced::Size::new(640., 480.), 1., std::time::Instant::now())
        .unwrap();
    let p = position(&c, id, 1., 104.);
    c.update(ChartMessage::Click {
        pane: 0,
        position: p,
    })
    .unwrap();
    assert!(!c.is_animating());
    c.remove_price_line(line).unwrap();
    c.prepare(iced::Size::new(640., 480.), 1., std::time::Instant::now())
        .unwrap();
    assert!(c.is_animating());
    let events = c
        .update(ChartMessage::Tick(std::time::Instant::now()))
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e,ChartEvent::CrosshairMoved(d) if d.hit.is_none()))
    );
    assert!(!c.is_animating());
}

#[test]
fn ordinary_hover_order_is_temporary_and_filled_sources_split_without_repeating_fills() {
    use feather_charts::ui::line_chart::SeriesPassKind::*;
    for kind in [
        SeriesType::Line,
        SeriesType::Area,
        SeriesType::Baseline,
        SeriesType::Bar,
        SeriesType::Candlestick,
        SeriesType::Histogram,
    ] {
        let mut c = Chart::default();
        let lower = c
            .add_series(options(kind), 0, PriceScalePosition::Right)
            .unwrap();
        c.set_data(lower, (0..10).map(|i| row(kind, i)).collect())
            .unwrap();
        let upper = c
            .add_series(options(SeriesType::Line), 0, PriceScalePosition::Right)
            .unwrap();
        c.set_data(upper, (0..10).map(|i| row(SeriesType::Line, i)).collect())
            .unwrap();
        c.fit_content();
        c.prepare(iced::Size::new(640., 480.), 1.25, std::time::Instant::now())
            .unwrap();
        let mut retained = c.frame().plot.clone();
        retained.hovered_series = Some(lower);
        let passes: Vec<_> = retained
            .series_draw_passes()
            .map(|p| (p.layer, p.kind))
            .collect();
        if matches!(kind, SeriesType::Area | SeriesType::Baseline) {
            assert_eq!(passes, vec![(0, Background), (1, Normal), (0, Foreground)]);
        } else {
            assert_eq!(passes, vec![(1, Normal), (0, Normal)]);
        }
        let before = retained.hit_test(position(&c, lower, 3., 103.));
        retained.hovered_series_on_top = false;
        assert_eq!(
            retained
                .series_draw_passes()
                .map(|p| (p.layer, p.kind))
                .collect::<Vec<_>>(),
            vec![(0, Normal), (1, Normal)]
        );
        assert_eq!(retained.hit_test(position(&c, lower, 3., 103.)), before);
        assert_eq!(c.frame().plot.hovered_series, None);
        assert!(c.hovered_series_on_top());
        c.set_hovered_series_on_top(false);
        assert!(!c.frame().plot.hovered_series_on_top);
        assert_eq!(c.frame().plot.series_id, Some(lower));
        assert_eq!(c.frame().plot.series_layers[0].series_id, Some(upper));
    }
}
#[test]
fn raised_line_does_not_stick_at_equal_overlap_and_exit_clears_hover() {
    let (mut c, lower) = chart();
    let upper = c
        .add_series(options(SeriesType::Line), 0, PriceScalePosition::Right)
        .unwrap();
    let mut data: Vec<_> = (0..10).map(|i| row(SeriesType::Line, i)).collect();
    if let BuiltInSeriesDataItem::Line(LineDataItem::Data(d)) = &mut data[1] {
        d.value += 3.;
    }
    c.set_data(upper, data).unwrap();
    c.prepare(iced::Size::new(640., 480.), 1.25, std::time::Instant::now())
        .unwrap();
    let p = position(&c, lower, 1., 101.);
    c.update(ChartMessage::Click {
        pane: 0,
        position: p,
    })
    .unwrap();
    assert_eq!(c.frame().plot.hovered_series, Some(lower));
    assert_eq!(c.frame().plot.series_draw_passes().last().unwrap().layer, 0);
    let overlap = position(&c, lower, 4., 104.);
    c.update(ChartMessage::Click {
        pane: 0,
        position: overlap,
    })
    .unwrap();
    assert_eq!(c.frame().plot.hovered_series, Some(upper));
    c.update(ChartMessage::Interaction(
        feather_charts::ui::chart_input::ChartInteraction {
            position: None,
            navigation: None,
            capture: false,
        },
    ))
    .unwrap();
    assert_eq!(c.frame().plot.hovered_series, None);
    assert!(!c.is_animating());
}

#[test]
fn native_widget_reports_all_families_and_price_lines_before_draw() {
    use iced::{
        Point, Rectangle, mouse,
        widget::canvas::{Event, Program},
    };
    for kind in [
        SeriesType::Line,
        SeriesType::Area,
        SeriesType::Baseline,
        SeriesType::Bar,
        SeriesType::Candlestick,
        SeriesType::Histogram,
    ] {
        let mut c = Chart::default();
        let id = c
            .add_series(options(kind), 0, PriceScalePosition::Right)
            .unwrap();
        c.set_data(id, (0..10).map(|i| row(kind, i)).collect())
            .unwrap();
        let level = c.add_price_line(id, PriceLineOptions::new(106.)).unwrap();
        c.fit_content();
        let size = iced::Size::new(640., 480.);
        c.prepare(size, 1.5, std::time::Instant::now()).unwrap();
        for logical in [position(&c, id, 3., 103.), position(&c, id, 1., 106.)] {
            let expected = c.hit_test(0, logical).unwrap();
            assert_eq!(expected.series, id);
            if logical == position(&c, id, 1., 106.) {
                assert!(
                    matches!(expected.target,HitTarget::PriceLine{line:PriceLineKind::Custom(l),..} if l==level)
                );
            }
            let plot = c.frame().layout.plot;
            let p = Point::new(
                logical.x + plot.origin.x as f32,
                logical.y + plot.origin.y as f32,
            );
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
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e,ChartEvent::Click(p) if p.hit.as_ref()==Some(&expected)))
            );
        }
    }
}
#[test]
fn selection_tracks_modes_visibility_resize_and_independent_panes() {
    use feather_charts::model::{
        axis_snapshots::PriceAxisSide,
        price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
    };
    let mut c = Chart::default();
    let mut ids = vec![];
    for (pane, kind) in [
        SeriesType::Area,
        SeriesType::Histogram,
        SeriesType::Candlestick,
    ]
    .into_iter()
    .enumerate()
    {
        if pane > 0 {
            c.add_pane();
        }
        let id = c
            .add_series(options(kind), pane, PriceScalePosition::Right)
            .unwrap();
        c.set_data(id, (0..10).map(|i| row(kind, i)).collect())
            .unwrap();
        ids.push(id);
    }
    c.fit_content();
    for mode in [
        PriceScaleMode::Normal,
        PriceScaleMode::Logarithmic,
        PriceScaleMode::Percentage,
        PriceScaleMode::IndexedTo100,
    ] {
        for inverted in [false, true] {
            for (pane, &id) in ids.iter().enumerate() {
                c.apply_price_axis_options(
                    pane,
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
                        iced::Size::new(450., 600.),
                        ratio,
                        std::time::Instant::now(),
                    )
                    .unwrap();
                    let p = position(&c, id, 4., 104.);
                    assert_eq!(c.hit_test(pane, p).unwrap().series, id);
                    assert!(c.hit_test(ids.len(), p).is_none());
                    assert!(c.hit_test(pane, iced::Point::new(f32::NAN, p.y)).is_none());
                }
            }
        }
    }
    let mut hidden = options(SeriesType::Area);
    if let SeriesOptionsMap::Area(o) = &mut hidden {
        o.common.visible = false;
    }
    c.replace_series_options(ids[0], hidden).unwrap();
    c.prepare(iced::Size::new(450., 600.), 1., std::time::Instant::now())
        .unwrap();
    assert!(c.frame().plot.hit_series.is_empty());
    assert!(c.hit_test(0, iced::Point::new(100., 50.)).is_none());
}

#[test]
fn offscreen_history_is_not_retained_as_hover_geometry() {
    let (mut c, id) = chart();
    c.set_data(id, (0..20000).map(|i| row(SeriesType::Line, i)).collect())
        .unwrap();
    c.set_logical_range(feather_charts::model::time_data::LogicalRange {
        from: 19900.0.into(),
        to: 19999.0.into(),
    });
    c.prepare(iced::Size::new(640., 480.), 1.25, std::time::Instant::now())
        .unwrap();
    let feather_charts::ui::hit_testing::HitGeometry::Line { data, .. } =
        &c.frame().plot.hit_series[0].geometry
    else {
        panic!("Line geometry")
    };
    assert!(
        data.items.len() <= 104,
        "only visible rows plus curve neighbors may be retained"
    );
    assert_eq!(
        c.hit_test(0, position(&c, id, 19950., 20050.))
            .unwrap()
            .series,
        id
    );
}

#[test]
fn built_in_levels_duplicate_external_ids_and_clipped_lines_keep_distinct_identity() {
    use feather_charts::model::{
        axis_snapshots::PriceAxisSide,
        price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
    };
    let (mut c, id) = chart();
    let mut options = LineSeriesOptions::default();
    options.common.price_line_visible = true;
    c.replace_series_options(id, SeriesOptionsMap::Line(options))
        .unwrap();
    let mut a = PriceLineOptions::new(105.);
    a.external_id = Some("same".into());
    let first = c.add_price_line(id, a.clone()).unwrap();
    a.price = 107.;
    let second = c.add_price_line(id, a).unwrap();
    let mut duplicate_last = PriceLineOptions::new(109.);
    duplicate_last.external_id = Some("last".into());
    c.add_price_line(id, duplicate_last).unwrap();
    c.prepare(iced::Size::new(640., 480.), 1.25, std::time::Instant::now())
        .unwrap();
    for (price, line) in [(105., first), (107., second)] {
        let hit = c.hit_test(0, position(&c, id, 1., price)).unwrap();
        assert!(
            matches!(hit.target,HitTarget::PriceLine{line:PriceLineKind::Custom(l),external_id:Some(s)} if l==line &&s=="same")
        );
    }
    assert!(matches!(
        c.hit_test(0, position(&c, id, 1., 109.)).unwrap().target,
        HitTarget::PriceLine {
            line: PriceLineKind::LastValue,
            ..
        }
    ));
    for mode in [PriceScaleMode::Percentage, PriceScaleMode::IndexedTo100] {
        for invert in [false, true] {
            c.apply_price_axis_options(
                0,
                PriceAxisSide::Right,
                PriceScaleOptionsPatch {
                    mode: Some(mode),
                    invert_scale: Some(invert),
                    ..Default::default()
                },
            )
            .unwrap();
            c.prepare(iced::Size::new(640., 480.), 1.25, std::time::Instant::now())
                .unwrap();
            assert!(matches!(
                c.hit_test(0, position(&c, id, 7., 100.)).unwrap().target,
                HitTarget::PriceLine {
                    line: PriceLineKind::RelativeBase,
                    ..
                }
            ));
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
    c.prepare(iced::Size::new(640., 480.), 1.25, std::time::Instant::now())
        .unwrap();
    let offscreen_price = c.model().panes()[0]
        .right_price_scale()
        .coordinate_to_price((-5.).into(), 100.);
    let mut offscreen = PriceLineOptions::new(offscreen_price);
    offscreen.axis_label_visible = false;
    let clipped = c.add_price_line(id, offscreen).unwrap();
    c.prepare(iced::Size::new(640., 480.), 1.25, std::time::Instant::now())
        .unwrap();
    assert!(
        !c.frame()
            .plot
            .price_line_kinds
            .contains(&PriceLineKind::Custom(clipped))
    );
    assert!(
        matches!(c.hit_test(0,iced::Point::new(100.,1.)).unwrap().target,HitTarget::PriceLine{line:PriceLineKind::Custom(l),..} if l==clipped)
    );
}

#[test]
fn raw_hover_is_independent_of_magnet_and_reconciles_after_navigation() {
    let (mut c, id) = chart();
    let on = position(&c, id, 4., 104.);
    let raw = iced::Point::new(on.x, on.y + 20.);
    let events = c
        .update(ChartMessage::Click {
            pane: 0,
            position: raw,
        })
        .unwrap();
    let event = events
        .iter()
        .find_map(|e| match e {
            ChartEvent::Click(p) => Some(p),
            _ => None,
        })
        .unwrap();
    assert_eq!(event.point, Some(raw));
    assert!(event.hit.is_none());
    assert!(event.series_data.iter().any(|(series, _)| *series == id));
    assert_ne!(
        c.model().crosshair().position().unwrap().y.unwrap() as f32,
        raw.y
    );
    c.update(ChartMessage::Click {
        pane: 0,
        position: on,
    })
    .unwrap();
    assert_eq!(c.frame().plot.hovered_series, Some(id));
    c.set_logical_range(feather_charts::model::time_data::LogicalRange {
        from: 8.0.into(),
        to: 12.0.into(),
    });
    c.prepare(iced::Size::new(450., 300.), 1.5, std::time::Instant::now())
        .unwrap();
    let expected = c.hit_test(0, on);
    let events = c
        .update(ChartMessage::Tick(std::time::Instant::now()))
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e,ChartEvent::CrosshairMoved(p) if p.hit==expected))
    );
    assert!(!c.is_animating());
    assert!(
        c.update(ChartMessage::Tick(std::time::Instant::now()))
            .unwrap()
            .is_empty()
    );
}
