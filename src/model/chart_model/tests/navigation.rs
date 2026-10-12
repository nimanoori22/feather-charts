use super::*;
use crate::model::coordinate::Coordinate;

fn populated() -> Model {
    let mut model = model_with(PaneOptions::default(), true);
    model.set_width(600.).unwrap();
    model.set_pane_height(0, 300.).unwrap();
    let id = SeriesId::new(1);
    model
        .register_series(
            id,
            SeriesType::Line,
            line_options(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let mut layer = Layer::new(HorzScaleBehaviorTime::default());
    model
        .apply_data_update(
            layer
                .set_series_data(
                    id,
                    SeriesType::Line,
                    (0..100).map(|i| line(i as f64, i as f64)).collect(),
                )
                .unwrap(),
        )
        .unwrap();
    view(&mut model, 10., 60.);
    model
}

#[test]
fn crosshair_tracks_raw_pointer_and_clears_after_data_removal() {
    use crate::model::crosshair::{CrosshairMode, CrosshairOptions};
    use crate::renderers::line_renderer::Point;
    let mut model = populated();
    model
        .set_crosshair_options(CrosshairOptions {
            mode: CrosshairMode::Normal,
            ..Default::default()
        })
        .unwrap();
    model
        .set_crosshair_position(0, Some(Point { x: 240., y: 120. }))
        .unwrap();
    let before = model.crosshair().position().unwrap();
    assert!((before.y.unwrap() - 120.).abs() < 1e-8);
    model.zoom_time(Coordinate::new(100.), 1.);
    assert_eq!(model.crosshair().origin().unwrap().point.x, 240.);
    assert_ne!(model.crosshair().position().unwrap().x, before.x);
    model
        .set_crosshair_position(
            0,
            Some(Point {
                x: f64::NAN,
                y: 120.,
            }),
        )
        .unwrap();
    assert!(model.crosshair().position().is_none());
    model
        .set_crosshair_position(0, Some(Point { x: 240., y: 120. }))
        .unwrap();
    let mut layer = Layer::new(HorzScaleBehaviorTime::default());
    layer
        .set_series_data(SeriesId::new(1), SeriesType::Line, vec![line(0., 1.)])
        .unwrap();
    let removal = layer.remove_series(SeriesId::new(1)).unwrap();
    model.remove_series(SeriesId::new(1), removal).unwrap();
    assert!(model.crosshair().position().is_none());
}

#[test]
fn magnet_uses_exact_visible_values_and_recomputes_after_updates() {
    use crate::renderers::line_renderer::Point;
    let mut model = populated();
    let id = SeriesId::new(1);
    let x = model
        .time_scale()
        .index_to_coordinate(TimePointIndex::new(30.))
        .value();
    let base = model.panes()[0].right_price_scale().first_value().unwrap();
    let expected = model.panes()[0]
        .right_price_scale()
        .price_to_coordinate(30., base)
        .value();
    model
        .set_crosshair_position(
            0,
            Some(Point {
                x,
                y: expected + 12.,
            }),
        )
        .unwrap();
    assert!((model.crosshair().position().unwrap().y.unwrap() - expected).abs() < 1e-8);
    let mut options = line_options();
    if let SeriesOptionsMap::Line(options) = &mut options {
        options.common.visible = false;
    }
    model.replace_series_options(id, options).unwrap();
    assert!(model.crosshair().position().unwrap().y.is_none());
}

#[test]
fn magnet_compares_coordinates_across_scales_and_excludes_overlays() {
    use crate::renderers::line_renderer::Point;
    let mut model = populated();
    let mut layer = Layer::new(HorzScaleBehaviorTime::default());
    let left = SeriesId::new(2);
    model
        .register_series(
            left,
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
                    left,
                    SeriesType::Line,
                    (0..100)
                        .map(|i| line(i as f64, 100. + (i % 10) as f64 * 5.))
                        .collect(),
                )
                .unwrap(),
        )
        .unwrap();
    let index = TimePointIndex::new(30.);
    let x = model.time_scale().index_to_coordinate(index).value();
    let scale = model.panes()[0].left_price_scale();
    let expected = scale
        .price_to_coordinate(100., scale.first_value().unwrap())
        .value();
    model
        .set_crosshair_position(
            0,
            Some(Point {
                x,
                y: expected - 2.,
            }),
        )
        .unwrap();
    assert!((model.crosshair().position().unwrap().y.unwrap() - expected).abs() < 1e-8);
    let overlay = SeriesId::new(3);
    model
        .register_series(
            overlay,
            SeriesType::Line,
            line_options(),
            0,
            PriceScalePosition::Overlay("study".into()),
        )
        .unwrap();
    model
        .apply_data_update(
            layer
                .set_series_data(
                    overlay,
                    SeriesType::Line,
                    (0..100).map(|i| line(i as f64, 200. + i as f64)).collect(),
                )
                .unwrap(),
        )
        .unwrap();
    model
        .set_crosshair_position(
            0,
            Some(Point {
                x,
                y: expected - 2.,
            }),
        )
        .unwrap();
    assert!((model.crosshair().position().unwrap().y.unwrap() - expected).abs() < 1e-8);
    let axes = model.prepare_axis_snapshots(0).unwrap();
    assert_eq!(
        axes.left.crosshair_label.as_ref().unwrap().coordinate,
        axes.right.crosshair_label.as_ref().unwrap().coordinate
    );
    assert_ne!(
        axes.left.crosshair_label.as_ref().unwrap().text,
        axes.right.crosshair_label.as_ref().unwrap().text
    );
}

#[test]
fn navigation_preserves_anchor_recalculates_and_ends_drag() {
    let mut model = populated();
    let spacing = model.time_scale().bar_spacing();
    let index = TimePointIndex::new(30.);
    let x = model.time_scale().index_to_coordinate(index);
    model.zoom_time(x, 1.);
    assert!((model.time_scale().bar_spacing() - spacing * 1.1).abs() < 1e-9);
    assert!((model.time_scale().index_to_coordinate(index).value() - x.value()).abs() < 1e-4);
    assert!(model.take_invalidation().is_some());
    let offset = model.time_scale().right_offset();
    model.start_scroll_time(Coordinate::new(200.));
    model.scroll_time_to(Coordinate::new(230.));
    assert!(
        (model.time_scale().right_offset() - offset + 30. / model.time_scale().bar_spacing()).abs()
            < 1e-9
    );
    model.end_scroll_time();
    let end = model.time_scale().right_offset();
    model.scroll_time_to(Coordinate::new(500.));
    assert_eq!(model.time_scale().right_offset(), end);
}

#[test]
fn zoom_respects_right_bar_policy_and_invalid_inputs() {
    let mut model = populated();
    let mut layer = Layer::new(HorzScaleBehaviorTime::default());
    model.apply_time_scale_options(
        &mut layer,
        HorzScaleOptionsPatch {
            right_bar_stays_on_scroll: Some(true),
            ..Default::default()
        },
    );
    let offset = model.time_scale().right_offset();
    model.zoom_time(Coordinate::new(120.), 1.);
    assert_eq!(model.time_scale().right_offset(), offset);
    let spacing = model.time_scale().bar_spacing();
    model.zoom_time(Coordinate::new(f64::NAN), 1.);
    model.zoom_time(Coordinate::new(120.), f64::INFINITY);
    assert_eq!(model.time_scale().bar_spacing(), spacing);
    let mut empty = model_with(PaneOptions::default(), true);
    empty.zoom_time(Coordinate::new(10.), 1.);
    assert_eq!(
        empty.time_scale().bar_spacing(),
        HorzScaleOptions::default().bar_spacing
    );
}

#[test]
fn crosshair_hidden_mode_and_invalid_options_preserve_raw_state() {
    use crate::{model::crosshair::CrosshairMode, renderers::line_renderer::Point};
    let mut model = populated();
    model
        .set_crosshair_position(0, Some(Point { x: 240., y: 120. }))
        .unwrap();
    let origin = model.crosshair().origin();
    let mut options = model.crosshair().options().clone();
    options.horizontal.color = "invalid-color".into();
    assert_eq!(
        model.set_crosshair_options(options),
        Err(ChartModelError::InvalidCrosshairOptions)
    );
    assert!(model.crosshair().position().is_some());
    let mut options = model.crosshair().options().clone();
    options.mode = CrosshairMode::Hidden;
    model.set_crosshair_options(options.clone()).unwrap();
    assert!(model.crosshair().position().is_none());
    assert!(model.crosshair_markers(0).is_empty());
    assert!(
        model
            .prepare_axis_snapshots(0)
            .unwrap()
            .time
            .crosshair_label
            .is_none()
    );
    assert_eq!(model.crosshair().origin(), origin);
    options.mode = CrosshairMode::Normal;
    model.set_crosshair_options(options).unwrap();
    assert!((model.crosshair().position().unwrap().y.unwrap() - 120.).abs() < 1e-8);
    model.set_pane_height(0, 50.).unwrap();
    assert!(model.crosshair().position().is_none());
}

#[test]
fn whitespace_has_no_magnet_candidate_and_optional_index_search_prefers_left_tie() {
    use crate::renderers::line_renderer::Point;
    let mut model = populated();
    let mut layer = Layer::new(HorzScaleBehaviorTime::default());
    let id = SeriesId::new(1);
    model
        .apply_data_update(
            layer
                .set_series_data(
                    id,
                    SeriesType::Line,
                    (0..100)
                        .map(|i| {
                            if i == 30 {
                                whitespace(i as f64)
                            } else {
                                line(i as f64, i as f64)
                            }
                        })
                        .collect(),
                )
                .unwrap(),
        )
        .unwrap();
    let x = model.time_scale().index_to_coordinate(30.0.into()).value();
    model
        .set_crosshair_position(0, Some(Point { x, y: 120. }))
        .unwrap();
    assert_eq!(model.crosshair().position().unwrap().index, 30.0.into());
    assert!((model.crosshair().position().unwrap().y.unwrap() - 120.).abs() < 1e-8);
    assert!(model.crosshair_markers(0).is_empty());
    let mut options = model.crosshair().options().clone();
    options.do_not_snap_to_hidden_series_indices = true;
    model.set_crosshair_options(options).unwrap();
    assert_eq!(model.crosshair().position().unwrap().index, 29.0.into());
    model
        .apply_data_update(
            layer
                .set_series_data(
                    id,
                    SeriesType::Line,
                    (0..100).map(|i| whitespace(i as f64)).collect(),
                )
                .unwrap(),
        )
        .unwrap();
    assert!(model.crosshair().position().unwrap().y.is_none());
    assert!(model.crosshair_markers(0).is_empty());
    model
        .apply_data_update(layer.set_series_data(id, SeriesType::Line, vec![]).unwrap())
        .unwrap();
    assert!(model.crosshair().position().is_none());
}
