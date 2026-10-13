use super::{Chart, ChartError, MarkerCollectionId, SeriesMarker, SeriesMarkersOptions};
use crate::model::{
    data_layer::SeriesId,
    horz_scale_behavior_time::{time_utils::convert_time, types::Time},
    series_markers::{MarkerCollection, MarkerPosition},
};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_COLLECTION: AtomicU64 = AtomicU64::new(1);

fn validate(markers: &[SeriesMarker<Time>]) -> Result<(), ChartError> {
    let mut previous = None;
    let mut parser = crate::model::colors::ColorParser::default();
    for (index, marker) in markers.iter().enumerate() {
        let time = convert_time(&marker.time)
            .map_err(ChartError::Time)?
            .timestamp
            .seconds();
        let price = match marker.position {
            MarkerPosition::AtPriceTop(p)
            | MarkerPosition::AtPriceBottom(p)
            | MarkerPosition::AtPriceMiddle(p) => Some(p),
            _ => None,
        };
        if previous.is_some_and(|t| time < t)
            || !marker.size.is_finite()
            || marker.size.abs() > f64::from(f32::MAX) / 64.
            || price.is_some_and(|p| !p.is_finite() || p.abs() > 90_071_992_547_409.9)
            || parser.parse_color(&marker.color).is_err()
        {
            return Err(ChartError::InvalidMarker { index });
        }
        previous = Some(time);
    }
    Ok(())
}
impl Chart {
    pub fn add_series_markers(
        &mut self,
        series: SeriesId,
        markers: Vec<SeriesMarker<Time>>,
        options: SeriesMarkersOptions,
    ) -> Result<MarkerCollectionId, ChartError> {
        if !self.kinds.contains_key(&series) {
            return Err(ChartError::UnknownSeries(series));
        }
        validate(&markers)?;
        let id = NEXT_COLLECTION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map(MarkerCollectionId)
            .map_err(|_| ChartError::IdExhausted)?;
        let order = self.next_extension_order();
        self.marker_orders.insert(id, order);
        self.model
            .insert_marker_collection(series, MarkerCollection::new(id, markers, options));
        Ok(id)
    }
    /// Replace a collection. Equal times are permitted; decreasing times are rejected.
    pub fn set_series_markers(
        &mut self,
        id: MarkerCollectionId,
        markers: Vec<SeriesMarker<Time>>,
    ) -> Result<(), ChartError> {
        self.series_marker_options(id)?;
        validate(&markers)?;
        self.model
            .edit_marker_collection(id, |collection| collection.markers = markers);
        Ok(())
    }
    pub fn series_markers(
        &self,
        id: MarkerCollectionId,
    ) -> Result<Vec<SeriesMarker<Time>>, ChartError> {
        self.model
            .marker_collection(id)
            .map(|c| c.markers)
            .ok_or(ChartError::UnknownMarkerCollection(id))
    }
    pub fn series_marker_options(
        &self,
        id: MarkerCollectionId,
    ) -> Result<SeriesMarkersOptions, ChartError> {
        for series in self.kinds.keys() {
            if let Some(series) = self.model.series(*series)
                && let Some(collection) = series.marker_collections.iter().find(|c| c.id == id)
            {
                return Ok(collection.options);
            }
        }
        Err(ChartError::UnknownMarkerCollection(id))
    }
    pub fn replace_series_marker_options(
        &mut self,
        id: MarkerCollectionId,
        options: SeriesMarkersOptions,
    ) -> Result<(), ChartError> {
        self.series_marker_options(id)?;
        self.model
            .edit_marker_collection(id, |c| c.options = options);
        Ok(())
    }
    pub fn remove_series_markers(&mut self, id: MarkerCollectionId) -> Result<(), ChartError> {
        self.series_marker_options(id)?;
        self.model.remove_marker_collection(id);
        self.marker_orders.remove(&id);
        Ok(())
    }
}
