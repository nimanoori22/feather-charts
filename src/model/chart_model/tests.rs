use super::*;
mod axis_snapshots;
mod price_axis;
use crate::{
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem, TimedData, WhitespaceData},
        data_layer::DataUpdateResponse,
        get_series_plot_row_creator::CustomPlotRowBuilder,
        horz_scale_behavior_time::{
            horz_scale_behavior_time::HorzScaleBehaviorTime,
            time_based_chart_options::TimeBehaviorOptions,
            types::{Time, UtcTimestamp},
        },
        icustom_series::{CustomData, CustomSeriesDataItem},
        layout_options::{Background, ColorSpace, LayoutPanesOptions},
        localization_options::LocalizationOptions,
        series::CustomSeries,
        series_options::{
            CustomSeriesOptions, CustomStyleOptions, LastPriceAnimationMode, LineStyleOptions,
            PriceFormat, PriceFormatBuiltIn, PriceFormatBuiltInType, PriceLineSource,
            SeriesOptions, SeriesOptionsCommon,
        },
        time_data::{Logical, TimePointIndex},
        time_scale_options::HorzScaleOptions,
    },
    renderers::draw_line::{LineStyle, LineType, LineWidth},
    views::time_scale_animation::{ActiveTimeScaleAnimation, TimeScaleAnimationController},
};
use std::time::{Duration, Instant};

type Model = ChartModel<HorzScaleBehaviorTime>;
type Layer = DataLayer<HorzScaleBehaviorTime>;

