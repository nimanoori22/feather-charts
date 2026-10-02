//! Public time-domain types for the default horizontal scale behavior.

/// A Unix timestamp expressed in seconds rather than milliseconds.
///
/// This remains an `f64` newtype because Lightweight Charts accepts the full
/// JavaScript `number` domain, including sub-second timestamps.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct UtcTimestamp(f64);

impl UtcTimestamp {
    pub const fn new(seconds: f64) -> Self {
        Self(seconds)
    }

    pub const fn seconds(self) -> f64 {
        self.0
    }
}

impl From<f64> for UtcTimestamp {
    fn from(seconds: f64) -> Self {
        Self::new(seconds)
    }
}

impl From<UtcTimestamp> for f64 {
    fn from(timestamp: UtcTimestamp) -> Self {
        timestamp.seconds()
    }
}

/// A calendar day used for business-day chart data.
///
/// Calendar validation and conversion to a timestamp belong to the time
/// conversion component, mirroring the source architecture.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BusinessDay {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

/// A user-provided horizontal time value.
///
/// `BusinessDayString` is intentionally retained until preprocessing parses
/// it into a [`BusinessDay`].
#[derive(Clone, Debug, PartialEq)]
pub enum Time {
    UtcTimestamp(UtcTimestamp),
    BusinessDay(BusinessDay),
    BusinessDayString(String),
}

impl From<UtcTimestamp> for Time {
    fn from(timestamp: UtcTimestamp) -> Self {
        Self::UtcTimestamp(timestamp)
    }
}

impl From<BusinessDay> for Time {
    fn from(day: BusinessDay) -> Self {
        Self::BusinessDay(day)
    }
}

impl From<String> for Time {
    fn from(day: String) -> Self {
        Self::BusinessDayString(day)
    }
}

impl From<&str> for Time {
    fn from(day: &str) -> Self {
        Self::BusinessDayString(day.to_owned())
    }
}

/// A normalized internal time point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimePoint {
    pub timestamp: UtcTimestamp,
    pub business_day: Option<BusinessDay>,
}

/// Returns whether a user time value is a calendar business day object.
pub const fn is_business_day(time: &Time) -> bool {
    matches!(time, Time::BusinessDay(_))
}

/// Returns whether a user time value is a Unix timestamp.
pub const fn is_utc_timestamp(time: &Time) -> bool {
    matches!(time, Time::UtcTimestamp(_))
}

/// The semantic category of a horizontal-axis tick label.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum TickMarkType {
    #[default]
    Year = 0,
    Month = 1,
    DayOfMonth = 2,
    Time = 3,
    TimeWithSeconds = 4,
}

/// The significance of the time transition represented by a tick mark.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(u8)]
pub enum TickMarkWeight {
    #[default]
    LessThanSecond = 0,
    Second = 10,
    Minute1 = 20,
    Minute5 = 21,
    Minute30 = 22,
    Hour1 = 30,
    Hour3 = 31,
    Hour6 = 32,
    Hour12 = 33,
    Day = 50,
    Month = 60,
    Year = 70,
}

#[cfg(test)]
mod tests {
    use super::{
        BusinessDay, TickMarkType, TickMarkWeight, Time, TimePoint, UtcTimestamp, is_business_day,
        is_utc_timestamp,
    };

    #[test]
    fn distinguishes_all_supported_user_time_forms() {
        let timestamp = Time::from(UtcTimestamp::new(1_529_899_200.25));
        let business_day = Time::from(BusinessDay {
            year: 2019,
            month: 6,
            day: 1,
        });
        let text_day = Time::from("2021-02-03");

        assert!(is_utc_timestamp(&timestamp));
        assert!(!is_business_day(&timestamp));
        assert!(is_business_day(&business_day));
        assert!(!is_utc_timestamp(&business_day));
        assert!(!is_business_day(&text_day));
        assert!(!is_utc_timestamp(&text_day));
    }

    #[test]
    fn preserves_timestamp_seconds_and_normalized_business_day() {
        let timestamp = UtcTimestamp::new(1_529_899_200.5);
        let point = TimePoint {
            timestamp,
            business_day: Some(BusinessDay {
                year: 2019,
                month: 6,
                day: 25,
            }),
        };

        assert_eq!(f64::from(timestamp), 1_529_899_200.5);
        assert_eq!(point.business_day.unwrap().day, 25);
    }

    #[test]
    fn tick_values_match_time_scale_weighting_contract() {
        assert_eq!(TickMarkType::Year as u8, 0);
        assert_eq!(TickMarkType::TimeWithSeconds as u8, 4);
        assert_eq!(TickMarkWeight::LessThanSecond as u8, 0);
        assert_eq!(TickMarkWeight::Minute30 as u8, 22);
        assert_eq!(TickMarkWeight::Hour12 as u8, 33);
        assert_eq!(TickMarkWeight::Year as u8, 70);
    }
}
