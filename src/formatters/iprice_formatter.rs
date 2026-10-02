//! Common contract for price and percentage label formatters.

/// Formats price-scale values, including a consistent batch operation for tick marks.
pub trait PriceValueFormatter {
    fn format(&self, price: f64) -> String;

    fn format_tickmarks(&self, prices: &[f64]) -> Vec<String> {
        prices.iter().map(|&price| self.format(price)).collect()
    }
}
