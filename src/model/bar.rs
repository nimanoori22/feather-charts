//! OHLC bar values and their chart-coordinate representation.

use crate::model::coordinate::Coordinate;

/// A price from a series bar.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct BarPrice(f64);

impl BarPrice {
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> f64 {
        self.0
    }
}

impl From<f64> for BarPrice {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<BarPrice> for f64 {
    fn from(value: BarPrice) -> Self {
        value.value()
    }
}

/// The open, high, low, and close prices of one bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarPrices {
    pub open: BarPrice,
    pub high: BarPrice,
    pub low: BarPrice,
    pub close: BarPrice,
}

/// The y-axis positions of a bar's open, high, low, and close prices.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarCoordinates {
    pub open_y: Coordinate,
    pub high_y: Coordinate,
    pub low_y: Coordinate,
    pub close_y: Coordinate,
}

#[cfg(test)]
mod tests {
    use crate::model::coordinate::Coordinate;

    use super::{BarCoordinates, BarPrice, BarPrices};

    #[test]
    fn keeps_prices_and_coordinates_as_distinct_typed_data() {
        let prices = BarPrices {
            open: BarPrice::new(100.25),
            high: BarPrice::new(110.0),
            low: BarPrice::new(98.5),
            close: BarPrice::new(107.75),
        };
        let coordinates = BarCoordinates {
            open_y: Coordinate::new(75.0),
            high_y: Coordinate::new(20.5),
            low_y: Coordinate::new(105.25),
            close_y: Coordinate::new(40.0),
        };

        assert_eq!(prices.close.value(), 107.75);
        assert_eq!(coordinates.low_y.value(), 105.25);
    }
}
