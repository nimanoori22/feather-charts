//! Localized formatting callbacks supplied to horizontal-scale behaviors.

use crate::model::price_formatter_fn::*;
use std::rc::Rc;

/// Custom formatting for a horizontal-scale item.
pub type TimeFormatterFn<T> = Rc<dyn Fn(&T) -> String>;

/// Locale-wide price and percentage formatting hooks.
pub struct LocalizationOptionsBase {
    pub locale: String,
    pub price_formatter: Option<PriceFormatterFn>,
    pub tickmarks_price_formatter: Option<TickmarksPriceFormatterFn>,
    pub percentage_formatter: Option<PercentageFormatterFn>,
    pub tickmarks_percentage_formatter: Option<TickmarksPercentageFormatterFn>,
}

/// Locale-wide formatting hooks, including the horizontal-scale formatter.
pub struct LocalizationOptions<T> {
    pub base: LocalizationOptionsBase,
    pub time_formatter: Option<TimeFormatterFn<T>>,
    pub date_format: String,
}

impl<T> LocalizationOptions<T> {
    pub fn new(locale: impl Into<String>, date_format: impl Into<String>) -> Self {
        Self {
            base: LocalizationOptionsBase {
                locale: locale.into(),
                price_formatter: None,
                tickmarks_price_formatter: None,
                percentage_formatter: None,
                tickmarks_percentage_formatter: None,
            },
            time_formatter: None,
            date_format: date_format.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LocalizationOptions;
    use std::rc::Rc;

    #[test]
    fn retains_locale_and_optional_time_formatter() {
        let mut options = LocalizationOptions::new("en-US", "dd MMM 'yy");
        options.time_formatter = Some(Rc::new(|value: &u32| format!("T{value}")));

        assert_eq!(options.base.locale, "en-US");
        assert_eq!(options.date_format, "dd MMM 'yy");
        assert_eq!(options.time_formatter.as_ref().unwrap()(&4), "T4");
    }
}
