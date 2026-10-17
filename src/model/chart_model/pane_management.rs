use super::*;

impl<B, M> ChartModel<B, M>
where
    B: HorzScaleBehavior,
    B::InternalItem: 'static,
    B::Item: 'static,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone + 'static,
{
    pub fn series_order(&self, id: SeriesId) -> Result<usize, ChartModelError> {
        let pane = self
            .pane_for_series(id)
            .ok_or(ChartModelError::UnknownSeries(id))?;
        self.panes[pane]
            .series_order(id)
            .ok_or(ChartModelError::UnknownSeries(id))
    }

    pub fn set_series_order(&mut self, id: SeriesId, order: usize) -> Result<(), ChartModelError> {
        let pane = self
            .pane_for_series(id)
            .ok_or(ChartModelError::UnknownSeries(id))?;
        let effects = self.panes[pane]
            .set_series_order(id, order)
            .map_err(ChartModelError::Pane)?;
        if effects.is_empty() {
            return Ok(());
        }
        self.consume_pane_effects(effects);
        self.flush_effects();
        Ok(())
    }

    pub fn move_pane(&mut self, from: usize, to: usize) -> Result<(), ChartModelError> {
        self.validate_pane(from)?;
        self.validate_pane(to)?;
        if from == to {
            return Ok(());
        }
        let mut order: Vec<_> = (0..self.panes.len()).collect();
        order.remove(from);
        order.insert(to, from);
        let pane = self.panes.remove(from);
        self.panes.insert(to, pane);
        self.reordered_panes(&order);
        Ok(())
    }

    pub fn swap_panes(&mut self, first: usize, second: usize) -> Result<(), ChartModelError> {
        self.validate_pane(first)?;
        self.validate_pane(second)?;
        if first == second {
            return Ok(());
        }
        let mut order: Vec<_> = (0..self.panes.len()).collect();
        order.swap(first, second);
        self.panes.swap(first, second);
        self.reordered_panes(&order);
        Ok(())
    }

    pub(crate) fn validate_pane(&self, pane: usize) -> Result<(), ChartModelError> {
        if pane < self.panes.len() {
            Ok(())
        } else {
            Err(ChartModelError::InvalidPane(pane))
        }
    }

    pub fn move_series_to_pane(
        &mut self,
        id: SeriesId,
        target: usize,
    ) -> Result<(), ChartModelError> {
        let from = self
            .pane_for_series(id)
            .ok_or(ChartModelError::UnknownSeries(id))?;
        if from == target {
            return Ok(());
        }
        let target = if target >= self.panes.len() {
            self.add_pane()
        } else {
            target
        };
        let effects = if from < target {
            let (before, after) = self.panes.split_at_mut(target);
            before[from].transfer_source_to(id, &mut after[0])
        } else {
            let (before, after) = self.panes.split_at_mut(from);
            after[0].transfer_source_to(id, &mut before[target])
        }
        .map_err(ChartModelError::Pane)?;
        self.series.get_mut(&id).unwrap().pane = target;
        self.consume_pane_effects(effects);
        if self.panes.len() > 1
            && self.panes[from].attached_source_ids().is_empty()
            && !self.panes[from].preserve_empty_pane()
        {
            self.remove_empty_pane(from);
        }
        self.crosshair.origin = None;
        self.crosshair.position = None;
        self.recalculate_requested = true;
        self.full_update();
        self.flush_effects();
        Ok(())
    }

    pub(crate) fn remove_empty_pane(&mut self, pane: usize) {
        assert!(self.panes[pane].attached_source_ids().is_empty());
        assert!(self.panes.len() > 1);
        self.panes.remove(pane);
        for (index, state) in self.panes.iter_mut().enumerate() {
            state.set_index(index);
        }
        for entry in self.series.values_mut() {
            if entry.pane > pane {
                entry.pane -= 1;
            }
        }
        if let Some(pending) = &mut self.pending {
            pending.remove_pane(pane);
        }
        self.crosshair.origin = None;
        self.crosshair.position = None;
        self.recalculate_requested = true;
        self.full_update();
    }

    fn reordered_panes(&mut self, order: &[usize]) {
        let mut mapping = vec![0; order.len()];
        for (new, &old) in order.iter().enumerate() {
            mapping[old] = new;
            self.panes[new].set_index(new);
        }
        for entry in self.series.values_mut() {
            entry.pane = mapping[entry.pane];
        }
        if let Some(pending) = &mut self.pending {
            pending.reorder_panes(&mapping);
        }
        self.crosshair.origin = None;
        self.crosshair.position = None;
        self.recalculate_requested = true;
        self.full_update();
        self.flush_effects();
    }
}
