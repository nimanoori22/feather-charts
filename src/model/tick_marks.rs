//! Tick-mark data shared by horizontal-scale behavior and the later cache.

use crate::model::time_data::{TickMarkWeightValue, TimePointIndex};

/// A selectable horizontal-axis tick mark.
#[derive(Clone, Debug, PartialEq)]
pub struct TickMark<InternalItem, OriginalItem> {
    pub index: TimePointIndex,
    pub time: InternalItem,
    pub weight: TickMarkWeightValue,
    pub original_time: OriginalItem,
}
