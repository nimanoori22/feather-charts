use super::*;
use crate::model::ipane_primitive::PrimitiveAutoscaleProvider;
impl<B, M> ChartModel<B, M>
where
    B: HorzScaleBehavior,
    B::Item: 'static,
    B::InternalItem: 'static,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone + 'static,
{
    pub(crate) fn primitive_scale_auto(&mut self, series: SeriesId, enabled: bool) -> Option<bool> {
        let pane = self.pane_for_series(series)?;
        let position = self.panes[pane].source_position(series)?.clone();
        let scale = self.panes[pane].price_scale_mut_existing(&position)?;
        let old = scale.is_auto_scale();
        scale.set_auto_scale(enabled);
        self.recalculate_requested = true;
        self.full_update();
        self.flush_effects();
        Some(old)
    }
    pub(crate) fn ordered_pane_sources(&mut self, pane: usize) -> Vec<SeriesId> {
        self.panes[pane].ordered_source_ids().to_vec()
    }
    pub(crate) fn set_primitive_autoscale(
        &mut self,
        owner: SeriesId,
        id: u64,
        provider: Option<PrimitiveAutoscaleProvider>,
    ) {
        if let Some(handle) = self.series.get(&owner).map(|s| &s.projection) {
            let mut s = handle.borrow_mut();
            if let Some(index) = s
                .primitive_autoscale_mut()
                .iter()
                .position(|(key, _)| *key == id)
            {
                if let Some(provider) = provider {
                    s.primitive_autoscale_mut()[index].1 = provider;
                } else {
                    s.primitive_autoscale_mut().remove(index);
                }
            } else if let Some(provider) = provider {
                s.primitive_autoscale_mut().push((id, provider));
            }
        }
        self.recalculate_requested = true;
        self.full_update();
        self.flush_effects();
    }
}
