//! Conversion and validation for default timestamp and business-day inputs.

use chrono::{NaiveDate, TimeZone, Utc};
use std::{error::Error, fmt};

use crate::{
    formatters::format_date::UtcDateTime,
    model::{
        data_consumer::TimedData,
        horz_scale_behavior_time::types::{BusinessDay, Time, TimePoint, UtcTimestamp},
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimeConversionError {
    InvalidBusinessDayFormat(String),
    InvalidBusinessDay { year: i32, month: u8, day: u8 },
    InvalidTimestamp,
    MixedTimeKinds,
    EmptyData,
}

impl fmt::Display for TimeConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBusinessDayFormat(value) => {
                write!(
                    formatter,
                    "invalid business-day string {value:?}, expected yyyy-mm-dd"
                )
            }
            Self::InvalidBusinessDay { year, month, day } => {
                write!(
                    formatter,
                    "invalid business day {year:04}-{month:02}-{day:02}"
                )
            }
            Self::InvalidTimestamp => {
                write!(formatter, "timestamp is outside the supported UTC range")
            }
            Self::MixedTimeKinds => write!(
                formatter,
                "a time scale data set must not mix timestamps and business days"
            ),
            Self::EmptyData => write!(
                formatter,
                "a time converter requires at least one data item"
            ),
        }
    }
}

impl Error for TimeConversionError {}

/// The normalized input family selected for one series data set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeConverter {
    Timestamp,
    BusinessDay,
}

impl TimeConverter {
    pub fn convert(self, time: &Time) -> Result<TimePoint, TimeConversionError> {
        match self {
            Self::Timestamp => timestamp_converter(time),
            Self::BusinessDay => business_day_converter(time),
        }
    }
}

pub fn string_to_business_day(value: &str) -> Result<BusinessDay, TimeConversionError> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
    {
        return Err(TimeConversionError::InvalidBusinessDayFormat(
            value.to_owned(),
        ));
    }

    let year = value[0..4]
        .parse::<i32>()
        .map_err(|_| TimeConversionError::InvalidBusinessDayFormat(value.to_owned()))?;
    let month = value[5..7]
        .parse::<u8>()
        .map_err(|_| TimeConversionError::InvalidBusinessDayFormat(value.to_owned()))?;
    let day = value[8..10]
        .parse::<u8>()
        .map_err(|_| TimeConversionError::InvalidBusinessDayFormat(value.to_owned()))?;
    let business_day = BusinessDay { year, month, day };
    validate_business_day(business_day)?;
    Ok(business_day)
}

pub fn validate_business_day(day: BusinessDay) -> Result<(), TimeConversionError> {
    NaiveDate::from_ymd_opt(day.year, day.month.into(), day.day.into())
        .map(|_| ())
        .ok_or(TimeConversionError::InvalidBusinessDay {
            year: day.year,
            month: day.month,
            day: day.day,
        })
}

pub fn business_day_to_time_point(day: BusinessDay) -> Result<TimePoint, TimeConversionError> {
    validate_business_day(day)?;
    let date = NaiveDate::from_ymd_opt(day.year, day.month.into(), day.day.into())
        .expect("validated business day must be constructible");
    let midnight = date
        .and_hms_opt(0, 0, 0)
        .expect("midnight is always valid for a valid date");
    let timestamp = Utc.from_utc_datetime(&midnight).timestamp() as f64;
    Ok(TimePoint {
        timestamp: UtcTimestamp::new(timestamp),
        business_day: Some(day),
    })
}

pub fn timestamp_to_time_point(timestamp: UtcTimestamp) -> Result<TimePoint, TimeConversionError> {
    if UtcDateTime::from_unix_timestamp(timestamp.seconds()).is_none() {
        return Err(TimeConversionError::InvalidTimestamp);
    }
    Ok(TimePoint {
        timestamp,
        business_day: None,
    })
}

pub fn business_day_converter(time: &Time) -> Result<TimePoint, TimeConversionError> {
    match time {
        Time::BusinessDay(day) => business_day_to_time_point(*day),
        Time::BusinessDayString(value) => {
            string_to_business_day(value).and_then(business_day_to_time_point)
        }
        Time::UtcTimestamp(_) => Err(TimeConversionError::MixedTimeKinds),
    }
}

pub fn timestamp_converter(time: &Time) -> Result<TimePoint, TimeConversionError> {
    match time {
        Time::UtcTimestamp(timestamp) => timestamp_to_time_point(*timestamp),
        Time::BusinessDay(_) | Time::BusinessDayString(_) => {
            Err(TimeConversionError::MixedTimeKinds)
        }
    }
}

pub fn convert_time(time: &Time) -> Result<TimePoint, TimeConversionError> {
    match time {
        Time::UtcTimestamp(_) => timestamp_converter(time),
        Time::BusinessDay(_) | Time::BusinessDayString(_) => business_day_converter(time),
    }
}

pub fn convert_string_to_business_day<D>(value: &mut D) -> Result<(), TimeConversionError>
where
    D: TimedData<Item = Time>,
{
    if let Time::BusinessDayString(text) = value.time() {
        *value.time_mut() = Time::BusinessDay(string_to_business_day(text)?);
    }
    Ok(())
}

pub fn convert_strings_to_business_days<D>(data: &mut [D]) -> Result<(), TimeConversionError>
where
    D: TimedData<Item = Time>,
{
    for value in data {
        convert_string_to_business_day(value)?;
    }
    Ok(())
}

pub fn select_time_converter<D>(data: &[D]) -> Result<TimeConverter, TimeConversionError>
where
    D: TimedData<Item = Time>,
{
    let Some(first) = data.first() else {
        return Err(TimeConversionError::EmptyData);
    };
    let converter = match first.time() {
        Time::UtcTimestamp(_) => TimeConverter::Timestamp,
        Time::BusinessDay(_) | Time::BusinessDayString(_) => TimeConverter::BusinessDay,
    };
    for item in data {
        converter.convert(item.time())?;
    }
    Ok(converter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::data_consumer::WhitespaceData;

    #[test]
    fn validates_and_converts_business_days_at_utc_midnight() {
        let day = string_to_business_day("2024-02-29").unwrap();
        let point = business_day_to_time_point(day).unwrap();

        assert_eq!(point.business_day, Some(day));
        assert_eq!(
            point.timestamp.to_date_time().unwrap().inner().timestamp(),
            1_709_164_800
        );
        assert!(matches!(
            string_to_business_day("2024-02-30"),
            Err(TimeConversionError::InvalidBusinessDay { .. })
        ));
        assert!(matches!(
            string_to_business_day("2024-2-3"),
            Err(TimeConversionError::InvalidBusinessDayFormat(_))
        ));
    }

    #[test]
    fn preprocesses_strings_and_rejects_mixed_time_kinds() {
        let mut data = [WhitespaceData {
            time: Time::from("2024-01-02"),
            custom_values: None::<()>,
        }];
        convert_strings_to_business_days(&mut data).unwrap();
        assert!(matches!(data[0].time, Time::BusinessDay(_)));

        let mixed = [
            WhitespaceData {
                time: Time::from(UtcTimestamp::new(1.0)),
                custom_values: None::<()>,
            },
            WhitespaceData {
                time: Time::from(BusinessDay {
                    year: 2024,
                    month: 1,
                    day: 1,
                }),
                custom_values: None::<()>,
            },
        ];
        assert_eq!(
            select_time_converter(&mixed),
            Err(TimeConversionError::MixedTimeKinds)
        );
    }
}
