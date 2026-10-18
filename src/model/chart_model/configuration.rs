use super::*;
use crate::model::{
    price_range_impl::PriceRangeImpl,
    price_scale::{PriceScale, PriceScaleOptions, PriceScaleOptionsPatch},
};

impl<B, M> ChartModel<B, M>
where
    B: HorzScaleBehavior,
    B::InternalItem: 'static,
    B::Item: 'static,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone + 'static,
{
    pub(crate) fn price_scale(
        &self,
        pane: usize,
        position: &PriceScalePosition,
    ) -> Result<&PriceScale, ChartModelError> {
        let pane = self
            .panes
            .get(pane)
            .ok_or(ChartModelError::InvalidPane(pane))?;
        let id = match position {
            PriceScalePosition::Left => "left",
            PriceScalePosition::Right => "right",
            PriceScalePosition::Overlay(id) => id,
        };
        pane.price_scale_by_id(id)
            .ok_or_else(|| ChartModelError::Pane(PaneError::UnknownPriceScale(position.clone())))
    }
    pub(crate) fn set_custom_price_range(
        &mut self,
        pane: usize,
        position: &PriceScalePosition,
        range: PriceRangeImpl,
    ) {
        let scale = self.panes[pane]
            .price_scale_mut_existing(position)
            .expect("validated scale");
        scale.set_auto_scale(false);
        scale.set_custom_price_range(Some(range));
        self.full_update();
        self.refresh_crosshair();
    }
    pub(crate) fn apply_scale_options(
        &mut self,
        pane: usize,
        position: &PriceScalePosition,
        patch: PriceScaleOptionsPatch,
    ) {
        let scale = self.panes[pane]
            .price_scale_mut_existing(position)
            .expect("validated scale");
        scale.apply_options(patch.clone()).expect("validated patch");
        let defaults = match position {
            PriceScalePosition::Left => Some(&mut self.options.pane.left_price_scale),
            PriceScalePosition::Right => Some(&mut self.options.pane.right_price_scale),
            PriceScalePosition::Overlay(_) => None,
        };
        if let Some(defaults) = defaults {
            *defaults = patched_scale_options(defaults, &patch, self.options.layout.font_size)
                .expect("validated defaults");
        }
        self.recalculate_requested = true;
        self.full_update();
        self.flush_effects();
    }
}

pub(crate) fn patched_scale_options(
    options: &PriceScaleOptions,
    patch: &PriceScaleOptionsPatch,
    font: f64,
) -> Result<PriceScaleOptions, crate::model::price_scale::PriceScaleOptionsError> {
    let mut scale = PriceScale::new_with_font_size("validation", options.clone(), font);
    scale.apply_options(patch.clone())?;
    Ok(scale.options().clone())
}
