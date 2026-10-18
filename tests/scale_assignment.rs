use feather_charts::{
    api::Chart,
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        horz_scale_behavior_time::types::{Time, TimePoint, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{CustomSeriesOptions, LineSeriesOptions, SeriesOptionsMap},
    },
};
#[path = "numeric_custom_series/support.rs"]
mod support;
fn time(t: f64) -> Time {
    UtcTimestamp::new(t).into()
}
fn row(t: f64, v: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: time(t),
        value: v,
        color: None,
        custom_values: None,
    }))
}
#[test]
fn option_replacement_moves_builtin_and_custom_membership() {
    let mut c = Chart::default();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(id, vec![row(1., 10.), row(2., 20.)]).unwrap();
    let h = c
        .add_custom_series(
            support::View::<Time, TimePoint>::default(),
            Default::default(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_custom_data(
        &h,
        vec![support::datum(time(1.), 10.), support::datum(time(2.), 20.)],
    )
    .unwrap();
    let mut options = LineSeriesOptions::default();
    options.common.price_scale_id = Some("left".into());
    c.replace_series_options(id, SeriesOptionsMap::Line(options))
        .unwrap();
    c.fit_content();
    c.prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    assert!(
        !c.model().panes()[0].left_price_scale().is_empty(),
        "new left scale must own data after replacement"
    );
    let mut options = CustomSeriesOptions::default();
    options.common.price_scale_id = Some("volume".into());
    c.replace_custom_series_options(&h, options).unwrap();
    assert!(c.model().panes()[0].price_scale_by_id("volume").is_some());
    assert_eq!(c.series_data(id).unwrap().len(), 2);
    assert_eq!(h.data().len(), 2);
}

#[test]
fn source_scale_assignment_matrix() {
    use feather_charts::api::PriceRange;
    for custom in [false, true] {
        let mut c = Chart::default();
        c.add_pane();
        let h = if custom {
            Some(
                c.add_custom_series(
                    support::View::<Time, TimePoint>::default(),
                    Default::default(),
                    0,
                    PriceScalePosition::Right,
                )
                .unwrap(),
            )
        } else {
            None
        };
        let id = if let Some(h) = &h {
            c.set_custom_data(
                h,
                vec![
                    support::datum(time(1.), 10.),
                    support::datum(time(2.), 20.),
                    support::datum(time(3.), 30.),
                ],
            )
            .unwrap();
            h.id()
        } else {
            let id = c
                .add_series(
                    SeriesOptionsMap::Line(Default::default()),
                    0,
                    PriceScalePosition::Right,
                )
                .unwrap();
            c.set_data(id, vec![row(1., 10.), row(2., 20.), row(3., 30.)])
                .unwrap();
            id
        };
        for (position, base) in [
            (PriceScalePosition::Right, 40.),
            (PriceScalePosition::Overlay("shared".into()), 90.),
        ] {
            let peer = c
                .add_series(SeriesOptionsMap::Line(Default::default()), 0, position)
                .unwrap();
            c.set_data(
                peer,
                vec![
                    row(1., base + 10.),
                    row(2., base + 20.),
                    row(3., base + 30.),
                ],
            )
            .unwrap();
        }
        c.set_series_order(id, 1).unwrap();
        c.fit_content();
        c.prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
            .unwrap();
        c.set_visible_price_range(
            0,
            &PriceScalePosition::Overlay("shared".into()),
            PriceRange {
                min_value: 0.,
                max_value: 200.,
            },
        )
        .unwrap();
        for (stage, target) in [
            ("initial", None),
            ("left", Some("left")),
            ("same-left", Some("left")),
            ("shared", Some("shared")),
            ("other", Some("other")),
            ("empty", Some("")),
            ("right", Some("right")),
            ("omitted", None),
            ("new-pane", None),
            ("left-new-pane", Some("left")),
        ] {
            let before = c.series_price_scale(id).unwrap();
            if stage == "new-pane" {
                c.move_series_to_pane(id, 1).unwrap();
            } else if stage != "initial" {
                if let Some(h) = &h {
                    let mut opts = CustomSeriesOptions::default();
                    opts.common.price_scale_id = target.map(String::from);
                    c.replace_custom_series_options(h, opts).unwrap();
                } else {
                    let mut opts = LineSeriesOptions::default();
                    opts.common.price_scale_id = target.map(String::from);
                    c.replace_series_options(id, SeriesOptionsMap::Line(opts))
                        .unwrap();
                }
            }
            c.prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
                .unwrap();
            let position = c.series_price_scale(id).unwrap();
            let pane = c.model().pane_for_series(id).unwrap();
            let option = if let Some(h) = &h {
                h.price_scale_id().unwrap()
            } else {
                c.model()
                    .series(id)
                    .unwrap()
                    .common_options()
                    .price_scale_id
                    .clone()
                    .unwrap()
            };
            assert_eq!(option, position.id());
            let old = if let PriceScalePosition::Overlay(name) = before {
                Some(c.model().panes()[pane].price_scale_by_id(&name).is_some())
            } else {
                None
            };
            let shared = c.model().panes()[pane]
                .price_scale_by_id("shared")
                .map(|s| s.is_auto_scale());
            let json = |v: Option<bool>| v.map_or("null".into(), |v| v.to_string());
            let order = c.series_order(id).unwrap();
            let points = if let Some(h) = &h {
                h.data().len()
            } else {
                c.series_data(id).unwrap().len()
            };
            println!(
                "SCALE_REFERENCE {{\"custom\":{custom},\"stage\":{stage:?},\"position\":{:?},\"option\":{option:?},\"order\":{order},\"pane\":{pane},\"points\":{points},\"oldOverlayAlive\":{},\"sharedAuto\":{}}}",
                position.id(),
                json(old),
                json(shared)
            );
        }
    }
}

fn prepared_chart() -> (Chart, feather_charts::model::data_layer::SeriesId) {
    let mut c = Chart::default();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(id, (1..30).map(|i| row(i as f64, i as f64)).collect())
        .unwrap();
    c.fit_content();
    c.prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    (c, id)
}
fn start_pan(c: &mut Chart) {
    use feather_charts::{
        api::ChartMessage,
        ui::chart_input::{ChartInteraction, Navigation},
    };
    let point = iced::Point::new(200., 100.);
    c.update(ChartMessage::Interaction(ChartInteraction {
        position: Some(point),
        navigation: Some(Navigation::Pan {
            start: Some(point),
            position: point,
        }),
        capture: true,
    }))
    .unwrap();
    assert!(c.is_panning());
}
#[test]
fn checked_assignment_preserves_annotations_order_data_and_noop_interaction() {
    let (mut c, id) = prepared_chart();
    let other = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_series_order(id, 1).unwrap();
    let line = c
        .add_price_line(id, feather_charts::api::PriceLineOptions::new(15.))
        .unwrap();
    let order = c.series_order(id).unwrap();
    let rows = c.series_data(id).unwrap();
    start_pan(&mut c);
    let generation = c.model().panes()[0].right_price_scale().marks_generation();
    c.set_series_price_scale(id, PriceScalePosition::Right)
        .unwrap();
    assert!(c.is_panning());
    assert_eq!(
        c.model().panes()[0].right_price_scale().marks_generation(),
        generation
    );
    assert!(
        c.set_series_price_scale(id, PriceScalePosition::Overlay("left".into()))
            .is_err()
    );
    assert!(c.is_panning());
    let mut foreign = Chart::default();
    let foreign_id = foreign
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    assert!(
        c.set_series_price_scale(foreign_id, PriceScalePosition::Left)
            .is_err()
    );
    assert!(c.is_panning());
    c.set_series_price_scale(id, PriceScalePosition::Left)
        .unwrap();
    assert!(!c.is_panning());
    assert_eq!(c.series_price_scale(id).unwrap(), PriceScalePosition::Left);
    assert_eq!(
        c.model()
            .series(id)
            .unwrap()
            .common_options()
            .price_scale_id
            .as_deref(),
        Some("left")
    );
    assert_eq!(c.series_order(id).unwrap(), order);
    assert_eq!(c.series_data(id).unwrap(), rows);
    assert_eq!(c.price_line_options(line).unwrap().price, 15.);
    assert_eq!(
        c.series_price_scale(other).unwrap(),
        PriceScalePosition::Right
    );
}
#[test]
fn invalid_mixed_option_replacements_preserve_membership_options_and_pan() {
    use feather_charts::model::series_options::AreaSeriesOptions;
    let mut c = Chart::default();
    let id = c
        .add_series(
            SeriesOptionsMap::Area(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let custom = c
        .add_custom_series(
            support::View::<Time, TimePoint>::default(),
            Default::default(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(id, vec![]).unwrap();
    c.prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    // Plot panning needs fulfilled bars; use a populated line alongside the candidates.
    let peer = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(peer, (1..30).map(|i| row(i as f64, i as f64)).collect())
        .unwrap();
    c.fit_content();
    c.prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    start_pan(&mut c);
    let mut options = AreaSeriesOptions::default();
    options.common.price_scale_id = Some("left".into());
    options.style.line_color = "invalid-color".into();
    assert!(
        c.replace_series_options(id, SeriesOptionsMap::Area(options))
            .is_err()
    );
    assert!(c.is_panning());
    assert_eq!(c.series_price_scale(id).unwrap(), PriceScalePosition::Right);
    assert_eq!(
        c.model()
            .series(id)
            .unwrap()
            .common_options()
            .price_scale_id
            .as_deref(),
        Some("right")
    );
    let mut options = CustomSeriesOptions::default();
    options.common.price_scale_id = Some("left".into());
    options.style.color = "invalid-color".into();
    assert!(c.replace_custom_series_options(&custom, options).is_err());
    assert!(c.is_panning());
    assert_eq!(
        c.series_price_scale(custom.id()).unwrap(),
        PriceScalePosition::Right
    );
    assert_eq!(custom.price_scale_id().as_deref(), Some("right"));
    assert!(
        c.replace_series_options(id, SeriesOptionsMap::Line(Default::default()))
            .is_err()
    );
    assert!(c.is_panning());
}
#[test]
fn creation_selector_wins_and_omitted_replacement_retains_assignment_after_pane_move() {
    let mut c = Chart::default();
    let mut options = LineSeriesOptions::default();
    options.common.price_scale_id = Some("left".into());
    let id = c
        .add_series(
            SeriesOptionsMap::Line(options),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    assert_eq!(
        c.model()
            .series(id)
            .unwrap()
            .common_options()
            .price_scale_id
            .as_deref(),
        Some("right")
    );
    let mut options = CustomSeriesOptions::default();
    options.common.price_scale_id = Some("different".into());
    let h = c
        .add_custom_series(
            support::View::<Time, TimePoint>::default(),
            options,
            0,
            PriceScalePosition::Overlay("".into()),
        )
        .unwrap();
    assert_eq!(h.price_scale_id().as_deref(), Some(""));
    c.move_series_to_pane(h.id(), 1).unwrap();
    c.replace_custom_series_options(&h, Default::default())
        .unwrap();
    assert_eq!(
        c.series_price_scale(h.id()).unwrap(),
        PriceScalePosition::Overlay("".into())
    );
    assert_eq!(h.price_scale_id().as_deref(), Some(""));
    c.remove_series(h.id()).unwrap();
    assert!(c.series_price_scale(h.id()).is_err());
    assert_eq!(h.price_scale_id().as_deref(), Some(""));
}
#[test]
fn shared_manual_overlay_survives_until_last_source_leaves_then_recreates_automatic() {
    use feather_charts::api::PriceRange;
    let (mut c, id) = prepared_chart();
    let peer = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(peer, vec![row(1., 10.), row(2., 20.)]).unwrap();
    let overlay = PriceScalePosition::Overlay("shared".into());
    c.set_series_price_scale(id, overlay.clone()).unwrap();
    c.set_visible_price_range(
        0,
        &overlay,
        PriceRange {
            min_value: 0.,
            max_value: 200.,
        },
    )
    .unwrap();
    c.set_series_price_scale(peer, overlay.clone()).unwrap();
    c.set_series_price_scale(id, PriceScalePosition::Right)
        .unwrap();
    assert!(!c.price_scale_options(0, &overlay).unwrap().auto_scale);
    assert_eq!(
        c.visible_price_range(0, &overlay).unwrap(),
        Some(PriceRange {
            min_value: 0.,
            max_value: 200.
        })
    );
    c.set_series_price_scale(peer, PriceScalePosition::Right)
        .unwrap();
    assert!(c.price_scale_options(0, &overlay).is_err());
    c.set_series_price_scale(id, overlay.clone()).unwrap();
    assert!(c.price_scale_options(0, &overlay).unwrap().auto_scale);
}

#[test]
fn transfers_keep_custom_view_alive_and_do_not_clone_canonical_payloads() {
    use std::{cell::Cell, rc::Rc};
    let mut c = Chart::default();
    let view = support::View::<Time, TimePoint>::default();
    let destroyed = view.destroyed.clone();
    let h = c
        .add_custom_series(view, Default::default(), 0, PriceScalePosition::Right)
        .unwrap();
    let clones = Rc::new(Cell::new(0));
    let rows = (1..4)
        .map(|i| {
            feather_charts::model::icustom_series::CustomSeriesDataItem::Data(support::Datum {
                time: time(i as f64),
                value: i as f64,
                low: i as f64 - 1.,
                high: i as f64 + 1.,
                clones: Some(clones.clone()),
            })
        })
        .collect();
    c.set_custom_data(&h, rows).unwrap();
    let before = clones.get();
    for position in [
        PriceScalePosition::Left,
        PriceScalePosition::Overlay("new".into()),
        PriceScalePosition::Right,
    ] {
        c.set_series_price_scale(h.id(), position).unwrap();
        assert_eq!(destroyed.get(), 0);
        assert_eq!(clones.get(), before);
    }
    c.remove_series(h.id()).unwrap();
    assert_eq!(destroyed.get(), 1);
    assert_eq!(h.data().len(), 3);
}

#[test]
fn diagonal_plot_drag_pans_the_overlay_after_reassignment() {
    use feather_charts::{
        api::ChartMessage,
        ui::chart_input::{ChartInteraction, Navigation},
    };
    let (mut c, id) = prepared_chart();
    let position = PriceScalePosition::Overlay("only".into());
    c.set_series_price_scale(id, position.clone()).unwrap();
    c.prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
        .unwrap();
    let before = c.visible_price_range(0, &position).unwrap().unwrap();
    start_pan(&mut c);
    let point = iced::Point::new(220., 140.);
    c.update(ChartMessage::Interaction(ChartInteraction {
        position: Some(point),
        navigation: Some(Navigation::Pan {
            start: None,
            position: point,
        }),
        capture: true,
    }))
    .unwrap();
    assert_ne!(
        c.visible_price_range(0, &position).unwrap(),
        Some(before),
        "plot dragging must pan the actual default overlay scale"
    );
    assert!(!c.price_scale_options(0, &position).unwrap().auto_scale);
}
