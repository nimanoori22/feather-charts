//! Typed numeric frontend; the calendar and yield charts share one owner/widget.
use super::*;
use crate::model::{
    chart_model::ChartModelOptions,
    data_consumer::{BuiltInSeriesDataItem, LineDataItem, WhitespaceData},
    data_layer::SeriesId,
    ihorz_scale_behavior::HorzScaleBehavior,
    localization_options::LocalizationOptions,
    pane::PriceScalePosition,
    series_options::{
        AreaSeriesOptions, AreaSeriesOptionsPatch, LineSeriesOptions, LineSeriesOptionsPatch,
        SeriesOptionsMap, SeriesType,
    },
    time_scale_options::HorzScaleOptions,
    yield_curve_horz_scale_behavior::{ScaffoldState, YieldCurveHorzScaleBehavior},
};
use std::{ops::Deref, rc::Rc, time::Instant};

pub struct YieldCurveChart {
    core: Chart<YieldCurveHorzScaleBehavior>,
    support: SeriesId,
    scaffold: Option<ScaffoldState>,
    dirty: bool,
    rebuilds: u64,
}
impl Deref for YieldCurveChart {
    type Target = Chart<YieldCurveHorzScaleBehavior>;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}
impl YieldCurveChart {
    pub fn visible_logical_range(&mut self) -> Option<crate::model::time_data::LogicalRange> {
        self.core.visible_logical_range()
    }
    pub fn visible_horizontal_range(&mut self) -> Option<HorizontalRange<Maturity>> {
        self.core.visible_horizontal_range()
    }
    pub fn set_visible_horizontal_range(
        &mut self,
        range: HorizontalRange<Maturity>,
    ) -> Result<(), ChartError> {
        self.core.set_visible_horizontal_range(range)
    }
    pub fn reset_time_scale(&mut self) {
        self.core.reset_time_scale();
    }
    pub fn scroll_to_real_time(&mut self) {
        self.core.scroll_to_real_time();
    }
    pub fn remove_price_line(&mut self, id: PriceLineId) -> Result<(), ChartError> {
        self.core.remove_price_line(id)
    }
    pub fn last_value_data(
        &mut self,
        id: SeriesId,
        global: bool,
    ) -> Result<Option<crate::model::price_line::LastValueData>, ChartError> {
        self.core.last_value_data(id, global)
    }
    pub fn price_to_coordinate(
        &mut self,
        id: SeriesId,
        price: f64,
    ) -> Result<Option<f64>, ChartError> {
        self.core.price_to_coordinate(id, price)
    }
    pub fn coordinate_to_price(&mut self, id: SeriesId, y: f64) -> Result<Option<f64>, ChartError> {
        self.core.coordinate_to_price(id, y)
    }
    pub fn set_crosshair_at(
        &mut self,
        id: SeriesId,
        item: &Maturity,
        price: f64,
    ) -> Result<bool, ChartError> {
        self.core.set_crosshair_at(id, item, price)
    }
    pub fn clear_crosshair_position(&mut self) -> Result<(), ChartError> {
        self.core.clear_crosshair_position()
    }
    pub fn set_event_interests(&mut self, interests: super::EventInterests) {
        self.core.set_event_interests(interests);
    }
    pub fn take_events(&mut self) -> Vec<ChartEvent<Maturity>> {
        self.core.take_events()
    }
    pub fn apply_chart_options(&mut self, patch: ChartOptionsPatch) -> Result<(), ChartError> {
        self.core.apply_chart_options(patch)
    }
    pub fn apply_localization_options(&mut self, patch: LocalizationOptionsPatch<Maturity>) {
        self.core.apply_localization_options(patch);
    }
    pub fn set_series_price_scale(
        &mut self,
        id: SeriesId,
        position: PriceScalePosition,
    ) -> Result<(), ChartError> {
        self.core.set_series_price_scale(id, position)
    }
    pub fn set_axis_interaction_options(&mut self, options: super::AxisInteractionOptions) {
        self.core.set_axis_interaction_options(options);
    }
    pub fn set_mouse_navigation_options(&mut self, options: MouseNavigationOptions) {
        self.core.set_mouse_navigation_options(options);
    }
    pub fn set_visible_price_range(
        &mut self,
        pane: usize,
        scale: &crate::model::pane::PriceScalePosition,
        range: crate::model::price_range_impl::PriceRange,
    ) -> Result<(), ChartError> {
        self.core.set_visible_price_range(pane, scale, range)
    }
    pub fn set_price_scale_auto_scale(
        &mut self,
        pane: usize,
        scale: &crate::model::pane::PriceScalePosition,
        enabled: bool,
    ) -> Result<(), ChartError> {
        self.core.set_price_scale_auto_scale(pane, scale, enabled)
    }
    pub fn apply_price_scale_options(
        &mut self,
        pane: usize,
        scale: &crate::model::pane::PriceScalePosition,
        patch: crate::model::price_scale::PriceScaleOptionsPatch,
    ) -> Result<(), ChartError> {
        self.core.apply_price_scale_options(pane, scale, patch)
    }
    pub fn new(options: YieldCurveOptions) -> Result<Self, ChartError> {
        let mut chart_options = ChartModelOptions::default();
        chart_options.pane.left_price_scale.visible = true;
        chart_options.pane.right_price_scale.visible = false;
        chart_options.pane.default_visible_price_scale =
            crate::model::pane::DefaultVisiblePriceScale::Left;
        Self::with_chart_options(
            options,
            chart_options,
            LocalizationOptions::new("en-US", ""),
        )
    }
    pub fn with_chart_options(
        options: YieldCurveOptions,
        chart_options: ChartModelOptions,
        mut localization: LocalizationOptions<Maturity>,
    ) -> Result<Self, ChartError> {
        options.scaffold(0.).map_err(ChartError::YieldCurve)?;
        let mut data = YieldCurveHorzScaleBehavior::default();
        data.set_options(options);
        let mut scale = YieldCurveHorzScaleBehavior::default();
        scale.set_options(options);
        if localization.base.price_formatter.is_none() {
            localization.base.price_formatter = Some(Rc::new(|v| {
                let value = v.value();
                let text = if value.is_finite() {
                    crate::formatters::fixed::fixed(value, 3)
                } else {
                    format!("{value:.3}")
                };
                format!("{text}%")
            }));
        }
        let mut core = Chart::with_behavior(
            chart_options,
            data,
            scale,
            HorzScaleOptions {
                ignore_whitespace_indices: true,
                ..Default::default()
            },
            localization,
        );
        let mut support_options = LineSeriesOptions::default();
        yield_defaults(&mut support_options.common);
        let support = core.add_series(
            SeriesOptionsMap::Line(support_options),
            0,
            PriceScalePosition::Left,
        )?;
        core.kinds.remove(&support);
        core.views[0].retain(|(id, _)| *id != support);
        let mut chart = Self {
            core,
            support,
            scaffold: None,
            dirty: true,
            rebuilds: 0,
        };
        chart.settle()?;
        Ok(chart)
    }
    pub fn add_line_series(
        &mut self,
        patch: LineSeriesOptionsPatch,
        pane: usize,
        position: PriceScalePosition,
    ) -> Result<SeriesId, ChartError> {
        let mut options = LineSeriesOptions::default();
        yield_defaults(&mut options.common);
        options.apply_patch(patch);
        self.core
            .add_series(SeriesOptionsMap::Line(options), pane, position)
    }
    pub fn add_area_series(
        &mut self,
        patch: AreaSeriesOptionsPatch,
        pane: usize,
        position: PriceScalePosition,
    ) -> Result<SeriesId, ChartError> {
        let mut options = AreaSeriesOptions::default();
        yield_defaults(&mut options.common);
        options.apply_patch(patch);
        self.core
            .add_series(SeriesOptionsMap::Area(options), pane, position)
    }
    pub fn set_data(
        &mut self,
        id: SeriesId,
        rows: Vec<BuiltInSeriesDataItem<Maturity>>,
    ) -> Result<(), ChartError> {
        self.core.set_data(id, rows)?;
        self.dirty = true;
        Ok(())
    }
    pub fn update_data(
        &mut self,
        id: SeriesId,
        row: BuiltInSeriesDataItem<Maturity>,
        historical: bool,
    ) -> Result<(), ChartError> {
        self.core.update_data(id, row, historical)?;
        self.dirty = true;
        Ok(())
    }
    pub fn set_up_down_data(
        &mut self,
        id: UpDownMarkersId,
        rows: Vec<BuiltInSeriesDataItem<Maturity>>,
    ) -> Result<(), ChartError> {
        self.core.set_up_down_data(id, rows)?;
        self.dirty = true;
        Ok(())
    }
    pub fn update_up_down_data(
        &mut self,
        id: UpDownMarkersId,
        row: BuiltInSeriesDataItem<Maturity>,
        historical: bool,
        now: Instant,
    ) -> Result<(), ChartError> {
        self.core.update_up_down_data(id, row, historical, now)?;
        self.dirty = true;
        Ok(())
    }
    pub fn pop_data(
        &mut self,
        id: SeriesId,
        count: usize,
    ) -> Result<Vec<BuiltInSeriesDataItem<Maturity>>, ChartError> {
        let removed = self.core.pop_data(id, count)?;
        self.dirty |= !removed.is_empty();
        Ok(removed)
    }
    pub fn remove_series(&mut self, id: SeriesId) -> Result<(), ChartError> {
        self.core.remove_series(id)?;
        self.dirty = true;
        Ok(())
    }
    pub fn replace_yield_curve_options(
        &mut self,
        options: YieldCurveOptions,
    ) -> Result<(), ChartError> {
        let last = self
            .core
            .model
            .time_scale()
            .last_time()
            .map_or(0., |v| v.months());
        options.scaffold(last).map_err(ChartError::YieldCurve)?;
        self.core
            .model
            .configure_horizontal_behavior(&mut self.core.layer, || options);
        self.dirty = true;
        Ok(())
    }
    pub fn set_maturity_formatter(
        &mut self,
        formatter: Option<crate::model::localization_options::TimeFormatterFn<Maturity>>,
    ) {
        self.core
            .model
            .set_time_formatter(&mut self.core.layer, formatter);
    }
    pub fn primitive_context(
        &mut self,
        id: PrimitiveId,
    ) -> Result<PrimitiveContext<'_, YieldCurveHorzScaleBehavior>, ChartError> {
        self.core.primitive_context(id)
    }
    pub fn preview_primitive_update(&mut self, id: PrimitiveEditId) -> Result<(), ChartError> {
        self.core.preview_primitive_update(id)
    }
    pub fn request_primitive_update(&mut self, id: PrimitiveId) -> Result<(), ChartError> {
        self.core.request_primitive_update(id)
    }
    pub fn add_price_line(
        &mut self,
        id: SeriesId,
        options: PriceLineOptions,
    ) -> Result<PriceLineId, ChartError> {
        self.core.add_price_line(id, options)
    }

    fn settle(&mut self) -> Result<(), ChartError> {
        if !self.dirty {
            return Ok(());
        }
        let scale = self.core.model.time_scale();
        let last = scale.last_time().map_or(0., |v| v.months());
        let state = scale
            .behavior()
            .options()
            .scaffold(last)
            .map_err(ChartError::YieldCurve)?;
        if self.scaffold != Some(state) {
            let rows = state
                .items()
                .map(|time| {
                    BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(WhitespaceData {
                        time,
                        custom_values: None,
                    }))
                })
                .collect();
            let update = self
                .core
                .layer
                .set_series_data(self.support, SeriesType::Line, rows)
                .map_err(ChartError::YieldData)?;
            self.core.model.apply_data_update(update)?;
            self.scaffold = Some(state);
            self.rebuilds += 1;
        }
        self.dirty = false;
        Ok(())
    }
    pub fn prepare(
        &mut self,
        size: iced::Size,
        ratio: f32,
        now: Instant,
    ) -> Result<(), ChartError> {
        self.settle()?;
        self.core.prepare(size, ratio, now)
    }
    pub fn prepare_screenshot(
        &mut self,
        layers: ScreenshotLayers,
        now: Instant,
    ) -> Result<ChartScreenshot, ChartError> {
        self.settle()?;
        self.core.prepare_screenshot(layers, now)
    }
    pub fn prepare_snapshot(
        &mut self,
        size: iced::Size,
        ratio: f32,
        now: Instant,
    ) -> Result<crate::ui::multi_pane_frame::MultiPaneChartFrameSnapshot, ChartError> {
        self.settle()?;
        self.core.prepare_snapshot(size, ratio, now)
    }
    pub fn update(
        &mut self,
        message: ChartMessage,
    ) -> Result<Vec<ChartEvent<Maturity>>, ChartError> {
        self.settle()?;
        self.core.update(message)
    }
    pub fn fit_content(&mut self) {
        self.core.fit_content();
    }
    pub fn set_logical_range(&mut self, range: crate::model::time_data::LogicalRange) {
        self.core.set_logical_range(range);
    }
    pub fn set_right_offset(&mut self, offset: f64) {
        self.core.set_right_offset(offset);
    }
    pub fn add_pane(&mut self) -> usize {
        self.core.add_pane()
    }
    pub fn move_pane(&mut self, from: usize, to: usize) -> Result<(), ChartError> {
        self.core.move_pane(from, to)
    }
    /// Temporarily raise the hovered series without changing its stored order.
    pub fn set_hovered_series_on_top(&mut self, enabled: bool) {
        self.core.set_hovered_series_on_top(enabled);
    }
    /// Set pane-local order, counting the private whitespace support slot.
    pub fn set_series_order(&mut self, id: SeriesId, order: usize) -> Result<(), ChartError> {
        self.core.set_series_order(id, order)
    }
    pub fn swap_panes(&mut self, first: usize, second: usize) -> Result<(), ChartError> {
        self.core.swap_panes(first, second)
    }
    pub fn move_series_to_pane(&mut self, id: SeriesId, target: usize) -> Result<(), ChartError> {
        self.core.move_series_to_pane(id, target)
    }
    pub fn remove_pane(&mut self, pane: usize) -> Result<(), ChartError> {
        if self.core.model.panes().len() == 1 {
            return Ok(());
        }
        self.core.model.validate_pane(pane)?;
        if self.core.model.pane_for_series(self.support) == Some(pane) {
            // Support is chart infrastructure, not a publicly removable series.
            self.core.cancel_interaction();
            self.core.model.set_preserve_empty_pane(pane, true)?;
            self.core
                .model
                .move_series_to_pane(self.support, if pane == 0 { 1 } else { 0 })?;
        }
        self.core.remove_pane(pane)?;
        self.dirty = true;
        Ok(())
    }
    pub fn set_preserve_empty_pane(
        &mut self,
        pane: usize,
        preserve: bool,
    ) -> Result<(), ChartError> {
        self.core.set_preserve_empty_pane(pane, preserve)
    }
    pub fn set_pane_height(&mut self, pane: usize, height: f64) -> Result<(), ChartError> {
        self.core.set_pane_height(pane, height)
    }
    pub fn set_pane_stretch_factor(&mut self, pane: usize, factor: f64) -> Result<(), ChartError> {
        self.core.set_pane_stretch_factor(pane, factor)
    }
    pub fn apply_time_scale_options(
        &mut self,
        patch: crate::model::time_scale_options::HorzScaleOptionsPatch,
    ) {
        self.core.apply_time_scale_options(patch);
    }
    pub fn apply_price_axis_options(
        &mut self,
        pane: usize,
        side: crate::model::axis_snapshots::PriceAxisSide,
        patch: crate::model::price_scale::PriceScaleOptionsPatch,
    ) -> Result<(), ChartError> {
        self.core.apply_price_axis_options(pane, side, patch)
    }
    pub fn add_up_down_markers(
        &mut self,
        id: SeriesId,
        options: UpDownMarkersOptions,
    ) -> Result<UpDownMarkersId, ChartError> {
        self.core.add_up_down_markers(id, options)
    }
    pub fn set_up_down_markers(
        &mut self,
        id: UpDownMarkersId,
        markers: Vec<UpDownMarker<Maturity>>,
    ) -> Result<(), ChartError> {
        self.core.set_up_down_markers(id, markers)
    }
    pub fn up_down_markers(
        &mut self,
        id: UpDownMarkersId,
        now: Instant,
    ) -> Result<Vec<UpDownMarker<Maturity>>, ChartError> {
        self.core.up_down_markers(id, now)
    }
    pub fn clear_up_down_markers(&mut self, id: UpDownMarkersId) -> Result<(), ChartError> {
        self.core.clear_up_down_markers(id)
    }
    pub fn replace_up_down_marker_options(
        &mut self,
        id: UpDownMarkersId,
        options: UpDownMarkersOptions,
    ) -> Result<(), ChartError> {
        self.core.replace_up_down_marker_options(id, options)
    }
    pub fn remove_up_down_markers(&mut self, id: UpDownMarkersId) -> Result<(), ChartError> {
        self.core.remove_up_down_markers(id)
    }
    pub fn add_pane_primitive(
        &mut self,
        pane: usize,
        primitive: impl DrawingPrimitive<YieldCurveHorzScaleBehavior>,
    ) -> Result<PrimitiveId, ChartError> {
        self.core.add_pane_primitive(pane, primitive)
    }
    pub fn add_series_primitive(
        &mut self,
        id: SeriesId,
        primitive: impl DrawingPrimitive<YieldCurveHorzScaleBehavior>,
    ) -> Result<PrimitiveId, ChartError> {
        self.core.add_series_primitive(id, primitive)
    }
    pub fn replace_primitive(
        &mut self,
        id: PrimitiveId,
        primitive: impl DrawingPrimitive<YieldCurveHorzScaleBehavior>,
    ) -> Result<(), ChartError> {
        self.core.replace_primitive(id, primitive)
    }
    pub fn remove_primitive(&mut self, id: PrimitiveId) -> Result<(), ChartError> {
        self.core.remove_primitive(id)
    }
    pub fn replace_series_options(
        &mut self,
        id: SeriesId,
        options: SeriesOptionsMap,
    ) -> Result<(), ChartError> {
        self.core.replace_series_options(id, options)
    }
}
fn yield_defaults(options: &mut crate::model::series_options::SeriesOptionsCommon) {
    options.last_value_visible = false;
    options.price_line_visible = false;
}

#[cfg(test)]
mod tests;

mod custom_series;
