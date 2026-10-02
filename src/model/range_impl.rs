//! Inclusive numeric ranges used by the horizontal time scale.

/// An immutable inclusive interval over a chart-model numeric domain.
///
/// Unlike Rust's standard ranges, both endpoints are included. Construction
/// rejects reversed (and unordered, such as `NaN`) endpoints, matching the
/// invariant enforced by Lightweight Charts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RangeImpl<T> {
    left: T,
    right: T,
}

impl<T> RangeImpl<T>
where
    T: Copy + PartialOrd,
{
    /// Creates an inclusive range whose left endpoint is not after its right
    /// endpoint.
    pub fn new(left: T, right: T) -> Self {
        assert!(left <= right, "right should be >= left");

        Self { left, right }
    }

    /// Returns the inclusive left endpoint.
    pub const fn left(&self) -> T {
        self.left
    }

    /// Returns the inclusive right endpoint.
    pub const fn right(&self) -> T {
        self.right
    }

    /// Returns whether `value` lies within the inclusive endpoints.
    pub fn contains(&self, value: T) -> bool {
        self.left <= value && value <= self.right
    }
}

impl<T> RangeImpl<T>
where
    T: Copy + Into<f64>,
{
    /// Returns the inclusive numeric length of this range.
    ///
    /// A logical range may have fractional endpoints, so this is deliberately
    /// not a collection length and is represented as `f64`.
    pub fn count(&self) -> f64 {
        self.right.into() - self.left.into() + 1.0
    }
}

impl<T> RangeImpl<T>
where
    T: Copy + PartialEq,
{
    /// Returns whether both inclusive endpoints are equal.
    ///
    /// Rust callers can normally use `==`; this method maps the source
    /// component's explicit comparison operation for future ports.
    pub fn equals(&self, other: &Self) -> bool {
        self == other
    }
}

/// Compares two optional ranges, treating two absent ranges as equal.
pub fn are_ranges_equal<T>(first: Option<RangeImpl<T>>, second: Option<RangeImpl<T>>) -> bool
where
    T: Copy + PartialEq,
{
    first == second
}

#[cfg(test)]
mod tests {
    use super::{RangeImpl, are_ranges_equal};

    #[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
    struct TimePointIndex(f64);

    impl From<TimePointIndex> for f64 {
        fn from(value: TimePointIndex) -> Self {
            value.0
        }
    }

    #[test]
    fn retains_typed_inclusive_endpoints() {
        let range = RangeImpl::new(TimePointIndex(2.0), TimePointIndex(5.0));

        assert_eq!(range.left(), TimePointIndex(2.0));
        assert_eq!(range.right(), TimePointIndex(5.0));
        assert!(range.contains(TimePointIndex(2.0)));
        assert!(range.contains(TimePointIndex(5.0)));
        assert!(!range.contains(TimePointIndex(5.5)));
        assert_eq!(range.count(), 4.0);
    }

    #[test]
    fn count_preserves_fractional_logical_range_semantics() {
        let range = RangeImpl::new(3.25_f64, 8.0_f64);

        assert_eq!(range.count(), 5.75);
    }

    #[test]
    fn range_equality_includes_absence_semantics() {
        let first = RangeImpl::new(1.0_f64, 3.0_f64);
        let same = RangeImpl::new(1.0_f64, 3.0_f64);
        let different = RangeImpl::new(1.0_f64, 4.0_f64);

        assert!(first.equals(&same));
        assert!(!first.equals(&different));
        assert!(are_ranges_equal(Some(first), Some(same)));
        assert!(!are_ranges_equal(Some(first), Some(different)));
        assert!(are_ranges_equal::<f64>(None, None));
        assert!(!are_ranges_equal(Some(first), None));
    }

    #[test]
    #[should_panic(expected = "right should be >= left")]
    fn rejects_reversed_endpoints() {
        let _ = RangeImpl::new(4.0_f64, 3.0_f64);
    }

    #[test]
    #[should_panic(expected = "right should be >= left")]
    fn rejects_unordered_endpoints() {
        let _ = RangeImpl::new(f64::NAN, 3.0_f64);
    }
}
