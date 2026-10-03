//! Synchronous chart ownership and effect processing, independent of the UI.

use crate::model::{
    chart_data_coordinator::{ChartDataCoordinator, ChartDataEffect},
    data_layer::{
        BuiltInDataUpdate, CustomDataUpdateResponse, DataLayer, SeriesId, TimeScaleChanges,
    },
    grid::GridOptions,
    ihorz_scale_behavior::{HorzScaleBehavior, HorzScaleItemConverter},
    invalidate_mask::{
        AnimationRequest, InvalidateMask, InvalidationLevel, PaneInvalidation,
        TimeScaleInvalidation,
    },
    layout_options::LayoutOptions,
    pane::{
        DEFAULT_STRETCH_FACTOR, Pane, PaneEffect, PaneError, PaneOptions, PriceScalePosition,
        SeriesHandle,
    },
    series::{
        BuiltInSeriesTarget, CustomSeriesHandle, CustomSeriesTarget, Series,
        SeriesConstructionError,
    },
    series_options::{SeriesOptionsMap, SeriesType},
    time_data::LogicalRange,
    time_scale::TimeScale,
    time_scale_host::{ScrollAnimation, TimeScaleEffect, TimeScaleHost, TimeScaleLayoutContext},
    time_scale_options::HorzScaleOptionsPatch,
};
use std::{
    cell::{Ref, RefCell},
    collections::{BTreeMap, BTreeSet},
    hash::Hash,
    rc::Rc,
};

pub struct ChartModelOptions {
    pub layout: LayoutOptions,
    pub pane: PaneOptions,
    pub grid: GridOptions,
    pub add_default_pane: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ChartModelError {
    DuplicateSeries(SeriesId),
    UnknownSeries(SeriesId),
    InvalidPane(usize),
    InvalidDimensions,
    Construction(SeriesConstructionError),
    Pane(PaneError),
    CustomSeriesRequiresTypedRegistration,
    NotCustomSeries(SeriesId),
    CustomHandleMismatch(SeriesId),
    WrongUpdateKind(SeriesId),
}

struct RegisteredSeries<I, O, M> {
    pane: usize,
    state: RegisteredSeriesState<I, O, M>,
}
enum RegisteredSeriesState<I, O, M> {
    BuiltIn(SeriesHandle<I, O, M>),
    Custom(Rc<dyn std::any::Any>),
}
impl<I, O, M> RegisteredSeries<I, O, M> {
    fn built_in(&self) -> Option<&SeriesHandle<I, O, M>> {
        match &self.state {
            RegisteredSeriesState::BuiltIn(handle) => Some(handle),
            RegisteredSeriesState::Custom(_) => None,
        }
    }
}
pub type SeriesRef<'a, I, O, M = ()> = Ref<'a, Series<I, O, M>>;

/// ChartModel owns no DataLayer and no clock. Series adapters share handles but
/// never reference this owner, so dropping it releases the ownership graph.
pub struct ChartModel<B: HorzScaleBehavior, M = ()>
where
    B::CacheKey: Eq + Hash,
{
    time_scale: TimeScale<B>,
    options: ChartModelOptions,
    panes: Vec<Pane<B::InternalItem, B::Item, M>>,
    series: BTreeMap<SeriesId, RegisteredSeries<B::InternalItem, B::Item, M>>,
    coordinator: ChartDataCoordinator<B::InternalItem, B::Item, M>,
    pending: Option<InvalidateMask>,
    recalculate_requested: bool,
    animation_requests: Vec<ScrollAnimation>,
    options_applied: bool,
}

impl<B, M> ChartModel<B, M>
where
    B: HorzScaleBehavior,
    B::InternalItem: 'static,
    B::Item: 'static,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone + 'static,
{
    pub fn new(time_scale: TimeScale<B>, options: ChartModelOptions) -> Self {
        let mut model = Self {
            time_scale,
            options,
            panes: Vec::new(),
            series: BTreeMap::new(),
            coordinator: ChartDataCoordinator::new(),
            pending: None,
            recalculate_requested: false,
            animation_requests: Vec::new(),
            options_applied: false,
        };
        if model.options.add_default_pane {
            let index = model.add_pane();
            let effects = model.panes[index].set_stretch_factor(DEFAULT_STRETCH_FACTOR * 2.0);
            model.consume_pane_effects(effects);
            model.flush_effects();
        }
        model
    }

    pub fn time_scale(&self) -> &TimeScale<B> {
        &self.time_scale
    }
    pub fn visible_logical_range(
        &mut self,
    ) -> Option<crate::model::range_impl::RangeImpl<crate::model::time_data::Logical>> {
        self.time_scale.visible_logical_range()
    }
    pub fn visible_strict_range(
        &mut self,
    ) -> Option<crate::model::range_impl::RangeImpl<crate::model::time_data::TimePointIndex>> {
        self.time_scale.visible_strict_range()
    }
    pub fn panes(&self) -> &[Pane<B::InternalItem, B::Item, M>] {
        &self.panes
    }
    pub fn series_count(&self) -> usize {
        self.series.len()
    }
    pub fn series(&self, id: SeriesId) -> Option<SeriesRef<'_, B::InternalItem, B::Item, M>> {
        self.series
            .get(&id)?
            .built_in()
            .map(|handle| handle.borrow())
    }
    pub fn pane_for_series(&self, id: SeriesId) -> Option<usize> {
        self.series.get(&id).map(|entry| entry.pane)
    }

