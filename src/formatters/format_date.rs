//! UTC calendar values and Lightweight Charts date-format tokens.
use std::str::FromStr;

use chrono::{DateTime, Locale, TimeZone, Utc};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UtcDateTime(DateTime<Utc>);
impl UtcDateTime {
    pub fn from_unix_timestamp(seconds: f64) -> Option<Self> {
        if !seconds.is_finite() {
            return None;
        }
        let whole = seconds.floor() as i64;
        let nanos = ((seconds - whole as f64) * 1_000_000_000.0).round() as u32;
        Utc.timestamp_opt(whole, nanos).single().map(Self)
    }
    pub const fn inner(&self) -> &DateTime<Utc> {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formats_leap_day_in_english_and_french() {
        let value = UtcDateTime::from_unix_timestamp(1_582_934_400.0).unwrap();
        assert_eq!(
            format_date(&value, "yyyy-MM-dd MMMM", "en-US"),
            "2020-02-29 February"
        );
        assert_eq!(format_date(&value, "dd MMM yyyy", "fr-FR"), "29 févr. 2020");
    }
    #[test]
    fn preserves_negative_fractional_timestamps() {
        assert_eq!(
            UtcDateTime::from_unix_timestamp(-0.5)
                .unwrap()
                .inner()
                .timestamp(),
            -1
        );
    }
}
fn locale(value: &str) -> Locale {
    let normalized = match value {
        "default" | "en" => "en_US".into(),
        value if !value.contains('_') && value.len() == 2 => {
            format!("{}_{}", value, value.to_ascii_uppercase())
        }
        value => value.replace('-', "_"),
    };
    Locale::from_str(&normalized).unwrap_or(Locale::en_US)
}
pub fn format_date(date: &UtcDateTime, format: &str, requested_locale: &str) -> String {
    let value = date.inner();
    let month_locale = locale(requested_locale);
    format
        .replace("yyyy", &value.format("%Y").to_string())
        .replace("yy", &value.format("%y").to_string())
        .replace(
            "MMMM",
            &value.format_localized("%B", month_locale).to_string(),
        )
        .replace(
            "MMM",
            &value.format_localized("%b", month_locale).to_string(),
        )
        .replace("MM", &value.format("%m").to_string())
        .replace("dd", &value.format("%d").to_string())
}
