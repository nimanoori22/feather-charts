use crate::{
    model::{
        chart_model::ChartModel, data_layer::SeriesId, ihorz_scale_behavior::HorzScaleBehavior,
        plot_data::PlotRowLike, series_options::LastPriceAnimationMode,
    },
    renderers::series_last_price_animation_renderer::LastPriceAnimation,
    ui::line_chart::{FrameError, color_with_parser},
};
use std::{hash::Hash, time::Instant};

pub(crate) fn prepare<B, M>(
    model: &mut ChartModel<B, M>,
    id: SeriesId,
    ratio: f32,
    now: Instant,
) -> Result<Option<LastPriceAnimation>, FrameError>
where
    B: HorzScaleBehavior,
    B::Item: 'static,
    B::InternalItem: 'static,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone + 'static,
{
    let Some(visible) = model.visible_strict_range() else {
        return Ok(None);
    };
    let Some(series) = model.series(id) else {
        return Ok(None);
    };
    let Some((mode, width)) = series.animation_options() else {
        return Ok(None);
    };
    if mode == LastPriceAnimationMode::Disabled || !series.common_options().visible {
        return Ok(None);
    }
    let Some(row) = series.bars().last() else {
        return Ok(None);
    };
    let index = row.index();
    if !visible.contains(index) {
        return Ok(None);
    }
    let Some(first) = series.first_value(Some(&visible)) else {
        return Ok(None);
    };
    let Some(scale) = model
        .pane_for_series(id)
        .and_then(|p| model.panes()[p].price_scale_for_source(id))
        .filter(|s| !s.is_empty())
    else {
        return Ok(None);
    };
    let y = scale
        .price_to_coordinate(row.values()[3], first.value)
        .value();
    // The settled time scale is read below after releasing the series borrow.
    let color = color_with_parser(
        series.bar_color(row).expect("line-family row color"),
        &model.color_parser(),
    )?;
    let state = series
        .last_price_animation
        .expect("line-family animation state");
    drop(series);
    let x = model.time_scale().index_to_coordinate(index).value();
    if !x.is_finite() || !y.is_finite() {
        return Ok(None);
    }
    let stroke = ratio.floor().max(1.);
    let mut marker = LastPriceAnimation {
        series: id,
        center: iced::Point::new(
            ((x * f64::from(ratio) + 0.5).floor() as f32 + (stroke % 2.) / 2.) / ratio,
            y as f32,
        ),
        dot_radius: (width * 1.5).max(2.) as f32,
        radius: 0.,
        stroke_width: stroke / ratio,
        color,
        fill_color: color,
        stroke_color: color,
        state,
        mode,
        active: false,
    };
    marker.sample(now);
    Ok(Some(marker))
}
