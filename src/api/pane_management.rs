use super::*;

impl<B: ChartBehavior, M: Clone + 'static> Chart<B, M> {
    /// Request a pane height in logical pixels using the current settled layout.
    /// The request becomes stretch allocation, so later window resizing scales it.
    /// Other panes share the difference in pane order, with source minimums of 30px.
    /// A single pane or a layout too short for positive allocations is unchanged.
    /// Finite negative heights clamp to the minimum; non-finite values are errors.
    pub fn set_pane_height(&mut self, pane: usize, height: f64) -> Result<(), ChartError> {
        use crate::model::{chart_model::ChartModelError, pane::MIN_PANE_HEIGHT};
        self.model.validate_pane(pane)?;
        if !height.is_finite() {
            return Err(ChartModelError::InvalidDimensions.into());
        }
        let panes = self.model.panes();
        if panes.len() < 2 {
            return Ok(());
        }
        let total_height: f64 = panes.iter().map(|p| p.height()).sum();
        let total_stretch: f64 = panes.iter().map(|p| p.stretch_factor()).sum();
        let maximum = total_height - MIN_PANE_HEIGHT * (panes.len() - 1) as f64;
        if maximum <= 0. || total_height == 0. {
            return Ok(());
        }
        let height = height.max(MIN_PANE_HEIGHT).min(maximum);
        let pixel_stretch = total_stretch / total_height;
        let mut difference = height - panes[pane].height();
        let mut remaining = panes.len() - 1;
        let factors: Vec<_> = panes
            .iter()
            .enumerate()
            .map(|(index, state)| {
                let height = if index == pane {
                    height
                } else {
                    let next = (state.height() - difference / remaining as f64)
                        .max(MIN_PANE_HEIGHT)
                        .min(maximum);
                    difference -= state.height() - next;
                    remaining -= 1;
                    next
                };
                height * pixel_stretch
            })
            .collect();
        if factors.iter().any(|f| !f.is_finite() || *f <= 0.)
            || !factors.iter().sum::<f64>().is_finite()
        {
            return Err(ChartModelError::InvalidDimensions.into());
        }
        if panes
            .iter()
            .zip(&factors)
            .all(|(p, f)| p.stretch_factor() == *f)
        {
            return Ok(());
        }
        self.cancel_interaction();
        for (index, factor) in factors.into_iter().enumerate() {
            self.model.set_pane_stretch_factor(index, factor)?;
        }
        Ok(())
    }

    /// Current zero-based position within the pane, including private support sources.
    /// Removal and pane transfers can change this position.
    pub fn series_order(
        &self,
        id: crate::model::data_layer::SeriesId,
    ) -> Result<usize, ChartError> {
        self.kind(id)?;
        Ok(self.model.series_order(id)?)
    }

    /// Change drawing order within the current pane. Oversized indices clamp to
    /// the last position. Existing owners and data are retained; active gestures
    /// are cancelled only when the order actually changes.
    pub fn set_series_order(
        &mut self,
        id: crate::model::data_layer::SeriesId,
        order: usize,
    ) -> Result<(), ChartError> {
        let current = self.series_order(id)?;
        let pane = self.model.pane_for_series(id).expect("validated owner");
        let ids = self.model.panes()[pane].attached_source_ids();
        let target = order.min(ids.len() - 1);
        if current == target {
            return Ok(());
        }
        self.cancel_interaction();
        self.model.set_series_order(id, target)?;
        let source = self.views[pane]
            .iter()
            .position(|(s, _)| *s == id)
            .expect("registered view");
        let ordered = self.model.panes()[pane].attached_source_ids();
        let target = ordered[..target]
            .iter()
            .filter(|s| self.kinds.contains_key(s))
            .count();
        let view = self.views[pane].remove(source);
        self.views[pane].insert(target, view);
        self.last_hit = None;
        self.last_primitive = None;
        Ok(())
    }

    /// Remove a pane and its live series. The final pane is retained.
    /// Detached custom handles retain readable data; their views are released.
    pub fn remove_pane(&mut self, pane: usize) -> Result<(), ChartError> {
        if self.model.panes().len() == 1 {
            return Ok(());
        }
        self.model.validate_pane(pane)?;
        let ids = self.model.panes()[pane].attached_source_ids();
        for id in &ids {
            self.kind(*id)?;
        }
        self.cancel_interaction();
        // Keep the selected index stable while ordinary series cleanup runs.
        self.model.set_preserve_empty_pane(pane, true)?;
        for id in ids {
            self.remove_series(id)?;
        }
        self.model.remove_empty_pane(pane);
        self.removed_pane_views(pane);
        self.last_hit = None;
        self.last_primitive = None;
        Ok(())
    }
    /// Move an existing owner. Targets past the end create one appended pane.
    pub fn move_series_to_pane(
        &mut self,
        series: crate::model::data_layer::SeriesId,
        target: usize,
    ) -> Result<(), ChartError> {
        self.kind(series)?;
        let from = self
            .model
            .pane_for_series(series)
            .ok_or(ChartError::UnknownSeries(series))?;
        if from == target {
            return Ok(());
        }
        self.cancel_interaction();
        let target = if target >= self.views.len() {
            self.add_pane()
        } else {
            target
        };
        self.model.move_series_to_pane(series, target)?;
        let index = self.views[from]
            .iter()
            .position(|(id, _)| *id == series)
            .expect("registered view");
        let view = self.views[from].remove(index);
        if self.views.len() > self.model.panes().len() {
            self.removed_pane_views(from);
        }
        let target = self
            .model
            .pane_for_series(series)
            .expect("transferred owner");
        self.views[target].push(view);
        self.last_hit = None;
        self.last_primitive = None;
        Ok(())
    }

    pub(super) fn removed_pane_views(&mut self, pane: usize) {
        self.views.remove(pane);
        let removed = self.primitive_panes.remove(pane);
        self.primitives
            .retain(|a| a.owner != PrimitiveOwner::Pane(removed));
    }
    /// Move a pane to a current position. Pane indices may change after this call.
    pub fn move_pane(&mut self, from: usize, to: usize) -> Result<(), ChartError> {
        self.model.validate_pane(from)?;
        self.model.validate_pane(to)?;
        if from == to {
            return Ok(());
        }
        self.cancel_interaction();
        self.model.move_pane(from, to)?;
        let views = self.views.remove(from);
        self.views.insert(to, views);
        let id = self.primitive_panes.remove(from);
        self.primitive_panes.insert(to, id);
        self.last_hit = None;
        self.last_primitive = None;
        Ok(())
    }

    /// Swap two current pane positions, retaining their contents and attachments.
    pub fn swap_panes(&mut self, first: usize, second: usize) -> Result<(), ChartError> {
        self.model.validate_pane(first)?;
        self.model.validate_pane(second)?;
        if first == second {
            return Ok(());
        }
        self.cancel_interaction();
        self.model.swap_panes(first, second)?;
        self.views.swap(first, second);
        self.primitive_panes.swap(first, second);
        self.last_hit = None;
        self.last_primitive = None;
        Ok(())
    }
}