    pub fn add_pane(&mut self) -> usize {
        let index = self.panes.len();
        let mut pane = Pane::new(index, self.options.pane.clone(), &self.options.layout);
        let effects = pane.set_width(self.time_scale.width());
        self.panes.push(pane);
        self.consume_pane_effects(effects);
        let mut mask = InvalidateMask::full();
        mask.invalidate_pane(
            index,
            PaneInvalidation {
                level: InvalidationLevel::None,
                auto_scale: true,
            },
        );
        self.queue_invalidation(mask);
        self.flush_effects();
        index
    }

    pub fn register_series(
        &mut self,
        id: SeriesId,
        kind: SeriesType,
        options: SeriesOptionsMap,
        pane: usize,
        position: PriceScalePosition,
    ) -> Result<(), ChartModelError> {
        self.validate_registration(id, pane)?;
        if kind == SeriesType::Custom {
            return Err(ChartModelError::CustomSeriesRequiresTypedRegistration);
        }
        let handle = Rc::new(RefCell::new(
            Series::new(id, kind, options).map_err(ChartModelError::Construction)?,
        ));
        let effects = self.panes[pane]
            .attach_series(handle.clone(), position)
            .map_err(ChartModelError::Pane)?;
        self.coordinator
            .insert_target(id, BuiltInSeriesTarget::new(handle.clone()));
        self.series.insert(
            id,
            RegisteredSeries {
                pane,
                state: RegisteredSeriesState::BuiltIn(handle),
            },
        );
        self.finish_registration(effects);
        Ok(())
    }

