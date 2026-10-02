//! Options consumed by the default time horizontal-scale behavior.

use crate::model::{
    horz_scale_behavior_time::types::{TickMarkType, Time},
    localization_options::LocalizationOptions,
    time_scale_options::HorzScaleOptions,
};

/// Overrides a generated tick label. Returning `None` requests the default
/// localized formatter.
pub type TickMarkFormatter = Box<dyn Fn(&Time, TickMarkType, &str) -> Option<String>>;

/// Time-specific options layered on the shared horizontal-scale options.
#[derive(Default)]
pub struct TimeScaleOptions {
    pub base: HorzScaleOptions,
    pub tick_mark_formatter: Option<TickMarkFormatter>,
}

/// The narrow chart-option subset required by the default time behavior.
pub struct TimeBehaviorOptions {
    pub time_scale: TimeScaleOptions,
    pub localization: LocalizationOptions<Time>,
}

impl Default for TimeBehaviorOptions {
    fn default() -> Self {
        Self {
            time_scale: TimeScaleOptions::default(),
            localization: LocalizationOptions::new("en-US", "dd MMM 'yy"),
        }
    }
}
