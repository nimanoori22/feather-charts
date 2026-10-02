//! Logical chart coordinates.

/// A position in chart-coordinate space.
///
/// Coordinates remain `f64` throughout the chart model, matching JavaScript's
/// number precision. A rendering adapter converts them to backend pixel types
/// only at the drawing boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Coordinate(f64);

impl Coordinate {
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> f64 {
        self.0
    }
}

impl From<f64> for Coordinate {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<Coordinate> for f64 {
    fn from(value: Coordinate) -> Self {
        value.value()
    }
}

#[cfg(test)]
mod tests {
    use super::Coordinate;

    #[test]
    fn preserves_fractional_chart_coordinates() {
        let coordinate = Coordinate::new(42.25);
        assert_eq!(coordinate.value(), 42.25);
        assert_eq!(f64::from(coordinate), 42.25);
    }
}
