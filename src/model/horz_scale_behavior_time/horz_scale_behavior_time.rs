//! Default timestamp and business-day horizontal-scale behavior.

use crate::{
    formatters::{date_formatter::DateFormatter, date_time_formatter::DateTimeFormatter},
    model::{
        data_consumer::TimedData,
        horz_scale_behavior_time::{
            default_tick_mark_formatter::default_tick_mark_formatter,
            time_based_chart_options::TimeBehaviorOptions,
            time_scale_point_weight_generator::fill_weights_for_points,
            time_utils::{
                TimeConversionError, TimeConverter, convert_strings_to_business_days, convert_time,
                select_time_converter,
            },
            types::{TickMarkType, TickMarkWeight, Time, TimePoint},
        },
        ihorz_scale_behavior::{HorzScaleBehavior, HorzScaleItemConverter, TimeMark},
        localization_options::LocalizationOptions,
        tick_marks::TickMark,
        time_data::{TickMarkWeightValue, TimeScalePoint},
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TimeKey(u64);

impl TimeKey {
    fn new(timestamp: f64) -> Self {
        let bits = timestamp.to_bits();
        Self(if bits >> 63 == 0 {
            bits | (1_u64 << 63)
        } else {
            !bits
        })
    }
}

impl HorzScaleItemConverter<Time, TimePoint> for TimeConverter {
    type Error = TimeConversionError;

    fn convert(&mut self, item: &Time) -> Result<TimePoint, Self::Error> {
        TimeConverter::convert(*self, item)
    }
}

enum ItemFormatter {
    Date(DateFormatter),
    DateTime(DateTimeFormatter),
}

impl ItemFormatter {
    fn format(&self, point: &TimePoint) -> Result<String, TimeConversionError> {
        let date = point
            .timestamp
            .to_date_time()
            .ok_or(TimeConversionError::InvalidTimestamp)?;
        Ok(match self {
            Self::Date(formatter) => formatter.format(&date),
            Self::DateTime(formatter) => formatter.format(&date),
        })
    }
}

/// The built-in synchronous behavior for Lightweight Charts `Time` values.
pub struct HorzScaleBehaviorTime {
    options: TimeBehaviorOptions,
    item_formatter: ItemFormatter,
}

impl HorzScaleBehaviorTime {
    pub fn new(options: TimeBehaviorOptions) -> Self {
        let item_formatter = Self::build_item_formatter(&options);
        Self {
            options,
            item_formatter,
        }
    }

    fn build_item_formatter(options: &TimeBehaviorOptions) -> ItemFormatter {
        let locale = &options.localization.base.locale;
        let date_format = &options.localization.date_format;
        if options.time_scale.base.time_visible {
            ItemFormatter::DateTime(DateTimeFormatter::new(
                Some(date_format),
                Some(if options.time_scale.base.seconds_visible {
                    "%h:%m:%s"
                } else {
                    "%h:%m"
                }),
                Some("   "),
                Some(locale),
            ))
        } else {
            ItemFormatter::Date(DateFormatter::new(Some(date_format), Some(locale)))
        }
    }

    fn tick_mark_type(&self, weight: TickMarkWeightValue) -> TickMarkType {
        match weight.value() {
            value
                if value == TickMarkWeight::LessThanSecond as i32
                    || value == TickMarkWeight::Second as i32 =>
            {
                if self.options.time_scale.base.time_visible {
                    if self.options.time_scale.base.seconds_visible {
                        TickMarkType::TimeWithSeconds
                    } else {
                        TickMarkType::Time
                    }
                } else {
                    TickMarkType::DayOfMonth
                }
            }
            value
                if value == TickMarkWeight::Minute1 as i32
                    || value == TickMarkWeight::Minute5 as i32
                    || value == TickMarkWeight::Minute30 as i32
                    || value == TickMarkWeight::Hour1 as i32
                    || value == TickMarkWeight::Hour3 as i32
                    || value == TickMarkWeight::Hour6 as i32
                    || value == TickMarkWeight::Hour12 as i32 =>
            {
                if self.options.time_scale.base.time_visible {
                    TickMarkType::Time
                } else {
                    TickMarkType::DayOfMonth
                }
            }
            value if value == TickMarkWeight::Month as i32 => TickMarkType::Month,
            value if value == TickMarkWeight::Year as i32 => TickMarkType::Year,
            _ => TickMarkType::DayOfMonth,
        }
    }
}

impl Default for HorzScaleBehaviorTime {
    fn default() -> Self {
        Self::new(TimeBehaviorOptions::default())
    }
}

impl HorzScaleBehavior for HorzScaleBehaviorTime {
    type Item = Time;
    type InternalItem = TimePoint;
    type Key = TimeKey;
    type CacheKey = i64;
    type Options = TimeBehaviorOptions;
    type Converter = TimeConverter;
    type Error = TimeConversionError;

    fn options(&self) -> &Self::Options {
        &self.options
    }

    fn set_options(&mut self, options: Self::Options) {
        self.item_formatter = Self::build_item_formatter(&options);
        self.options = options;
    }

    fn preprocess_data<D: TimedData<Item = Self::Item>>(
        &mut self,
        data: &mut [D],
    ) -> Result<(), Self::Error> {
        convert_strings_to_business_days(data)
    }

    fn to_internal(&self, item: &Self::Item) -> Result<Self::InternalItem, Self::Error> {
        convert_time(item)
    }

    fn create_converter_to_internal<D: TimedData<Item = Self::Item>>(
        &mut self,
        data: &[D],
    ) -> Result<Self::Converter, Self::Error> {
        select_time_converter(data)
    }

    fn key(&self, item: &Self::InternalItem) -> Self::Key {
        TimeKey::new(item.timestamp.seconds())
    }

    fn cache_key(&self, item: &Self::InternalItem) -> Self::CacheKey {
        item.timestamp
            .to_date_time()
            .map(|date| date.inner().timestamp_millis())
            .unwrap_or(i64::MIN)
    }

    fn update_formatter(&mut self, options: &LocalizationOptions<Self::Item>) {
        self.options.localization =
            LocalizationOptions::new(&options.base.locale, &options.date_format);
        self.options.localization.time_formatter = options.time_formatter.clone();
        self.item_formatter = Self::build_item_formatter(&self.options);
    }

    fn format_item(&self, item: &Self::InternalItem) -> String {
        if let Some(formatter) = &self.options.localization.time_formatter {
            return formatter(
                &item
                    .business_day
                    .map(Time::BusinessDay)
                    .unwrap_or(Time::UtcTimestamp(item.timestamp)),
            );
        }
        self.item_formatter.format(item).unwrap_or_default()
    }

    fn format_tickmark(
        &self,
        item: &TickMark<Self::InternalItem, Self::Item>,
        localization_options: &LocalizationOptions<Self::Item>,
    ) -> String {
        let tick_mark_type = self.tick_mark_type(item.weight);
        if let Some(formatter) = &self.options.time_scale.tick_mark_formatter
            && let Some(value) = formatter(
                &item.original_time,
                tick_mark_type,
                &localization_options.base.locale,
            )
        {
            return value;
        }
        default_tick_mark_formatter(
            &item.time,
            tick_mark_type,
            &localization_options.base.locale,
        )
        .unwrap_or_default()
    }

    fn max_tick_mark_weight(&self, marks: &[TimeMark]) -> TickMarkWeightValue {
        let Some(maximum) = marks.iter().map(|mark| mark.weight).max() else {
            return TickMarkWeightValue::default();
        };
        if maximum.value() > TickMarkWeight::Hour1 as i32
            && maximum.value() < TickMarkWeight::Day as i32
        {
            TickMarkWeightValue::from(TickMarkWeight::Hour1)
        } else {
            maximum
        }
    }

    fn fill_weights_for_points(
        &mut self,
        points: &mut [TimeScalePoint<Self::InternalItem, Self::Item>],
        start_index: usize,
    ) {
        fill_weights_for_points(points, start_index)
            .expect("time-scale points must contain valid normalized UTC timestamps");
    }
}

#[cfg(test)]
mod tests {
    use super::HorzScaleBehaviorTime;
    use crate::model::{
        data_consumer::WhitespaceData,
        horz_scale_behavior_time::{
            time_based_chart_options::{TimeBehaviorOptions, TimeScaleOptions},
            types::{BusinessDay, TickMarkWeight, Time, UtcTimestamp},
        },
        ihorz_scale_behavior::{HorzScaleBehavior, TimeMark},
        time_data::TickMarkWeightValue,
    };

    #[test]
    fn normalizes_business_days_formats_ticks_and_reduces_intraday_bold_weight() {
        let mut behavior = HorzScaleBehaviorTime::default();
        let mut data = [WhitespaceData {
            time: Time::from("2024-01-02"),
            custom_values: None::<()>,
        }];
        behavior.preprocess_data(&mut data).unwrap();
        let converter = behavior.create_converter_to_internal(&data).unwrap();
        let internal = converter.convert(&data[0].time).unwrap();
        assert_eq!(
            internal.business_day,
            Some(BusinessDay {
                year: 2024,
                month: 1,
                day: 2
            })
        );

        assert_eq!(
            behavior.max_tick_mark_weight(&[TimeMark {
                need_align_coordinate: false,
                coordinate: 0.0,
                label: String::new(),
                weight: TickMarkWeightValue::from(TickMarkWeight::Hour6),
            }]),
            TickMarkWeightValue::from(TickMarkWeight::Hour1)
        );
    }

    #[test]
    fn switches_item_formatting_when_time_visibility_changes() {
        let mut options = TimeBehaviorOptions::default();
        options.time_scale = TimeScaleOptions::default();
        options.time_scale.base.time_visible = true;
        options.time_scale.base.seconds_visible = false;
        let behavior = HorzScaleBehaviorTime::new(options);
        let internal = behavior
            .to_internal(&Time::from(UtcTimestamp::new(1_704_198_645.0)))
            .unwrap();

        assert_eq!(behavior.format_item(&internal), "02 Jan '24   12:30");
    }
}
