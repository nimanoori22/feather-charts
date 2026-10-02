//! Autoscale values shared by series options and price-scale calculations.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PriceRange {
    pub min_value: f64,
    pub max_value: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoScaleMargins {
    pub below: f64,
    pub above: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AutoscaleInfo {
    pub price_range: Option<PriceRange>,
    pub margins: Option<AutoScaleMargins>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AutoscaleInfoImpl {
    price_range: Option<PriceRange>,
    margins: Option<AutoScaleMargins>,
}

impl AutoscaleInfoImpl {
    pub const fn new(price_range: Option<PriceRange>, margins: Option<AutoScaleMargins>) -> Self {
        Self {
            price_range,
            margins,
        }
    }
    pub const fn price_range(&self) -> Option<PriceRange> {
        self.price_range
    }
    pub const fn margins(&self) -> Option<AutoScaleMargins> {
        self.margins
    }
    pub fn to_raw(&self) -> AutoscaleInfo {
        AutoscaleInfo {
            price_range: self.price_range,
            margins: self.margins,
        }
    }
    pub fn from_raw(raw: Option<AutoscaleInfo>) -> Option<Self> {
        raw.map(|raw| Self::new(raw.price_range, raw.margins))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raw_round_trip_preserves_optional_values() {
        let info = AutoscaleInfoImpl::new(
            Some(PriceRange {
                min_value: 1.0,
                max_value: 2.0,
            }),
            Some(AutoScaleMargins {
                above: 3.0,
                below: 4.0,
            }),
        );
        assert_eq!(AutoscaleInfoImpl::from_raw(Some(info.to_raw())), Some(info));
        assert_eq!(AutoscaleInfoImpl::from_raw(None), None);
    }
}
