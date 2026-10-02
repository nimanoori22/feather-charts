//! Owner boundary for synchronous horizontal-scale model effects.

use std::{collections::BTreeSet, time::Duration};

/// Layout information read by the horizontal scale without coupling it to a
/// chart-model implementation or rendering backend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeScaleLayoutContext {
    pub font_size: f64,
}

/// A caller-driven scroll animation request.
///
/// The chart engine does not own a clock. An Iced adapter can sample this
/// request and feed interpolated offsets back into the time scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollAnimation {
    pub start_offset: f64,
    pub target_offset: f64,
    pub duration: Duration,
}

/// Work requested by a horizontal-scale mutation after its exclusive borrow
/// has ended.
#[derive(Clone, Debug, PartialEq)]
pub enum TimeScaleEffect {
    RecalculateAllPanes,
    LightUpdate,
    StartScrollAnimation(ScrollAnimation),
    OptionsApplied,
}

/// The minimal chart-model boundary consumed by a horizontal scale.
///
/// `TimeScale` must not own this trait object: the future chart model owns its
/// scale, and effects are dispatched only after that scale mutation completes.
pub trait TimeScaleHost {
    fn layout_context(&self) -> TimeScaleLayoutContext;
    fn fulfilled_time_indices(&self) -> BTreeSet<usize>;
    fn apply_time_scale_effects(&mut self, effects: Vec<TimeScaleEffect>);
}

/// Effects common to offset, spacing, and visible-range mutations.
pub fn viewport_changed_effects() -> Vec<TimeScaleEffect> {
    vec![
        TimeScaleEffect::RecalculateAllPanes,
        TimeScaleEffect::LightUpdate,
    ]
}

/// Obtains the data-bearing time indexes only when whitespace filtering is
/// enabled, avoiding an unnecessary model snapshot on the normal path.
pub fn indices_with_data(
    host: &impl TimeScaleHost,
    ignore_whitespace_indices: bool,
) -> BTreeSet<usize> {
    if ignore_whitespace_indices {
        host.fulfilled_time_indices()
    } else {
        BTreeSet::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ScrollAnimation, TimeScaleEffect, TimeScaleHost, TimeScaleLayoutContext, indices_with_data,
        viewport_changed_effects,
    };
    use std::{collections::BTreeSet, time::Duration};

    #[derive(Default)]
    struct FakeHost {
        effects: Vec<TimeScaleEffect>,
    }

    impl TimeScaleHost for FakeHost {
        fn layout_context(&self) -> TimeScaleLayoutContext {
            TimeScaleLayoutContext { font_size: 13.0 }
        }

        fn fulfilled_time_indices(&self) -> BTreeSet<usize> {
            BTreeSet::from([1, 4, 8])
        }

        fn apply_time_scale_effects(&mut self, effects: Vec<TimeScaleEffect>) {
            self.effects.extend(effects);
        }
    }

    #[test]
    fn viewport_mutations_request_recalculation_then_redraw() {
        let mut host = FakeHost::default();
        host.apply_time_scale_effects(viewport_changed_effects());

        assert_eq!(
            host.effects,
            vec![
                TimeScaleEffect::RecalculateAllPanes,
                TimeScaleEffect::LightUpdate,
            ]
        );
    }

    #[test]
    fn scroll_animation_is_pure_data_for_the_ui_to_schedule() {
        let animation = ScrollAnimation {
            start_offset: -2.0,
            target_offset: 3.5,
            duration: Duration::from_millis(400),
        };
        let mut host = FakeHost::default();
        host.apply_time_scale_effects(vec![TimeScaleEffect::StartScrollAnimation(animation)]);

        assert_eq!(
            host.effects,
            vec![TimeScaleEffect::StartScrollAnimation(animation)]
        );
        assert_eq!(host.layout_context().font_size, 13.0);
    }

    #[test]
    fn only_snapshots_fulfilled_indexes_when_whitespace_filtering_is_enabled() {
        let host = FakeHost::default();

        assert!(indices_with_data(&host, false).is_empty());
        assert_eq!(indices_with_data(&host, true), BTreeSet::from([1, 4, 8]));
    }
}
