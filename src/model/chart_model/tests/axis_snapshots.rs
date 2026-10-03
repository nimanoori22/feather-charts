use super::*;
use crate::model::{
    axis_snapshots::{AxisSnapshotError, PriceAxisOptionsError, PriceAxisSide, TimeLabelEmphasis},
    price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
};

fn assert_matches_marks(model: &mut Model) {
    let axes = model.prepare_axis_snapshots(0).unwrap();
    assert_eq!(
        axes.left.marks,
        model.panes[0]
            .axis_price_scale_mut(PriceAxisSide::Left)
            .marks()
    );
    assert_eq!(
        axes.right.marks,
        model.panes[0]
            .axis_price_scale_mut(PriceAxisSide::Right)
            .marks()
    );
    let context = model.layout_context();
    let marks = model.time_scale.marks(context).unwrap_or_default().to_vec();
    assert_eq!(
        axes.time
            .ticks
            .iter()
            .map(|tick| tick.mark.clone())
            .collect::<Vec<_>>(),
        marks
    );
    let grid = model.panes()[0]
        .grid()
        .pane_view()
        .renderer()
        .data()
        .unwrap();
    assert_eq!(grid.price_marks, axes.right.marks);
    assert_eq!(
        grid.time_marks
            .iter()
            .map(|mark| mark.coordinate.value())
            .collect::<Vec<_>>(),
        axes.time
            .ticks
            .iter()
            .map(|tick| tick.mark.coordinate)
            .collect::<Vec<_>>()
    );
}

#[test]
fn axis_snapshots_copy_scale_marks_and_share_grid_coordinates() {
    let (_, mut model, _) = fixture();
    assert_matches_marks(&mut model);
    let axes = model.prepare_axis_snapshots(0).unwrap();
    assert!(axes.left.marks.is_empty());
    assert!(!axes.right.marks.is_empty());
    assert!(!axes.time.ticks.is_empty());
    assert_eq!(axes.left.side, PriceAxisSide::Left);
    assert_eq!(axes.right.side, PriceAxisSide::Right);
    let threshold = model.time_scale().behavior().max_tick_mark_weight(
        &axes
            .time
            .ticks
            .iter()
            .map(|tick| tick.mark.clone())
            .collect::<Vec<_>>(),
    );
    for tick in axes.time.ticks {
        assert_eq!(tick.priority, tick.mark.weight >= threshold);
        assert_eq!(
            tick.emphasis == TimeLabelEmphasis::Emphasized,
            tick.priority
        );
    }
}

#[test]
fn axis_snapshots_prepare_independent_left_and_right_scales() {
    let (mut layer, mut model, _) = fixture();
    let left_id = SeriesId::new(2);
    model
        .register_series(
            left_id,
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
                    left_id,
                    SeriesType::Line,
                    (1..=20)
                        .map(|n| line(n as f64, 1000. + n as f64 * 100.))
                        .collect(),
                )
                .unwrap(),
        )
        .unwrap();
    let axes = model.prepare_axis_snapshots(0).unwrap();
    assert!(!axes.left.marks.is_empty());
    assert!(axes.left.marks.iter().all(|mark| mark.logical > 1000.));
    assert!(axes.right.marks.iter().all(|mark| mark.logical < 100.));
    assert_matches_marks(&mut model);
}

#[test]
fn axis_snapshots_preserve_all_price_modes_and_formatted_labels() {
    let (_, mut model, _) = fixture();
    for mode in [
        PriceScaleMode::Normal,
        PriceScaleMode::Percentage,
        PriceScaleMode::IndexedTo100,
        PriceScaleMode::Logarithmic,
    ] {
        model
            .apply_price_axis_options(
                0,
                PriceAxisSide::Right,
                PriceScaleOptionsPatch {
                    mode: Some(mode),
                    ..Default::default()
                },
            )
            .unwrap();
        let axes = model.prepare_axis_snapshots(0).unwrap();
        assert!(!axes.right.marks.is_empty(), "{mode:?}");
        assert_matches_marks(&mut model);
        assert!(
            axes.right
                .marks
                .iter()
                .all(|mark| mark.coord.value().is_finite() && !mark.label.is_empty())
        );
        if mode == PriceScaleMode::Percentage {
            assert!(
                axes.right
                    .marks
                    .iter()
                    .all(|mark| mark.label.ends_with('%'))
            );
        }
    }
}

