//! UI-owned driver for synchronous time-scale animation effects.
use crate::model::time_scale_host::{ScrollAnimation, TimeScaleEffect};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct ActiveTimeScaleAnimation {
    pub start_offset: f64,
    pub target_offset: f64,
    pub started_at: Instant,
    pub duration: Duration,
}
impl ActiveTimeScaleAnimation {
    pub fn from_effect(value: ScrollAnimation, now: Instant) -> Self {
        Self {
            start_offset: value.start_offset,
            target_offset: value.target_offset,
            started_at: now,
            duration: value.duration,
        }
    }
    pub fn sample(self, now: Instant) -> (f64, bool) {
        let p = (now.saturating_duration_since(self.started_at).as_secs_f64()
            / self.duration.as_secs_f64())
        .min(1.0);
        (
            self.start_offset + (self.target_offset - self.start_offset) * p,
            p >= 1.0,
        )
    }
}
#[derive(Default)]
pub struct TimeScaleAnimationController {
    active: Option<ActiveTimeScaleAnimation>,
    redraw_requested: bool,
}
impl TimeScaleAnimationController {
    pub fn apply_effects(
        &mut self,
        effects: impl IntoIterator<Item = TimeScaleEffect>,
        now: Instant,
    ) {
        for effect in effects {
            match effect {
                TimeScaleEffect::StartScrollAnimation(a) => {
                    self.active = Some(ActiveTimeScaleAnimation::from_effect(a, now))
                }
                TimeScaleEffect::LightUpdate | TimeScaleEffect::OptionsApplied => {
                    self.redraw_requested = true
                }
                TimeScaleEffect::RecalculateAllPanes => {}
            }
        }
    }
    pub fn frame(&mut self, now: Instant) -> Option<f64> {
        let active = self.active?;
        let (offset, done) = active.sample(now);
        self.redraw_requested = true;
        if done {
            self.active = None;
        }
        Some(offset)
    }
    pub fn cancel(&mut self) {
        self.active = None;
    }
    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }
    pub fn take_redraw_request(&mut self) -> bool {
        std::mem::take(&mut self.redraw_requested)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn samples_and_completes() {
        let now = Instant::now();
        let mut c = TimeScaleAnimationController::default();
        c.apply_effects(
            [TimeScaleEffect::StartScrollAnimation(ScrollAnimation {
                start_offset: 0.,
                target_offset: 10.,
                duration: Duration::from_secs(1),
            })],
            now,
        );
        assert_eq!(c.frame(now + Duration::from_millis(500)), Some(5.));
        assert_eq!(c.frame(now + Duration::from_secs(1)), Some(10.));
        assert!(!c.is_active());
    }
}
