//! Stateless preparation with an owned output; no back-reference to ChartModel.
use crate::model::plot_data::PlotRowLike;
use crate::{
    model::{
        chart_model::ChartModel, data_layer::SeriesId, ihorz_scale_behavior::HorzScaleBehavior,
        price_scale::PriceScale, series::Series, series_data::SeriesPlotRow,
        series_options::SeriesOptionsMap, time_data::visible_range_by, time_scale::TimeScale,
    },
    renderers::histogram_renderer::{HistogramPoint, HistogramRendererData},
};
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistogramPreparationError {
    UnknownSeries(SeriesId),
    NotHistogramSeries(SeriesId),
    UnsupportedConflation,
    MissingPriceScale(SeriesId),
    NonFiniteCoordinates,
}

#[derive(Default)]
pub struct HistogramPaneView {
    data: Option<HistogramRendererData>,
}
impl HistogramPaneView {
    pub fn data(&self) -> Option<&HistogramRendererData> {
        self.data.as_ref()
    }
    pub fn clear(&mut self) {
        self.data = None;
    }
    pub fn refresh<B, M>(
        &mut self,
        model: &mut ChartModel<B, M>,
        id: SeriesId,
    ) -> Result<(), HistogramPreparationError>
    where
        B: HorzScaleBehavior,
        B::Item: 'static,
        B::InternalItem: 'static,
        B::Key: PartialOrd,
        B::CacheKey: Eq + Hash,
        M: Clone + 'static,
    {
        self.clear();
        self.data = model.prepare_histogram(id)?;
        Ok(())
    }
}

pub(crate) fn prepare<B, M>(
    series: &Series<B::InternalItem, B::Item, M>,
    time: &mut TimeScale<B>,
    price: &PriceScale,
    height: f64,
) -> Result<Option<HistogramRendererData>, HistogramPreparationError>
where
    B: HorzScaleBehavior,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone,
{
    let SeriesOptionsMap::Histogram(options) = series.options() else {
        return Err(HistogramPreparationError::NotHistogramSeries(series.id()));
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
        return Err(HistogramPreparationError::UnsupportedConflation);
    }
    let Some(visible) = time.visible_strict_range() else {
        return Ok(None);
    };
    let Some(first) = series.first_value(Some(&visible)) else {
        return Ok(None);
    };
    let range = visible_range_by(series.bars().rows(), &visible, true, PlotRowLike::index);
    if range.from >= range.to {
        return Ok(None);
    }
    let mut items = Vec::with_capacity(range.to - range.from);
    for row in &series.bars().rows()[range.from..range.to] {
        let SeriesPlotRow::Histogram(row) = row else {
            unreachable!("histogram Series contains histogram rows");
        };
        let y = price
            .price_to_coordinate(row.base.value[3], first.value)
            .value();
        let x = time.index_to_coordinate(row.base.index).value();
        if !x.is_finite() || !y.is_finite() {
            return Err(HistogramPreparationError::NonFiniteCoordinates);
        }
        items.push(HistogramPoint {
            index: row.base.index.value(),
            x,
            y,
            color: row
                .color
                .clone()
                .unwrap_or_else(|| options.style.color.clone()),
        });
    }
    let baseline = price
        .price_to_coordinate(options.style.base, first.value)
        .value();
    if !baseline.is_finite() {
        return Err(HistogramPreparationError::NonFiniteCoordinates);
    }
    let visible_range = crate::model::time_data::SeriesItemsIndexesRange {
        from: 0,
        to: items.len(),
    };
    Ok(Some(HistogramRendererData {
        items,
        visible_range,
        bar_spacing: time.bar_spacing(),
        baseline,
    }))
}
