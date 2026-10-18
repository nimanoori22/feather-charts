//! Localized formatting callbacks supplied to horizontal-scale behaviors.

use crate::model::price_formatter_fn::*;
use std::rc::Rc;

/// Custom formatting for a horizontal-scale item.
pub type TimeFormatterFn<T> = Rc<dyn Fn(&T) -> String>;

/// Locale-wide price and percentage formatting hooks.
#[derive(Clone)]
pub struct LocalizationOptionsBase {
    pub locale: String,
    pub price_formatter: Option<PriceFormatterFn>,
    pub tickmarks_price_formatter: Option<TickmarksPriceFormatterFn>,
    pub percentage_formatter: Option<PercentageFormatterFn>,
    pub tickmarks_percentage_formatter: Option<TickmarksPercentageFormatterFn>,
}

impl Default for LocalizationOptionsBase {
    fn default() -> Self {
        Self {
            locale: "en-US".into(),
            price_formatter: None,
            tickmarks_price_formatter: None,
            percentage_formatter: None,
            tickmarks_percentage_formatter: None,
        }
    }
}

/// Locale-wide formatting hooks, including the horizontal-scale formatter.
pub struct LocalizationOptions<T> {
    pub base: LocalizationOptionsBase,
    pub time_formatter: Option<TimeFormatterFn<T>>,
    pub date_format: String,
}

impl<T> Clone for LocalizationOptions<T> {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            time_formatter: self.time_formatter.clone(),
            date_format: self.date_format.clone(),
        }
    }
}

/// Atomic localization update. Omitted fields retain their value; Clear removes a
/// callback and restores the underlying formatter. Unlike JS undefined, Clear is explicit.
pub struct LocalizationOptionsPatch<T> {
    pub locale: Option<String>,
    pub date_format: Option<String>,
    pub price_formatter: crate::model::series_options::FieldPatch<PriceFormatterFn>,
    pub tickmarks_price_formatter:
        crate::model::series_options::FieldPatch<TickmarksPriceFormatterFn>,
    pub percentage_formatter: crate::model::series_options::FieldPatch<PercentageFormatterFn>,
    pub tickmarks_percentage_formatter:
        crate::model::series_options::FieldPatch<TickmarksPercentageFormatterFn>,
    pub time_formatter: crate::model::series_options::FieldPatch<TimeFormatterFn<T>>,
}
impl<T> Default for LocalizationOptionsPatch<T> {
    fn default() -> Self {
        Self {
            locale: None,
            date_format: None,
            price_formatter: Default::default(),
            tickmarks_price_formatter: Default::default(),
            percentage_formatter: Default::default(),
            tickmarks_percentage_formatter: Default::default(),
            time_formatter: Default::default(),
        }
    }
}
impl<T> LocalizationOptionsPatch<T> {
    pub(crate) fn apply_to(self, options: &mut LocalizationOptions<T>) -> bool {
        let mut changed = false;
        if let Some(locale) = self.locale
            && options.base.locale != locale
        {
            options.base.locale = locale;
            changed = true;
        }
        if let Some(format) = self.date_format
            && options.date_format != format
        {
            options.date_format = format;
            changed = true;
        }
        changed |= patch_callback(&mut options.base.price_formatter, self.price_formatter);
        changed |= patch_callback(
            &mut options.base.tickmarks_price_formatter,
            self.tickmarks_price_formatter,
        );
        changed |= patch_callback(
            &mut options.base.percentage_formatter,
            self.percentage_formatter,
        );
        changed |= patch_callback(
            &mut options.base.tickmarks_percentage_formatter,
            self.tickmarks_percentage_formatter,
        );
        changed |= patch_callback(&mut options.time_formatter, self.time_formatter);
        changed
    }
}
fn patch_callback<T: ?Sized>(
    target: &mut Option<Rc<T>>,
    patch: crate::model::series_options::FieldPatch<Rc<T>>,
) -> bool {
    use crate::model::series_options::FieldPatch;
    let next = match patch {
        FieldPatch::Unchanged => return false,
        FieldPatch::Set(value) => Some(value),
        FieldPatch::Clear => None,
    };
    let equal = match (&*target, &next) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    };
    if equal {
        return false;
    }
    *target = next;
    true
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
