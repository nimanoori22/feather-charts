//! Tick-weight generation for timestamp and business-day time points.

use chrono::Datelike;

use crate::model::{
    horz_scale_behavior_time::{
        time_utils::TimeConversionError,
        types::{TickMarkWeight, TimePoint, UtcTimestamp},
    },
    time_data::{TickMarkWeightValue, TimeScalePoint},
};

const INTRADAY_DIVISORS: &[(i64, TickMarkWeight)] = &[
    (1_000, TickMarkWeight::Second),
    (60_000, TickMarkWeight::Minute1),
    (5 * 60_000, TickMarkWeight::Minute5),
    (30 * 60_000, TickMarkWeight::Minute30),
    (60 * 60_000, TickMarkWeight::Hour1),
    (3 * 60 * 60_000, TickMarkWeight::Hour3),
    (6 * 60 * 60_000, TickMarkWeight::Hour6),
    (12 * 60 * 60_000, TickMarkWeight::Hour12),
];

fn date(point: &TimePoint) -> Result<chrono::DateTime<chrono::Utc>, TimeConversionError> {
    point
        .timestamp
        .to_date_time()
        .map(|value| *value.inner())
        .ok_or(TimeConversionError::InvalidTimestamp)
}

fn weight_by_time(
    current: &TimePoint,
    previous: &TimePoint,
) -> Result<TickMarkWeight, TimeConversionError> {
    let current_date = date(current)?;
    let previous_date = date(previous)?;
    if current_date.year() != previous_date.year() {
        return Ok(TickMarkWeight::Year);
    }
    if current_date.month() != previous_date.month() {
        return Ok(TickMarkWeight::Month);
    }
    if current_date.day() != previous_date.day() {
        return Ok(TickMarkWeight::Day);
    }

    let current_millis = current_date.timestamp_millis();
    let previous_millis = previous_date.timestamp_millis();
    for &(divisor, weight) in INTRADAY_DIVISORS.iter().rev() {
        if previous_millis.div_euclid(divisor) != current_millis.div_euclid(divisor) {
            return Ok(weight);
        }
    }
    Ok(TickMarkWeight::LessThanSecond)
}

/// Fills tick weights from `start_index` onward, preserving the unaffected
/// prefix during incremental data updates.
pub fn fill_weights_for_points<OriginalItem>(
    points: &mut [TimeScalePoint<TimePoint, OriginalItem>],
    start_index: usize,
) -> Result<(), TimeConversionError> {
    if points.is_empty() || start_index >= points.len() {
        return Ok(());
    }

    let mut previous = if start_index == 0 {
        None
    } else {
        Some(points[start_index - 1].time)
    };
    let mut total_time_difference = 0.0;

    for point in &mut points[start_index..] {
        if let Some(previous_point) = previous {
            point.time_weight =
                TickMarkWeightValue::from(weight_by_time(&point.time, &previous_point)?);
            total_time_difference +=
                point.time.timestamp.seconds() - previous_point.timestamp.seconds();
        }
        previous = Some(point.time);
    }

    if start_index == 0 && points.len() > 1 {
        let average_difference = (total_time_difference / (points.len() - 1) as f64).ceil();
        let first = points[0].time;
        let approximate_previous = TimePoint {
            timestamp: UtcTimestamp::new(first.timestamp.seconds() - average_difference),
            business_day: None,
        };
        points[0].time_weight =
            TickMarkWeightValue::from(weight_by_time(&first, &approximate_previous)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::fill_weights_for_points;
    use crate::model::{
        horz_scale_behavior_time::types::{TickMarkWeight, TimePoint, UtcTimestamp},
        time_data::{TickMarkWeightValue, TimeScalePoint},
    };

    fn point(timestamp: f64) -> TimeScalePoint<TimePoint, ()> {
        TimeScalePoint {
            time_weight: TickMarkWeightValue::default(),
            time: TimePoint {
                timestamp: UtcTimestamp::new(timestamp),
                business_day: None,
            },
            original_time: (),
        }
    }

    #[test]
    fn assigns_calendar_and_intraday_transition_weights() {
        let mut points = [
            point(1_704_067_200.0), // 2024-01-01T00:00:00Z
            point(1_704_067_260.0),
            point(1_704_153_600.0), // next day
            point(1_706_832_000.0), // next month
        ];
        fill_weights_for_points(&mut points, 0).unwrap();

        assert_eq!(
            points[1].time_weight,
            TickMarkWeightValue::from(TickMarkWeight::Minute1)
        );
        assert_eq!(
            points[2].time_weight,
            TickMarkWeightValue::from(TickMarkWeight::Day)
        );
        assert_eq!(
            points[3].time_weight,
            TickMarkWeightValue::from(TickMarkWeight::Month)
        );
    }

    #[test]
    fn leaves_the_unaffected_prefix_untouched_for_incremental_updates() {
        let mut points = [
            point(1_704_067_200.0),
            point(1_704_067_260.0),
            point(1_704_067_320.0),
        ];
        points[0].time_weight = TickMarkWeightValue::new(99);
        points[1].time_weight = TickMarkWeightValue::new(98);

        fill_weights_for_points(&mut points, 2).unwrap();

        assert_eq!(points[0].time_weight, TickMarkWeightValue::new(99));
        assert_eq!(points[1].time_weight, TickMarkWeightValue::new(98));
        assert_eq!(
            points[2].time_weight,
            TickMarkWeightValue::from(TickMarkWeight::Minute1)
        );
    }
}
