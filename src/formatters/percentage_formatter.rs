//! Percentage labels built on the decimal price formatter.

use crate::formatters::{
    iprice_formatter::PriceValueFormatter,
    price_formatter::{PriceFormatter, PriceFormatterError},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PercentageFormatter {
    decimal: PriceFormatter,
}

impl PercentageFormatter {
    pub fn new(price_scale: Option<f64>) -> Result<Self, PriceFormatterError> {
        Ok(Self {
            decimal: PriceFormatter::new(price_scale.or(Some(100.0)), None)?,
        })
    }

    pub fn format(&self, price: f64) -> String {
        format!("{}%", self.decimal.format(price))
    }

    pub fn format_tickmarks(&self, prices: &[f64]) -> Vec<String> {
        prices.iter().map(|&price| self.format(price)).collect()
    }
}

impl Default for PercentageFormatter {
    fn default() -> Self {
        Self::new(None).expect("default percentage formatter configuration is valid")
    }
}

impl PriceValueFormatter for PercentageFormatter {
    fn format(&self, price: f64) -> String {
        Self::format(self, price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_a_percent_sign_after_decimal_formatting() {
        assert_eq!(PercentageFormatter::default().format(-1.25), "−1.25%");
    }
}
