//! UTC calendar values and Lightweight Charts date-format tokens.
use chrono::{DateTime, Datelike, Locale, Utc};

use super::time_locale::resolve;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UtcDateTime(DateTime<Utc>);
impl UtcDateTime {
    pub fn from_unix_timestamp(seconds: f64) -> Option<Self> {
        if !seconds.is_finite() {
            return None;
        }
        // JS Date truncates fractional milliseconds before extracting UTC fields.
        let millis = (seconds * 1000.).trunc();
        if millis.abs() > 8_640_000_000_000_000. {
            return None;
        }
        DateTime::from_timestamp_millis(millis as i64).map(Self)
    }
    pub const fn inner(&self) -> &DateTime<Utc> {
        &self.0
    }
}

pub fn format_date(date: &UtcDateTime, format: &str, requested_locale: &str) -> String {
    let value = date.inner();
    let month_locale = resolve(requested_locale).month;
    let long_month = value.format_localized("%B", month_locale).to_string();
    let short_month = match month_locale {
        Locale::ar_EG => long_month.clone(),
        Locale::fr_FR if value.month() == 4 => "avr.".into(),
        _ => value.format_localized("%b", month_locale).to_string(),
    };
    format
        .replace("yyyy", &value.format("%Y").to_string())
        .replace("yy", &value.format("%y").to_string())
        .replace("MMMM", &long_month)
        .replace("MMM", &short_month)
        .replace("MM", &value.format("%m").to_string())
        .replace("dd", &value.format("%d").to_string())
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
