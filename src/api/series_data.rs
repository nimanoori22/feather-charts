//! Owned canonical data, independent of prepared or conflated geometry.
use super::{Chart, ChartBehavior, ChartError};
use crate::model::{
    data_consumer::*,
    data_layer::SeriesId,
    plot_data::{MutablePlotRow, PlotRowLike},
    plot_list::MismatchDirection,
    range_impl::RangeImpl,
    series_data::SeriesPlotRow,
    time_data::{LogicalRange, TimePointIndex},
    time_scale_visible_range::TimeScaleVisibleRange,
};

#[derive(Clone, Debug, PartialEq)]
pub struct BarsInfo<T> {
    pub bars_before: f64,
    pub bars_after: f64,
    pub data_range: Option<(T, T)>,
}

pub(super) fn public_data<I, O: Clone>(
    row: &SeriesPlotRow<I, O>,
) -> Option<BuiltInSeriesDataItem<O>> {
    Some(match row {
        SeriesPlotRow::Line(row) => BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time: row.original_time().clone(),
            value: PlotRowLike::values(row)[3],
            color: row.color.clone(),
            custom_values: row.base.custom_values,
        })),
        SeriesPlotRow::Area(row) => BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
            time: row.original_time().clone(),
            value: PlotRowLike::values(row)[3],
            line_color: row.line_color.clone(),
            top_color: row.top_color.clone(),
            bottom_color: row.bottom_color.clone(),
            custom_values: row.base.custom_values,
        })),
        SeriesPlotRow::Bar(row) => {
            let v = PlotRowLike::values(row);
            BuiltInSeriesDataItem::Bar(BarDataItem::Data(BarData {
                time: row.original_time().clone(),
                open: v[0],
                high: v[1],
                low: v[2],
                close: v[3],
                color: row.color.clone(),
                custom_values: row.base.custom_values,
            }))
        }
        SeriesPlotRow::Baseline(row) => {
            BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(BaselineData {
                time: row.original_time().clone(),
                value: PlotRowLike::values(row)[3],
                top_line_color: row.top_line_color.clone(),
                bottom_line_color: row.bottom_line_color.clone(),
                top_fill_color1: row.top_fill_color1.clone(),
                top_fill_color2: row.top_fill_color2.clone(),
                bottom_fill_color1: row.bottom_fill_color1.clone(),
                bottom_fill_color2: row.bottom_fill_color2.clone(),
                custom_values: row.base.custom_values,
            }))
        }
        SeriesPlotRow::Histogram(row) => {
            BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
                time: row.original_time().clone(),
                value: PlotRowLike::values(row)[3],
                color: row.color.clone(),
                custom_values: row.base.custom_values,
            }))
        }
        SeriesPlotRow::Candlestick(row) => {
            let v = PlotRowLike::values(row);
            BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(CandlestickData {
                time: row.original_time().clone(),
                open: v[0],
                high: v[1],
                low: v[2],
                close: v[3],
                color: row.color.clone(),
                border_color: row.border_color.clone(),
                wick_color: row.wick_color.clone(),
                custom_values: row.base.custom_values,
            }))
        }
        _ => return None,
    })
}