    pub fn register_custom_series<D: 'static>(
        &mut self,
        handle: CustomSeriesHandle<B::InternalItem, B::Item, D, M>,
        pane: usize,
        position: PriceScalePosition,
    ) -> Result<(), ChartModelError> {
        let id = handle.borrow().id();
        self.validate_registration(id, pane)?;
        let effects = self.panes[pane]
            .attach_custom_series(handle.clone(), position)
            .map_err(ChartModelError::Pane)?;
        self.coordinator
            .insert_target(id, CustomSeriesTarget::new(handle.clone()));
        self.series.insert(
            id,
            RegisteredSeries {
                pane,
                state: RegisteredSeriesState::Custom(handle),
            },
        );
        self.finish_registration(effects);
        Ok(())
    }

    fn validate_registration(&self, id: SeriesId, pane: usize) -> Result<(), ChartModelError> {
        if self.series.contains_key(&id) {
            return Err(ChartModelError::DuplicateSeries(id));
        }
        if pane >= self.panes.len() {
            return Err(ChartModelError::InvalidPane(pane));
        }
        Ok(())
    }
    fn finish_registration(&mut self, effects: Vec<PaneEffect>) {
        self.consume_pane_effects(effects);
        if self.series.len() == 1 {
            self.full_update();
        } else {
            self.light_update();
        }
        self.flush_effects();
    }

    /// The caller first obtains DataLayer's removal response, then hands it here.
    pub fn remove_series(
        &mut self,
        id: SeriesId,
        update: BuiltInDataUpdate<B::InternalItem, B::Item, M>,
    ) -> Result<(), ChartModelError> {
        let pane = self
            .pane_for_series(id)
            .ok_or(ChartModelError::UnknownSeries(id))?;
        self.validate_update(&update, Some(id))?;
        let effects = self.panes[pane]
            .detach_series(id)
            .map_err(ChartModelError::Pane)?;
        self.coordinator.remove_target(id);
        self.series.remove(&id);
        self.consume_pane_effects(effects);
        if self.panes.len() > 1
            && self.panes[pane].attached_source_ids().is_empty()
            && !self.panes[pane].preserve_empty_pane()
        {
            self.panes.remove(pane);
            for (index, remaining) in self.panes.iter_mut().enumerate() {
                remaining.set_index(index);
            }
            for entry in self.series.values_mut() {
                if entry.pane > pane {
                    entry.pane -= 1;
                }
            }
            if let Some(pending) = &mut self.pending {
                pending.remove_pane(pane);
            }
            self.full_update();
        }
        let effects = self.coordinator.apply(&mut self.time_scale, update);
        self.consume_data_effects(effects);
        self.flush_effects();
        Ok(())
    }

    pub fn apply_data_update(
        &mut self,
        update: BuiltInDataUpdate<B::InternalItem, B::Item, M>,
    ) -> Result<(), ChartModelError> {
        self.validate_update(&update, None)?;
        let effects = self.coordinator.apply(&mut self.time_scale, update);
        self.consume_data_effects(effects);
        self.flush_effects();
        Ok(())
    }

    pub fn apply_custom_data_update<D: 'static>(
        &mut self,
        handle: &CustomSeriesHandle<B::InternalItem, B::Item, D, M>,
        mut update: CustomDataUpdateResponse<B::InternalItem, B::Item, D, M>,
    ) -> Result<(), ChartModelError> {
        let id = handle.borrow().id();
        let entry = self
            .series
            .get(&id)
            .ok_or(ChartModelError::UnknownSeries(id))?;
        let RegisteredSeriesState::Custom(registered_handle) = &entry.state else {
            return Err(ChartModelError::NotCustomSeries(id));
        };
        let registered = registered_handle.downcast_ref::<RefCell<crate::model::series::CustomSeries<B::InternalItem, B::Item, D, M>>>();
        if !registered.is_some_and(|value| std::ptr::eq(value, &**handle)) {
            return Err(ChartModelError::CustomHandleMismatch(id));
        }
        let time_scale = std::mem::replace(
            &mut update.time_scale,
            TimeScaleChanges {
                points: None,
                first_changed_point_index: None,
                base_index: None,
            },
        );
        let shared = BuiltInDataUpdate {
            series: std::mem::take(&mut update.series),
            custom: update.custom.clone(),
            time_scale,
        };
        self.validate_update(&shared, None)?;
        let effects = self
            .coordinator
            .apply_with_rows(&mut self.time_scale, shared, || {
                handle.borrow_mut().apply_update(update)
            });
        self.consume_data_effects(effects);
        self.flush_effects();
        Ok(())
    }

    fn validate_update(
        &self,
        update: &BuiltInDataUpdate<B::InternalItem, B::Item, M>,
        removed: Option<SeriesId>,
    ) -> Result<(), ChartModelError> {
        for id in update.series.keys().chain(update.custom.keys()) {
            if Some(*id) != removed && !self.series.contains_key(id) {
                return Err(ChartModelError::UnknownSeries(*id));
            }
        }
        for id in update.series.keys() {
            if Some(*id) != removed && self.series[id].built_in().is_none() {
                return Err(ChartModelError::WrongUpdateKind(*id));
            }
        }
        for id in update.custom.keys() {
            if Some(*id) != removed && self.series[id].built_in().is_some() {
                return Err(ChartModelError::WrongUpdateKind(*id));
            }
        }
        Ok(())
    }

    pub fn set_width(&mut self, width: f64) -> Result<(), ChartModelError> {
        if !width.is_finite() || width < 0.0 {
            return Err(ChartModelError::InvalidDimensions);
        }
        let effects = self.time_scale.set_width(width);
        self.consume_time_effects(effects);
        for index in 0..self.panes.len() {
            let effects = self.panes[index].set_width(width);
            self.consume_pane_effects(effects);
        }
        self.flush_effects();
        Ok(())
    }
    pub fn set_pane_height(&mut self, pane: usize, height: f64) -> Result<(), ChartModelError> {
        if !height.is_finite() || height < 0.0 {
            return Err(ChartModelError::InvalidDimensions);
        }
        let effects = self
            .panes
            .get_mut(pane)
            .ok_or(ChartModelError::InvalidPane(pane))?
            .set_height(height);
        self.consume_pane_effects(effects);
        self.flush_effects();
        Ok(())
    }
    pub fn set_preserve_empty_pane(
        &mut self,
        pane: usize,
        preserve: bool,
    ) -> Result<(), ChartModelError> {
        let effects = self
            .panes
            .get_mut(pane)
            .ok_or(ChartModelError::InvalidPane(pane))?
            .set_preserve_empty_pane(preserve);
        self.consume_pane_effects(effects);
        self.flush_effects();
        Ok(())
    }

    pub fn fit_content(&mut self) {
        let mut mask = InvalidateMask::light();
        mask.fit_content();
        self.queue_invalidation(mask);
    }
    pub fn set_logical_range(&mut self, range: LogicalRange) {
        let mut mask = InvalidateMask::light();
        mask.apply_range(range);
        self.queue_invalidation(mask);
    }
    pub fn set_bar_spacing(&mut self, spacing: f64) {
        let mut mask = InvalidateMask::light();
        mask.set_bar_spacing(spacing);
        self.queue_invalidation(mask);
    }
    pub fn set_right_offset(&mut self, offset: f64) {
        let mut mask = InvalidateMask::light();
        mask.set_right_offset(offset);
        self.queue_invalidation(mask);
    }
    pub fn reset_time_scale(&mut self) {
        let mut mask = InvalidateMask::light();
        mask.reset_time_scale();
        self.queue_invalidation(mask);
    }
    pub fn set_time_scale_animation(&mut self, animation: AnimationRequest) {
        let mut mask = InvalidateMask::light();
        mask.set_animation(animation);
        self.queue_invalidation(mask);
    }
    pub fn stop_time_scale_animation(&mut self) {
        let mut mask = InvalidateMask::light();
        mask.stop_animation();
        self.queue_invalidation(mask);
    }
    pub fn light_update(&mut self) {
        self.queue_invalidation(InvalidateMask::light());
    }
    pub fn full_update(&mut self) {
        self.queue_invalidation(InvalidateMask::full());
    }

    pub fn queue_invalidation(&mut self, mask: InvalidateMask) {
        if !mask.time_scale_invalidations().is_empty() {
            self.animation_requests.clear();
        }
        for pane in &mut self.panes {
            pane.invalidate_grid();
        }
        match &mut self.pending {
            Some(pending) => pending.merge(&mask),
            None => self.pending = Some(mask),
        }
    }
    pub fn take_invalidation(&mut self) -> Option<InvalidateMask> {
        self.pending.take()
    }

    /// Executes model commands without consuming the retained frame mask.
    /// Animation sampling and continuation are handled by the UI frame owner.
    pub fn apply_invalidation(&mut self, mask: &InvalidateMask) {
        if mask.global_level() < InvalidationLevel::Light {
            return;
        }
        for index in 0..self.panes.len() {
            if mask.pane_invalidation(index).auto_scale {
                let effects = self.panes[index].momentary_auto_scale();
                self.consume_pane_effects(effects);
            }
        }
        for command in mask.time_scale_invalidations() {
            let effects = match command {
                TimeScaleInvalidation::FitContent => self.time_scale.fit_content(),
                TimeScaleInvalidation::ApplyRange(range) => {
                    self.time_scale.set_logical_range(*range)
                }
                TimeScaleInvalidation::ApplyBarSpacing(spacing) => {
                    self.time_scale.set_bar_spacing(*spacing)
                }
                TimeScaleInvalidation::ApplyRightOffset(offset) => {
                    self.time_scale.set_right_offset(*offset)
                }
                TimeScaleInvalidation::Reset => self.time_scale.restore_default(),
                TimeScaleInvalidation::Animation(_) | TimeScaleInvalidation::StopAnimation => {
                    continue;
                }
            };
            self.consume_time_effects(effects);
        }
        self.flush_effects();
        self.refresh_grids();
    }
    pub fn apply_animation_offset(&mut self, offset: f64) {
        let effects = self.time_scale.set_right_offset(offset);
        self.consume_time_effects(effects);
        self.flush_effects();
        self.refresh_grids();
    }
    pub fn take_scroll_animation_requests(&mut self) -> Vec<ScrollAnimation> {
        std::mem::take(&mut self.animation_requests)
    }
    pub fn scroll_to_offset_animated(&mut self, offset: f64, duration: std::time::Duration) {
        let effects = self.time_scale.scroll_to_offset_animated(offset, duration);
        self.consume_time_effects(effects);
        self.flush_effects();
    }
    pub fn take_options_applied(&mut self) -> bool {
        std::mem::take(&mut self.options_applied)
    }

    pub fn apply_time_scale_options(
        &mut self,
        data_layer: &mut DataLayer<B, M>,
        patch: HorzScaleOptionsPatch,
    ) where
        B::Converter: HorzScaleItemConverter<B::Item, B::InternalItem, Error = B::Error>,
    {
        let effects = self.time_scale.apply_options(patch);
        data_layer
            .behavior_mut()
            .update_scale_options(self.time_scale.options());
        data_layer
            .behavior_mut()
            .update_formatter(self.time_scale.localization());
        self.consume_time_effects(effects);
        self.recalculate_requested = true;
        self.full_update();
        self.flush_effects();
    }

    /// Builds equivalent owned options twice; neither B nor its callback-bearing
    /// options need Clone. Changes to time-key interpretation require new data.
    pub fn configure_horizontal_behavior(
        &mut self,
        data_layer: &mut DataLayer<B, M>,
        mut make_options: impl FnMut() -> B::Options,
    ) where
        B::Converter: HorzScaleItemConverter<B::Item, B::InternalItem, Error = B::Error>,
    {
        data_layer.behavior_mut().set_options(make_options());
        data_layer
            .behavior_mut()
            .update_scale_options(self.time_scale.options());
        data_layer
            .behavior_mut()
            .update_formatter(self.time_scale.localization());
        let effects = self.time_scale.configure_behavior(make_options());
        self.consume_time_effects(effects);
        self.flush_effects();
    }

    fn consume_pane_effects(&mut self, effects: Vec<PaneEffect>) {
        for effect in effects {
            match effect {
                PaneEffect::RecalculateAllPanes => self.recalculate_requested = true,
                PaneEffect::LightUpdate => self.light_update(),
            }
        }
    }
    fn consume_data_effects(&mut self, effects: Vec<ChartDataEffect>) {
        for effect in effects {
            match effect {
                ChartDataEffect::RecalculateAllPanes => self.recalculate_requested = true,
                ChartDataEffect::LightUpdate => self.light_update(),
            }
        }
    }
    fn consume_time_effects(&mut self, effects: Vec<TimeScaleEffect>) {
        for effect in effects {
            match effect {
                TimeScaleEffect::RecalculateAllPanes => self.recalculate_requested = true,
                TimeScaleEffect::LightUpdate => self.light_update(),
                TimeScaleEffect::StartScrollAnimation(animation) => {
                    self.animation_requests.push(animation);
                    self.light_update();
                }
                TimeScaleEffect::OptionsApplied => {
                    self.options_applied = true;
                    self.light_update();
                }
            }
        }
    }
    fn flush_effects(&mut self) {
        if !std::mem::take(&mut self.recalculate_requested) {
            return;
        }
        let visible = self.time_scale.visible_strict_range();
        let mut redraw = false;
        for pane in &mut self.panes {
            let effects = pane.recalculate(visible.as_ref());
            // Pane emits RecalculateAllPanes when its range changes; this pass
            // already visits every scale with the same viewport snapshot.
            redraw |= effects.contains(&PaneEffect::LightUpdate);
        }
        if redraw {
            self.light_update();
        }
    }
    fn refresh_grids(&mut self) {
        let layout = self.layout_context();
        let marks = self.time_scale.marks(layout).unwrap_or_default().to_vec();
        for pane in &mut self.panes {
            pane.refresh_grid(&self.options.grid, &marks);
        }
    }
}

impl<B, M> TimeScaleHost for ChartModel<B, M>
where
    B: HorzScaleBehavior,
    B::InternalItem: 'static,
    B::Item: 'static,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone + 'static,
{
    fn layout_context(&self) -> TimeScaleLayoutContext {
        TimeScaleLayoutContext {
            font_size: self.options.layout.font_size,
        }
    }
    fn fulfilled_time_indices(&self) -> BTreeSet<usize> {
        self.coordinator.fulfilled_indices().clone()
    }
    fn apply_time_scale_effects(&mut self, effects: Vec<TimeScaleEffect>) {
        self.consume_time_effects(effects);
        self.flush_effects();
    }
}

#[cfg(test)]
mod tests;
