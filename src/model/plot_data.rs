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
impl<I, O, C> PlotRowLike for PlotRow<I, O, C> {
    fn index(&self) -> TimePointIndex {
        self.index
    }
    fn values(&self) -> &PlotRowValue {
        &self.value
    }
}
