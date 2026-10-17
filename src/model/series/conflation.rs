//! Cached drawing rows borrow canonical styles without replacing source data.
use super::*;
use crate::model::{
    data_conflater::{Bar, line_value},
    ihorz_scale_behavior::HorzScaleBehavior,
    time_data::{SeriesItemsIndexesRange, visible_range_by},
    time_scale::TimeScale,
};
use std::{cell::Ref, ops::Range};

pub(crate) struct RenderRows<'a, I, O, M> {
    raw: &'a [SeriesPlotRow<I, O, (), M>],
    bars: Option<Ref<'a, [Bar]>>,
}
pub(crate) struct RenderRow<'a, I, O, M> {
    pub row: &'a SeriesPlotRow<I, O, (), M>,
    pub value: [f64; 4],
    count: usize,
}
impl<I, O, M> RenderRow<'_, I, O, M> {
    pub fn line_value(&self) -> f64 {
        line_value(self.value, self.count)
    }
}
impl<I, O, M> RenderRows<'_, I, O, M> {
    pub fn len(&self) -> usize {
        self.bars.as_ref().map_or(self.raw.len(), |v| v.len())
    }
    pub fn visible(
        &self,
        range: &RangeImpl<TimePointIndex>,
        extended: bool,
    ) -> SeriesItemsIndexesRange {
        if let Some(bars) = &self.bars {
            visible_range_by(bars, range, extended, PlotRowLike::index)
        } else {
            visible_range_by(self.raw, range, extended, PlotRowLike::index)
        }
    }
    pub fn iter(&self, range: Range<usize>) -> impl Iterator<Item = RenderRow<'_, I, O, M>> {
        range.map(|i| {
            if let Some(bars) = &self.bars {
                let bar = &bars[i];
                RenderRow {
                    row: &self.raw[bar.first],
                    value: bar.value,
                    count: bar.count,
                }
            } else {
                let row = &self.raw[i];
                RenderRow {
                    row,
                    value: *PlotRowLike::values(row),
                    count: row.original_data_count().unwrap_or(1),
                }
            }
        })
    }
}
impl<I: Clone, O: Clone, M: Clone> Series<I, O, M> {
    pub(crate) fn render_rows<B: HorzScaleBehavior>(
        &self,
        time: &TimeScale<B>,
    ) -> RenderRows<'_, I, O, M>
    where
        B::Key: PartialOrd,
        B::CacheKey: Eq + std::hash::Hash,
    {
        let level =
            time.effective_conflation_factor(self.common_options().conflation_threshold_factor);
        let raw = self.data.rows();
        let bars = if level > 1 {
            self.conflation.borrow_mut().ensure(raw, level);
            Some(Ref::map(self.conflation.borrow(), |cache| {
                cache.rows(level)
            }))
        } else {
            None
        };
        RenderRows { raw, bars }
    }
}
