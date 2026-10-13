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
            .projection
            .borrow_mut()
            .marker_collections_mut()
            .push(collection);
        self.markers_changed();
    }
    pub(crate) fn marker_collection(
        &self,
        id: MarkerCollectionId,
    ) -> Option<MarkerCollection<B::Item>> {
        self.series.values().map(|e| &e.projection).find_map(|s| {
            s.borrow()
                .marker_collections()
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
            let handle = &entry.projection;
            let mut series = handle.borrow_mut();
            if let Some(c) = series
                .marker_collections_mut()
                .iter_mut()
                .find(|c| c.id == id)
            {
                edit(c);
                c.invalidate();
                drop(series);
                self.markers_changed();
                return;
            }
        }
    }
    pub(crate) fn remove_marker_collection(&mut self, id: MarkerCollectionId) {
        for entry in self.series.values() {
            let handle = &entry.projection;
            handle
                .borrow_mut()
                .marker_collections_mut()
                .retain(|c| c.id != id);
        }
        self.markers_changed();
    }
    fn markers_changed(&mut self) {
        self.recalculate_requested = true;
        self.light_update();
        self.flush_effects();
    }
    pub(crate) fn prepare_series_markers(
        &mut self,
        id: SeriesId,
    ) -> Vec<crate::renderers::series_markers_renderer::PreparedMarkerCollection> {
        let Some(entry) = self.series.get(&id) else {
            return vec![];
        };
        let handle = &entry.projection;
        let series = handle.borrow();
        let Some(scale) = self.panes[entry.pane].price_scale_for_source(id) else {
            return vec![];
        };
        crate::model::series::markers_pane_view::prepare(
            &*series,
            &mut self.time_scale,
            scale,
            self.options.layout.font_size,
            &self.options.layout.font_family,
        )
    }
}
