//! Time-scale indexes, points, and visible-range calculations.

use crate::model::{
    coordinate::Coordinate, horz_scale_behavior_time::types::TickMarkWeight, range_impl::RangeImpl,
};

/// Numeric importance of a time-scale tick. Custom horizontal behaviors may
/// use values beyond the built-in time weights.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct TickMarkWeightValue(i32);

impl TickMarkWeightValue {
    pub const fn new(value: i32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> i32 {
        self.0
    }
}

impl From<TickMarkWeight> for TickMarkWeightValue {
    fn from(weight: TickMarkWeight) -> Self {
        Self::new(weight as i32)
    }
}

/// Index of a point on the horizontal scale.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct TimePointIndex(f64);

impl TimePointIndex {
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> f64 {
        self.0
    }

    pub fn is_integer(self) -> bool {
        self.0.is_finite() && self.0.fract() == 0.0
    }

    pub fn as_usize(self) -> Option<usize> {
        if self.is_integer() && self.0 >= 0.0 && self.0 <= usize::MAX as f64 {
            Some(self.0 as usize)
        } else {
            None
        }
    }
}

impl From<f64> for TimePointIndex {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<TimePointIndex> for f64 {
    fn from(index: TimePointIndex) -> Self {
        index.value()
    }
}

/// A possibly fractional index on the continuous logical scale.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Logical(f64);

impl Logical {
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> f64 {
        self.0
    }
}

impl From<f64> for Logical {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<Logical> for f64 {
    fn from(index: Logical) -> Self {
        index.value()
    }
}

/// A generic closed-open range expressed as public API `from`/`to` values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValueRange<T> {
    pub from: T,
    pub to: T,
}

pub type LogicalRange = ValueRange<Logical>;

/// A normalized time-scale point and the original user item it represents.
#[derive(Clone, Debug, PartialEq)]
pub struct TimeScalePoint<InternalItem, OriginalItem> {
    pub time_weight: TickMarkWeightValue,
    pub time: InternalItem,
    pub original_time: OriginalItem,
}

/// A normalized time point without its display-weight metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct TimePointValue<InternalItem, OriginalItem> {
    pub time: InternalItem,
    pub original_time: OriginalItem,
}

pub type TimePointsRange<InternalItem, OriginalItem> =
    ValueRange<TimePointValue<InternalItem, OriginalItem>>;

/// An item whose horizontal point has already been converted to an index.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimedValue {
    pub time: TimePointIndex,
    pub x: Coordinate,
}

pub type SeriesItemsIndexesRange = ValueRange<usize>;

/// Finds the index interval of sorted timed values intersecting `range`.
///
/// When `extended_range` is set, one neighbor on each available side is
/// included so renderers can join lines at the viewport edge.
pub fn visible_timed_values(
    items: &[TimedValue],
    range: &RangeImpl<TimePointIndex>,
    extended_range: bool,
) -> SeriesItemsIndexesRange {
    let first_bar = range.left();
    let last_bar = range.right();
    let from = items.partition_point(|item| item.time < first_bar);
    let to = items.partition_point(|item| item.time <= last_bar);

    if !extended_range {
        return ValueRange { from, to };
    }

    ValueRange {
        from: if from > 0 && from < items.len() && items[from].time >= first_bar {
            from - 1
        } else {
            from
        },
        to: if to > 0 && to < items.len() && items[to - 1].time <= last_bar {
            to + 1
        } else {
            to
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::{model::coordinate::Coordinate, model::range_impl::RangeImpl};

    use super::{
        TickMarkWeightValue, TimePointIndex, TimedValue, ValueRange, visible_timed_values,
    };

    #[test]
    fn finds_visible_values_and_optional_edge_neighbors() {
        let items = [0.0, 2.0, 4.0, 6.0].map(|time| TimedValue {
            time: TimePointIndex::new(time),
            x: Coordinate::new(time),
        });
        let range = RangeImpl::new(TimePointIndex::new(2.0), TimePointIndex::new(4.0));

        assert_eq!(
            visible_timed_values(&items, &range, false),
            ValueRange { from: 1, to: 3 }
        );
        assert_eq!(
            visible_timed_values(&items, &range, true),
            ValueRange { from: 0, to: 4 }
        );
    }

    #[test]
    fn keeps_custom_tick_weights_typed() {
        assert_eq!(TickMarkWeightValue::new(8).value(), 8);
    }
}
