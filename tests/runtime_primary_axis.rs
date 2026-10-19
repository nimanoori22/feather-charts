use feather_charts::{
    api::*,
    model::{
        data_consumer::*,
        data_layer::SeriesId,
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        price_scale::PriceScaleOptionsPatch,
        series_options::SeriesOptionsMap,
    },
};
use iced::Size;
use std::time::Instant;

fn row(t: f64, value: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: UtcTimestamp::new(t).into(),
        value,
        color: None,
        custom_values: None,
    }))
}
fn prepare(chart: &mut Chart) {
    chart
        .prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
}
fn patch(side: DefaultVisiblePriceScale) -> ChartOptionsPatch {
    ChartOptionsPatch {
        model: ChartModelOptionsPatch {
            default_visible_price_scale: Some(side),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn chart() -> (Chart, SeriesId, SeriesId) {
    let mut chart = Chart::default();
    chart
        .apply_chart_options(ChartOptionsPatch {
            model: ChartModelOptionsPatch {
                left_price_scale: Some(PriceScaleOptionsPatch {
                    visible: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    let left = chart
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Left,
        )
        .unwrap();
    let right = chart
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(left, vec![row(1., 10.), row(2., 20.)])
        .unwrap();
    chart
        .set_data(right, vec![row(1., 100.), row(2., 200.)])
        .unwrap();
    chart.fit_content();
    prepare(&mut chart);
    (chart, left, right)
}
#[test]
fn preference_updates_existing_future_and_reordered_panes_without_moving_series() {
    let (mut c, left, right) = chart();
    let data = c.series_data(left).unwrap();
    let old = c.runtime_options();
    c.add_pane();
    c.apply_chart_options(patch(DefaultVisiblePriceScale::Left))
        .unwrap();
    assert_eq!(
        c.runtime_options().model.pane.default_visible_price_scale,
        DefaultVisiblePriceScale::Left
    );
    for p in c.model().panes() {
        assert_eq!(p.default_visible_price_scale().unwrap().id(), "left");
    }
    assert_eq!(c.model().panes()[0].default_price_scale().id(), "left");
    let added = c.add_pane();
    assert_eq!(
        c.model().panes()[added]
            .default_visible_price_scale()
            .unwrap()
            .id(),
        "left"
    );
    c.swap_panes(0, added).unwrap();
    c.apply_chart_options(patch(DefaultVisiblePriceScale::Right))
        .unwrap();
    for p in c.model().panes() {
        assert_eq!(p.default_visible_price_scale().unwrap().id(), "right");
    }
    assert_eq!(
        c.series_price_scale(left).unwrap(),
        PriceScalePosition::Left
    );
    assert_eq!(
        c.series_price_scale(right).unwrap(),
        PriceScalePosition::Right
    );
    assert_eq!(c.series_data(left).unwrap(), data);
    assert_eq!(
        old.model.pane.default_visible_price_scale,
        DefaultVisiblePriceScale::Right
    );
}
#[test]
fn invalid_mixed_patch_is_atomic_and_noop_preserves_pending_data_events() {
    let (mut c, left, _) = chart();
    c.set_event_interests(EventInterests::ALL);
    c.update_data(left, row(3., 30.), false).unwrap();
    let before = c.runtime_options();
    let mut invalid = patch(DefaultVisiblePriceScale::Left);
    invalid.model.layout = Some(LayoutOptionsPatch {
        font_size: Some(f64::NAN),
        ..Default::default()
    });
    assert!(c.apply_chart_options(invalid).is_err());
    assert_eq!(c.runtime_options(), before);
    c.apply_chart_options(patch(DefaultVisiblePriceScale::Right))
        .unwrap();
    assert_eq!(c.runtime_options(), before);
    assert!(
        c.take_events()
            .iter()
            .any(|e| matches!(e,ChartEvent::DataChanged {series,..} if *series==left))
    );
}
#[test]
fn hidden_or_unpopulated_preferred_axis_keeps_source_fallbacks() {
    let (mut c, left, _) = chart();
    c.apply_chart_options(patch(DefaultVisiblePriceScale::Left))
        .unwrap();
    assert_eq!(c.model().panes()[0].default_price_scale().id(), "left");
    c.apply_price_scale_options(
        0,
        &PriceScalePosition::Left,
        PriceScaleOptionsPatch {
            visible: Some(false),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        c.model().panes()[0]
            .default_visible_price_scale()
            .unwrap()
            .id(),
        "right"
    );
    assert_eq!(c.model().panes()[0].default_price_scale().id(), "right");
    c.apply_price_scale_options(
        0,
        &PriceScalePosition::Right,
        PriceScaleOptionsPatch {
            visible: Some(false),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(c.model().panes()[0].default_visible_price_scale().is_none());
    assert_eq!(c.model().panes()[0].default_price_scale().id(), "left");
    c.remove_series(left).unwrap();
    assert_eq!(c.model().panes()[0].default_price_scale().id(), "right");
}

#[test]
fn source_primary_axis_states() {
    let (mut c, left, right) = chart();
    let mut states = Vec::new();
    let mut emit = |c: &Chart, name: &str, pane: usize| {
        let preference = match c.runtime_options().model.pane.default_visible_price_scale {
            DefaultVisiblePriceScale::Left => "left",
            DefaultVisiblePriceScale::Right => "right",
        };
        let p = &c.model().panes()[pane];
        let visible = p.default_visible_price_scale().map_or("none", |s| s.id());
        states.push(format!(r#"{{"name":"{name}","preference":"{preference}","primary":"{}","visible":"{visible}"}}"#,p.default_price_scale().id()));
    };
    emit(&c, "initial", 0);
    c.apply_chart_options(patch(DefaultVisiblePriceScale::Left))
        .unwrap();
    emit(&c, "left", 0);
    c.add_pane();
    emit(&c, "new-empty", 1);
    c.swap_panes(0, 1).unwrap();
    emit(&c, "moved", 1);
    c.apply_chart_options(patch(DefaultVisiblePriceScale::Right))
        .unwrap();
    emit(&c, "right", 1);
    emit(&c, "right-empty", 0);
    c.apply_price_scale_options(
        1,
        &PriceScalePosition::Right,
        PriceScaleOptionsPatch {
            visible: Some(false),
            ..Default::default()
        },
    )
    .unwrap();
    emit(&c, "right-hidden", 1);
    c.apply_price_scale_options(
        1,
        &PriceScalePosition::Left,
        PriceScaleOptionsPatch {
            visible: Some(false),
            ..Default::default()
        },
    )
    .unwrap();
    emit(&c, "both-hidden", 1);
    c.remove_series(left).unwrap();
    emit(&c, "only-right-hidden", 1);
    c.remove_series(right).unwrap();
    emit(&c, "empty-fallback", 0);
    println!("PRIMARY_REFERENCE [{}]", states.join(","));
}

#[test]
fn grid_and_overlay_labels_follow_preference_and_retained_frames_stay_owned() {
    let (mut c, _, _) = chart();
    let overlay = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Overlay("overlay".into()),
        )
        .unwrap();
    c.set_data(overlay, vec![row(1., 5.), row(2., 8.)]).unwrap();
    let mut retained = None;
    for side in [
        DefaultVisiblePriceScale::Left,
        DefaultVisiblePriceScale::Right,
    ] {
        c.apply_chart_options(patch(side)).unwrap();
        prepare(&mut c);
        let axes = c.frame().plot.axes.as_ref().unwrap();
        let expected = if side == DefaultVisiblePriceScale::Left {
            &axes.left
        } else {
            &axes.right
        };
        let other = if side == DefaultVisiblePriceScale::Left {
            &axes.right
        } else {
            &axes.left
        };
        let grid = c.model().panes()[0]
            .grid()
            .pane_view()
            .renderer()
            .data()
            .unwrap();
        assert!(!expected.marks.is_empty());
        assert_eq!(grid.price_marks, expected.marks);
        assert!(expected.labels.iter().any(|l| l.source == Some(overlay)));
        assert!(!other.labels.iter().any(|l| l.source == Some(overlay)));
        if retained.is_none() {
            retained = Some(c.frame().clone());
        }
    }
    let old = retained.unwrap();
    assert!(
        old.plot
            .axes
            .as_ref()
            .unwrap()
            .left
            .labels
            .iter()
            .any(|l| l.source == Some(overlay))
    );
    assert!(
        !old.plot
            .axes
            .as_ref()
            .unwrap()
            .right
            .labels
            .iter()
            .any(|l| l.source == Some(overlay))
    );
}

#[test]
fn changed_preference_ends_pan_but_equal_patch_keeps_it() {
    use feather_charts::ui::chart_input::{ChartInteraction, Navigation};
    let (mut c, _, _) = chart();
    let pan = |start, position| {
        ChartMessage::Interaction(ChartInteraction {
            position: None,
            navigation: Some(Navigation::Pan { start, position }),
            capture: true,
        })
    };
    c.update(pan(
        Some(iced::Point::new(400., 150.)),
        iced::Point::new(400., 150.),
    ))
    .unwrap();
    assert!(c.is_panning());
    c.apply_chart_options(patch(DefaultVisiblePriceScale::Right))
        .unwrap();
    assert!(c.is_panning());
    let mut invalid = patch(DefaultVisiblePriceScale::Left);
    invalid.model.layout = Some(LayoutOptionsPatch {
        font_size: Some(f64::NAN),
        ..Default::default()
    });
    assert!(c.apply_chart_options(invalid).is_err());
    assert!(c.is_panning());
    c.apply_chart_options(patch(DefaultVisiblePriceScale::Left))
        .unwrap();
    assert!(!c.is_panning());
    let after = c.scroll_position();
    c.update(pan(None, iced::Point::new(250., 150.))).unwrap();
    assert_eq!(c.scroll_position(), after);
}

#[path = "numeric_custom_series/support.rs"]
mod support;
#[test]
fn custom_data_manual_ranges_and_crosshair_use_the_current_primary_axis() {
    use feather_charts::model::{
        crosshair::CrosshairMode, horz_scale_behavior_time::types::TimePoint,
        price_range_impl::PriceRange,
    };
    let (mut c, left, right) = chart();
    let custom = c
        .add_custom_series(
            support::View::<Time, TimePoint>::default(),
            Default::default(),
            0,
            PriceScalePosition::Left,
        )
        .unwrap();
    c.set_custom_data(
        &custom,
        vec![
            support::datum(UtcTimestamp::new(1.).into(), 12.),
            support::datum(UtcTimestamp::new(2.).into(), 15.),
        ],
    )
    .unwrap();
    let data = custom.data();
    c.set_visible_price_range(
        0,
        &PriceScalePosition::Left,
        PriceRange {
            min_value: 0.,
            max_value: 20.,
        },
    )
    .unwrap();
    c.set_visible_price_range(
        0,
        &PriceScalePosition::Right,
        PriceRange {
            min_value: 100.,
            max_value: 200.,
        },
    )
    .unwrap();
    let mut crosshair = c.runtime_options().model.crosshair;
    crosshair.mode = CrosshairMode::Normal;
    c.apply_chart_options(ChartOptionsPatch {
        model: ChartModelOptionsPatch {
            crosshair: Some(crosshair),
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
    c.set_event_interests(EventInterests::ALL);
    for (side, id) in [
        (DefaultVisiblePriceScale::Left, left),
        (DefaultVisiblePriceScale::Right, right),
    ] {
        c.apply_chart_options(patch(side)).unwrap();
        prepare(&mut c);
        let expected = c.coordinate_to_price(id, 100.).unwrap().unwrap();
        let events = c
            .update(ChartMessage::Click {
                pane: 0,
                position: iced::Point::new(300., 100.),
            })
            .unwrap();
        let price = events
            .iter()
            .find_map(|e| {
                if let ChartEvent::Click(p) = e {
                    p.price
                } else {
                    None
                }
            })
            .unwrap();
        assert!((price - expected).abs() < 1e-8);
        assert!(
            !c.price_scale_options(0, &PriceScalePosition::Left)
                .unwrap()
                .auto_scale
        );
        assert!(
            !c.price_scale_options(0, &PriceScalePosition::Right)
                .unwrap()
                .auto_scale
        );
        assert_eq!(custom.data(), data);
        assert_eq!(
            c.series_price_scale(custom.id()).unwrap(),
            PriceScalePosition::Left
        );
    }
}
