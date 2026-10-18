use super::*;
use crate::model::chart_configuration::*;
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
    pub(crate) fn attribution_options(&self) -> (bool, &str) {
        (
            self.options.layout.attribution_logo,
            &self.options.layout.text_color,
        )
    }
    pub fn runtime_options(&self) -> RuntimeModelOptions {
        let l = &self.options.layout;
        RuntimeModelOptions {
            layout: LayoutAppearance {
                attribution_logo: l.attribution_logo,
                background: l.background.clone(),
                text_color: l.text_color.clone(),
                font_size: l.font_size,
                font_family: l.font_family.clone(),
                panes: l.panes.clone(),
            },
            grid: self.options.grid.clone(),
            pane: self.options.pane.clone(),
            time_scale: self.time_scale.options().clone(),
            crosshair: self.crosshair.options.clone(),
        }
    }
    pub(crate) fn preview_runtime_options(
        &self,
        patch: &ChartModelOptionsPatch,
    ) -> Result<(RuntimeModelOptions, bool, bool), ChartModelError> {
        let before = self.runtime_options();
        let mut next = before.clone();
        if let Some(p) = patch.layout.clone() {
            p.apply(&mut next.layout);
        }
        if let Some(p) = patch.grid.clone() {
            p.apply(&mut next.grid);
        }
        if let Some(p) = patch.time_scale.clone() {
            next.time_scale = p.merged(next.time_scale);
        }
        if let Some(p) = &patch.crosshair {
            next.crosshair = p.clone();
        }
        for (o, p) in [
            (&mut next.pane.left_price_scale, &patch.left_price_scale),
            (&mut next.pane.right_price_scale, &patch.right_price_scale),
            (
                &mut next.pane.overlay_price_scale,
                &patch.overlay_price_scales,
            ),
        ] {
            if let Some(p) = p {
                *o = patched_scale_options(o, p, next.layout.font_size)
                    .map_err(|_| ChartModelError::InvalidChartOptions("scale margins"))?;
            }
            if !valid_scale(o) {
                return Err(ChartModelError::InvalidChartOptions("price scale"));
            }
        }
        let l = &next.layout;
        if !l.font_size.is_finite()
            || l.font_size <= 0.
            || l.font_size > f64::from(f32::MAX)
            || l.font_family.trim().is_empty()
        {
            return Err(ChartModelError::InvalidTypography);
        }
        let mut colors = vec![
            l.text_color.as_str(),
            l.panes.separator_color.as_str(),
            l.panes.separator_hover_color.as_str(),
            next.grid.vert_lines.color.as_str(),
            next.grid.horz_lines.color.as_str(),
            next.time_scale.border_color.as_str(),
        ];
        match &l.background {
            crate::model::layout_options::Background::Solid { color } => colors.push(color),
            crate::model::layout_options::Background::VerticalGradient {
                top_color,
                bottom_color,
            } => colors.extend([top_color.as_str(), bottom_color.as_str()]),
        };
        for line in [&next.crosshair.vertical, &next.crosshair.horizontal] {
            colors.extend([line.color.as_str(), line.label_background_color.as_str()]);
        }
        if !colors.into_iter().all(valid_color) {
            return Err(ChartModelError::InvalidChartOptions("color"));
        }
        if !valid_horizontal_options(&next.time_scale) {
            return Err(ChartModelError::InvalidChartOptions("horizontal scale"));
        }
        let mut changed = before != next;
        let mut geometry_time = before.time_scale.clone();
        geometry_time.border_color = next.time_scale.border_color.clone();
        let mut geometry = before.layout.font_size != l.font_size
            || before.layout.font_family != l.font_family
            || before.layout.panes.enable_resize != l.panes.enable_resize
            || geometry_time != next.time_scale;
        for pane in &self.panes {
            let (c, g) = pane.preview_runtime_scales(patch)?;
            changed |= c;
            geometry |= g;
        }
        Ok((next, changed, geometry))
    }
    pub(crate) fn apply_runtime_options(
        &mut self,
        layer: &mut DataLayer<B, M>,
        patch: &ChartModelOptionsPatch,
        next: RuntimeModelOptions,
    ) where
        B::Converter: HorzScaleItemConverter<B::Item, B::InternalItem, Error = B::Error>,
    {
        let mut content_options = self.runtime_options();
        content_options.layout.attribution_logo = next.layout.attribution_logo;
        let decoration_only = content_options == next
            && patch.time_scale.is_none()
            && patch.grid.is_none()
            && patch.crosshair.is_none()
            && patch.left_price_scale.is_none()
            && patch.right_price_scale.is_none()
            && patch.overlay_price_scales.is_none();
        self.options.layout.attribution_logo = next.layout.attribution_logo;
        if decoration_only {
            // Full invalidation may shrink settled axis allocations. A screen
            // decoration must not change the viewport or recalculate autoscale.
            self.light_update();
            self.flush_effects();
            return;
        }
        self.options.layout.background = next.layout.background;
        self.options.layout.text_color = next.layout.text_color;
        self.options.layout.font_size = next.layout.font_size;
        self.options.layout.font_family = next.layout.font_family;
        self.options.layout.panes = next.layout.panes;
        self.options.grid = next.grid;
        self.options.pane = next.pane;
        self.crosshair.options = next.crosshair;
        for pane in &mut self.panes {
            pane.set_font_size(self.options.layout.font_size);
            pane.apply_runtime_scales(patch, &self.options.pane);
        }
        if let Some(p) = patch.time_scale.clone() {
            let effects = self.time_scale.apply_options(p);
            layer
                .behavior_mut()
                .update_scale_options(self.time_scale.options());
            layer
                .behavior_mut()
                .update_formatter(self.time_scale.localization());
            self.consume_time_effects(effects);
        }
        self.recalculate_requested = true;
        self.full_update();
        self.flush_effects();
    }
    pub(crate) fn price_scale(
        &self,
        pane: usize,
        position: &PriceScalePosition,
    ) -> Result<&PriceScale, ChartModelError> {
        let pane = self
            .panes
            .get(pane)
            .ok_or(ChartModelError::InvalidPane(pane))?;
        pane.price_scale_existing(position)
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
        match position {
            PriceScalePosition::Left => self.options.pane.left_price_scale.auto_scale = false,
            PriceScalePosition::Right => self.options.pane.right_price_scale.auto_scale = false,
            PriceScalePosition::Overlay(_) => {}
        }
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
    pub(crate) fn scale_defaults(
        &self,
        position: &PriceScalePosition,
    ) -> Option<&PriceScaleOptions> {
        match position {
            PriceScalePosition::Left => Some(&self.options.pane.left_price_scale),
            PriceScalePosition::Right => Some(&self.options.pane.right_price_scale),
            PriceScalePosition::Overlay(_) => None,
        }
    }
}

pub(crate) fn patched_scale_options(
    options: &PriceScaleOptions,
    patch: &PriceScaleOptionsPatch,
    font: f64,
) -> Result<PriceScaleOptions, crate::model::price_scale::PriceScaleOptionsError> {
    let mut result = patched_live_scale_options(options, patch, font)?;
    // Source chart defaults are merged values; live special-mode scales force autoscale separately.
    result.auto_scale = patch.auto_scale.unwrap_or(options.auto_scale);
    Ok(result)
}
pub(crate) fn patched_live_scale_options(
    options: &PriceScaleOptions,
    patch: &PriceScaleOptionsPatch,
    font: f64,
) -> Result<PriceScaleOptions, crate::model::price_scale::PriceScaleOptionsError> {
    let mut scale = PriceScale::new_with_font_size("validation", options.clone(), font);
    scale.apply_options(patch.clone())?;
    Ok(scale.options().clone())
}
