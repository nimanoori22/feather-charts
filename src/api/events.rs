//! Owned application notifications. Delivery never calls host code during mutation.
use super::{Chart, ChartBehavior, ChartEvent};
use crate::model::{data_layer::SeriesId, time_data::ValueRange};

pub type HorizontalRange<T> = ValueRange<T>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataChangeScope {
    Full,
    Update,
}

/// New event families are opt-in. Existing pointer/edit events remain enabled.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EventInterests {
    pub data_changed: bool,
    pub double_click: bool,
    pub visible_logical_range: bool,
    pub visible_horizontal_range: bool,
    pub horizontal_axis_size: bool,
}
impl EventInterests {
    pub const ALL: Self = Self {
        data_changed: true,
        double_click: true,
        visible_logical_range: true,
        visible_horizontal_range: true,
        horizontal_axis_size: true,
    };
}

impl<B: ChartBehavior> Chart<B> {
    /// Changes future observation only; already queued events remain deliverable.
    pub fn set_event_interests(&mut self, interests: EventInterests) {
        if self.event_interests == interests {
            return;
        }
        self.event_interests = interests;
        self.range_observer = None;
        if interests.visible_logical_range || interests.visible_horizontal_range {
            let queue = std::rc::Rc::downgrade(&self.pending_events);
            self.range_observer = Some(self.model.time_scale().observe_ranges(move |change| {
                use crate::model::time_scale::VisibleRangeChange;
                let event = match change {
                    VisibleRangeChange::Horizontal(range) if interests.visible_horizontal_range => {
                        Some(ChartEvent::VisibleHorizontalRangeChanged(range.clone()))
                    }
                    VisibleRangeChange::Logical(range) if interests.visible_logical_range => {
                        Some(ChartEvent::VisibleLogicalRangeChanged(*range))
                    }
                    _ => None,
                };
                if let (Some(queue), Some(event)) = (queue.upgrade(), event) {
                    queue.borrow_mut().push(event);
                }
            }));
        }
    }
    pub fn event_interests(&self) -> EventInterests {
        self.event_interests
    }
    /// Consumes the same stream returned by `update`. Drain after each batch when
    /// interests are enabled; the queue is lossless and has no implicit size cap.
    pub fn take_events(&mut self) -> Vec<ChartEvent<B::Item>> {
        self.pending_edit_cancellation = false;
        std::mem::take(&mut *self.pending_events.borrow_mut())
    }
    pub(super) fn push_event(&mut self, event: ChartEvent<B::Item>) {
        self.pending_events.borrow_mut().push(event);
    }
    pub(super) fn data_changed(&mut self, series: SeriesId, scope: DataChangeScope) {
        if self.event_interests.data_changed {
            self.push_event(ChartEvent::DataChanged { series, scope });
        }
    }
}
