//! Mutable price ranges used by autoscaling and price-scale conversion.

/// Serializable public representation of a price range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PriceRange {
    pub min_value: f64,
    pub max_value: f64,
}

/// Internal range with the scale and merge operations required by the model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PriceRangeImpl {
    min_value: f64,
    max_value: f64,
}

impl PriceRangeImpl {
    pub const fn new(min_value: f64, max_value: f64) -> Self {
        Self {
            min_value,
            max_value,
        }
    }

    pub const fn min_value(&self) -> f64 {
        self.min_value
    }

    pub const fn max_value(&self) -> f64 {
        self.max_value
    }

    pub fn length(&self) -> f64 {
        self.max_value - self.min_value
    }

    pub fn is_empty(&self) -> bool {
        self.min_value == self.max_value || self.min_value.is_nan() || self.max_value.is_nan()
    }

    pub fn merge(&self, other: Option<&Self>) -> Self {
        let Some(other) = other else { return *self };
        Self::new(
            finite_result(f64::min, self.min_value, other.min_value, f64::NEG_INFINITY),
            finite_result(f64::max, self.max_value, other.max_value, f64::INFINITY),
        )
    }

    pub fn scale_around_center(&mut self, coefficient: f64) {
        if !coefficient.is_finite() {
            return;
        }

        let delta = self.max_value - self.min_value;
        if delta == 0.0 {
            return;
        }

        let center = (self.max_value + self.min_value) * 0.5;
        self.max_value = center + (self.max_value - center) * coefficient;
        self.min_value = center + (self.min_value - center) * coefficient;
    }

    pub fn shift(&mut self, delta: f64) {
        if !delta.is_finite() {
            return;
        }
        self.max_value += delta;
        self.min_value += delta;
    }

    pub const fn to_raw(&self) -> PriceRange {
        PriceRange {
            min_value: self.min_value,
            max_value: self.max_value,
        }
    }

    pub fn from_raw(raw: Option<PriceRange>) -> Option<Self> {
        raw.map(|raw| Self::new(raw.min_value, raw.max_value))
    }
}

fn finite_result(
    operation: impl FnOnce(f64, f64) -> f64,
    first: f64,
    second: f64,
    fallback: f64,
) -> f64 {
    match (first.is_finite(), second.is_finite()) {
        (true, true) => operation(first, second),
        (true, false) => first,
        (false, true) => second,
        (false, false) => fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_finite_values_and_handles_non_finite_bounds() {
        let range = PriceRangeImpl::new(3.0, 8.0);
        assert_eq!(
            range.merge(Some(&PriceRangeImpl::new(1.0, 6.0))),
            PriceRangeImpl::new(1.0, 8.0)
        );
        assert_eq!(
            range.merge(Some(&PriceRangeImpl::new(f64::NAN, f64::NAN))),
            range
        );
        let non_finite = PriceRangeImpl::new(f64::NAN, f64::NAN).merge(None);
        assert!(non_finite.min_value().is_nan());
        assert!(non_finite.max_value().is_nan());
    }

    #[test]
    fn scales_shifts_and_round_trips_raw_ranges() {
        let mut range = PriceRangeImpl::new(2.0, 6.0);
        range.scale_around_center(2.0);
        assert_eq!(range, PriceRangeImpl::new(0.0, 8.0));
        range.shift(-1.0);
        assert_eq!(range, PriceRangeImpl::new(-1.0, 7.0));
        assert_eq!(PriceRangeImpl::from_raw(Some(range.to_raw())), Some(range));
    }
}
