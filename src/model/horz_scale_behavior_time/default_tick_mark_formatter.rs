//! Default localized labels for time-axis tick marks.

use chrono::{Datelike, Timelike};

use crate::{
    formatters::format_date::format_date,
    model::horz_scale_behavior_time::{
        time_utils::{TimeConversionError, business_day_to_time_point},
        types::{TickMarkType, TimePoint},
    },
};

fn date_time(
    point: &TimePoint,
) -> Result<crate::formatters::format_date::UtcDateTime, TimeConversionError> {
    if let Some(day) = point.business_day {
        let normalized = business_day_to_time_point(day)?;
        return normalized
            .timestamp
            .to_date_time()
            .ok_or(TimeConversionError::InvalidTimestamp);
    }
    point
        .timestamp
        .to_date_time()
        .ok_or(TimeConversionError::InvalidTimestamp)
}

/// Formats the built-in tick label for a normalized time point.
pub fn default_tick_mark_formatter(
    point: &TimePoint,
    tick_mark_type: TickMarkType,
    locale: &str,
) -> Result<String, TimeConversionError> {
    let date = date_time(point)?;
    let value = date.inner();
    Ok(match tick_mark_type {
        TickMarkType::Year => value.year().to_string(),
        TickMarkType::Month => format_date(&date, "MMM", locale),
        TickMarkType::DayOfMonth => value.day().to_string(),
        TickMarkType::Time => format!("{:02}:{:02}", value.hour(), value.minute()),
        TickMarkType::TimeWithSeconds => {
            format!(
                "{:02}:{:02}:{:02}",
                value.hour(),
                value.minute(),
                value.second()
            )
        }
    })
}

#[cfg(test)]
mod tests {
    use super::default_tick_mark_formatter;
    use crate::model::horz_scale_behavior_time::types::{TickMarkType, TimePoint, UtcTimestamp};

    #[test]
    fn formats_each_default_tick_mark_variant_in_utc() {
        let point = TimePoint {
            timestamp: UtcTimestamp::new(1_704_198_645.0),
            business_day: None,
        };

        assert_eq!(
            default_tick_mark_formatter(&point, TickMarkType::Year, "en-US").unwrap(),
            "2024"
        );
        assert_eq!(
            default_tick_mark_formatter(&point, TickMarkType::Month, "en-US").unwrap(),
            "Jan"
        );
        assert_eq!(
            default_tick_mark_formatter(&point, TickMarkType::DayOfMonth, "en-US").unwrap(),
            "2"
        );
        assert_eq!(
            default_tick_mark_formatter(&point, TickMarkType::Time, "en-US").unwrap(),
            "12:30"
        );
        assert_eq!(
            default_tick_mark_formatter(&point, TickMarkType::TimeWithSeconds, "en-US").unwrap(),
            "12:30:45"
        );
    }
}
