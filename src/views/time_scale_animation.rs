//! UI-owned driver for synchronous time-scale animation effects.
use crate::model::{
    chart_model::ChartModel,
    ihorz_scale_behavior::HorzScaleBehavior,
    invalidate_mask::{AnimationRequest, TimeScaleAnimation},
    time_scale_host::{ScrollAnimation, TimeScaleEffect},
};
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
        let p = if self.duration.is_zero() {
            1.0
        } else {
            (now.saturating_duration_since(self.started_at).as_secs_f64()
                / self.duration.as_secs_f64())
            .min(1.0)
        };
        (
            self.start_offset + (self.target_offset - self.start_offset) * p,
            p >= 1.0,
        )
    }
}
impl TimeScaleAnimation for ActiveTimeScaleAnimation {
    fn sample(&self, now: Instant) -> (f64, bool) {
        (*self).sample(now)
    }
}
#[derive(Default)]
pub struct TimeScaleAnimationController {
    active: Option<AnimationRequest>,
    redraw_requested: bool,
}
impl TimeScaleAnimationController {
    /// Starts newly requested scrolls once and processes the next model frame.
    /// Returns its mask so the UI can render or replay after changing layout.
    pub fn draw_frame<B, M>(
        &mut self,
        model: &mut ChartModel<B, M>,
        now: Instant,
    ) -> Option<crate::model::invalidate_mask::InvalidateMask>
    where
        B: HorzScaleBehavior,
        B::InternalItem: 'static,
        B::Item: 'static,
        B::Key: PartialOrd,
        B::CacheKey: Eq + std::hash::Hash,
        M: Clone + 'static,
    {
        for descriptor in model.take_scroll_animation_requests() {
            model.set_time_scale_animation(AnimationRequest::new(
                ActiveTimeScaleAnimation::from_effect(descriptor, now),
            ));
        }
        let mask = model.take_invalidation()?;
        self.apply_frame(model, &mask, now);
        Some(mask)
    }
    /// Applies a retained frame mask, samples its surviving animation, and queues
    /// continuation before later navigation requests can merge into pending work.
    /// The UI applies layout dimensions before calling this method.
    pub fn apply_frame<B, M>(
        &mut self,
        model: &mut ChartModel<B, M>,
        mask: &crate::model::invalidate_mask::InvalidateMask,
        now: Instant,
    ) where
        B: HorzScaleBehavior,
        B::InternalItem: 'static,
        B::Item: 'static,
        B::Key: PartialOrd,
        B::CacheKey: Eq + std::hash::Hash,
        M: Clone + 'static,
    {
        if mask.global_level() < crate::model::invalidate_mask::InvalidationLevel::Light {
            return;
        }
        model.apply_invalidation(mask);
        self.cancel();
        for command in mask.time_scale_invalidations() {
            if let crate::model::invalidate_mask::TimeScaleInvalidation::Animation(request) =
                command
            {
                self.apply_animation(request.clone());
                if let Some(offset) = self.frame(now) {
                    model.apply_animation_offset(offset);
                }
                if self.is_active() {
                    model.set_time_scale_animation(request.clone());
                }
            }
        }
    }
    pub fn apply_effects(
        &mut self,
        effects: impl IntoIterator<Item = TimeScaleEffect>,
        now: Instant,
    ) {
        for effect in effects {
            match effect {
                TimeScaleEffect::StartScrollAnimation(a) => {
                    self.apply_animation(AnimationRequest::new(
                        ActiveTimeScaleAnimation::from_effect(a, now),
                    ));
                }
                TimeScaleEffect::LightUpdate | TimeScaleEffect::OptionsApplied => {
                    self.redraw_requested = true
                }
                TimeScaleEffect::RecalculateAllPanes => {}
            }
        }
    }
    /// Installs a request retaining the time origin established by its producer.
    /// Use this for mask replay; `apply_effects` creates a new timed animation.
    pub fn apply_animation(&mut self, animation: AnimationRequest) {
        self.active = Some(animation);
    }
    pub fn frame(&mut self, now: Instant) -> Option<f64> {
        let active = self.active.as_ref()?;
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
    use crate::model::invalidate_mask::{InvalidateMask, TimeScaleInvalidation};
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

    #[test]
    fn replayed_masks_do_not_restart_the_scroll_animation() {
        let now = Instant::now();
        let mut mask = InvalidateMask::light();
        mask.set_animation(AnimationRequest::new(
            ActiveTimeScaleAnimation::from_effect(
                ScrollAnimation {
                    start_offset: 0.0,
                    target_offset: 10.0,
                    duration: Duration::from_secs(1),
                },
                now,
            ),
        ));
        let mut controller = TimeScaleAnimationController::default();
        for (millis, expected) in [(250, 2.5), (750, 7.5), (1000, 10.0)] {
            let mut layout_pass = InvalidateMask::full();
            layout_pass.merge(&mask);
            for command in layout_pass.time_scale_invalidations() {
                if let TimeScaleInvalidation::Animation(request) = command {
                    controller.apply_animation(request.clone());
                }
            }
            assert_eq!(
                controller.frame(now + Duration::from_millis(millis)),
                Some(expected)
            );
        }
        assert!(!controller.is_active());
    }

    #[test]
    fn zero_duration_scroll_finishes_at_its_target() {
        let now = Instant::now();
        let animation = ActiveTimeScaleAnimation::from_effect(
            ScrollAnimation {
                start_offset: 2.0,
                target_offset: 8.0,
                duration: Duration::ZERO,
            },
            now,
        );
        assert_eq!(animation.sample(now), (8.0, true));
    }

    #[test]
    fn ui_can_cancel_an_active_animation_after_manual_navigation() {
        let now = Instant::now();
        let mut controller = TimeScaleAnimationController::default();
        controller.apply_effects(
            [TimeScaleEffect::StartScrollAnimation(ScrollAnimation {
                start_offset: 0.0,
                target_offset: 10.0,
                duration: Duration::from_secs(1),
            })],
            now,
        );
        controller.cancel();
        assert_eq!(controller.frame(now + Duration::from_millis(500)), None);
        assert!(!controller.is_active());
    }
}