#[test]
fn axis_snapshots_preserve_visibility_flags_without_filtering_labels() {
    let (mut layer, mut model, _) = fixture();
    let before = model.prepare_axis_snapshots(0).unwrap();
    for visible in [false, true] {
        for border in [false, true] {
            for ticks in [false, true] {
                model
                    .apply_price_axis_options(
                        0,
                        PriceAxisSide::Right,
                        PriceScaleOptionsPatch {
                            visible: Some(visible),
                            border_visible: Some(border),
                            ticks_visible: Some(ticks),
                            ..Default::default()
                        },
                    )
                    .unwrap();
                model.apply_time_scale_options(
                    &mut layer,
                    HorzScaleOptionsPatch {
                        visible: Some(visible),
                        border_visible: Some(border),
                        ticks_visible: Some(ticks),
                        ..Default::default()
                    },
                );
                let axes = model.prepare_axis_snapshots(0).unwrap();
                assert_eq!(
                    (
                        axes.right.visible,
                        axes.right.border_visible,
                        axes.right.ticks_visible
                    ),
                    (visible, border, ticks)
                );
                assert_eq!(
                    (
                        axes.time.visible,
                        axes.time.border_visible,
                        axes.time.ticks_visible
                    ),
                    (visible, border, ticks)
                );
                assert_eq!(axes.right.marks, before.right.marks);
                assert_eq!(axes.time.ticks, before.time.ticks);
                assert_eq!(axes.right.renderer_options.tick_length, 5.);
                assert_eq!(axes.time.renderer_options.tick_length, 5.);
            }
        }
    }
}

