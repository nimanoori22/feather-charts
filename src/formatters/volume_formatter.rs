//! Compact K/M/B formatting for volume series.
use crate::formatters::iprice_formatter::PriceValueFormatter;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VolumeFormatter {
    precision: u32,
}
impl VolumeFormatter {
    pub const fn new(precision: u32) -> Self {
        Self { precision }
    }
    pub fn format(&self, volume: f64) -> String {
        let (sign, value) = if volume < 0.0 {
            ("-", -volume)
        } else {
            ("", volume)
        };
        let (value, suffix) = if value < 995.0 {
            (value, "")
        } else if value < 999_995.0 {
            (value / 1_000.0, "K")
        } else if value < 999_999_995.0 {
            ((value / 1_000.0).round() / 1_000.0, "M")
        } else {
            ((value / 1_000_000.0).round() / 1_000.0, "B")
        };
        format!("{sign}{}{suffix}", self.format_number(value))
    }
    fn format_number(&self, value: f64) -> String {
        let scale = 10_f64.powi(self.precision as i32);
        let value = (value * scale).round() / scale;
        let raw = if (1e-15..1.0).contains(&value) {
            format!("{value:.precision$}", precision = self.precision as usize)
        } else {
            value.to_string()
        };
        if raw.contains('.') {
            raw.trim_end_matches('0').trim_end_matches('.').to_owned()
        } else {
            raw
        }
    }
}
impl PriceValueFormatter for VolumeFormatter {
    fn format(&self, price: f64) -> String {
        Self::format(self, price)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compacts_volume_boundaries() {
        let f = VolumeFormatter::new(2);
        assert_eq!(f.format(994.4), "994.4");
        assert_eq!(f.format(995.), "1K");
        assert_eq!(f.format(1_000_000.), "1M");
        assert_eq!(f.format(-2_500.), "-2.5K");
    }
    #[test]
    fn preserves_integer_zeroes_and_zero_in_axis_labels() {
        for precision in [0, 2] {
            let formatter = VolumeFormatter::new(precision);
            assert_eq!(formatter.format(0.), "0");
            assert_eq!(formatter.format(100.), "100");
            assert_eq!(formatter.format(40_000.), "40K");
            assert_eq!(formatter.format(-30_000_000.), "-30M");
        }
    }
}
