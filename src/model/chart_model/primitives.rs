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
    pub(crate) fn ordered_pane_sources(&mut self, pane: usize) -> Vec<SeriesId> {
        self.panes[pane].ordered_source_ids().to_vec()
    }
    pub(crate) fn set_primitive_autoscale(
        &mut self,
        owner: SeriesId,
        id: u64,
        provider: Option<PrimitiveAutoscaleProvider>,
    ) {
        if let Some(handle) = self.series.get(&owner).and_then(|s| s.built_in()) {
            let mut s = handle.borrow_mut();
            if let Some(index) = s.primitive_autoscale.iter().position(|(key, _)| *key == id) {
                if let Some(provider) = provider {
                    s.primitive_autoscale[index].1 = provider;
                } else {
                    s.primitive_autoscale.remove(index);
                }
            } else if let Some(provider) = provider {
                s.primitive_autoscale.push((id, provider));
            }
        }
        self.recalculate_requested = true;
        self.full_update();
        self.flush_effects();
    }
}
