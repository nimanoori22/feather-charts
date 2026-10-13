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
        time_data::{TimedValue, visible_timed_values},
        time_scale::TimeScale,
    },
    renderers::bars_renderer::{BarPoint, BarsRendererData},
};
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BarPreparationError {
    UnknownSeries(SeriesId),
    NotBarSeries(SeriesId),
    UnsupportedConflation,
    MissingPriceScale(SeriesId),
    NonFiniteCoordinates,
}

#[derive(Default)]
pub struct BarsPaneView {
    data: Option<BarsRendererData>,
}
impl BarsPaneView {
    pub fn data(&self) -> Option<&BarsRendererData> {
        self.data.as_ref()
    }
    pub fn clear(&mut self) {
        self.data = None;
    }
    pub fn refresh<B, M>(
        &mut self,
        model: &mut ChartModel<B, M>,
        id: SeriesId,
    ) -> Result<(), BarPreparationError>
    where
        B: HorzScaleBehavior,
        B::Item: 'static,
        B::InternalItem: 'static,
        B::Key: PartialOrd,
        B::CacheKey: Eq + Hash,
        M: Clone + 'static,
    {
        self.clear();
        self.data = model.prepare_bars(id)?;
        Ok(())
    }
}

pub(crate) fn prepare<B, M>(
    series: &Series<B::InternalItem, B::Item, M>,
    time: &mut TimeScale<B>,
    price: &PriceScale,
    height: f64,
) -> Result<Option<BarsRendererData>, BarPreparationError>
where
    B: HorzScaleBehavior,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone,
{
    let SeriesOptionsMap::Bar(options) = series.options() else {
        return Err(BarPreparationError::NotBarSeries(series.id()));
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
        return Err(BarPreparationError::UnsupportedConflation);
    }
    let Some(visible) = time.visible_strict_range() else {
        return Ok(None);
    };
    let Some(first) = series.first_value(Some(&visible)) else {
        return Ok(None);
    };
    let timed = series
        .bars()
        .rows()
        .iter()
        .map(|row| TimedValue {
            time: row.index(),
            x: 0.0.into(),
        })
        .collect::<Vec<_>>();
    let range = visible_timed_values(&timed, &visible, false);
    if range.from >= range.to {
        return Ok(None);
    }
    let mut items = Vec::with_capacity(range.to - range.from);
    for row in &series.bars().rows()[range.from..range.to] {
        let SeriesPlotRow::Bar(row) = row else {
            unreachable!("Bar Series contains Bar rows");
        };
        let [open, high, low, close] = row.base.value;
        let coordinates = [open, high, low, close]
            .map(|value| price.price_to_coordinate(value, first.value).value());
        let x = time.index_to_coordinate(row.base.index).value();
        if !x.is_finite() || coordinates.iter().any(|y| !y.is_finite()) {
            return Err(BarPreparationError::NonFiniteCoordinates);
        }
        let up = open <= close;
        let style = &options.style;
        items.push(BarPoint {
            x,
            open_y: coordinates[0],
            high_y: coordinates[1],
            low_y: coordinates[2],
            close_y: coordinates[3],
            color: row.color.clone().unwrap_or_else(|| {
                if up {
                    &style.up_color
                } else {
                    &style.down_color
                }
                .clone()
            }),
        });
    }
    let visible_range = crate::model::time_data::SeriesItemsIndexesRange {
        from: 0,
        to: items.len(),
    };
    Ok(Some(BarsRendererData {
        items,
        visible_range,
        bar_spacing: time.bar_spacing(),
        open_visible: options.style.open_visible,
        thin_bars: options.style.thin_bars,
    }))
}
