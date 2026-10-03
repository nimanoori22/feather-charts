use super::*;
use crate::{
    model::{
        axis_snapshots::PriceAxisSide,
        price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
        text_width_cache::{TextMeasurer, TextMetrics},
    },
    renderers::{
        grid_renderer::PixelRatio,
        price_axis_renderer::{
            AxisBounds, PriceAxisCommand, measure_price_axis, prepare_price_axis,
        },
    },
};
struct FixedMetrics;
impl TextMeasurer for FixedMetrics {
    fn measure_text(&mut self, label: &str) -> TextMetrics {
        TextMetrics {
            width: label.len() as f32 * 7.,
            ..Default::default()
        }
    }
}
fn assert_axis_grid_and_model(model: &mut Model) -> Vec<(String, f64)> {
    let axes = model.prepare_axis_snapshots(0).unwrap();
    let grid = model.panes()[0]
        .grid()
        .pane_view()
        .renderer()
        .data()
        .unwrap();
    assert_eq!(axes.right.marks, grid.price_marks);
    let measurement = measure_price_axis(&axes.right, &mut FixedMetrics).unwrap();
    let prepared = prepare_price_axis(
        &measurement,
        AxisBounds {
            width: 120.,
            height: model.panes()[0].height(),
        },
        PixelRatio {
            horizontal: 1.5,
            vertical: 1.5,
        },
    )
    .unwrap();
    let text = prepared
        .commands
        .iter()
        .filter_map(|command| match command {
            PriceAxisCommand::Text {
                label, position, ..
            } => Some((label.clone(), position.y)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        text,
        axes.right
            .marks
            .iter()
            .rev()
            .map(|mark| (mark.label.clone(), mark.coord.value()))
            .collect::<Vec<_>>()
    );
    text
}

#[test]
fn price_axis_rendering_real_model_preserves_all_mode_labels_and_grid_y() {
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
        assert!(!assert_axis_grid_and_model(&mut model).is_empty());
    }
}

#[test]
fn price_axis_rendering_real_model_resize_append_history_and_removal() {
    let (mut layer, mut model, id) = fixture();
    let original = assert_axis_grid_and_model(&mut model);
    model.fit_content();
    flush(&mut model);
    model.set_width(300.).unwrap();
    model.set_pane_height(0, 400.).unwrap();
    assert_ne!(assert_axis_grid_and_model(&mut model), original);
    model
        .apply_data_update(
            layer
                .update_series_data(id, line(21., 1000.), false)
                .unwrap(),
        )
        .unwrap();
    assert!(!assert_axis_grid_and_model(&mut model).is_empty());
    view(&mut model, 1., 10.);
    let before = model.visible_logical_range();
    model
        .apply_data_update(
            layer
                .update_series_data(id, line(8., -2000.), true)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(model.visible_logical_range(), before);
    assert!(!assert_axis_grid_and_model(&mut model).is_empty());
    model
        .remove_series(id, layer.remove_series(id).unwrap())
        .unwrap();
    assert!(assert_axis_grid_and_model(&mut model).is_empty());
}

#[test]
fn price_axis_two_series_can_be_removed_sequentially_without_stale_ids() {
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
                .set_series_data(second, SeriesType::Line, vec![line(1., 1000.)])
                .unwrap(),
        )
        .unwrap();
    model
        .remove_series(first, layer.remove_series(first).unwrap())
        .unwrap();
    let response = layer.remove_series(second).unwrap();
    assert!(!response.series.contains_key(&first));
    model.remove_series(second, response).unwrap();
    let axes = model.prepare_axis_snapshots(0).unwrap();
    assert!(axes.left.marks.is_empty() && axes.right.marks.is_empty());
}
