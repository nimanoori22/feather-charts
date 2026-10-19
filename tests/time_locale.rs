use chrono::{TimeZone, Utc};
use feather_charts::{
    formatters::format_date::{UtcDateTime, format_date},
    model::horz_scale_behavior_time::{
        default_tick_mark_formatter::default_tick_mark_formatter,
        types::{TickMarkType, TimePoint, UtcTimestamp},
    },
};
const LOCALES: &[&str] = &[
    "en-US",
    "fr-FR",
    "de-DE",
    "ar-EG",
    "en",
    "fr",
    "de",
    "ar",
    "default",
    "EN-us",
    "de-DE-u-nu-arab",
    "ar-EG-u-nu-latn",
    "ar-EG-u-nu-arabext",
    "en-US-u-hc-h23",
    "de-DE-u-ca-gregory-nu-latn",
    "de-DE-u-nu-arab-ca-gregory",
];
const TIMES: &[f64] = &[
    1704198645.,
    1725148800.,
    1704067199.,
    0.,
    -0.5,
    -0.0001,
    1.9999999995,
];
fn quote(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    )
}
#[test]
fn reference_matrix() {
    for locale in LOCALES {
        let ticks = TIMES
            .iter()
            .map(|timestamp| {
                let point = TimePoint {
                    timestamp: UtcTimestamp::new(*timestamp),
                    business_day: None,
                };
                let values = [
                    TickMarkType::Year,
                    TickMarkType::Month,
                    TickMarkType::DayOfMonth,
                    TickMarkType::Time,
                    TickMarkType::TimeWithSeconds,
                ]
                .map(|kind| {
                    quote(
                        &default_tick_mark_formatter(&point, kind, locale)
                            .unwrap_or_else(|_| "ERROR".into()),
                    )
                });
                format!(
                    "{{\"timestamp\":{timestamp},\"values\":[{}]}}",
                    values.join(",")
                )
            })
            .collect::<Vec<_>>();
        let months = (1..=12)
            .map(|m| {
                let t = Utc
                    .with_ymd_and_hms(2024, m, 2, 0, 0, 0)
                    .single()
                    .unwrap()
                    .timestamp();
                quote(&format_date(
                    &UtcDateTime::from_unix_timestamp(t as f64).unwrap(),
                    "yyyy-MM-dd MMM MMMM",
                    locale,
                ))
            })
            .collect::<Vec<_>>();
        println!(
            "TIME_LOCALE_REFERENCE {{\"locale\":{},\"ticks\":[{}],\"months\":[{}]}}",
            quote(locale),
            ticks.join(","),
            months.join(",")
        );
    }
    let point = TimePoint {
        timestamp: UtcTimestamp::new(TIMES[0]),
        business_day: None,
    };
    assert_eq!(
        default_tick_mark_formatter(&point, TickMarkType::Year, "ar-EG").unwrap(),
        "٢٠٢٤"
    );
}

#[test]
fn support_and_fallback_are_explicit() {
    use feather_charts::formatters::time_locale::{TimeLocaleSupport::*, time_locale_support};
    for locale in LOCALES {
        assert_eq!(time_locale_support(locale), Supported, "{locale}");
    }
    for locale in [
        "fa-IR",
        "es-ES",
        "ja-JP",
        "en_US",
        "de-DE-u-ca-islamic",
        "en-US-u-nu-foobar",
        "ar-EG-u-nu-latn-nu-arab",
    ] {
        assert_eq!(time_locale_support(locale), BestEffort, "{locale}");
    }
    let point = TimePoint {
        timestamp: UtcTimestamp::new(TIMES[0]),
        business_day: None,
    };
    for locale in ["zz-ZZ", "not_a_locale", "", "x-foo"] {
        assert_eq!(time_locale_support(locale), EnglishFallback, "{locale}");
        assert_eq!(
            default_tick_mark_formatter(&point, TickMarkType::Year, locale).unwrap(),
            "2024"
        );
        assert_eq!(
            format_date(
                &UtcDateTime::from_unix_timestamp(TIMES[0]).unwrap(),
                "MMM",
                locale
            ),
            "Jan"
        );
    }
}

#[test]
fn utc_fields_use_js_millisecond_truncation_without_changing_keys() {
    use feather_charts::model::{
        horz_scale_behavior_time::horz_scale_behavior_time::HorzScaleBehaviorTime,
        ihorz_scale_behavior::HorzScaleBehavior,
    };
    for (seconds, millis) in [
        (-0.0001, 0),
        (-0.5, -500),
        (1.9999999995, 1999),
        (-1.9999999995, -1999),
    ] {
        assert_eq!(
            UtcDateTime::from_unix_timestamp(seconds)
                .unwrap()
                .inner()
                .timestamp_millis(),
            millis
        );
    }
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e30] {
        assert!(UtcDateTime::from_unix_timestamp(invalid).is_none());
    }
    let behavior = HorzScaleBehaviorTime::default();
    let a = behavior
        .to_internal(&UtcTimestamp::new(-0.0001).into())
        .unwrap();
    let b = behavior.to_internal(&UtcTimestamp::new(0.).into()).unwrap();
    assert_ne!(behavior.key(&a), behavior.key(&b));
    assert_eq!(behavior.cache_key(&a), behavior.cache_key(&b));
}

#[test]
fn custom_ticks_receive_original_locale_and_none_restores_default() {
    use feather_charts::model::{
        horz_scale_behavior_time::{
            horz_scale_behavior_time::HorzScaleBehaviorTime,
            time_based_chart_options::TimeBehaviorOptions,
            types::{TickMarkWeight, Time},
        },
        ihorz_scale_behavior::HorzScaleBehavior,
        localization_options::LocalizationOptions,
        tick_marks::TickMark,
    };
    use std::{cell::Cell, rc::Rc};
    let enabled = Rc::new(Cell::new(true));
    let mut options = TimeBehaviorOptions::default();
    options.time_scale.tick_mark_formatter = Some(Box::new({
        let enabled = enabled.clone();
        move |_, _, locale| {
            assert_eq!(locale, "AR-eg-u-nu-arabext");
            enabled.get().then(|| "custom ASCII 2024".into())
        }
    }));
    let behavior = HorzScaleBehaviorTime::new(options);
    let item: Time = UtcTimestamp::new(TIMES[0]).into();
    let mark = TickMark {
        index: 0.0.into(),
        time: behavior.to_internal(&item).unwrap(),
        weight: TickMarkWeight::Year.into(),
        original_time: item,
    };
    let localization = LocalizationOptions::new("AR-eg-u-nu-arabext", "yyyy-MM-dd");
    assert_eq!(
        behavior.format_tickmark(&mark, &localization),
        "custom ASCII 2024"
    );
    enabled.set(false);
    assert_eq!(behavior.format_tickmark(&mark, &localization), "۲۰۲۴");
}
