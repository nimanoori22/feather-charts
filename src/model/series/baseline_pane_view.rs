//! Stateless preparation with an owned output; no back-reference to ChartModel.
use crate::model::plot_data::PlotRowLike;
use crate::{
    model::{
        chart_model::ChartModel,
        data_layer::SeriesId,
        ihorz_scale_behavior::HorzScaleBehavior,
        price_scale::PriceScale,
        series::Series,
        series_data::SeriesPlotRow,
        series_options::SeriesOptionsMap,
        time_data::{SeriesItemsIndexesRange, visible_range_by},
        time_scale::TimeScale,
    },
    renderers::{
        baseline_renderer::BaselineRendererData,
        line_renderer::{LinePoint, LineRendererData},
    },
};
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BaselinePreparationError {
    UnknownSeries(SeriesId),
    NotBaselineSeries(SeriesId),
    UnsupportedConflation,
    MissingPriceScale(SeriesId),
    NonFiniteCoordinates,
}

#[derive(Default)]
pub struct BaselinePaneView {
    data: Option<BaselineRendererData>,
}
impl BaselinePaneView {
    pub fn data(&self) -> Option<&BaselineRendererData> {
        self.data.as_ref()
    }
    pub fn clear(&mut self) {
        self.data = None;
    }
    pub fn refresh<B, M>(
        &mut self,
        model: &mut ChartModel<B, M>,
        id: SeriesId,
    ) -> Result<(), BaselinePreparationError>
    where
        B: HorzScaleBehavior,
        B::Item: 'static,
        B::InternalItem: 'static,
        B::Key: PartialOrd,
        B::CacheKey: Eq + Hash,
        M: Clone + 'static,
    {
        self.clear();
        self.data = model.prepare_baseline(id)?;
        Ok(())
    }
}

pub(crate) fn prepare<B, M>(
    series: &Series<B::InternalItem, B::Item, M>,
    time: &mut TimeScale<B>,
    price: &PriceScale,
    height: f64,
) -> Result<Option<BaselineRendererData>, BaselinePreparationError>
where
    B: HorzScaleBehavior,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone,
{
    let SeriesOptionsMap::Baseline(options) = series.options() else {
        return Err(BaselinePreparationError::NotBaselineSeries(series.id()));
    };
    if !options.common.visible
        || time.width() <= 0.0
        || height <= 0.0
        || time.is_empty()
        || price.is_empty()
    {
        return Ok(None);
    }
    if time.options().enable_conflation {
        return Err(BaselinePreparationError::UnsupportedConflation);
    }
    let Some(visible) = time.visible_strict_range() else {
        return Ok(None);
    };
    let Some(first) = series.first_value(Some(&visible)) else {
        return Ok(None);
    };
    let rows = series.bars().rows();
    let visible_rows = visible_range_by(rows, &visible, true, PlotRowLike::index);
    if visible_rows.from >= visible_rows.to {
        return Ok(None);
    }
    // Curved segments need one control-point neighbor beyond each drawing edge.
    let from = visible_rows.from.saturating_sub(1);
    let to = (visible_rows.to + 1).min(rows.len());
    let range = SeriesItemsIndexesRange {
        from: visible_rows.from - from,
        to: visible_rows.to - from,
    };
    let mut items = Vec::with_capacity(to - from);
    let mut fill_colors = Vec::with_capacity(items.capacity());
    let mut stroke_colors = Vec::with_capacity(items.capacity());
    for row in &rows[from..to] {
        let color = series.bar_color(row).unwrap().to_owned();
        let SeriesPlotRow::Baseline(row) = row else {
            unreachable!("Baseline Series contains Baseline rows");
        };
        let index = row.base.index;
        let x = time.index_to_coordinate(index).value();
        let y = price
            .price_to_coordinate(row.base.value[3], first.value)
            .value();
        if !x.is_finite() || !y.is_finite() {
            return Err(BaselinePreparationError::NonFiniteCoordinates);
        }
        fill_colors.push([
            row.top_fill_color1
                .clone()
                .unwrap_or_else(|| options.style.top_fill_color1.clone()),
            row.top_fill_color2
                .clone()
                .unwrap_or_else(|| options.style.top_fill_color2.clone()),
            row.bottom_fill_color1
                .clone()
                .unwrap_or_else(|| options.style.bottom_fill_color1.clone()),
            row.bottom_fill_color2
                .clone()
                .unwrap_or_else(|| options.style.bottom_fill_color2.clone()),
        ]);
        stroke_colors.push([
            row.top_line_color
                .clone()
                .unwrap_or_else(|| options.style.top_line_color.clone()),
            row.bottom_line_color
                .clone()
                .unwrap_or_else(|| options.style.bottom_line_color.clone()),
        ]);
        items.push(LinePoint { index, x, y, color });
    }
    let style = &options.style;
    let crate::model::series_options::BaseValueType::Price(base) = &style.base_value;
    let base = price.price_to_coordinate(base.price, first.value).value();
    if !base.is_finite() {
        return Err(BaselinePreparationError::NonFiniteCoordinates);
    }
    let top = style.relative_gradient.then(|| {
        items[range.from..range.to]
            .iter()
            .map(|p| p.y)
            .fold(f64::INFINITY, f64::min)
    });
    let bottom = style.relative_gradient.then(|| {
        items[range.from..range.to]
            .iter()
            .map(|p| p.y)
            .fold(f64::NEG_INFINITY, f64::max)
    });
    Ok(Some(BaselineRendererData {
        fill_colors,
        stroke_colors,
        fill_type: style.line_type,
        height,
        base,
        top,
        bottom,
        line: LineRendererData {
            items,
            visible_range: range,
            bar_width: time.bar_spacing(),
            line_type: style.line_visible.then_some(style.line_type),
            line_width: style.line_width,
            line_style: style.line_style,
            point_markers_radius: style.point_markers_visible.then(|| {
                style
                    .point_markers_radius
                    .filter(|radius| *radius != 0.0)
                    .unwrap_or(f64::from(style.line_width.pixels()) / 2.0 + 2.0)
            }),
        },
    }))
}