fn layout() -> LayoutOptions {
    LayoutOptions {
        background: Background::Solid {
            color: "#fff".into(),
        },
        text_color: "#000".into(),
        font_size: 12.0,
        font_family: "sans-serif".into(),
        panes: LayoutPanesOptions {
            enable_resize: true,
            separator_color: String::new(),
            separator_hover_color: String::new(),
        },
        attribution_logo: false,
        color_space: ColorSpace::Srgb,
        color_parsers: vec![],
    }
}
fn common() -> SeriesOptionsCommon {
    SeriesOptionsCommon {
        last_value_visible: true,
        title: String::new(),
        price_scale_id: None,
        series_last_value_mode: None,
        visible: true,
        hit_test_tolerance: 3.0,
        price_line_visible: true,
        price_line_source: PriceLineSource::LastBar,
        price_line_width: LineWidth::One,
        price_line_color: String::new(),
        price_line_style: LineStyle::Solid,
        price_format: PriceFormat::BuiltIn(PriceFormatBuiltIn {
            kind: PriceFormatBuiltInType::Price,
            precision: 2,
            min_move: 0.01,
            base: Some(100.0),
        }),
        base_line_visible: true,
        base_line_color: String::new(),
        base_line_width: LineWidth::One,
        base_line_style: LineStyle::Solid,
        autoscale_info_provider: None,
        conflation_threshold_factor: None,
    }
}
fn line_options() -> SeriesOptionsMap {
    SeriesOptionsMap::Line(SeriesOptions {
        common: common(),
        style: LineStyleOptions {
            color: "#000".into(),
            line_style: LineStyle::Solid,
            line_width: LineWidth::One,
            line_type: LineType::Simple,
            line_visible: true,
            point_markers_visible: false,
            point_markers_radius: None,
            crosshair_marker_visible: true,
            crosshair_marker_radius: 4.0,
            crosshair_marker_border_color: String::new(),
            crosshair_marker_background_color: String::new(),
            crosshair_marker_border_width: 2.0,
            last_price_animation: LastPriceAnimationMode::Disabled,
        },
    })
}
fn time(value: f64) -> Time {
    Time::from(UtcTimestamp::new(value))
}
fn line(at: f64, value: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: time(at),
        value,
        color: None,
        custom_values: None,
    }))
}
fn whitespace(at: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(WhitespaceData {
        time: time(at),
        custom_values: None,
    }))
}
fn model_with(pane: PaneOptions, add_default_pane: bool) -> Model {
    ChartModel::new(
        TimeScale::new(
            HorzScaleBehaviorTime::default(),
            HorzScaleOptions::default(),
            LocalizationOptions::new("en-US", "dd MMM 'yy"),
        ),
        ChartModelOptions {
            layout: layout(),
            pane,
            grid: GridOptions::default(),
            add_default_pane,
        },
    )
}
fn flush(model: &mut Model) {
    if let Some(mask) = model.take_invalidation() {
        model.apply_invalidation(&mask);
    }
    model.take_invalidation();
}
fn view(model: &mut Model, from: f64, to: f64) {
    model.set_logical_range(LogicalRange {
        from: Logical::new(from),
        to: Logical::new(to),
    });
    flush(model);
}
fn fixture() -> (Layer, Model, SeriesId) {
    let mut layer = DataLayer::new(HorzScaleBehaviorTime::default());
    let mut model = model_with(PaneOptions::default(), true);
    let id = SeriesId::new(1);
    model.set_width(100.0).unwrap();
    model.set_pane_height(0, 200.0).unwrap();
    model
        .register_series(
            id,
            SeriesType::Line,
            line_options(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    model
        .apply_data_update(
            layer
                .set_series_data(
                    id,
                    SeriesType::Line,
                    (1..=20).map(|n| line(n as f64, n as f64)).collect(),
                )
                .unwrap(),
        )
        .unwrap();
    view(&mut model, 10.0, 19.0);
    (layer, model, id)
}

#[test]
fn line_snapshot_matches_selected_scale_and_extends_both_visible_edges() {
    let (_, mut model, id) = fixture();
    view(&mut model, 5., 10.);
    let data = model.prepare_line(id).unwrap().unwrap();
    let range = model.visible_strict_range().unwrap();
    let first = model.series(id).unwrap().first_value(Some(&range)).unwrap();
    assert_eq!(
        data.visible_range,
        crate::model::time_data::ValueRange { from: 4, to: 12 }
    );
    for point in &data.items {
        assert_eq!(
            point.x,
            model.time_scale().index_to_coordinate(point.index).value()
        );
        assert_eq!(
            point.y,
            model.panes()[0]
                .right_price_scale()
                .price_to_coordinate(point.index.value() + 1., first.value)
                .value()
        );
    }
}

#[test]
fn line_snapshot_uses_percentage_indexed_log_and_left_scale_coordinates() {
    use crate::model::price_scale::PriceScaleMode;
    for mode in [
        PriceScaleMode::Percentage,
        PriceScaleMode::IndexedTo100,
        PriceScaleMode::Logarithmic,
    ] {
        let mut options = PaneOptions::default();
        options.left_price_scale.mode = mode;
        let mut model = model_with(options, true);
        let mut layer = Layer::new(HorzScaleBehaviorTime::default());
        let id = SeriesId::new(10);
        model
            .register_series(
                id,
                SeriesType::Line,
                line_options(),
                0,
                PriceScalePosition::Left,
            )
            .unwrap();
        model.set_width(200.).unwrap();
        model.set_pane_height(0, 150.).unwrap();
        model
            .apply_data_update(
                layer
                    .set_series_data(
                        id,
                        SeriesType::Line,
                        (0..10)
                            .map(|i| line(i as f64, 100. + i as f64 * 10.))
                            .collect(),
                    )
                    .unwrap(),
            )
            .unwrap();
        view(&mut model, 4., 8.);
        let data = model.prepare_line(id).unwrap().unwrap();
        for p in &data.items {
            assert_eq!(
                p.y,
                model.panes()[0]
                    .left_price_scale()
                    .price_to_coordinate(100. + p.index.value() * 10., 140.)
                    .value()
            );
        }
    }
}

#[test]
fn line_snapshot_connects_across_whitespace_and_uses_row_color_fallback() {
    let (mut layer, mut model, id) = fixture();
    let mut colored = line(1., 10.);
    if let BuiltInSeriesDataItem::Line(LineDataItem::Data(data)) = &mut colored {
        data.color = Some("red".into());
    }
    model
        .apply_data_update(
            layer
                .set_series_data(
                    id,
                    SeriesType::Line,
                    vec![colored, whitespace(2.), line(3., 20.)],
                )
                .unwrap(),
        )
        .unwrap();
    model.fit_content();
    flush(&mut model);
    let data = model.prepare_line(id).unwrap().unwrap();
    assert_eq!(data.items.len(), 2);
    assert_eq!(data.items[0].color, "red");
    assert_eq!(data.items[1].color, "#000");
    assert_eq!(data.items[1].index.value(), 2.);
    let commands = data.draw_commands(crate::renderers::grid_renderer::PixelRatio {
        horizontal: 1.,
        vertical: 1.,
    });
    assert_eq!(commands.strokes.len(), 1);
    assert_eq!(commands.strokes[0].path.len(), 2);
}

#[test]
fn line_view_clears_stale_output_for_hidden_empty_zero_size_and_unknown_series() {
    use crate::model::series::line_pane_view::{LinePaneView, LinePreparationError};
    let (mut layer, mut model, id) = fixture();
    let mut render_view = LinePaneView::default();
    render_view.refresh(&mut model, id).unwrap();
    assert!(render_view.data().is_some());
    let mut options = line_options();
    if let SeriesOptionsMap::Line(options) = &mut options {
        options.common.visible = false;
    }
    model.replace_series_options(id, options).unwrap();
    render_view.refresh(&mut model, id).unwrap();
    assert!(render_view.data().is_none());
    model.replace_series_options(id, line_options()).unwrap();
    flush(&mut model);
    render_view.refresh(&mut model, id).unwrap();
    assert!(render_view.data().is_some());
    model.set_width(0.).unwrap();
    render_view.refresh(&mut model, id).unwrap();
    assert!(render_view.data().is_none());
    model.set_width(100.).unwrap();
    model.set_pane_height(0, 0.).unwrap();
    render_view.refresh(&mut model, id).unwrap();
    assert!(render_view.data().is_none());
    model.set_pane_height(0, 200.).unwrap();
    model
        .apply_data_update(
            layer
                .set_series_data(id, SeriesType::Line, vec![whitespace(1.), whitespace(2.)])
                .unwrap(),
        )
        .unwrap();
    render_view.refresh(&mut model, id).unwrap();
    assert!(render_view.data().is_none());
    model
        .remove_series(id, layer.remove_series(id).unwrap())
        .unwrap();
    assert_eq!(
        render_view.refresh(&mut model, id),
        Err(LinePreparationError::UnknownSeries(id))
    );
    assert!(render_view.data().is_none());
}

#[test]
fn line_preparation_rejects_non_line_and_enabled_conflation() {
    use crate::model::series::line_pane_view::LinePreparationError;
    let (mut layer, mut model, id) = fixture();
    model.apply_time_scale_options(
        &mut layer,
        HorzScaleOptionsPatch {
            enable_conflation: Some(true),
            ..Default::default()
        },
    );
    flush(&mut model);
    assert_eq!(
        model.prepare_line(id),
        Err(LinePreparationError::UnsupportedConflation)
    );
    model.apply_time_scale_options(
        &mut layer,
        HorzScaleOptionsPatch {
            enable_conflation: Some(false),
            ..Default::default()
        },
    );
    let custom = SeriesId::new(77);
    let handle = Rc::new(RefCell::new(CustomSeries::<_, _, Datum>::new(
        custom,
        CustomSeriesOptions {
            common: common(),
            style: CustomStyleOptions {
                color: "#000".into(),
            },
        },
    )));
    model
        .register_custom_series(handle, 0, PriceScalePosition::Right)
        .unwrap();
    assert_eq!(
        model.prepare_line(custom),
        Err(LinePreparationError::NotLineSeries(custom))
    );
}

#[test]
fn line_options_and_marker_defaults_refresh_without_a_cache() {
    let (_, mut model, id) = fixture();
    for radius in [None, Some(0.), Some(4.)] {
        let mut options = line_options();
        if let SeriesOptionsMap::Line(options) = &mut options {
            options.style.line_visible = false;
            options.style.point_markers_visible = true;
            options.style.point_markers_radius = radius;
        }
        model.replace_series_options(id, options).unwrap();
        flush(&mut model);
        let data = model.prepare_line(id).unwrap().unwrap();
        assert_eq!(data.line_type, None);
        assert_eq!(
            data.point_markers_radius,
            Some(radius.filter(|r| *r != 0.).unwrap_or(2.5))
        );
    }
}

#[test]
fn frame_validation_does_not_mutate_model_for_invalid_layout_or_wrong_pane() {
    use crate::{
        model::series::line_pane_view::LinePaneView,
        renderers::grid_renderer::PixelRatio,
        ui::line_chart::{FrameError, FrameLayout, prepare_frame},
    };
    let (_, mut model, id) = fixture();
    let mut controller = TimeScaleAnimationController::default();
    let mut render_view = LinePaneView::default();
    let mut layout = FrameLayout {
        pane: 0,
        size: iced::Size::new(300., f32::NAN),
        pixel_ratio: PixelRatio {
            horizontal: 1.,
            vertical: 1.,
        },
        background: "white".into(),
    };
    assert!(matches!(
        prepare_frame(
            &mut model,
            &mut controller,
            &mut render_view,
            Some(id),
            &layout,
            Instant::now()
        ),
        Err(FrameError::Model(ChartModelError::InvalidDimensions))
    ));
    assert_eq!(model.time_scale().width(), 100.);
    layout.size = iced::Size::new(300., 200.);
    layout.pane = model.add_pane();
    assert!(matches!(
        prepare_frame(
            &mut model,
            &mut controller,
            &mut render_view,
            Some(id),
            &layout,
            Instant::now()
        ),
        Err(FrameError::SeriesPaneMismatch { .. })
    ));
    assert_eq!(model.time_scale().width(), 100.);
    assert!(render_view.data().is_none());
}

#[test]
fn rendering_snapshots_rebuild_on_resize_append_and_historical_updates() {
    let (mut layer, mut model, id) = fixture();
    let old = model.prepare_line(id).unwrap().unwrap();
    model.set_width(300.).unwrap();
    model.set_pane_height(0, 400.).unwrap();
    flush(&mut model);
    let resized = model.prepare_line(id).unwrap().unwrap();
    assert_ne!(resized.items[15].x, old.items[15].x);
    assert_ne!(resized.items[15].y, old.items[15].y);
    view(&mut model, 5., 10.);
    let before = model.prepare_line(id).unwrap().unwrap();
    model
        .apply_data_update(layer.update_series_data(id, line(21., 21.), false).unwrap())
        .unwrap();
    flush(&mut model);
    let appended = model.prepare_line(id).unwrap().unwrap();
    assert_eq!(before.items[7].x, appended.items[7].x);
    model
        .apply_data_update(layer.update_series_data(id, line(8., 100.), true).unwrap())
        .unwrap();
    flush(&mut model);
    let updated = model.prepare_line(id).unwrap().unwrap();
    assert_ne!(appended.items[7].y, updated.items[7].y);
}

#[test]
fn prepared_ui_frames_retain_replay_mask_align_animation_and_clear_on_removal() {
    use crate::{
        model::series::line_pane_view::LinePaneView,
        renderers::grid_renderer::PixelRatio,
        ui::line_chart::{FrameLayout, prepare_frame},
    };
    let (mut layer, mut model, id) = fixture();
    let mut controller = TimeScaleAnimationController::default();
    let mut render_view = LinePaneView::default();
    let layout = FrameLayout {
        pane: 0,
        size: iced::Size::new(300., 200.),
        pixel_ratio: PixelRatio {
            horizontal: 1.5,
            vertical: 1.5,
        },
        background: "#fff".into(),
    };
    model.fit_content();
    let now = Instant::now();
    let (snapshot, mask) = prepare_frame(
        &mut model,
        &mut controller,
        &mut render_view,
        Some(id),
        &layout,
        now,
    )
    .unwrap();
    assert!(!snapshot.lines.is_empty());
    let mask = mask.unwrap();
    let before = render_view.data().unwrap().clone();
    controller.apply_frame(&mut model, &mask, now);
    render_view.refresh(&mut model, id).unwrap();
    assert_eq!(render_view.data(), Some(&before));
    model.scroll_to_offset_animated(-5., Duration::from_secs(1));
    prepare_frame(
        &mut model,
        &mut controller,
        &mut render_view,
        Some(id),
        &layout,
        now,
    )
    .unwrap();
    let (snapshot, _) = prepare_frame(
        &mut model,
        &mut controller,
        &mut render_view,
        Some(id),
        &layout,
        now + Duration::from_millis(500),
    )
    .unwrap();
    assert!(controller.is_active());
    assert!(!snapshot.grid.is_empty());
    for p in &render_view.data().unwrap().items {
        assert_eq!(p.x, model.time_scale().index_to_coordinate(p.index).value());
    }
    model
        .remove_series(id, layer.remove_series(id).unwrap())
        .unwrap();
    let (empty, _) = prepare_frame(
        &mut model,
        &mut controller,
        &mut render_view,
        None,
        &layout,
        now + Duration::from_millis(600),
    )
    .unwrap();
    assert!(empty.lines.is_empty());
    assert!(empty.markers.is_empty());
    assert!(render_view.data().is_none());
}

#[test]
fn loaded_series_fit_resize_and_price_coordinates_use_shared_state() {
    let (_, mut model, id) = fixture();
    assert_eq!(model.series(id).unwrap().bars().size(), 20);
    model.fit_content();
    flush(&mut model);
    assert_eq!(model.visible_strict_range().unwrap().left().value(), 0.0);
    assert_eq!(model.visible_strict_range().unwrap().right().value(), 19.0);
    let scale = model.panes()[0].right_price_scale();
    let range = scale.price_range().unwrap();
    assert!(range.min_value() <= 1.0 && range.max_value() >= 20.0);
    let y = scale.price_to_coordinate(10.0, 1.0);
    assert!((scale.coordinate_to_price(y, 1.0) - 10.0).abs() < 1e-9);
    let x = model
        .time_scale()
        .index_to_coordinate(TimePointIndex::new(19.0));
    model.set_width(200.0).unwrap();
    model.set_pane_height(0, 300.0).unwrap();
    assert_eq!(model.panes()[0].width(), 200.0);
    assert_eq!(model.panes()[0].right_price_scale().height(), 300.0);
    assert_ne!(
        model
            .time_scale()
            .index_to_coordinate(TimePointIndex::new(19.0)),
        x
    );
}

#[test]
fn append_follows_last_bar_only_when_visible_and_enabled() {
    for shift in [false, true] {
        for at_right in [false, true] {
            let (mut layer, mut model, id) = fixture();
            model.apply_time_scale_options(
                &mut layer,
                HorzScaleOptionsPatch {
                    shift_visible_range_on_new_bar: Some(shift),
                    ..Default::default()
                },
            );
            if !at_right {
                view(&mut model, 0.0, 9.0);
            }
            let old_range = model.visible_logical_range().unwrap();
            let old_offset = model.time_scale().right_offset();
            model
                .apply_data_update(
                    layer
                        .update_series_data(id, line(21.0, 21.0), false)
                        .unwrap(),
                )
                .unwrap();
            let next = model.visible_logical_range().unwrap();
            if shift && at_right {
                assert_eq!(model.time_scale().right_offset(), old_offset);
                assert_eq!(next.right().value(), old_range.right().value() + 1.0);
            } else {
                assert_eq!(model.time_scale().right_offset(), old_offset - 1.0);
                assert_eq!(next, old_range);
            }
            assert_eq!(model.series(id).unwrap().bars().size(), 21);
        }
    }
}

#[test]
fn replacing_existing_whitespace_honors_both_shift_options() {
    for shift in [false, true] {
        for allow_whitespace in [false, true] {
            let (mut layer, mut model, id) = fixture();
            let mut data = (1..=20)
                .map(|n| line(n as f64, n as f64))
                .collect::<Vec<_>>();
            data.push(whitespace(21.0));
            model
                .apply_data_update(layer.set_series_data(id, SeriesType::Line, data).unwrap())
                .unwrap();
            model.apply_time_scale_options(
                &mut layer,
                HorzScaleOptionsPatch {
                    shift_visible_range_on_new_bar: Some(shift),
                    allow_shift_visible_range_on_whitespace_replacement: Some(allow_whitespace),
                    ..Default::default()
                },
            );
            view(&mut model, 10.0, 19.0);
            let before = model.visible_logical_range().unwrap();
            let update = layer
                .update_series_data(id, line(21.0, 100.0), false)
                .unwrap();
            assert_eq!(update.time_scale.first_changed_point_index, None);
            model.apply_data_update(update).unwrap();
            let after = model.visible_logical_range().unwrap();
            assert_eq!(
                after.right().value(),
                before.right().value() + if shift && allow_whitespace { 1.0 } else { 0.0 }
            );
        }
    }
}

#[test]
fn earlier_history_reindexes_rows_without_append_compensation() {
    let (mut layer, mut model, id) = fixture();
    let old_offset = model.time_scale().right_offset();
    let mut data = vec![line(0.0, 0.0)];
    data.extend((1..=20).map(|n| line(n as f64, n as f64)));
    model
        .apply_data_update(layer.set_series_data(id, SeriesType::Line, data).unwrap())
        .unwrap();
    assert_eq!(model.time_scale().right_offset(), old_offset);
    assert_eq!(
        model.series(id).unwrap().bars().last_index(),
        Some(TimePointIndex::new(20.0))
    );
    assert_eq!(
        model
            .time_scale()
            .index_to_time(TimePointIndex::new(0.0))
            .unwrap()
            .timestamp
            .seconds(),
        0.0
    );
}

#[test]
fn historical_and_latest_bar_updates_recalculate_price_range_without_shifting() {
    let (mut layer, mut model, id) = fixture();
    let before = model.visible_logical_range().unwrap();
    model
        .apply_data_update(
            layer
                .update_series_data(id, line(15.0, 150.0), true)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(model.visible_logical_range(), Some(before));
    assert!(
        model.panes()[0]
            .right_price_scale()
            .price_range()
            .unwrap()
            .max_value()
            >= 150.0
    );
    model
        .apply_data_update(
            layer
                .update_series_data(id, line(20.0, 200.0), false)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(model.visible_logical_range(), Some(before));
    assert!(
        model.panes()[0]
            .right_price_scale()
            .price_range()
            .unwrap()
            .max_value()
            >= 200.0
    );
}

#[test]
fn second_series_and_removal_reindex_surviving_rows_and_fulfilled_indices() {
    let (mut layer, mut model, first) = fixture();
    let second = SeriesId::new(2);
    model
        .register_series(
            second,
            SeriesType::Line,
            line_options(),
            0,
            PriceScalePosition::Left,
        )
        .unwrap();
    model
        .apply_data_update(
            layer
                .set_series_data(
                    second,
                    SeriesType::Line,
                    vec![line(1.5, 50.0), line(22.0, 60.0)],
                )
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        model.series(first).unwrap().bars().last_index(),
        Some(TimePointIndex::new(20.0))
    );
    assert_eq!(model.fulfilled_time_indices().len(), 22);
    let update = layer.remove_series(second).unwrap();
    model.remove_series(second, update).unwrap();
    assert_eq!(model.series_count(), 1);
    assert!(model.series(second).is_none());
    assert_eq!(
        model.series(first).unwrap().bars().last_index(),
        Some(TimePointIndex::new(19.0))
    );
    assert_eq!(model.fulfilled_time_indices().len(), 20);
    assert_eq!(model.panes()[0].attached_source_ids(), vec![first]);
}

#[test]
fn removing_empty_pane_reindexes_associations_and_pending_autoscale() {
    let (mut layer, mut model, first) = fixture();
    let pane = model.add_pane();
    let second = SeriesId::new(2);
    model
        .register_series(
            second,
            SeriesType::Line,
            line_options(),
            pane,
            PriceScalePosition::Right,
        )
        .unwrap();
    model
        .apply_data_update(
            layer
                .set_series_data(second, SeriesType::Line, vec![line(1.0, 20.0)])
                .unwrap(),
        )
        .unwrap();
    let update = layer.remove_series(first).unwrap();
    model.remove_series(first, update).unwrap();
    assert_eq!(model.panes().len(), 1);
    assert_eq!(model.panes()[0].index(), 0);
    assert_eq!(model.pane_for_series(second), Some(0));
    let mask = model.take_invalidation().unwrap();
    assert_eq!(mask.global_level(), InvalidationLevel::Full);
    assert!(mask.pane_invalidation(0).auto_scale);
    assert!(!mask.pane_invalidation(1).auto_scale);
}

#[test]
fn preserve_empty_pane_and_final_series_removal_leave_valid_empty_state() {
    let (mut layer, mut model, id) = fixture();
    model.add_pane();
    model.set_preserve_empty_pane(0, true).unwrap();
    model
        .remove_series(id, layer.remove_series(id).unwrap())
        .unwrap();
    assert_eq!(model.panes().len(), 2);
    assert_eq!(model.series_count(), 0);
    assert!(model.fulfilled_time_indices().is_empty());
    assert!(model.time_scale().is_empty());
    assert!(model.panes()[0].right_price_scale().price_range().is_none());
}

#[test]
fn startup_empty_data_and_zero_width_resize_are_supported() {
    let mut layer = Layer::new(HorzScaleBehaviorTime::default());
    let mut model = model_with(PaneOptions::default(), false);
    assert!(model.panes().is_empty());
    let pane = model.add_pane();
    let id = SeriesId::new(1);
    model
        .register_series(
            id,
            SeriesType::Line,
            line_options(),
            pane,
            PriceScalePosition::Right,
        )
        .unwrap();
    model
        .apply_data_update(layer.set_series_data(id, SeriesType::Line, vec![]).unwrap())
        .unwrap();
    model.fit_content();
    flush(&mut model);
    assert!(model.visible_logical_range().is_none());
    model
        .apply_data_update(
            layer
                .set_series_data(id, SeriesType::Line, vec![line(1.0, 1.0), line(2.0, 2.0)])
                .unwrap(),
        )
        .unwrap();
    assert!(model.visible_logical_range().is_none());
    model.set_width(100.0).unwrap();
    model.fit_content();
    flush(&mut model);
    assert!(model.visible_logical_range().is_some());
    model.set_width(0.0).unwrap();
    assert_eq!(model.time_scale().width(), 0.0);
    assert_eq!(model.panes()[0].width(), 0.0);
    assert!(model.visible_logical_range().is_none());
}

#[test]
fn registration_errors_leave_no_partial_state() {
    let (_, mut model, id) = fixture();
    assert_eq!(
        model.register_series(
            id,
            SeriesType::Line,
            line_options(),
            0,
            PriceScalePosition::Left
        ),
        Err(ChartModelError::DuplicateSeries(id))
    );
    assert_eq!(
        model.register_series(
            SeriesId::new(2),
            SeriesType::Line,
            line_options(),
            3,
            PriceScalePosition::Right
        ),
        Err(ChartModelError::InvalidPane(3))
    );
    assert!(matches!(
        model.register_series(
            SeriesId::new(2),
            SeriesType::Bar,
            line_options(),
            0,
            PriceScalePosition::Right
        ),
        Err(ChartModelError::Construction(_))
    ));
    assert_eq!(model.series_count(), 1);
    assert_eq!(model.panes()[0].attached_source_ids(), vec![id]);
    assert_eq!(model.coordinator.target_count(), 1);
}

#[test]
fn first_series_requests_full_redraw_and_new_panes_request_autoscale() {
    let mut model = model_with(PaneOptions::default(), true);
    let initial = model.take_invalidation().unwrap();
    assert!(initial.pane_invalidation(0).auto_scale);
    assert_eq!(
        model.panes()[0].stretch_factor(),
        2.0 * DEFAULT_STRETCH_FACTOR
    );
    model
        .register_series(
            SeriesId::new(1),
            SeriesType::Line,
            line_options(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    assert_eq!(
        model.take_invalidation().unwrap().global_level(),
        InvalidationLevel::Full
    );
    model
        .register_series(
            SeriesId::new(2),
            SeriesType::Line,
            line_options(),
            0,
            PriceScalePosition::Left,
        )
        .unwrap();
    assert_eq!(
        model.take_invalidation().unwrap().global_level(),
        InvalidationLevel::Light
    );
}

#[test]
fn momentary_autoscale_updates_both_manual_scales() {
    let mut options = PaneOptions::default();
    options.left_price_scale.auto_scale = false;
    options.right_price_scale.auto_scale = false;
    let mut model = model_with(options, true);
    let mut layer = Layer::new(HorzScaleBehaviorTime::default());
    model.set_width(100.0).unwrap();
    model.set_pane_height(0, 200.0).unwrap();
    for (id, position, high) in [
        (SeriesId::new(1), PriceScalePosition::Left, 10.0),
        (SeriesId::new(2), PriceScalePosition::Right, 100.0),
    ] {
        model
            .register_series(id, SeriesType::Line, line_options(), 0, position)
            .unwrap();
        model
            .apply_data_update(
                layer
                    .set_series_data(id, SeriesType::Line, vec![line(1.0, 1.0), line(2.0, high)])
                    .unwrap(),
            )
            .unwrap();
    }
    model.fit_content();
    flush(&mut model);
    let mut mask = InvalidateMask::light();
    mask.invalidate_pane(
        0,
        PaneInvalidation {
            level: InvalidationLevel::Light,
            auto_scale: true,
        },
    );
    model.apply_invalidation(&mask);
    let pane = &model.panes()[0];
    assert!(pane.left_price_scale().price_range().unwrap().max_value() >= 10.0);
    assert!(pane.right_price_scale().price_range().unwrap().max_value() >= 100.0);
    assert!(!pane.left_price_scale().is_auto_scale());
    assert!(!pane.right_price_scale().is_auto_scale());
}

#[test]
fn deferred_commands_are_ordered_and_replay_after_layout_changes() {
    let (_, mut model, _) = fixture();
    model.set_bar_spacing(2.0);
    model.fit_content();
    model.set_right_offset(1.0);
    let mask = model.take_invalidation().unwrap();
    assert!(matches!(
        mask.time_scale_invalidations(),
        [
            TimeScaleInvalidation::FitContent,
            TimeScaleInvalidation::StopAnimation,
            TimeScaleInvalidation::ApplyRightOffset(1.0)
        ]
    ));
    model.apply_invalidation(&mask);
    assert_eq!(model.time_scale().bar_spacing(), 5.0);
    assert_eq!(model.time_scale().right_offset(), 1.0);
    model.set_width(200.0).unwrap();
    model.apply_invalidation(&mask);
    assert_eq!(model.time_scale().bar_spacing(), 10.0);
    assert_eq!(model.time_scale().right_offset(), 1.0);
    assert_eq!(mask.time_scale_invalidations().len(), 3);
    assert!(model.take_invalidation().is_some());
}

#[test]
fn cursor_masks_do_not_execute_deferred_scale_commands() {
    let (_, mut model, _) = fixture();
    let before = model.time_scale().bar_spacing();
    let mut mask = InvalidateMask::new(InvalidationLevel::Cursor);
    mask.set_bar_spacing(99.0);
    model.apply_invalidation(&mask);
    assert_eq!(model.time_scale().bar_spacing(), before);
}

#[test]
fn animation_replay_keeps_origin_and_unrelated_redraw_preserves_continuation() {
    let (_, mut model, _) = fixture();
    let now = Instant::now();
    model.set_time_scale_animation(AnimationRequest::new(
        ActiveTimeScaleAnimation::from_effect(
            ScrollAnimation {
                start_offset: 0.0,
                target_offset: 2.0,
                duration: Duration::from_secs(1),
            },
            now,
        ),
    ));
    let mut controller = TimeScaleAnimationController::default();
    for millis in [250, 750, 1000] {
        model.light_update();
        let mask = model.take_invalidation().unwrap();
        controller.apply_frame(&mut model, &mask, now + Duration::from_millis(millis));
        assert_eq!(model.time_scale().right_offset(), millis as f64 / 500.0);
        let context = model.layout_context();
        let expected = model
            .time_scale
            .marks(context)
            .unwrap()
            .iter()
            .map(|mark| mark.coordinate)
            .collect::<Vec<_>>();
        let rendered = model.panes()[0]
            .grid()
            .pane_view()
            .renderer()
            .data()
            .unwrap()
            .time_marks
            .iter()
            .map(|mark| mark.coordinate.value())
            .collect::<Vec<_>>();
        assert_eq!(rendered, expected);
    }
    assert!(!controller.is_active());
    let final_mask = model.take_invalidation().unwrap();
    assert!(
        !final_mask
            .time_scale_invalidations()
            .iter()
            .any(|value| matches!(value, TimeScaleInvalidation::Animation(_)))
    );
}

#[test]
fn navigation_cancels_animation_without_resurrection() {
    let operations: [fn(&mut Model); 6] = [
        Model::fit_content,
        Model::reset_time_scale,
        |model| model.set_bar_spacing(8.0),
        |model| model.set_right_offset(-1.0),
        Model::stop_time_scale_animation,
        |model| {
            model.set_logical_range(LogicalRange {
                from: Logical::new(0.0),
                to: Logical::new(9.0),
            })
        },
    ];
    for operation in operations {
        let (_, mut model, _) = fixture();
        let now = Instant::now();
        model.set_time_scale_animation(AnimationRequest::new(
            ActiveTimeScaleAnimation::from_effect(
                ScrollAnimation {
                    start_offset: 0.0,
                    target_offset: 2.0,
                    duration: Duration::from_secs(1),
                },
                now,
            ),
        ));
        let mask = model.take_invalidation().unwrap();
        let mut controller = TimeScaleAnimationController::default();
        controller.apply_frame(&mut model, &mask, now + Duration::from_millis(250));
        operation(&mut model);
        let mask = model.take_invalidation().unwrap();
        controller.apply_frame(&mut model, &mask, now + Duration::from_millis(500));
        assert!(!controller.is_active());
        assert_eq!(controller.frame(now + Duration::from_millis(750)), None);
        if let Some(pending) = model.take_invalidation() {
            assert!(
                !pending
                    .time_scale_invalidations()
                    .iter()
                    .any(|value| matches!(value, TimeScaleInvalidation::Animation(_)))
            );
        }
    }
}

#[test]
fn behavior_configuration_and_options_notifications_need_no_clone_behavior() {
    let (mut layer, mut model, _) = fixture();
    model.configure_horizontal_behavior(&mut layer, TimeBehaviorOptions::default);
    assert!(model.take_options_applied());
    assert!(!model.take_options_applied());
    model.apply_time_scale_options(
        &mut layer,
        HorzScaleOptionsPatch {
            time_visible: Some(true),
            seconds_visible: Some(true),
            ..Default::default()
        },
    );
    assert!(layer.behavior().options().time_scale.base.time_visible);
    assert!(
        model
            .time_scale()
            .behavior()
            .options()
            .time_scale
            .base
            .time_visible
    );
    assert!(model.take_options_applied());
    model.scroll_to_offset_animated(2.0, Duration::from_secs(1));
    assert_eq!(model.take_scroll_animation_requests().len(), 1);
    assert!(model.take_scroll_animation_requests().is_empty());
}

#[test]
fn dropping_model_releases_series_without_cycles() {
    let (_, model, id) = fixture();
    let weak = Rc::downgrade(model.series[&id].built_in().unwrap());
    drop(model);
    assert!(weak.upgrade().is_none());
}

#[derive(Clone)]
struct Datum {
    at: Time,
    value: f64,
}
impl TimedData for Datum {
    type Item = Time;
    fn time(&self) -> &Time {
        &self.at
    }
    fn time_mut(&mut self) -> &mut Time {
        &mut self.at
    }
}
impl CustomData for Datum {}
struct Builder;
impl CustomPlotRowBuilder<Datum, Time, ()> for Builder {
    fn values(&self, datum: &Datum) -> Vec<f64> {
        vec![datum.value]
    }
    fn is_whitespace(&self, item: &CustomSeriesDataItem<Datum, Time>) -> bool {
        matches!(item, CustomSeriesDataItem::Whitespace(_))
    }
}

#[test]
fn custom_payloads_stay_typed_and_reindex_with_built_in_series() {
    let (mut layer, mut model, _) = fixture();
    let id = SeriesId::new(2);
    let handle = Rc::new(RefCell::new(CustomSeries::new(
        id,
        CustomSeriesOptions {
            common: common(),
            style: CustomStyleOptions {
                color: "#000".into(),
            },
        },
    )));
    model
        .register_custom_series(handle.clone(), 0, PriceScalePosition::Left)
        .unwrap();
    let update = layer
        .set_custom_series_data(
            id,
            vec![
                CustomSeriesDataItem::Data(Datum {
                    at: time(1.5),
                    value: 500.0,
                }),
                CustomSeriesDataItem::Data(Datum {
                    at: time(22.0),
                    value: 600.0,
                }),
            ],
            &Builder,
        )
        .unwrap();
    model.apply_custom_data_update(&handle, update).unwrap();
    assert_eq!(handle.borrow().rows().len(), 2);
    assert_eq!(model.fulfilled_time_indices().len(), 22);
    model.fit_content();
    flush(&mut model);
    assert!(
        model.panes()[0]
            .left_price_scale()
            .price_range()
            .unwrap()
            .max_value()
            >= 600.0
    );
    model
        .remove_series(id, layer.remove_series(id).unwrap())
        .unwrap();
    assert_eq!(model.series_count(), 1);
}

#[test]
fn unknown_update_is_rejected_before_changing_model() {
    let (_, mut model, _) = fixture();
    let before = model.visible_logical_range();
    let mut unrelated = Layer::new(HorzScaleBehaviorTime::default());
    let update: DataUpdateResponse<_, _> = unrelated
        .set_series_data(
            SeriesId::new(99),
            SeriesType::Line,
            vec![line(100.0, 100.0)],
        )
        .unwrap();
    assert_eq!(
        model.apply_data_update(update),
        Err(ChartModelError::UnknownSeries(SeriesId::new(99)))
    );
    assert_eq!(model.visible_logical_range(), before);
    assert_eq!(model.series_count(), 1);
}

#[test]
fn whitespace_only_data_has_a_time_axis_but_no_price_range() {
    let mut layer = Layer::new(HorzScaleBehaviorTime::default());
    let mut model = model_with(PaneOptions::default(), true);
    let id = SeriesId::new(1);
    model.set_width(100.0).unwrap();
    model
        .register_series(
            id,
            SeriesType::Line,
            line_options(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    model
        .apply_data_update(
            layer
                .set_series_data(id, SeriesType::Line, vec![whitespace(1.0), whitespace(2.0)])
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        model.time_scale().base_index(),
        Some(TimePointIndex::new(0.0))
    );
    model.fit_content();
    flush(&mut model);
    assert!(model.visible_logical_range().is_some());
    assert!(model.series(id).unwrap().bars().is_empty());
    assert!(model.fulfilled_time_indices().is_empty());
    assert!(model.panes()[0].right_price_scale().price_range().is_none());
}

#[test]
fn navigation_before_ui_adopts_animation_descriptors_cancels_them() {
    let (_, mut model, _) = fixture();
    let now = Instant::now();
    model.scroll_to_offset_animated(2.0, Duration::from_secs(1));
    model.fit_content();
    let mut controller = TimeScaleAnimationController::default();
    controller.draw_frame(&mut model, now).unwrap();
    assert!(!controller.is_active());
    assert!(model.take_scroll_animation_requests().is_empty());
}

#[test]
fn ui_frame_driver_starts_descriptors_once_and_carries_progress() {
    let (_, mut model, _) = fixture();
    let now = Instant::now();
    model.scroll_to_offset_animated(2.0, Duration::from_secs(1));
    let mut controller = TimeScaleAnimationController::default();
    controller.draw_frame(&mut model, now).unwrap();
    assert!(controller.is_active());
    controller
        .draw_frame(&mut model, now + Duration::from_millis(500))
        .unwrap();
    assert_eq!(model.time_scale().right_offset(), 1.0);
    controller
        .draw_frame(&mut model, now + Duration::from_secs(1))
        .unwrap();
    assert_eq!(model.time_scale().right_offset(), 2.0);
    assert!(!controller.is_active());
}

#[test]
fn custom_incremental_updates_preserve_payloads_and_shared_axis() {
    let (mut layer, mut model, first) = fixture();
    let id = SeriesId::new(2);
    let handle = Rc::new(RefCell::new(CustomSeries::new(
        id,
        CustomSeriesOptions {
            common: common(),
            style: CustomStyleOptions {
                color: String::new(),
            },
        },
    )));
    model
        .register_custom_series(handle.clone(), 0, PriceScalePosition::Left)
        .unwrap();
    let update = layer
        .set_custom_series_data(
            id,
            vec![CustomSeriesDataItem::Data(Datum {
                at: time(5.0),
                value: 50.0,
            })],
            &Builder,
        )
        .unwrap();
    model.apply_custom_data_update(&handle, update).unwrap();
    let update = layer
        .update_custom_series_data(
            id,
            CustomSeriesDataItem::Data(Datum {
                at: time(5.0),
                value: 55.0,
            }),
            &Builder,
            false,
        )
        .unwrap();
    assert!(update.series.is_empty());
    model.apply_custom_data_update(&handle, update).unwrap();
    assert_eq!(handle.borrow().rows()[0].data.value, 55.0);
    let update = layer
        .update_custom_series_data(
            id,
            CustomSeriesDataItem::Data(Datum {
                at: time(21.0),
                value: 210.0,
            }),
            &Builder,
            false,
        )
        .unwrap();
    model.apply_custom_data_update(&handle, update).unwrap();
    assert_eq!(handle.borrow().rows().len(), 2);
    assert_eq!(
        model.series(first).unwrap().bars().last_index(),
        Some(TimePointIndex::new(19.0))
    );
    assert_eq!(
        model.time_scale().base_index(),
        Some(TimePointIndex::new(20.0))
    );
    assert!(
        handle
            .borrow()
            .last_update_info()
            .unwrap()
            .last_bar_updated_or_new_bars_added_to_the_right
    );
}

#[test]
fn custom_handle_with_matching_id_cannot_update_another_registered_instance() {
    let (mut layer, mut model, _) = fixture();
    let id = SeriesId::new(2);
    let make = || {
        Rc::new(RefCell::new(CustomSeries::new(
            id,
            CustomSeriesOptions {
                common: common(),
                style: CustomStyleOptions {
                    color: String::new(),
                },
            },
        )))
    };
    let registered = make();
    model
        .register_custom_series(registered.clone(), 0, PriceScalePosition::Left)
        .unwrap();
    let impostor = make();
    let update = layer
        .set_custom_series_data(
            id,
            vec![CustomSeriesDataItem::Data(Datum {
                at: time(5.0),
                value: 50.0,
            })],
            &Builder,
        )
        .unwrap();
    assert_eq!(
        model.apply_custom_data_update(&impostor, update),
        Err(ChartModelError::CustomHandleMismatch(id))
    );
    assert!(registered.borrow().rows().is_empty());
}

#[test]
fn rejected_dimensions_and_unknown_removal_leave_the_chart_unchanged() {
    let (mut layer, mut model, id) = fixture();
    let width = model.time_scale().width();
    assert_eq!(
        model.set_width(f64::NAN),
        Err(ChartModelError::InvalidDimensions)
    );
    assert_eq!(
        model.set_width(-1.0),
        Err(ChartModelError::InvalidDimensions)
    );
    assert_eq!(
        model.set_pane_height(4, 100.0),
        Err(ChartModelError::InvalidPane(4))
    );
    assert_eq!(
        model.set_pane_height(0, f64::INFINITY),
        Err(ChartModelError::InvalidDimensions)
    );
    let unknown = SeriesId::new(999);
    assert_eq!(
        model.remove_series(unknown, layer.remove_series(unknown).unwrap()),
        Err(ChartModelError::UnknownSeries(unknown))
    );
    assert_eq!(model.time_scale().width(), width);
    assert!(model.series(id).is_some());
}
