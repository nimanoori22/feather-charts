//! Canonical OHLC plot rows shared by every series.

use crate::model::time_data::TimePointIndex;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum PlotRowValueIndex {
    Open = 0,
    High = 1,
    Low = 2,
    Close = 3,
}

pub type PlotRowValue = [f64; 4];

#[derive(Clone, Debug, PartialEq)]
pub struct PlotRow<InternalTime, OriginalTime, CustomValues = ()> {
    pub index: TimePointIndex,
    pub time: InternalTime,
    pub original_time: OriginalTime,
    pub value: PlotRowValue,
    pub custom_values: Option<CustomValues>,
    pub original_data_count: Option<usize>,
}

pub trait PlotRowLike {
    fn index(&self) -> TimePointIndex;
    fn values(&self) -> &PlotRowValue;
}

/// Common mutable access required by DataLayer when a time point is inserted
/// or removed. Unlike [`PlotRowLike`], whitespace rows are supported and
/// therefore expose their OHLC values as optional.
pub trait MutablePlotRow {
    type InternalTime;
    type OriginalTime;

    fn index(&self) -> TimePointIndex;
    fn set_index(&mut self, index: TimePointIndex);
    fn time(&self) -> &Self::InternalTime;
    fn original_time(&self) -> &Self::OriginalTime;
    fn values(&self) -> Option<&PlotRowValue>;
}
impl<I, O, C> PlotRowLike for PlotRow<I, O, C> {
    fn index(&self) -> TimePointIndex {
        self.index
    }
    fn values(&self) -> &PlotRowValue {
        &self.value
    }
}

impl<I, O, C> MutablePlotRow for PlotRow<I, O, C> {
    type InternalTime = I;
    type OriginalTime = O;

    fn index(&self) -> TimePointIndex {
        self.index
    }
    fn set_index(&mut self, index: TimePointIndex) {
        self.index = index;
    }
    fn time(&self) -> &I {
        &self.time
    }
    fn original_time(&self) -> &O {
        &self.original_time
    }
    fn values(&self) -> Option<&PlotRowValue> {
        Some(&self.value)
    }
}
