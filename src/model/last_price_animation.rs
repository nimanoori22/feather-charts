//! Monotonic, synchronous last-price pulse timing.
use super::{data_layer::SeriesUpdateInfo, series_options::LastPriceAnimationMode};
use std::time::{Duration, Instant};

const PERIOD: Duration = Duration::from_millis(2600);

#[derive(Clone, Copy, Debug)]
pub(crate) struct LastPriceAnimationState {
    start: Instant,
    end: Option<Instant>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AnimationSample {
    pub radius: f64,
    pub fill_alpha: f64,
    pub stroke_alpha: f64,
}

impl LastPriceAnimationState {
    pub fn new(now: Instant) -> Self {
        Self {
            start: now,
            end: None,
        }
    }
    pub fn on_update(
        &mut self,
        info: Option<SeriesUpdateInfo>,
        empty: bool,
        mode: LastPriceAnimationMode,
        now: Instant,
    ) {
        if info.is_some_and(|i| i.last_bar_updated_or_new_bars_added_to_the_right) {
            if mode != LastPriceAnimationMode::OnDataUpdate {
                return;
            }
            if let Some(end) = self.end.filter(|end| *end > now) {
                if end.duration_since(now) < PERIOD / 4 {
                    self.end = Some(end + PERIOD);
                }
            } else {
                self.start = now;
                self.end = Some(now + PERIOD);
            }
        } else if empty {
            self.end = None;
        }
    }
    pub fn is_active(self, mode: LastPriceAnimationMode, now: Instant) -> bool {
        match mode {
            LastPriceAnimationMode::Disabled => false,
            LastPriceAnimationMode::Continuous => true,
            LastPriceAnimationMode::OnDataUpdate => self.end.is_some_and(|end| now <= end),
        }
    }
    pub fn sample(self, mode: LastPriceAnimationMode, now: Instant) -> AnimationSample {
        let millis = if self.is_active(mode, now) {
            now.saturating_duration_since(self.start).as_secs_f64() * 1000.
        } else {
            2599.
        };
        let stage = (millis % 2600.) / 2600.;
        if stage <= 0.25 {
            let t = stage / 0.25;
            AnimationSample {
                radius: 4. + 6. * t,
                fill_alpha: 0.25 * (1. - t),
                stroke_alpha: 0.4 + 0.4 * t,
            }
        } else if stage <= 0.525 {
            let t = (stage - 0.25) / 0.275;
            AnimationSample {
                radius: 10. + 4. * t,
                fill_alpha: 0.,
                stroke_alpha: 0.8 * (1. - t),
            }
        } else {
            AnimationSample {
                radius: 14.,
                fill_alpha: 0.,
                stroke_alpha: 0.,
            }
        }
    }
}

#[cfg(test)]
mod tests;
