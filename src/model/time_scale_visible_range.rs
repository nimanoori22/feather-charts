//! Logical and strict visible ranges for the horizontal scale.

use crate::model::{
    range_impl::RangeImpl,
    time_data::{Logical, TimePointIndex},
};

/// The currently visible logical range, if the horizontal scale has enough
/// state to calculate one.
///
/// A strict range deliberately expands fractional logical endpoints outward so
/// callers can safely include every bar that is even partially visible.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimeScaleVisibleRange {
    logical_range: Option<RangeImpl<Logical>>,
}

impl TimeScaleVisibleRange {
    /// Creates a visible-range value from an optional logical range.
    pub const fn new(logical_range: Option<RangeImpl<Logical>>) -> Self {
        Self { logical_range }
    }

    /// Returns an invalid range, used before the time scale has calculated its
    /// visible logical interval.
    pub const fn invalid() -> Self {
        Self::new(None)
    }

    /// Returns the inclusive integer bar range covering the full logical
    /// range. Fractional endpoints are expanded outwards.
    pub fn strict_range(&self) -> Option<RangeImpl<TimePointIndex>> {
        self.logical_range.map(|range| {
            RangeImpl::new(
                TimePointIndex::new(range.left().value().floor()),
                TimePointIndex::new(range.right().value().ceil()),
            )
        })
    }

    /// Returns the original logical range without rounding it.
    pub const fn logical_range(&self) -> Option<RangeImpl<Logical>> {
        self.logical_range
    }
}

#[cfg(test)]
mod tests {
    use super::TimeScaleVisibleRange;
    use crate::model::{range_impl::RangeImpl, time_data::Logical};

    #[test]
    fn expands_fractional_logical_boundaries_to_cover_visible_bars() {
        let visible =
            TimeScaleVisibleRange::new(Some(RangeImpl::new(Logical::new(2.25), Logical::new(7.1))));

        let strict = visible.strict_range().unwrap();
        assert_eq!(strict.left().value(), 2.0);
        assert_eq!(strict.right().value(), 8.0);
        assert_eq!(visible.logical_range().unwrap().left().value(), 2.25);
        assert_eq!(visible.logical_range().unwrap().right().value(), 7.1);
    }

    #[test]
    fn invalid_range_has_no_logical_or_strict_range() {
        let visible = TimeScaleVisibleRange::invalid();

        assert_eq!(visible.logical_range(), None);
        assert_eq!(visible.strict_range(), None);
    }
}
