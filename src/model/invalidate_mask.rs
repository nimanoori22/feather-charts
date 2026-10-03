//! Deferred redraw, momentary autoscale, and horizontal-scale requests.
//!
//! Merging replays incoming commands in order. It is directional: fit, range,
//! and reset replace prior commands, while a merged stop only removes animation.
//! Reading commands leaves them available for a second pass after layout changes.

use crate::model::time_data::LogicalRange;
use std::{collections::BTreeMap, fmt::Debug, rc::Rc, time::Instant};

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum InvalidationLevel {
    #[default]
    None,
    Cursor,
    Light,
    Full,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PaneInvalidation {
    pub level: InvalidationLevel,
    /// Request autoscaling for this update, without changing persistent options.
    pub auto_scale: bool,
}

/// A synchronous animation with an origin established by its UI owner.
///
/// Implementations retain their original timing across frames and must not
/// capture the chart owner. The caller supplies time; sampling needs no clock,
/// model mutation, or reference to a pane. Both linear and kinetic curves fit
/// this boundary without requiring the accumulator to implement either curve.
pub trait TimeScaleAnimation: Debug {
    /// Returns the right offset and whether the animation has finished.
    fn sample(&self, now: Instant) -> (f64, bool);
}

/// Cloning or replaying a request retains the same animation and time origin.
#[derive(Clone, Debug)]
pub struct AnimationRequest(Rc<dyn TimeScaleAnimation>);

impl AnimationRequest {
    pub fn new(animation: impl TimeScaleAnimation + 'static) -> Self {
        Self(Rc::new(animation))
    }

    pub fn sample(&self, now: Instant) -> (f64, bool) {
        self.0.sample(now)
    }
}

#[derive(Clone, Debug)]
pub enum TimeScaleInvalidation {
    FitContent,
    ApplyRange(LogicalRange),
    ApplyBarSpacing(f64),
    ApplyRightOffset(f64),
    Reset,
    Animation(AnimationRequest),
    /// Removes pending animation continuation; it is not a scale mutation.
    StopAnimation,
}

#[derive(Clone, Debug)]
pub struct InvalidateMask {
    global_level: InvalidationLevel,
    panes: BTreeMap<usize, PaneInvalidation>,
    time_scale: Vec<TimeScaleInvalidation>,
}

impl InvalidateMask {
    pub fn new(level: InvalidationLevel) -> Self {
        Self {
            global_level: level,
            panes: BTreeMap::new(),
            time_scale: Vec::new(),
        }
    }

    pub fn light() -> Self {
        Self::new(InvalidationLevel::Light)
    }

    pub fn full() -> Self {
        Self::new(InvalidationLevel::Full)
    }

    pub fn global_level(&self) -> InvalidationLevel {
        self.global_level
    }

    pub fn invalidate_pane(&mut self, index: usize, request: PaneInvalidation) {
        self.panes
            .entry(index)
            .and_modify(|pending| {
                pending.level = pending.level.max(request.level);
                pending.auto_scale |= request.auto_scale;
            })
            .or_insert(request);
    }

    pub fn pane_invalidation(&self, index: usize) -> PaneInvalidation {
        let request = self.panes.get(&index).copied().unwrap_or_default();
        PaneInvalidation {
            level: self.global_level.max(request.level),
            auto_scale: request.auto_scale,
        }
    }

    pub fn fit_content(&mut self) {
        self.replace_time_scale(TimeScaleInvalidation::FitContent);
    }

    pub fn apply_range(&mut self, range: LogicalRange) {
        self.replace_time_scale(TimeScaleInvalidation::ApplyRange(range));
    }

    pub fn reset_time_scale(&mut self) {
        self.replace_time_scale(TimeScaleInvalidation::Reset);
    }

    pub fn set_bar_spacing(&mut self, spacing: f64) {
        self.stop_animation();
        self.time_scale
            .push(TimeScaleInvalidation::ApplyBarSpacing(spacing));
    }

    pub fn set_right_offset(&mut self, offset: f64) {
        self.stop_animation();
        self.time_scale
            .push(TimeScaleInvalidation::ApplyRightOffset(offset));
    }

    pub fn set_animation(&mut self, animation: AnimationRequest) {
        self.remove_animation();
        self.time_scale
            .push(TimeScaleInvalidation::Animation(animation));
    }

    pub fn stop_animation(&mut self) {
        self.remove_animation();
        self.time_scale.push(TimeScaleInvalidation::StopAnimation);
    }

    pub fn time_scale_invalidations(&self) -> &[TimeScaleInvalidation] {
        &self.time_scale
    }

    /// Replays incoming commands without consuming or modifying their mask.
    ///
    /// Command setters intentionally do not raise the global redraw level.
    /// Producers request light/full redraw separately, as in Lightweight Charts.
    pub fn merge(&mut self, incoming: &Self) {
        for command in &incoming.time_scale {
            match command {
                TimeScaleInvalidation::FitContent => self.fit_content(),
                TimeScaleInvalidation::ApplyRange(range) => self.apply_range(*range),
                TimeScaleInvalidation::ApplyBarSpacing(spacing) => self.set_bar_spacing(*spacing),
                TimeScaleInvalidation::ApplyRightOffset(offset) => self.set_right_offset(*offset),
                TimeScaleInvalidation::Reset => self.reset_time_scale(),
                TimeScaleInvalidation::Animation(animation) => {
                    self.set_animation(animation.clone())
                }
                // Source replay removes animation without adding another stop marker.
                TimeScaleInvalidation::StopAnimation => self.remove_animation(),
            }
        }
        self.global_level = self.global_level.max(incoming.global_level);
        for (&index, &request) in &incoming.panes {
            self.invalidate_pane(index, request);
        }
    }

    fn replace_time_scale(&mut self, command: TimeScaleInvalidation) {
        self.time_scale.clear();
        self.time_scale.push(command);
    }

    pub(crate) fn remove_pane(&mut self, removed: usize) {
        self.panes = std::mem::take(&mut self.panes)
            .into_iter()
            .filter_map(|(index, request)| {
                if index == removed {
                    None
                } else {
                    Some((if index > removed { index - 1 } else { index }, request))
                }
            })
            .collect();
    }

    fn remove_animation(&mut self) {
        if let Some(index) = self
            .time_scale
            .iter()
            .position(|command| matches!(command, TimeScaleInvalidation::Animation(_)))
        {
            self.time_scale.remove(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::time_data::Logical;
    use std::{cell::Cell, time::Duration};

    #[derive(Debug)]
    struct TestAnimation {
        start: Instant,
        dropped: Rc<Cell<bool>>,
    }

    impl TimeScaleAnimation for TestAnimation {
        fn sample(&self, now: Instant) -> (f64, bool) {
            let elapsed = now.saturating_duration_since(self.start).as_secs_f64();
            (elapsed.min(1.0) * 10.0, elapsed >= 1.0)
        }
    }

    impl Drop for TestAnimation {
        fn drop(&mut self) {
            self.dropped.set(true);
        }
    }

    fn animation(start: Instant) -> AnimationRequest {
        AnimationRequest::new(TestAnimation {
            start,
            dropped: Rc::new(Cell::new(false)),
        })
    }

    fn range() -> LogicalRange {
        LogicalRange {
            from: Logical::new(-1.5),
            to: Logical::new(7.25),
        }
    }

    #[test]
    fn all_level_pairs_merge_by_priority_globally_and_per_pane() {
        let levels = [
            InvalidationLevel::None,
            InvalidationLevel::Cursor,
            InvalidationLevel::Light,
            InvalidationLevel::Full,
        ];
        for (i, &before) in levels.iter().enumerate() {
            for (j, &after) in levels.iter().enumerate() {
                let expected = levels[i.max(j)];
                let mut global = InvalidateMask::new(before);
                global.merge(&InvalidateMask::new(after));
                assert_eq!(global.global_level(), expected);
                assert_eq!(global.pane_invalidation(100).level, expected);
                let mut mixed = InvalidateMask::new(before);
                mixed.invalidate_pane(
                    0,
                    PaneInvalidation {
                        level: after,
                        auto_scale: false,
                    },
                );
                assert_eq!(mixed.pane_invalidation(0).level, expected);
                assert_eq!(mixed.global_level(), before);
                for before_autoscale in [false, true] {
                    for after_autoscale in [false, true] {
                        let mut pending = InvalidateMask::new(InvalidationLevel::None);
                        pending.invalidate_pane(
                            2,
                            PaneInvalidation {
                                level: before,
                                auto_scale: before_autoscale,
                            },
                        );
                        let mut incoming = InvalidateMask::new(InvalidationLevel::None);
                        incoming.invalidate_pane(
                            2,
                            PaneInvalidation {
                                level: after,
                                auto_scale: after_autoscale,
                            },
                        );
                        pending.merge(&incoming);
                        assert_eq!(
                            pending.pane_invalidation(2),
                            PaneInvalidation {
                                level: expected,
                                auto_scale: before_autoscale || after_autoscale,
                            }
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn pane_requests_are_independent_and_inherit_global_level() {
        let mut mask = InvalidateMask::light();
        mask.invalidate_pane(
            0,
            PaneInvalidation {
                level: InvalidationLevel::None,
                auto_scale: true,
            },
        );
        mask.invalidate_pane(
            1,
            PaneInvalidation {
                level: InvalidationLevel::Full,
                auto_scale: false,
            },
        );
        mask.invalidate_pane(
            0,
            PaneInvalidation {
                level: InvalidationLevel::Cursor,
                auto_scale: false,
            },
        );
        assert_eq!(mask.global_level(), InvalidationLevel::Light);
        assert_eq!(
            mask.pane_invalidation(0),
            PaneInvalidation {
                level: InvalidationLevel::Light,
                auto_scale: true
            }
        );
        assert_eq!(
            mask.pane_invalidation(1),
            PaneInvalidation {
                level: InvalidationLevel::Full,
                auto_scale: false
            }
        );
        assert_eq!(
            mask.pane_invalidation(usize::MAX),
            PaneInvalidation {
                level: InvalidationLevel::Light,
                auto_scale: false
            }
        );
    }

    #[test]
    fn fit_range_and_reset_replace_the_entire_sequence_even_during_merge() {
        for replacement in 0..3 {
            let mut pending = InvalidateMask::new(InvalidationLevel::Cursor);
            pending.set_bar_spacing(4.0);
            pending.set_right_offset(-3.0);
            pending.set_animation(animation(Instant::now()));
            let mut incoming = InvalidateMask::new(InvalidationLevel::None);
            match replacement {
                0 => incoming.fit_content(),
                1 => incoming.apply_range(range()),
                _ => incoming.reset_time_scale(),
            }
            pending.merge(&incoming);
            assert_eq!(pending.global_level(), InvalidationLevel::Cursor);
            match pending.time_scale_invalidations() {
                [TimeScaleInvalidation::FitContent] => assert_eq!(replacement, 0),
                [TimeScaleInvalidation::ApplyRange(value)] => {
                    assert_eq!(replacement, 1);
                    assert_eq!(*value, range());
                }
                [TimeScaleInvalidation::Reset] => assert_eq!(replacement, 2),
                other => panic!("unexpected sequence: {other:?}"),
            }
        }
    }

    #[test]
    fn spacing_fit_offset_normalizes_without_raising_redraw_level() {
        let mut mask = InvalidateMask::new(InvalidationLevel::None);
        mask.set_bar_spacing(5.0);
        mask.fit_content();
        mask.set_right_offset(-2.0);
        assert!(matches!(
            mask.time_scale_invalidations(),
            [
                TimeScaleInvalidation::FitContent,
                TimeScaleInvalidation::StopAnimation,
                TimeScaleInvalidation::ApplyRightOffset(-2.0),
            ]
        ));
        assert_eq!(mask.global_level(), InvalidationLevel::None);
    }

    #[test]
    fn merge_keeps_spacing_and_offset_order_and_payloads() {
        let mut pending = InvalidateMask::light();
        pending.set_bar_spacing(4.0);
        let mut incoming = InvalidateMask::light();
        incoming.set_right_offset(-2.0);
        incoming.set_bar_spacing(6.0);
        pending.merge(&incoming);
        assert!(matches!(
            pending.time_scale_invalidations(),
            [
                TimeScaleInvalidation::StopAnimation,
                TimeScaleInvalidation::ApplyBarSpacing(4.0),
                TimeScaleInvalidation::StopAnimation,
                TimeScaleInvalidation::ApplyRightOffset(-2.0),
                TimeScaleInvalidation::StopAnimation,
                TimeScaleInvalidation::ApplyBarSpacing(6.0),
            ]
        ));
    }

    #[test]
    fn new_animation_replaces_old_animation_and_moves_to_end() {
        let now = Instant::now();
        let mut mask = InvalidateMask::light();
        mask.set_animation(animation(now));
        mask.set_bar_spacing(3.0);
        mask.set_animation(animation(now));
        mask.set_animation(animation(now + Duration::from_millis(100)));
        match mask.time_scale_invalidations() {
            [
                TimeScaleInvalidation::StopAnimation,
                TimeScaleInvalidation::ApplyBarSpacing(3.0),
                TimeScaleInvalidation::Animation(value),
            ] => {
                assert_eq!(value.sample(now + Duration::from_millis(500)), (4.0, false));
            }
            other => panic!("unexpected sequence: {other:?}"),
        }
    }

    #[test]
    fn direct_stop_retains_markers_but_merged_stop_only_removes_animation() {
        let mut pending = InvalidateMask::light();
        pending.set_animation(animation(Instant::now()));
        let mut incoming = InvalidateMask::light();
        incoming.stop_animation();
        incoming.stop_animation();
        assert!(matches!(
            incoming.time_scale_invalidations(),
            [
                TimeScaleInvalidation::StopAnimation,
                TimeScaleInvalidation::StopAnimation
            ]
        ));
        pending.merge(&incoming);
        assert!(pending.time_scale_invalidations().is_empty());
        pending.stop_animation();
        assert!(matches!(
            pending.time_scale_invalidations(),
            [TimeScaleInvalidation::StopAnimation]
        ));
    }

    #[test]
    fn incoming_stop_then_animation_preserves_the_new_animation() {
        let now = Instant::now();
        let mut pending = InvalidateMask::light();
        pending.set_animation(animation(now));
        let mut incoming = InvalidateMask::light();
        incoming.stop_animation();
        incoming.set_animation(animation(now + Duration::from_millis(100)));
        pending.merge(&incoming);
        match pending.time_scale_invalidations() {
            [TimeScaleInvalidation::Animation(value)] => {
                assert_eq!(value.sample(now + Duration::from_millis(500)), (4.0, false))
            }
            other => panic!("unexpected sequence: {other:?}"),
        }
    }

    #[test]
    fn merge_direction_changes_the_result() {
        let mut fit = InvalidateMask::light();
        fit.fit_content();
        let mut offset = InvalidateMask::light();
        offset.set_right_offset(2.0);
        let mut fit_then_offset = fit.clone();
        fit_then_offset.merge(&offset);
        offset.merge(&fit);
        assert!(matches!(
            fit_then_offset.time_scale_invalidations(),
            [
                TimeScaleInvalidation::FitContent,
                TimeScaleInvalidation::StopAnimation,
                TimeScaleInvalidation::ApplyRightOffset(2.0)
            ]
        ));
        assert!(matches!(
            offset.time_scale_invalidations(),
            [TimeScaleInvalidation::FitContent]
        ));
    }

    #[test]
    fn layout_replay_keeps_incoming_requests_and_animation_origin() {
        let now = Instant::now();
        let mut incoming = InvalidateMask::light();
        incoming.apply_range(range());
        incoming.set_animation(animation(now));
        incoming.invalidate_pane(
            0,
            PaneInvalidation {
                level: InvalidationLevel::None,
                auto_scale: true,
            },
        );
        for elapsed in [250, 750, 1000] {
            let mut layout_pass = InvalidateMask::full();
            layout_pass.merge(&incoming);
            assert_eq!(layout_pass.global_level(), InvalidationLevel::Full);
            assert!(layout_pass.pane_invalidation(0).auto_scale);
            match layout_pass.time_scale_invalidations() {
                [
                    TimeScaleInvalidation::ApplyRange(value),
                    TimeScaleInvalidation::Animation(request),
                ] => {
                    assert_eq!(*value, range());
                    assert_eq!(
                        request.sample(now + Duration::from_millis(elapsed)),
                        (elapsed as f64 / 100.0, elapsed == 1000)
                    );
                }
                other => panic!("unexpected sequence: {other:?}"),
            }
        }
        assert_eq!(incoming.global_level(), InvalidationLevel::Light);
        assert_eq!(incoming.time_scale_invalidations().len(), 2);
    }

    #[test]
    fn animation_lives_until_the_last_request_is_dropped() {
        let dropped = Rc::new(Cell::new(false));
        let mut incoming = InvalidateMask::light();
        incoming.set_animation(AnimationRequest::new(TestAnimation {
            start: Instant::now(),
            dropped: dropped.clone(),
        }));
        let mut pending = InvalidateMask::light();
        pending.merge(&incoming);
        drop(incoming);
        assert!(!dropped.get());
        pending.stop_animation();
        assert!(dropped.get());
    }

    #[test]
    fn every_command_setter_preserves_redraw_priority_and_pane_requests() {
        let setters: [fn(&mut InvalidateMask); 7] = [
            InvalidateMask::fit_content,
            |mask| mask.apply_range(range()),
            InvalidateMask::reset_time_scale,
            |mask| mask.set_bar_spacing(4.0),
            |mask| mask.set_right_offset(-2.0),
            |mask| mask.set_animation(animation(Instant::now())),
            InvalidateMask::stop_animation,
        ];
        for setter in setters {
            let mut mask = InvalidateMask::new(InvalidationLevel::None);
            let pane = PaneInvalidation {
                level: InvalidationLevel::Cursor,
                auto_scale: true,
            };
            mask.invalidate_pane(0, pane);
            setter(&mut mask);
            assert_eq!(mask.global_level(), InvalidationLevel::None);
            assert_eq!(mask.pane_invalidation(0), pane);
        }
    }

    #[test]
    fn replaying_spacing_twice_retains_both_operations() {
        let mut incoming = InvalidateMask::light();
        incoming.set_bar_spacing(5.0);
        let mut pending = InvalidateMask::light();
        pending.merge(&incoming);
        pending.merge(&incoming);
        assert!(matches!(
            pending.time_scale_invalidations(),
            [
                TimeScaleInvalidation::StopAnimation,
                TimeScaleInvalidation::ApplyBarSpacing(5.0),
                TimeScaleInvalidation::StopAnimation,
                TimeScaleInvalidation::ApplyBarSpacing(5.0),
            ]
        ));
        assert_eq!(incoming.time_scale_invalidations().len(), 2);
    }

    #[test]
    fn empty_masks_and_constructors_have_no_commands() {
        let mut mask = InvalidateMask::light();
        mask.merge(&InvalidateMask::new(InvalidationLevel::None));
        assert!(mask.time_scale_invalidations().is_empty());
        assert_eq!(mask.global_level(), InvalidationLevel::Light);
        assert_eq!(
            InvalidateMask::full().global_level(),
            InvalidationLevel::Full
        );
    }
}
