//! Stateless preparation with an owned output; no back-reference to ChartModel.
use crate::model::plot_data::PlotRowLike;
use crate::{
    model::{
        chart_model::ChartModel, data_layer::SeriesId, ihorz_scale_behavior::HorzScaleBehavior,
        price_scale::PriceScale, series::Series, series_data::SeriesPlotRow,
        series_options::SeriesOptionsMap, time_data::visible_range_by, time_scale::TimeScale,
    },
    renderers::candlesticks_renderer::{CandlePoint, CandlestickRendererData},
};
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandlestickPreparationError {
    UnknownSeries(SeriesId),
    NotCandlestickSeries(SeriesId),
    UnsupportedConflation,
    MissingPriceScale(SeriesId),
    NonFiniteCoordinates,
}

#[derive(Default)]
pub struct CandlesticksPaneView {
    data: Option<CandlestickRendererData>,
}
impl CandlesticksPaneView {
    pub fn data(&self) -> Option<&CandlestickRendererData> {
        self.data.as_ref()
    }
    pub fn clear(&mut self) {
        self.data = None;
    }
    pub fn refresh<B, M>(
        &mut self,
        model: &mut ChartModel<B, M>,
        id: SeriesId,
    ) -> Result<(), CandlestickPreparationError>
    where
        B: HorzScaleBehavior,
        B::Item: 'static,
        B::InternalItem: 'static,
        B::Key: PartialOrd,
        B::CacheKey: Eq + Hash,
        M: Clone + 'static,
    {
        self.clear();
        self.data = model.prepare_candlesticks(id)?;
        Ok(())
    }
}

pub(crate) fn prepare<B, M>(
    series: &Series<B::InternalItem, B::Item, M>,
    time: &mut TimeScale<B>,
    price: &PriceScale,
    height: f64,
) -> Result<Option<CandlestickRendererData>, CandlestickPreparationError>
where
    B: HorzScaleBehavior,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone,
{
    let SeriesOptionsMap::Candlestick(options) = series.options() else {
        return Err(CandlestickPreparationError::NotCandlestickSeries(
            series.id(),
        ));
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
        return Err(CandlestickPreparationError::UnsupportedConflation);
    }
    let Some(visible) = time.visible_strict_range() else {
        return Ok(None);
    };
    let Some(first) = series.first_value(Some(&visible)) else {
        return Ok(None);
    };
    let range = visible_range_by(series.bars().rows(), &visible, false, PlotRowLike::index);
    if range.from >= range.to {
        return Ok(None);
    }
    let mut items = Vec::with_capacity(range.to - range.from);
    for row in &series.bars().rows()[range.from..range.to] {
        let SeriesPlotRow::Candlestick(row) = row else {
            unreachable!("candlestick Series contains candle rows");
        };
        let [open, high, low, close] = row.base.value;
        let coordinates = [open, high, low, close]
            .map(|value| price.price_to_coordinate(value, first.value).value());
        let x = time.index_to_coordinate(row.base.index).value();
        if !x.is_finite() || coordinates.iter().any(|y| !y.is_finite()) {
            return Err(CandlestickPreparationError::NonFiniteCoordinates);
        }
        let up = open <= close;
        let style = &options.style;
        items.push(CandlePoint {
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
            border_color: row.border_color.clone().unwrap_or_else(|| {
                if up {
                    &style.border_up_color
                } else {
                    &style.border_down_color
                }
                .clone()
            }),
            wick_color: row.wick_color.clone().unwrap_or_else(|| {
                if up {
                    &style.wick_up_color
                } else {
                    &style.wick_down_color
                }
                .clone()
            }),
        });
    }
    let visible_range = crate::model::time_data::SeriesItemsIndexesRange {
        from: 0,
        to: items.len(),
    };
    Ok(Some(CandlestickRendererData {
        items,
        visible_range,
        bar_spacing: time.bar_spacing(),
        wick_visible: options.style.wick_visible,
        border_visible: options.style.border_visible,
    }))
}
