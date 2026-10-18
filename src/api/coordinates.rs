//! Public conversions use current engine geometry; prepare after layout/data changes.
use super::{Chart, ChartBehavior, ChartError};
use crate::model::{coordinate::Coordinate, data_layer::SeriesId, time_data::TimePointIndex};
impl<B: ChartBehavior> Chart<B> {
    /// Pane-local coordinate using this series' scale and visible first value.
    pub fn price_to_coordinate(
        &mut self,
        id: SeriesId,
        price: f64,
    ) -> Result<Option<f64>, ChartError> {
        self.convert_price_coordinate(id, price, false)
    }
    pub fn coordinate_to_price(&mut self, id: SeriesId, y: f64) -> Result<Option<f64>, ChartError> {
        self.convert_price_coordinate(id, y, true)
    }
    fn convert_price_coordinate(
        &mut self,
        id: SeriesId,
        value: f64,
        inverse: bool,
    ) -> Result<Option<f64>, ChartError> {
        let pane = self.series_pane(id)?;
        if !value.is_finite() {
            return Err(ChartError::InvalidDataQuery);
        }
        let visible = self.model.visible_strict_range();
        let series = self
            .model
            .projection(id)
            .ok_or(ChartError::UnknownSeries(id))?;
        let scale = self.model.panes()[pane]
            .price_scale_for_source(id)
            .ok_or(ChartError::UnknownSeries(id))?;
        let Some(first) = series
            .first_value(visible.as_ref())
            .filter(|_| !scale.is_empty())
        else {
            return Ok(None);
        };
        let result = if inverse {
            scale.coordinate_to_price(Coordinate::new(value), first.value)
        } else {
            scale.price_to_coordinate(value, first.value).value()
        };
        Ok(result.is_finite().then_some(result))
    }
    /// Source index conversion: fractional logical indices map to zero.
    pub fn logical_to_coordinate(&self, logical: f64) -> Option<f64> {
        let scale = self.model.time_scale();
        if !logical.is_finite() || scale.is_empty() {
            return None;
        }
        let x = scale
            .index_to_coordinate(TimePointIndex::new(logical))
            .value();
        x.is_finite().then_some(x)
    }
    /// Source conversion rounds upward to an integer index (no whitespace search).
    pub fn coordinate_to_logical(&self, x: f64) -> Option<f64> {
        let scale = self.model.time_scale();
        if !x.is_finite() || scale.is_empty() {
            return None;
        }
        let index = scale.coordinate_to_index(Coordinate::new(x), false).value();
        index.is_finite().then_some(index)
    }
    pub fn horizontal_to_index(
        &self,
        item: &B::Item,
        nearest: bool,
    ) -> Result<Option<TimePointIndex>, ChartError> {
        let scale = self.model.time_scale();
        let item = scale
            .behavior()
            .to_internal(item)
            .map_err(B::conversion_error)?;
        Ok(scale.time_to_index(&item, nearest))
    }
    pub fn horizontal_to_coordinate(&self, item: &B::Item) -> Result<Option<f64>, ChartError> {
        Ok(self
            .horizontal_to_index(item, false)?
            .and_then(|index| self.logical_to_coordinate(index.value())))
    }
    pub fn coordinate_to_horizontal(&self, x: f64) -> Option<B::Item> {
        let index = self.coordinate_to_logical(x)?;
        self.model
            .time_scale()
            .index_to_original_time(TimePointIndex::new(index))
            .cloned()
    }
    /// Source synthetic placement: the selector chooses the pane; its default
    /// scale supplies Y. Uses nearest horizontal item and emits no pointer event.
    /// Returns false when there is no usable scale/domain; leaves position intact.
    pub fn set_crosshair_at(
        &mut self,
        id: SeriesId,
        item: &B::Item,
        price: f64,
    ) -> Result<bool, ChartError> {
        let pane = self.series_pane(id)?;
        if !price.is_finite() {
            return Err(ChartError::InvalidDataQuery);
        }
        let Some(index) = self.horizontal_to_index(item, true)? else {
            return Ok(false);
        };
        let scale = self.model.panes()[pane].default_price_scale();
        let Some(first) = scale.first_value().filter(|_| !scale.is_empty()) else {
            return Ok(false);
        };
        let y = scale.price_to_coordinate(price, first).value();
        let Some(x) = self.logical_to_coordinate(index.value()) else {
            return Ok(false);
        };
        if !y.is_finite() {
            return Ok(false);
        }
        self.model.set_synthetic_crosshair_position(
            pane,
            crate::renderers::line_renderer::Point { x, y },
        )?;
        self.synchronize_synthetic_selection();
        Ok(true)
    }
    /// Clear without generating an application pointer event.
    pub fn clear_crosshair_position(&mut self) -> Result<(), ChartError> {
        if let Some(origin) = self.model.crosshair().origin() {
            self.set_crosshair_position(origin.pane, None)?;
        }
        self.synchronize_synthetic_selection();
        Ok(())
    }
    fn synchronize_synthetic_selection(&mut self) {
        use crate::ui::primitive_hit_testing::PaneHit;
        let (hit, primitive) = match self.crosshair_selection() {
            Some(PaneHit::BuiltIn(hit, _)) => (Some(hit), None),
            Some(PaneHit::Primitive(hit)) => (None, Some(hit)),
            None => (None, None),
        };
        // Do not turn a silent API placement into a hover event on the next tick.
        self.last_hit = hit;
        self.last_primitive = primitive;
    }
}
