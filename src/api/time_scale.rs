//! Current scale queries and source-compatible viewport commands.
use super::{Chart, ChartBehavior, ChartError, HorizontalRange};
use crate::model::time_data::LogicalRange;

impl<B: ChartBehavior> Chart<B> {
    /// Resolves current lazy range state without consuming commands or queued events.
    /// Queued navigation settles during the next frame preparation.
    pub fn visible_logical_range(&mut self) -> Option<LogicalRange> {
        self.model
            .visible_logical_range()
            .map(|range| LogicalRange {
                from: range.left(),
                to: range.right(),
            })
    }
    /// Owned original horizontal items, clipped to the available data domain.
    pub fn visible_horizontal_range(&mut self) -> Option<HorizontalRange<B::Item>> {
        self.model.visible_horizontal_range()
    }
    /// Both endpoints select the next available item, clamped at domain edges.
    /// An empty domain is a no-op; reversed or unconvertible inputs are errors.
    pub fn set_visible_horizontal_range(
        &mut self,
        range: HorizontalRange<B::Item>,
    ) -> Result<(), ChartError> {
        let scale = self.model.time_scale();
        let behavior = scale.behavior();
        let from = behavior
            .to_internal(&range.from)
            .map_err(B::conversion_error)?;
        let to = behavior
            .to_internal(&range.to)
            .map_err(B::conversion_error)?;
        if behavior.key(&from) > behavior.key(&to) {
            return Err(ChartError::InvalidDataQuery);
        }
        if let Some(range) = scale.logical_range_for_time_range(HorizontalRange { from, to }) {
            self.set_logical_range(range);
        }
        Ok(())
    }
    pub fn scroll_position(&self) -> f64 {
        self.model.time_scale().right_offset()
    }
    /// Restore configured spacing and offset, cancelling active viewport navigation.
    pub fn reset_time_scale(&mut self) {
        self.cancel_viewport_interaction();
        self.model.reset_time_scale();
    }
    /// Animate to the configured right offset over the source's 400 ms duration.
    pub fn scroll_to_real_time(&mut self) {
        self.scroll_to_offset_animated(
            self.model.time_scale().options().right_offset,
            std::time::Duration::from_millis(400),
        );
    }
}