pub(super) fn validate_index(index: TimePointIndex) -> Result<(), ChartError> {
    if index.value().is_finite() {
        Ok(())
    } else {
        Err(ChartError::InvalidDataQuery)
    }
}
pub(super) fn find_row<R: PlotRowLike>(
    rows: &[R],
    index: TimePointIndex,
    direction: MismatchDirection,
) -> Option<&R> {
    let at = rows.partition_point(|r| PlotRowLike::index(r) < index);
    if rows.get(at).is_some_and(|r| PlotRowLike::index(r) == index) {
        return rows.get(at);
    }
    match direction {
        MismatchDirection::None => None,
        MismatchDirection::NearestLeft => at.checked_sub(1).and_then(|i| rows.get(i)),
        MismatchDirection::NearestRight => rows.get(at),
    }
}
pub(super) fn range_info<R, T: Clone>(
    rows: &[R],
    range: Option<LogicalRange>,
) -> Result<Option<BarsInfo<T>>, ChartError>
where
    R: PlotRowLike + MutablePlotRow<OriginalTime = T>,
{
    let Some(range) = range else { return Ok(None) };
    if !range.from.value().is_finite() || !range.to.value().is_finite() || range.from > range.to {
        return Err(ChartError::InvalidDataQuery);
    }
    let (Some(first), Some(last)) = (rows.first(), rows.last()) else {
        return Ok(None);
    };
    let first = PlotRowLike::index(first);
    let last = PlotRowLike::index(last);
    let strict = TimeScaleVisibleRange::new(Some(RangeImpl::new(range.from, range.to)))
        .strict_range()
        .expect("range present");
    let left = find_row(rows, strict.left(), MismatchDirection::NearestRight);
    let right = find_row(rows, strict.right(), MismatchDirection::NearestLeft);
    let gap = left
        .zip(right)
        .is_some_and(|(a, b)| PlotRowLike::index(a) > PlotRowLike::index(b));
    let before = if gap || left.is_none_or(|r| PlotRowLike::index(r) == first) {
        range.from.value() - first.value()
    } else {
        PlotRowLike::index(left.unwrap()).value() - first.value()
    };
    let after = if gap || right.is_none_or(|r| PlotRowLike::index(r) == last) {
        last.value() - range.to.value()
    } else {
        last.value() - PlotRowLike::index(right.unwrap()).value()
    };
    Ok(Some(BarsInfo {
        bars_before: before,
        bars_after: after,
        data_range: if gap {
            None
        } else {
            left.zip(right)
                .map(|(a, b)| (a.original_time().clone(), b.original_time().clone()))
        },
    }))
}
impl<B: ChartBehavior> Chart<B> {
    /// Canonical fulfilled bars, oldest first. Whitespace and conflation samples are excluded.
    pub fn series_data(
        &self,
        id: SeriesId,
    ) -> Result<Vec<BuiltInSeriesDataItem<B::Item>>, ChartError> {
        self.kind(id)?;
        let series = self.model.series(id).ok_or(ChartError::UnsupportedSeries)?;
        Ok(series
            .bars()
            .rows()
            .iter()
            .filter_map(public_data)
            .collect())
    }
    /// Read a shared logical index, optionally searching across gaps.
    pub fn data_by_index(
        &self,
        id: SeriesId,
        index: TimePointIndex,
        direction: MismatchDirection,
    ) -> Result<Option<BuiltInSeriesDataItem<B::Item>>, ChartError> {
        self.kind(id)?;
        validate_index(index)?;
        let series = self.model.series(id).ok_or(ChartError::UnsupportedSeries)?;
        Ok(series.bars().search(index, direction).and_then(public_data))
    }
    /// Fractional counts outside a logical range and paired original-item endpoints.
    pub fn bars_in_logical_range(
        &self,
        id: SeriesId,
        range: Option<LogicalRange>,
    ) -> Result<Option<BarsInfo<B::Item>>, ChartError> {
        self.kind(id)?;
        let series = self.model.series(id).ok_or(ChartError::UnsupportedSeries)?;
        range_info(series.bars().rows(), range)
    }
    /// Remove fulfilled trailing bars, returning newest first. The series remains registered.
    pub fn pop_data(
        &mut self,
        id: SeriesId,
        count: usize,
    ) -> Result<Vec<BuiltInSeriesDataItem<B::Item>>, ChartError> {
        self.kind(id)?;
        let empty = self
            .model
            .series(id)
            .ok_or(ChartError::UnsupportedSeries)?
            .bars()
            .is_empty();
        if count == 0 || empty {
            return Ok(vec![]);
        }
        let (removed, update) = self
            .layer
            .pop_series_data(id, count)
            .map_err(B::data_error)?;
        self.cancel_interaction();
        self.model.apply_data_update(update)?;
        self.last_hit = None;
        self.last_primitive = None;
        Ok(removed.iter().filter_map(public_data).collect())
    }
}