#[test]
fn axis_snapshots_resolve_overrides_and_source_spacing_and_size_policies() {
    let (mut layer, mut model, _) = fixture();
    let defaults = model.prepare_axis_snapshots(0).unwrap();
    assert_eq!(defaults.right.border_color, "#2B2B43");
    assert!(defaults.right.border_visible);
    assert!(!defaults.right.ticks_visible);
    assert_eq!(defaults.right.renderer_options.color, "#000");
    assert_eq!(defaults.right.renderer_options.padding_top, 2.5);
    assert_eq!(defaults.right.renderer_options.padding_inner, 5.);
    assert_eq!(defaults.time.renderer_options.padding_top, 3.);
    assert_eq!(defaults.time.renderer_options.padding_horizontal, 9.);
    assert_eq!(defaults.time.renderer_options.label_bottom_offset, 4.);
    assert_eq!(
        defaults.time.renderer_options.bold_font,
        "bold 12px sans-serif"
    );
    model
        .apply_price_axis_options(
            0,
            PriceAxisSide::Right,
            PriceScaleOptionsPatch {
                border_color: Some("#fed".into()),
                text_color: Some(Some("red".into())),
                minimum_width: Some(80.),
                ..Default::default()
            },
        )
        .unwrap();
    model.apply_time_scale_options(
        &mut layer,
        HorzScaleOptionsPatch {
            border_color: Some("blue".into()),
            minimum_height: Some(40.),
            ..Default::default()
        },
    );
    let changed = model.prepare_axis_snapshots(0).unwrap();
    assert_eq!(changed.right.renderer_options.color, "red");
    assert_eq!(changed.left.renderer_options.color, "#000");
    assert_eq!(changed.right.minimum_width, 80.);
    assert_eq!(changed.right.border_color, "#fed");
    assert_eq!(changed.time.border_color, "blue");
    assert_eq!(changed.time.minimum_height, 40.);
    model
        .apply_price_axis_options(
            0,
            PriceAxisSide::Right,
            PriceScaleOptionsPatch {
                text_color: Some(None),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        model
            .prepare_axis_snapshots(0)
            .unwrap()
            .right
            .renderer_options
            .color,
        "#000"
    );
}

#[test]
fn axis_snapshots_preserve_gradient_and_time_bottom_fill_policy() {
    let (_, mut model, _) = fixture();
    model.options.layout.background = Background::VerticalGradient {
        top_color: "#123".into(),
        bottom_color: "#456".into(),
    };
    model.options.layout.color_space = ColorSpace::DisplayP3;
    let axes = model.prepare_axis_snapshots(0).unwrap();
    assert_eq!(axes.left.background, model.options.layout.background);
    assert_eq!(axes.right.background, axes.time.background);
    assert_eq!(axes.right.color_space, ColorSpace::DisplayP3);
    assert_eq!(axes.time.color_space, ColorSpace::DisplayP3);
    assert_eq!(axes.right.renderer_options.pane_background_color, "#123");
    assert_eq!(axes.time.background_fill_color(), "#456");
}

#[test]
fn axis_snapshots_do_not_consume_invalidation_or_modify_previous_snapshots() {
    let (mut layer, mut model, id) = fixture();
    let original = model.prepare_axis_snapshots(0).unwrap();
    let saved = original.clone();
    model.fit_content();
    model.prepare_axis_snapshots(0).unwrap();
    let retained = model.take_invalidation().unwrap();
    assert!(
        retained
            .time_scale_invalidations()
            .iter()
            .any(|command| matches!(command, TimeScaleInvalidation::FitContent))
    );
    model.apply_invalidation(&retained);
    model.set_width(300.).unwrap();
    model.set_pane_height(0, 400.).unwrap();
    model
        .apply_data_update(
            layer
                .update_series_data(id, line(21., 200.), false)
                .unwrap(),
        )
        .unwrap();
    let new = model.prepare_axis_snapshots(0).unwrap();
    assert_ne!(new.right.marks, saved.right.marks);
    assert_eq!(original, saved);
    assert_matches_marks(&mut model);
}

#[test]
fn axis_snapshots_empty_removal_and_zero_dimensions_clear_old_marks() {
    let (_, mut model, _) = fixture();
    model.set_width(0.).unwrap();
    let zero = model.prepare_axis_snapshots(0).unwrap();
    assert!(
        zero.left.marks.is_empty() && zero.right.marks.is_empty() && zero.time.ticks.is_empty()
    );
    model.set_width(100.).unwrap();
    model.set_pane_height(0, 0.).unwrap();
    assert!(
        model
            .prepare_axis_snapshots(0)
            .unwrap()
            .right
            .marks
            .is_empty()
    );
    let (mut layer, mut model, id) = fixture();
    let old = model.prepare_axis_snapshots(0).unwrap();
    model
        .remove_series(id, layer.remove_series(id).unwrap())
        .unwrap();
    let removed = model.prepare_axis_snapshots(0).unwrap();
    assert!(removed.right.marks.is_empty() && removed.time.ticks.is_empty());
    assert!(!old.right.marks.is_empty());
    let empty = model_with(PaneOptions::default(), true)
        .prepare_axis_snapshots(0)
        .unwrap();
    assert!(
        empty.left.marks.is_empty() && empty.right.marks.is_empty() && empty.time.ticks.is_empty()
    );
}

#[test]
fn axis_snapshots_invalid_panes_and_options_are_atomic() {
    let (_, mut model, _) = fixture();
    let before = model.prepare_axis_snapshots(0).unwrap();
    model.take_invalidation();
    let generation = model.panes()[0].right_price_scale().marks_generation();
    assert_eq!(
        model.prepare_axis_snapshots(99),
        Err(AxisSnapshotError::InvalidPane(99))
    );
    assert_eq!(
        model.panes()[0].right_price_scale().marks_generation(),
        generation
    );
    assert!(model.take_invalidation().is_none());
    assert_eq!(
        model.apply_price_axis_options(99, PriceAxisSide::Left, Default::default()),
        Err(PriceAxisOptionsError::InvalidPane(99))
    );
    assert!(
        model
            .apply_price_axis_options(
                0,
                PriceAxisSide::Right,
                PriceScaleOptionsPatch {
                    text_color: Some(Some("red".into())),
                    scale_margins: Some(crate::model::price_scale::PriceScaleMarginsPatch {
                        top: Some(1.),
                        bottom: None
                    }),
                    ..Default::default()
                }
            )
            .is_err()
    );
    assert_eq!(model.prepare_axis_snapshots(0).unwrap(), before);
    assert!(model.take_invalidation().is_none());
}

#[test]
fn axis_snapshots_font_size_changes_refresh_price_and_time_mark_caches() {
    let (mut layer, mut model, id) = fixture();
    model.set_width(800.).unwrap();
    model.set_pane_height(0, 600.).unwrap();
    model
        .apply_data_update(
            layer
                .set_series_data(
                    id,
                    SeriesType::Line,
                    (1..=240)
                        .map(|n| line(n as f64 * 3600., n as f64))
                        .collect(),
                )
                .unwrap(),
        )
        .unwrap();
    model.fit_content();
    flush(&mut model);
    let small = model.prepare_axis_snapshots(0).unwrap();
    model.set_layout_typography(80., "Inter").unwrap();
    let large = model.prepare_axis_snapshots(0).unwrap();
    assert!(large.right.marks.len() < small.right.marks.len());
    assert!(large.time.ticks.len() < small.time.ticks.len());
    assert_eq!(large.right.renderer_options.font_size, 80.);
    assert_eq!(large.time.renderer_options.font_family, "Inter");
    assert_matches_marks(&mut model);
    // TimeScale's direct callers also get font-aware tick selection.
    let direct = model
        .time_scale
        .marks(TimeScaleLayoutContext { font_size: 12. })
        .unwrap()
        .to_vec();
    assert_eq!(
        direct,
        small
            .time
            .ticks
            .iter()
            .map(|tick| tick.mark.clone())
            .collect::<Vec<_>>()
    );
    for invalid in [0., -1., f64::NAN, f64::INFINITY, f64::MAX] {
        assert_eq!(
            model.set_layout_typography(invalid, "bad"),
            Err(ChartModelError::InvalidTypography)
        );
    }
    assert_eq!(model.prepare_axis_snapshots(0).unwrap(), large);
}

#[test]
fn axis_snapshots_time_alignment_and_priority_survive_bold_disabled() {
    let (mut layer, mut model, _) = fixture();
    model.apply_time_scale_options(
        &mut layer,
        HorzScaleOptionsPatch {
            fix_left_edge: Some(true),
            fix_right_edge: Some(true),
            ..Default::default()
        },
    );
    model.fit_content();
    flush(&mut model);
    let axes = model.prepare_axis_snapshots(0).unwrap();
    assert!(
        axes.time
            .ticks
            .iter()
            .any(|tick| tick.mark.need_align_coordinate)
    );
    assert!(axes.time.ticks.iter().any(|tick| tick.priority));
    model.apply_time_scale_options(
        &mut layer,
        HorzScaleOptionsPatch {
            allow_bold_labels: Some(false),
            ..Default::default()
        },
    );
    let plain = model.prepare_axis_snapshots(0).unwrap();
    for (before, after) in axes.time.ticks.iter().zip(&plain.time.ticks) {
        assert_eq!(before.mark, after.mark);
        assert_eq!(before.priority, after.priority);
        assert_eq!(after.emphasis, TimeLabelEmphasis::Normal);
    }
}

#[test]
fn axis_snapshots_follow_historical_updates_resize_and_animation_samples() {
    let (mut layer, mut model, id) = fixture();
    view(&mut model, 3., 12.);
    let initial = model.prepare_axis_snapshots(0).unwrap();
    let viewport = model.visible_logical_range();
    model
        .apply_data_update(layer.update_series_data(id, line(8., 150.), true).unwrap())
        .unwrap();
    assert_eq!(model.visible_logical_range(), viewport);
    assert_ne!(
        model.prepare_axis_snapshots(0).unwrap().right.marks,
        initial.right.marks
    );
    assert_matches_marks(&mut model);
    model.set_width(250.).unwrap();
    model.set_pane_height(0, 350.).unwrap();
    assert_matches_marks(&mut model);
    model.apply_animation_offset(-4.);
    assert_matches_marks(&mut model);
}

#[test]
fn axis_snapshots_ui_frame_shares_viewport_at_fractional_device_scales() {
    use crate::{
        model::series::line_pane_view::LinePaneView,
        renderers::grid_renderer::PixelRatio,
        ui::line_chart::{FrameLayout, prepare_frame},
    };
    let (_, mut model, id) = fixture();
    let mut controller = TimeScaleAnimationController::default();
    let mut line_view = LinePaneView::default();
    for ratio in [1., 1.25, 1.5, 2.] {
        let layout = FrameLayout {
            pane: 0,
            size: iced::Size::new(300., 200.),
            background: "#fff".into(),
            pixel_ratio: PixelRatio {
                horizontal: ratio,
                vertical: ratio,
            },
        };
        let (frame, _) = prepare_frame(
            &mut model,
            &mut controller,
            &mut line_view,
            Some(id),
            &layout,
            Instant::now(),
        )
        .unwrap();
        assert_eq!(
            frame.axes.unwrap(),
            model.prepare_axis_snapshots(0).unwrap()
        );
        assert_matches_marks(&mut model);
        assert!(!frame.lines.is_empty());
    }
    let now = Instant::now();
    model.scroll_to_offset_animated(-5., Duration::from_secs(1));
    let layout = FrameLayout {
        pane: 0,
        size: iced::Size::new(300., 200.),
        background: "#fff".into(),
        pixel_ratio: PixelRatio {
            horizontal: 1.5,
            vertical: 1.5,
        },
    };
    prepare_frame(
        &mut model,
        &mut controller,
        &mut line_view,
        Some(id),
        &layout,
        now,
    )
    .unwrap();
    let (frame, _) = prepare_frame(
        &mut model,
        &mut controller,
        &mut line_view,
        Some(id),
        &layout,
        now + Duration::from_millis(500),
    )
    .unwrap();
    assert!(controller.is_active());
    assert_eq!(
        frame.axes.unwrap(),
        model.prepare_axis_snapshots(0).unwrap()
    );
    assert_matches_marks(&mut model);
    for point in &line_view.data().unwrap().items {
        assert_eq!(
            point.x,
            model.time_scale().index_to_coordinate(point.index).value()
        );
    }
}
