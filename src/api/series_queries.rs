//! Checked current membership and scoped readbacks; no mutable model escape hatch.
use super::{Chart, ChartBehavior, ChartError, PriceLineId};
use crate::{
    formatters::iprice_formatter::PriceValueFormatter,
    model::{
        data_layer::SeriesId,
        price_line::LastValueData,
        series_options::{SeriesOptionsMap, SeriesType},
    },
};
impl<B: ChartBehavior, M: Clone + 'static> Chart<B, M> {
    pub fn series_type(&self, id: SeriesId) -> Result<SeriesType, ChartError> {
        self.kind(id)
    }
    pub fn series_pane(&self, id: SeriesId) -> Result<usize, ChartError> {
        self.kind(id)?;
        self.model
            .pane_for_series(id)
            .ok_or(ChartError::UnknownSeries(id))
    }
    /// Creation-order IDs copied from the current owner, including custom series.
    pub fn price_lines(&self, id: SeriesId) -> Result<Vec<PriceLineId>, ChartError> {
        self.kind(id)?;
        Ok(self
            .model
            .projection(id)
            .ok_or(ChartError::UnknownSeries(id))?
            .price_lines()
            .iter()
            .map(|(id, _)| *id)
            .collect())
    }
    /// Built-in options include non-cloneable application callbacks. Read them
    /// within this scope; use the typed custom handle for custom options.
    pub fn with_series_options<R>(
        &self,
        id: SeriesId,
        read: impl FnOnce(&SeriesOptionsMap) -> R,
    ) -> Result<R, ChartError> {
        self.kind(id)?;
        let series = self.model.series(id).ok_or(ChartError::UnsupportedSeries)?;
        Ok(read(series.options()))
    }
    /// The series' own formatter, independent of percentage/log scale display.
    pub fn with_series_formatter<R>(
        &self,
        id: SeriesId,
        read: impl FnOnce(&dyn PriceValueFormatter) -> R,
    ) -> Result<R, ChartError> {
        self.kind(id)?;
        let series = self
            .model
            .projection(id)
            .ok_or(ChartError::UnknownSeries(id))?;
        Ok(read(series.formatter()))
    }
    /// Owned current value. `global` selects the last stored rather than last
    /// visible bar. Requires settled layout/scale state, as do coordinate queries.
    pub fn last_value_data(
        &mut self,
        id: SeriesId,
        global: bool,
    ) -> Result<Option<LastValueData>, ChartError> {
        let pane = self.series_pane(id)?;
        let visible = self.model.visible_strict_range();
        let scale = self.model.panes()[pane]
            .price_scale_for_source(id)
            .ok_or(ChartError::UnknownSeries(id))?;
        Ok(self
            .model
            .projection(id)
            .ok_or(ChartError::UnknownSeries(id))?
            .last_value_data(scale, visible.as_ref(), global))
    }
}
