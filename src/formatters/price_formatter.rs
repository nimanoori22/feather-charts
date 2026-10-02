//! Decimal price formatting compatible with Lightweight Charts price scales.

use std::fmt;

use crate::formatters::iprice_formatter::PriceValueFormatter;

const DEFAULT_PRICE_SCALE: f64 = 100.0;
const DEFAULT_MIN_MOVE: f64 = 1.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PriceFormatterError {
    InvalidPriceScale,
    InvalidLeadingZeroLength,
}

impl fmt::Display for PriceFormatterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPriceScale => f.write_str("invalid price scale"),
            Self::InvalidLeadingZeroLength => f.write_str("invalid length"),
        }
    }
}

impl std::error::Error for PriceFormatterError {}

/// Pads a finite numeric value with zeroes to the requested width.
pub fn number_to_string_with_leading_zero(
    value: f64,
    length: usize,
) -> Result<String, PriceFormatterError> {
    if length > 16 {
        return Err(PriceFormatterError::InvalidLeadingZeroLength);
    }
    if !value.is_finite() {
        return Ok("n/a".into());
    }

    let value = format_number(value);
    if length == 0 {
        return Ok(value);
    }

    Ok(format!("{value:0>width$}", width = length))
}

/// Formats decimal prices according to a scale and minimum movement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PriceFormatter {
    price_scale: f64,
    min_move: f64,
    fractional_length: usize,
}

impl PriceFormatter {
    /// Creates a formatter. Missing or non-integral scales use the source default of 100;
    /// a missing or zero minimum movement uses 1.
    pub fn new(
        price_scale: Option<f64>,
        min_move: Option<f64>,
    ) -> Result<Self, PriceFormatterError> {
        let min_move = min_move
            .filter(|value| *value != 0.0)
            .unwrap_or(DEFAULT_MIN_MOVE);
        let price_scale = match price_scale {
            Some(value) if value.is_finite() && value.fract() == 0.0 => value,
            _ => DEFAULT_PRICE_SCALE,
        };

        if price_scale < 0.0 {
            return Err(PriceFormatterError::InvalidPriceScale);
        }

        let mut fractional_length = 0;
        if price_scale > 0.0 && min_move > 0.0 {
            let mut scale = price_scale;
            while scale > 1.0 {
                scale /= 10.0;
                fractional_length += 1;
            }
        }

        Ok(Self {
            price_scale,
            min_move,
            fractional_length,
        })
    }

    pub fn price_scale(&self) -> f64 {
        self.price_scale
    }

    pub fn min_move(&self) -> f64 {
        self.min_move
    }

    pub fn format(&self, price: f64) -> String {
        let sign = (price < 0.0).then_some('\u{2212}');
        let value = self.format_decimal(price.abs());
        sign.map_or(value.clone(), |sign| format!("{sign}{value}"))
    }

    pub fn format_tickmarks(&self, prices: &[f64]) -> Vec<String> {
        prices.iter().map(|&price| self.format(price)).collect()
    }

    fn format_decimal(&self, price: f64) -> String {
        let base = self.price_scale / self.min_move;
        let mut integer = price.floor();

        if base > 1.0 {
            let mut fraction = (price * base).round() - integer * base;
            if fraction >= base {
                fraction -= base;
                integer += 1.0;
            }
            let fractional_value = fraction.round() * self.min_move;
            let fraction =
                number_to_string_with_leading_zero(fractional_value, self.fractional_length)
                    .expect("formatter fractional length is bounded by the source contract");
            format!("{}.{fraction}", format_number(integer))
        } else {
            integer = (integer * base).round() / base;
            if self.fractional_length > 0 {
                format!(
                    "{}.{}",
                    format_number(integer),
                    "0".repeat(self.fractional_length)
                )
            } else {
                format_number(integer)
            }
        }
    }
}

impl Default for PriceFormatter {
    fn default() -> Self {
        Self::new(None, None).expect("default formatter configuration is valid")
    }
}

impl PriceValueFormatter for PriceFormatter {
    fn format(&self, price: f64) -> String {
        Self::format(self, price)
    }
}

fn format_number(value: f64) -> String {
    if value.is_nan() {
        return "NaN".into();
    }
    if value == f64::INFINITY {
        return "Infinity".into();
    }
    if value == f64::NEG_INFINITY {
        return "-Infinity".into();
    }
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_decimal_prices_with_source_rounding_and_unicode_minus() {
        let formatter = PriceFormatter::new(Some(100.0), Some(1.0)).unwrap();

        assert_eq!(formatter.format(12.345), "12.35");
        assert_eq!(formatter.format(-12.345), "−12.35");
        assert_eq!(formatter.format(9.999), "10.00");
        assert_eq!(formatter.format_tickmarks(&[1.0, 2.5]), ["1.00", "2.50"]);
    }

    #[test]
    fn validates_and_pads_leading_zero_values() {
        assert_eq!(number_to_string_with_leading_zero(7.0, 3).unwrap(), "007");
        assert_eq!(
            number_to_string_with_leading_zero(f64::NAN, 3).unwrap(),
            "n/a"
        );
        assert_eq!(
            number_to_string_with_leading_zero(1.0, 17),
            Err(PriceFormatterError::InvalidLeadingZeroLength)
        );
        assert_eq!(
            PriceFormatter::new(Some(-100.0), None),
            Err(PriceFormatterError::InvalidPriceScale)
        );
    }
}
