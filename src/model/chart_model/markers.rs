use super::*;
use crate::model::series_markers::{MarkerCollection, MarkerCollectionId};
impl<B, M> ChartModel<B, M>
where
    B: HorzScaleBehavior,
    B::Item: 'static,
    B::InternalItem: 'static,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone + 'static,
{
    pub(crate) fn insert_marker_collection(
        &mut self,
        series: SeriesId,
        collection: MarkerCollection<B::Item>,
    ) {
        self.series[&series]
            .built_in()
            .unwrap()
            .borrow_mut()
            .marker_collections
            .push(collection);
        self.markers_changed();
    }
    pub(crate) fn marker_collection(
        &self,
        id: MarkerCollectionId,
    ) -> Option<MarkerCollection<B::Item>> {
        self.series
            .values()
            .filter_map(|e| e.built_in())
            .find_map(|s| {
                s.borrow()
                    .marker_collections
                    .iter()
                    .find(|c| c.id == id)
                    .cloned()
            })
    }
    pub(crate) fn edit_marker_collection(
        &mut self,
        id: MarkerCollectionId,
        edit: impl FnOnce(&mut MarkerCollection<B::Item>),
    ) {
        for entry in self.series.values() {
            if let Some(handle) = entry.built_in() {
                let mut series = handle.borrow_mut();
                if let Some(c) = series.marker_collections.iter_mut().find(|c| c.id == id) {
                    edit(c);
                    drop(series);
                    self.markers_changed();
                    return;
                }
            }
        }
    }
    pub(crate) fn remove_marker_collection(&mut self, id: MarkerCollectionId) {
        for entry in self.series.values() {
            if let Some(handle) = entry.built_in() {
                handle
                    .borrow_mut()
                    .marker_collections
                    .retain(|c| c.id != id);
            }
        }
        self.markers_changed();
    }
    fn markers_changed(&mut self) {
        self.recalculate_requested = true;
        self.light_update();
        self.flush_effects();
    }
}
