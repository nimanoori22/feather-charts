use super::*;
use crate::{
    model::{series::line_pane_view::LinePaneView, time_scale_options::HorzScaleOptionsPatch},
    renderers::{
        grid_renderer::PixelRatio,
        line_renderer::PathOperation,
        price_axis_renderer::AxisBounds,
        time_axis_renderer::tests::Metrics,
        time_axis_renderer::{TimeAxisCommand, measure_time_axis, prepare_time_axis},
    },
    ui::line_chart::{FrameLayout, PlotSnapshot, prepare_frame},
};

fn frame(
    model: &mut Model,
    id: Option<SeriesId>,
    controller: &mut TimeScaleAnimationController,
    now: Instant,
) -> PlotSnapshot {
    let layout = FrameLayout {
        pane: 0,
        size: iced::Size::new(
            model.time_scale().width() as f32,
            model.panes()[0].height() as f32,
        ),
        pixel_ratio: PixelRatio {
            horizontal: 1.5,
            vertical: 1.5,
        },
        background: "#fff".into(),
    };
    let (snapshot, _) = prepare_frame(
        model,
        controller,
        &mut LinePaneView::default(),
        id,
        &layout,
        now,
    )
    .unwrap();
    let time = &snapshot.axes.as_ref().unwrap().time;
    let measurement = measure_time_axis(time, &mut Metrics(6.), &mut Metrics(8.)).unwrap();
    let axis = prepare_time_axis(
        &measurement,
        AxisBounds {
            width: f64::from(layout.size.width),
            height: 40.,
        },
        layout.pixel_ratio,
    )
    .unwrap();
    let rects = axis
        .commands
        .iter()
        .filter_map(|c| match c {
            TimeAxisCommand::Rectangle { rect, .. } if rect.size.height > 2. => Some(rect.origin.x),
            _ => None,
        })
        .collect::<Vec<_>>();
    if time.border_visible && time.ticks_visible {
        let grid_x = snapshot
            .grid
            .iter()
            .filter_map(|s| match s.path.first() {
                Some(PathOperation::MoveTo(p))
                    if s.path.iter().any(
                        |op| matches!(op, PathOperation::LineTo(q) if q.x == p.x && q.y != p.y),
                    ) =>
                {
                    Some(p.x)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(rects.len(), time.ticks.len());
        for (x, tick) in rects.iter().zip(time.ticks.iter().rev()) {
            let snapped = (tick.mark.coordinate * 1.5 + 0.5).floor() / 1.5;
            assert!((x - snapped).abs() < 1e-8);
            assert!(grid_x.iter().any(|gx| (gx - snapped).abs() < 1e-4));
        }
    }
    assert_eq!(
        axis.commands
            .iter()
            .filter(|c| matches!(c, TimeAxisCommand::Text { .. }))
            .count(),
        time.ticks.len()
    );
    snapshot
}

#[test]
fn connected_time_axis_fit_resize_updates_removal_and_owned_frames() {
    let (mut layer, mut model, id) = fixture();
    model.apply_time_scale_options(
        &mut layer,
        HorzScaleOptionsPatch {
            ticks_visible: Some(true),
            fix_left_edge: Some(true),
            fix_right_edge: Some(true),
            ..Default::default()
        },
    );
    let mut controller = TimeScaleAnimationController::default();
    let now = Instant::now();
    let first = frame(&mut model, Some(id), &mut controller, now);
    let original = first.axes.as_ref().unwrap().time.clone();
    assert!(!original.ticks.is_empty());
    model.fit_content();
    model.set_width(300.).unwrap();
    model.set_pane_height(0, 400.).unwrap();
    let resized = frame(&mut model, Some(id), &mut controller, now);
    assert_ne!(resized.axes.as_ref().unwrap().time.ticks, original.ticks);
    view(&mut model, 1., 10.);
    let before = model.visible_logical_range();
    model
        .apply_data_update(
            layer
                .update_series_data(id, line(21., 1000.), false)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(model.visible_logical_range(), before);
    frame(&mut model, Some(id), &mut controller, now);
    model
        .apply_data_update(layer.update_series_data(id, line(8., -100.), true).unwrap())
        .unwrap();
    frame(&mut model, Some(id), &mut controller, now);
    model
        .remove_series(id, layer.remove_series(id).unwrap())
        .unwrap();
    let removed = frame(&mut model, None, &mut controller, now);
    assert!(removed.axes.as_ref().unwrap().time.ticks.is_empty());
    assert!(removed.lines.is_empty());
    assert_eq!(first.axes.as_ref().unwrap().time, original);
    model.set_width(0.).unwrap();
    assert!(
        frame(&mut model, None, &mut controller, now)
            .axes
            .unwrap()
            .time
            .ticks
            .is_empty()
    );
}

#[test]
fn time_axis_animation_and_typography_use_the_same_settled_frame() {
    let (mut layer, mut model, id) = fixture();
    model.apply_time_scale_options(
        &mut layer,
        HorzScaleOptionsPatch {
            ticks_visible: Some(true),
            ..Default::default()
        },
    );
    let mut controller = TimeScaleAnimationController::default();
    let now = Instant::now();
    let before = frame(&mut model, Some(id), &mut controller, now);
    model.scroll_to_offset_animated(-10., Duration::from_secs(1));
    frame(&mut model, Some(id), &mut controller, now);
    let sample = frame(
        &mut model,
        Some(id),
        &mut controller,
        now + Duration::from_millis(500),
    );
    assert_ne!(
        sample.axes.as_ref().unwrap().time.ticks,
        before.axes.as_ref().unwrap().time.ticks
    );
    assert_ne!(format!("{:?}", sample.lines), format!("{:?}", before.lines));
    model.set_layout_typography(24., "monospace").unwrap();
    let updated = frame(
        &mut model,
        Some(id),
        &mut controller,
        now + Duration::from_millis(750),
    );
    assert_eq!(
        updated
            .axes
            .as_ref()
            .unwrap()
            .time
            .renderer_options
            .font_size,
        24.
    );
    let time = &updated.axes.as_ref().unwrap().time;
    assert_eq!(
        measure_time_axis(time, &mut Metrics(6.), &mut Metrics(8.))
            .unwrap()
            .required_height(),
        50.
    );
}

#[test]
fn fixed_edges_zero_width_frames_keep_spacing_and_offsets_finite() {
    let (mut layer, mut model, id) = fixture();
    model.set_width(0.).unwrap();
    model.apply_time_scale_options(
        &mut layer,
        HorzScaleOptionsPatch {
            fix_left_edge: Some(true),
            fix_right_edge: Some(true),
            ..Default::default()
        },
    );
    let mut controller = TimeScaleAnimationController::default();
    let now = Instant::now();
    let empty = frame(&mut model, Some(id), &mut controller, now);
    assert!(empty.axes.unwrap().time.ticks.is_empty());
    assert!(model.time_scale().bar_spacing().is_finite() && model.time_scale().bar_spacing() > 0.);
    assert!(model.time_scale().right_offset().is_finite());
    model.set_width(300.).unwrap();
    model.fit_content();
    assert!(
        !frame(&mut model, Some(id), &mut controller, now)
            .axes
            .unwrap()
            .time
            .ticks
            .is_empty()
    );
}
