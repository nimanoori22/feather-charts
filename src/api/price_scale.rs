use super::{Chart, ChartBehavior, ChartError};
use crate::model::{
    pane::PriceScalePosition,
    price_range_impl::{PriceRange, PriceRangeImpl},
    price_scale::{PriceScaleOptions, PriceScaleOptionsPatch},
};

impl<B: ChartBehavior, M: Clone + 'static> Chart<B, M> {
    pub fn visible_price_range(
        &self,
        pane: usize,
        scale: &PriceScalePosition,
    ) -> Result<Option<PriceRange>, ChartError> {
        Ok(self.model.price_scale(pane, scale)?.visible_price_range())
    }
    /// Endpoints use scale units, as in Lightweight Charts (including log units).
    pub fn set_visible_price_range(
        &mut self,
        pane: usize,
        scale: &PriceScalePosition,
        range: PriceRange,
    ) -> Result<(), ChartError> {
        let current = self.model.price_scale(pane, scale)?;
        let r = PriceRangeImpl::new(range.min_value, range.max_value);
        if !range.min_value.is_finite()
            || !range.max_value.is_finite()
            || !r.length().is_finite()
            || r.length() <= 0.
            || !current.accepts_custom_range(r)
        {
            return Err(ChartError::InvalidPriceRange);
        }
        if !current.is_auto_scale()
            && current.is_custom_price_range()
            && current.price_range() == Some(r)
        {
            return Ok(());
        }
        self.cancel_viewport_interaction();
        self.revision = self.revision.wrapping_add(1);
        self.last_hit = None;
        self.last_primitive = None;
        self.model.set_custom_price_range(pane, scale, r);
        Ok(())
    }
    pub fn price_scale_options(
        &self,
        pane: usize,
        scale: &PriceScalePosition,
    ) -> Result<PriceScaleOptions, ChartError> {
        Ok(self.model.price_scale(pane, scale)?.options().clone())
    }
    pub fn set_price_scale_auto_scale(
        &mut self,
        pane: usize,
        scale: &PriceScalePosition,
        enabled: bool,
    ) -> Result<(), ChartError> {
        self.apply_price_scale_options(
            pane,
            scale,
            PriceScaleOptionsPatch {
                auto_scale: Some(enabled),
                ..Default::default()
            },
        )
    }
    pub fn apply_price_scale_options(
        &mut self,
        pane: usize,
        scale: &PriceScalePosition,
        patch: PriceScaleOptionsPatch,
    ) -> Result<(), ChartError> {
        let current = self.model.price_scale(pane, scale)?;
        let mut validation = crate::model::price_scale::PriceScale::new_with_font_size(
            "validation",
            current.options().clone(),
            12.,
        );
        validation.apply_options(patch.clone()).map_err(|e| {
            ChartError::PriceAxis(crate::model::axis_snapshots::PriceAxisOptionsError::Options(e))
        })?;
        if !crate::model::chart_configuration::valid_scale(
            validation.options(),
            &mut self.color_parser(),
        ) {
            return Err(
                crate::model::chart_model::ChartModelError::InvalidChartOptions("price scale")
                    .into(),
            );
        }
        let defaults_changed = if let Some(defaults) = self.model.scale_defaults(scale) {
            let next = crate::model::chart_model::configuration::patched_scale_options(
                defaults, &patch, 12.,
            )
            .map_err(|e| {
                ChartError::PriceAxis(
                    crate::model::axis_snapshots::PriceAxisOptionsError::Options(e),
                )
            })?;
            defaults != &next
        } else {
            false
        };
        if current.options() == validation.options() && !defaults_changed {
            return Ok(());
        }
        if crate::model::chart_configuration::scale_geometry_changed(
            current.options(),
            validation.options(),
        ) {
            self.cancel_viewport_interaction();
            self.revision = self.revision.wrapping_add(1);
            self.last_hit = None;
            self.last_primitive = None;
        }
        self.model.apply_scale_options(pane, scale, patch);
        Ok(())
    }
}

impl<B: ChartBehavior, M: Clone + 'static> Chart<B, M> {
    /// Fresh assignment in the series' current pane. Named overlays have no axis width.
    pub fn series_price_scale(
        &self,
        id: crate::model::data_layer::SeriesId,
    ) -> Result<PriceScalePosition, ChartError> {
        self.kind(id)?;
        Ok(self.model.series_price_scale(id)?)
    }
    /// Move the existing series without replacing its data, handles or annotations.
    /// Missing overlays are created in the current pane. Equal assignments are inert.
    pub fn set_series_price_scale(
        &mut self,
        id: crate::model::data_layer::SeriesId,
        target: PriceScalePosition,
    ) -> Result<(), ChartError> {
        self.kind(id)?;
        if !self.model.preview_scale_assignment(id, &target)? {
            return Ok(());
        }
        self.cancel_interaction();
        self.model.set_series_price_scale(id, target)?;
        self.last_hit = None;
        self.last_primitive = None;
        Ok(())
    }
}
