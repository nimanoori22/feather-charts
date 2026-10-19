//! Supported Gregorian calendar locales and explicit best-effort fallback.
use std::str::FromStr;

use chrono::Locale;

/// Coverage of the built-in calendar formatter; custom and numeric-domain
/// formatters have their own contracts. See `docs/porting/time-locale.md`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeLocaleSupport {
    /// Verified en-US/fr-FR/de-DE/ar-EG profile or language alias, with supported
    /// Gregorian, numbering-system and hour-cycle extensions.
    Supported,
    /// Existing Chrono month data or an unsupported locale extension. Calendar
    /// arithmetic remains Gregorian; full Intl behavior is not promised.
    BestEffort,
    /// Unknown or invalid locale: deterministic English labels.
    EnglishFallback,
}

/// Reports built-in calendar locale coverage without changing chart options.
pub fn time_locale_support(requested: &str) -> TimeLocaleSupport {
    resolve(requested).support
}

pub(crate) struct TimeLocale {
    pub month: Locale,
    support: TimeLocaleSupport,
    zero: char,
}
impl TimeLocale {
    pub fn digits(&self, value: String) -> String {
        if self.zero == '0' {
            return value;
        }
        value
            .chars()
            .map(|c| {
                if c.is_ascii_digit() {
                    char::from_u32(self.zero as u32 + c as u32 - '0' as u32).unwrap()
                } else {
                    c
                }
            })
            .collect()
    }
}

pub(crate) fn resolve(requested: &str) -> TimeLocale {
    let lower = requested.to_ascii_lowercase();
    let (base, extension) = lower
        .split_once("-u-")
        .map_or((lower.as_str(), None), |(a, b)| (a, Some(b)));
    let profile = match base {
        "default" | "en" | "en-us" => Some(Locale::en_US),
        "fr" | "fr-fr" => Some(Locale::fr_FR),
        "de" | "de-de" => Some(Locale::de_DE),
        "ar" | "ar-eg" => Some(Locale::ar_EG),
        _ => None,
    };
    let month = profile.or_else(|| {
        let normalized = match requested {
            value if !value.contains('_') && value.len() == 2 => {
                format!("{}_{}", value, value.to_ascii_uppercase())
            }
            value => value.replace('-', "_"),
        };
        Locale::from_str(&normalized).ok()
    });
    let mut result = TimeLocale {
        month: month.unwrap_or(Locale::en_US),
        support: if profile.is_some() {
            TimeLocaleSupport::Supported
        } else if month.is_some() {
            TimeLocaleSupport::BestEffort
        } else {
            TimeLocaleSupport::EnglishFallback
        },
        zero: if base == "ar-eg" { '٠' } else { '0' },
    };
    if let Some(extension) = extension
        && profile.is_some()
    {
        // shortcut: only the verified Unicode keyword pairs are supported;
        // use a Unicode locale provider when broader BCP47 support is required.
        let mut parts = extension.split('-');
        let mut seen = 0;
        while let Some(key) = parts.next() {
            let value = parts.next();
            let bit = match key {
                "nu" => 1,
                "ca" => 2,
                "hc" => 4,
                _ => 0,
            };
            let valid = match (key, value) {
                ("nu", Some("latn")) => {
                    result.zero = '0';
                    true
                }
                ("nu", Some("arab")) => {
                    result.zero = '٠';
                    true
                }
                ("nu", Some("arabext")) => {
                    result.zero = '۰';
                    true
                }
                ("ca", Some("gregory")) | ("hc", Some("h23")) => true,
                _ => false,
            };
            if !valid || seen & bit != 0 {
                result.support = TimeLocaleSupport::BestEffort;
            }
            seen |= bit;
        }
    }
    result
}
