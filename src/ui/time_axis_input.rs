//! Synchronous horizontal-axis recognition; coordinates remain in logical pixels.
pub use super::axis_pointer::AxisGestureOptions;
use super::{
    axis_pointer::{Action, AxisPointer},
    chart_layout::ChartLayout,
};
use iced::{Point, widget::canvas::Event};
use std::time::Instant;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeAxisAction {
    Start { x: f64 },
    Move { x: f64 },
    End,
    Reset,
}
#[derive(Default)]
pub struct TimeAxisInput {
    pub options: AxisGestureOptions,
    pointer: AxisPointer,
    owner: Option<(ChartLayout, usize)>,
    scaling: bool,
}
impl TimeAxisInput {
    pub fn is_dragging(&self) -> bool {
        self.pointer.active()
    }
    pub fn is_scaling(&self) -> bool {
        self.scaling
    }
    pub fn suppresses_mouse(&self, now: Instant) -> bool {
        self.pointer.suppresses_mouse(now)
    }
    pub fn cancel(&mut self) -> Option<TimeAxisAction> {
        let active = self.pointer.cancel();
        self.owner = None;
        self.scaling = false;
        active.then_some(TimeAxisAction::End)
    }
    pub fn hit_test(point: Point, layouts: &[ChartLayout]) -> bool {
        layouts.iter().any(|l| {
            let r = l.time_axis;
            f64::from(point.x) >= r.origin.x
                && f64::from(point.x) < r.origin.x + r.size.width
                && f64::from(point.y) >= r.origin.y
                && f64::from(point.y) < r.origin.y + r.size.height
        })
    }
    pub fn update(
        &mut self,
        event: &Event,
        point: Option<Point>,
        layouts: &[ChartLayout],
        enabled: bool,
        now: Instant,
    ) -> Option<TimeAxisAction> {
        let AxisGestureOptions { drag, reset, .. } = self.options;
        let layout = layouts
            .iter()
            .find(|l| l.time_axis.size.height > 0. && l.time_axis.size.width > 0.);
        if self.owner.is_some_and(|(old, count)| {
            layout.is_none_or(|l| {
                l.available != old.available
                    || l.time_axis.origin.y != old.time_axis.origin.y
                    || l.time_axis.size.height != old.time_axis.size.height
            }) || layouts.len() != count
        }) || (!enabled || (!drag && !reset)) && self.is_dragging()
        {
            return self.cancel();
        }
        let eligible =
            enabled && (drag || reset) && point.is_some_and(|p| Self::hit_test(p, layouts));
        let action = self
            .pointer
            .update(event, point, eligible, self.options, true, now)?;
        match action {
            Action::Down(p) => {
                let l = *layout?;
                self.owner = Some((l, layouts.len()));
                self.scaling = drag;
                drag.then_some(TimeAxisAction::Start {
                    x: f64::from(p.x) - l.time_axis.origin.x,
                })
            }
            Action::Move(p) => drag.then(|| TimeAxisAction::Move {
                x: f64::from(p.x) - self.owner.unwrap().0.time_axis.origin.x,
            }),
            Action::Up { reset } => {
                self.owner = None;
                self.scaling = false;
                Some(if reset {
                    TimeAxisAction::Reset
                } else {
                    TimeAxisAction::End
                })
            }
            Action::Cancel => {
                self.owner = None;
                self.scaling = false;
                Some(TimeAxisAction::End)
            }
        }
    }
}
