use feather_charts::{
    api::{
        Chart, ChartMessage, EventInterests, LocalizationOptionsPatch, YieldCurveChart,
        YieldCurveOptions,
    },
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        ihorz_scale_behavior::HorzScaleBehavior,
        pane::PriceScalePosition,
        price_formatter_fn::PriceFormatterFn,
        series_options::{FieldPatch, SeriesOptionsMap},
    },
    renderers::price_axis_renderer::PriceAxisCommand,
    ui::chart_input::{ChartInteraction, Navigation},
};
use std::{rc::Rc, time::Instant};
fn row(t: f64, value: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: UtcTimestamp::new(t).into(),
        value,
        color: None,
        custom_values: None,
    }))
}
fn chart() -> (Chart, feather_charts::model::data_layer::SeriesId) {
    let mut c = Chart::default();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(id, (1..50).map(|i| row(i as f64, i as f64)).collect())
        .unwrap();
    c.fit_content();
    c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    (c, id)
}
#[test]
fn runtime_patch_updates_real_labels_preserves_old_snapshot_and_clears_callbacks() {
    let (mut c, id) = chart();
    let old = c
        .prepare_snapshot(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let format: PriceFormatterFn = Rc::new(|_| "runtime-price".into());
    let weak = Rc::downgrade(&format);
    c.apply_localization_options(LocalizationOptionsPatch {
        locale: Some("de-DE".into()),
        date_format: Some("yyyy-MM-dd".into()),
        price_formatter: FieldPatch::Set(format),
        tickmarks_price_formatter: FieldPatch::Set(Rc::new(|v| {
            v.iter().map(|_| "runtime-tick".into()).collect()
        })),
        percentage_formatter: FieldPatch::Set(Rc::new(|_| "runtime-percent".into())),
        time_formatter: FieldPatch::Set(Rc::new(|_| "runtime-time".into())),
        ..Default::default()
    });
    let readback = c.localization_options();
    assert_eq!(readback.base.locale, "de-DE");
    assert_eq!(readback.date_format, "yyyy-MM-dd");
    let now = c
        .prepare_snapshot(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    assert!(
        now.panes[0]
            .right
            .commands
            .iter()
            .any(|cmd| matches!(cmd,PriceAxisCommand::Text{label,..} if label=="runtime-tick"))
    );
    assert!(!old.panes[0].right.commands.iter().any(
        |cmd| matches!(cmd,PriceAxisCommand::Text{label,..} if label.starts_with("runtime-"))
    ));
    let scale = c.model().panes()[0].right_price_scale();
    assert_eq!(scale.format_price_absolute(20.), "runtime-price");
    assert_eq!(scale.format_price_percentage(20., 10.), "runtime-percent");
    let behavior = c.model().time_scale().behavior();
    assert_eq!(
        behavior.format_item(&behavior.to_internal(&UtcTimestamp::new(1.).into()).unwrap()),
        "runtime-time"
    );
    assert_eq!(c.series_data(id).unwrap().len(), 49);
    c.apply_localization_options(LocalizationOptionsPatch {
        price_formatter: FieldPatch::Clear,
        tickmarks_price_formatter: FieldPatch::Clear,
        percentage_formatter: FieldPatch::Clear,
        time_formatter: FieldPatch::Clear,
        ..Default::default()
    });
    c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    assert_eq!(
        c.model().panes()[0]
            .right_price_scale()
            .format_price_absolute(20.),
        "20.00"
    );
    let cleared = c
        .prepare_snapshot(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    assert!(!cleared.panes[0].right.commands.iter().any(
        |cmd| matches!(cmd,PriceAxisCommand::Text{label,..} if label.starts_with("runtime-"))
    ));

    assert_eq!(c.localization_options().base.locale, "de-DE");
    assert!(weak.upgrade().is_some(), "detached readback owns callback");
    drop(readback);
    assert!(
        weak.upgrade().is_none(),
        "old prepared snapshot must not retain callback"
    );
}
#[test]
fn empty_and_identity_equal_patches_preserve_events_marks_and_active_pan() {
    let (mut c, id) = chart();
    let format: PriceFormatterFn = Rc::new(|_| "same".into());
    c.apply_localization_options(LocalizationOptionsPatch {
        price_formatter: FieldPatch::Set(format.clone()),
        ..Default::default()
    });
    c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    c.set_event_interests(EventInterests {
        data_changed: true,
        ..Default::default()
    });
    c.update(ChartMessage::Interaction(ChartInteraction {
        position: Some(iced::Point::new(200., 200.)),
        navigation: Some(Navigation::Pan {
            start: Some(iced::Point::new(200., 200.)),
            position: iced::Point::new(200., 200.),
        }),
        capture: true,
    }))
    .unwrap();
    assert!(c.is_panning());
    c.update_data(id, row(49., 50.), false).unwrap();
    let before = c.model().panes()[0].right_price_scale().marks_generation();
    c.apply_localization_options(Default::default());
    c.apply_localization_options(LocalizationOptionsPatch {
        price_formatter: FieldPatch::Set(format),
        locale: Some("en-US".into()),
        ..Default::default()
    });
    assert_eq!(
        c.model().panes()[0].right_price_scale().marks_generation(),
        before
    );
    assert!(c.is_panning());
    c.apply_localization_options(LocalizationOptionsPatch {
        price_formatter: FieldPatch::Set(Rc::new(|_| "changed".into())),
        ..Default::default()
    });
    assert!(c.is_panning());
    assert!(c.take_events().iter().any(
        |e| matches!(e,feather_charts::api::ChartEvent::DataChanged{series,..} if *series==id)
    ));
}
#[test]
fn yield_runtime_clear_reveals_ordinary_format_for_existing_and_future_series() {
    let mut c = YieldCurveChart::new(YieldCurveOptions::default()).unwrap();
    assert!(c.localization_options().base.price_formatter.is_some());
    let id = c
        .add_line_series(Default::default(), 0, PriceScalePosition::Left)
        .unwrap();
    c.apply_localization_options(LocalizationOptionsPatch {
        price_formatter: FieldPatch::Clear,
        ..Default::default()
    });
    assert!(c.localization_options().base.price_formatter.is_none());
    let other = c
        .add_line_series(Default::default(), 0, PriceScalePosition::Left)
        .unwrap();
    for id in [id, other] {
        assert_eq!(c.model().series(id).unwrap().formatter().format(3.), "3.00");
    }
}

#[test]
fn runtime_percentage_batch_clear_and_new_overlay_inherit_policy() {
    use feather_charts::model::price_scale::{PriceScaleMode, PriceScaleOptionsPatch};
    let (mut c, _) = chart();
    c.apply_localization_options(LocalizationOptionsPatch {
        percentage_formatter: FieldPatch::Set(Rc::new(|_| "percent-scalar".into())),
        tickmarks_percentage_formatter: FieldPatch::Set(Rc::new(|v| {
            v.iter().map(|_| "percent-batch".into()).collect()
        })),
        ..Default::default()
    });
    let pane = c.add_pane();
    let position = PriceScalePosition::Overlay("new".into());
    let id = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            pane,
            position.clone(),
        )
        .unwrap();
    c.set_data(id, vec![row(1., 10.), row(2., 20.)]).unwrap();
    c.apply_price_scale_options(
        pane,
        &position,
        PriceScaleOptionsPatch {
            mode: Some(PriceScaleMode::Percentage),
            ..Default::default()
        },
    )
    .unwrap();
    c.prepare(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let scale = c.model().panes()[pane].price_scale_by_id("new").unwrap();
    assert_eq!(scale.format_logical(10.), "percent-scalar");
    assert_eq!(
        scale.format_logical_tickmarks(&[10., 20.]),
        vec!["percent-batch", "percent-batch"]
    );
    c.apply_localization_options(LocalizationOptionsPatch {
        tickmarks_percentage_formatter: FieldPatch::Clear,
        ..Default::default()
    });
    let scale = c.model().panes()[pane].price_scale_by_id("new").unwrap();
    assert_eq!(
        scale.format_logical_tickmarks(&[10., 20.]),
        vec!["percent-scalar", "percent-scalar"]
    );
    c.apply_localization_options(LocalizationOptionsPatch {
        percentage_formatter: FieldPatch::Clear,
        ..Default::default()
    });
    let scale = c.model().panes()[pane].price_scale_by_id("new").unwrap();
    assert_eq!(
        scale.format_logical_tickmarks(&[10., 20.]),
        vec!["10.00%", "20.00%"]
    );
}
