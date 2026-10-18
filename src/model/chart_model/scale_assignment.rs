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
    pub(crate) fn validate_scale_position(
        &self,
        position: &PriceScalePosition,
    ) -> Result<(), ChartModelError> {
        if matches!(position,PriceScalePosition::Overlay(id) if id=="left"||id=="right") {
            return Err(ChartModelError::InvalidChartOptions(
                "reserved overlay price scale id",
            ));
        }
        Ok(())
    }
    pub fn series_price_scale(&self, id: SeriesId) -> Result<PriceScalePosition, ChartModelError> {
        let entry = self
            .series
            .get(&id)
            .ok_or(ChartModelError::UnknownSeries(id))?;
        self.panes[entry.pane]
            .source_position(id)
            .cloned()
            .ok_or(ChartModelError::UnknownSeries(id))
    }
    pub(crate) fn preview_scale_assignment(
        &self,
        id: SeriesId,
        target: &PriceScalePosition,
    ) -> Result<bool, ChartModelError> {
        let current = self.series_price_scale(id)?;
        self.validate_scale_position(target)?;
        Ok(current != *target)
    }
    pub(crate) fn preview_series_options(
        &self,
        id: SeriesId,
        options: &SeriesOptionsMap,
    ) -> Result<PriceScalePosition, ChartModelError> {
        let entry = self
            .series
            .get(&id)
            .ok_or(ChartModelError::UnknownSeries(id))?;
        let source = entry
            .built_in()
            .ok_or(ChartModelError::WrongUpdateKind(id))?;
        source
            .borrow()
            .validate_replacement_options(options)
            .map_err(ChartModelError::Construction)?;
        let target = options
            .common()
            .price_scale_id
            .as_deref()
            .map(PriceScalePosition::from_id)
            .unwrap_or(self.series_price_scale(id)?);
        self.validate_scale_position(&target)?;
        Ok(target)
    }
    pub fn set_series_price_scale(
        &mut self,
        id: SeriesId,
        target: PriceScalePosition,
    ) -> Result<(), ChartModelError> {
        if !self.preview_scale_assignment(id, &target)? {
            return Ok(());
        }
        let entry = &self.series[&id];
        let pane = entry.pane;
        let effects = self.panes[pane]
            .move_series_to_scale(id, target.clone())
            .map_err(ChartModelError::Pane)?;
        entry
            .projection
            .borrow_mut()
            .set_price_scale_id(target.id().into());
        self.consume_pane_effects(effects);
        self.recalculate_requested = true;
        self.full_update();
        self.flush_effects();
        Ok(())
    }
}
