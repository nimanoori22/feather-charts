//! Optional synchronous warmup shares the existing native animation scheduler.
use super::Chart;
use crate::{
    model::time_scale_options::PrecomputeConflationPriority, ui::multi_pane_frame::SeriesPaneView,
};

impl<B: super::ChartBehavior> Chart<B> {
    fn warmup_levels(&self) -> Vec<usize> {
        let time = self.model.time_scale();
        if !time.options().enable_conflation || !time.options().precompute_conflation_on_init {
            return vec![];
        }
        time.possible_conflation_factors()
    }
    pub(super) fn has_pending_conflation(&self) -> bool {
        let levels = self.warmup_levels();
        self.views.iter().flatten().any(|(id, view)| match view {
            SeriesPaneView::Custom(view) => view.adapter.needs_precompute(&levels),
            _ => self
                .model
                .series(*id)
                .is_some_and(|series| series.needs_precompute(&levels)),
        })
    }
    pub(super) fn precompute_conflation_step(&mut self) {
        let levels = self.warmup_levels();
        if levels.is_empty() {
            return;
        }
        let mut budget = match self
            .model
            .time_scale()
            .options()
            .precompute_conflation_priority
        {
            PrecomputeConflationPriority::Background => 128,
            PrecomputeConflationPriority::UserVisible => 512,
            PrecomputeConflationPriority::UserBlocking => 2048,
        };
        for (id, view) in self.views.iter_mut().flatten() {
            if budget == 0 {
                break;
            }
            budget -= match view {
                SeriesPaneView::Custom(view) => view.adapter.precompute(&levels, budget),
                _ => self
                    .model
                    .series(*id)
                    .filter(|s| s.needs_precompute(&levels))
                    .map_or(0, |s| s.precompute_step(&levels, budget)),
            };
        }
    }
}
