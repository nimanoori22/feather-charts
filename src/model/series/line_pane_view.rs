//! Stateless preparation with an owned output; no back-reference to ChartModel.
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
    renderers::line_renderer::{LinePoint, LineRendererData},
};
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinePreparationError {
    UnknownSeries(SeriesId),
    NotLineSeries(SeriesId),
    UnsupportedConflation,
    MissingPriceScale(SeriesId),
    NonFiniteCoordinates,
}

#[derive(Default)]
pub struct LinePaneView {
    data: Option<LineRendererData>,
}
impl LinePaneView {
    pub fn data(&self) -> Option<&LineRendererData> {
        self.data.as_ref()
    }
    pub fn clear(&mut self) {
        self.data = None;
    }
    pub fn refresh<B, M>(
        &mut self,
        model: &mut ChartModel<B, M>,
        id: SeriesId,
    ) -> Result<(), LinePreparationError>
    where
        B: HorzScaleBehavior,
        B::Item: 'static,
        B::InternalItem: 'static,
        B::Key: PartialOrd,
        B::CacheKey: Eq + Hash,
        M: Clone + 'static,
    {
        self.clear();
        self.data = model.prepare_line(id)?;
        Ok(())
    }
}

pub(crate) fn prepare<B, M>(
    series: &Series<B::InternalItem, B::Item, M>,
    time: &mut TimeScale<B>,
    price: &PriceScale,
    height: f64,
) -> Result<Option<LineRendererData>, LinePreparationError>
where
    B: HorzScaleBehavior,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone,
{
    let SeriesOptionsMap::Line(options) = series.options() else {
        return Err(LinePreparationError::NotLineSeries(series.id()));
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
        return Err(LinePreparationError::UnsupportedConflation);
    }
    let Some(visible) = time.visible_strict_range() else {
        return Ok(None);
    };
    let Some(first) = series.first_value(Some(&visible)) else {
        return Ok(None);
    };
    let mut items = Vec::with_capacity(series.bars().rows().len());
    let mut timed = Vec::with_capacity(items.capacity());
    for row in series.bars().rows() {
        let SeriesPlotRow::Line(row) = row else {
            unreachable!("line Series contains line rows");
        };
        let index = row.base.index;
        let x = time.index_to_coordinate(index).value();
        let y = price
            .price_to_coordinate(row.base.value[3], first.value)
            .value();
        if !x.is_finite() || !y.is_finite() {
            return Err(LinePreparationError::NonFiniteCoordinates);
        }
        timed.push(TimedValue {
            time: index,
            x: x.into(),
        });
        items.push(LinePoint {
            index,
            x,
            y,
            color: row
                .color
                .clone()
                .unwrap_or_else(|| options.style.color.clone()),
        });
    }
    let range = visible_timed_values(&timed, &visible, true);
    if range.from >= range.to {
        return Ok(None);
    }
    let style = &options.style;
    Ok(Some(LineRendererData {
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
    }))
}
