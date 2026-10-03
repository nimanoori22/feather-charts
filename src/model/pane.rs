//! Synchronous Pane ownership and visible-range price-scale recalculation.

use crate::model::{
    autoscale_info_impl::AutoscaleInfoImpl,
    coordinate::Coordinate,
    data_layer::SeriesId,
    grid::{Grid, GridOptions, UpdateType},
    ihorz_scale_behavior::TimeMark,
    iprice_data_source::{FirstValue, PriceScaleDataSource},
    layout_options::LayoutOptions,
    price_scale::{PriceScale, PriceScaleOptions, PriceScaleStatePatch, PriceSourceHandle},
    price_scale_visible_range::PriceScaleVisibleRange,
    range_impl::RangeImpl,
    series::{CustomSeriesHandle, CustomSeriesPriceScaleSource, Series},
    sort_sources::{ZOrdered, sort_sources},
    time_data::TimePointIndex,
};
use std::{cell::RefCell, collections::BTreeMap, marker::PhantomData, rc::Rc};

pub const DEFAULT_STRETCH_FACTOR: f64 = 1.0;
pub const MIN_PANE_HEIGHT: f64 = 30.0;
pub const LEFT_PRICE_SCALE_ID: &str = "left";
pub const RIGHT_PRICE_SCALE_ID: &str = "right";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PriceScalePosition {
    Left,
    Right,
    Overlay(String),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DefaultVisiblePriceScale {
    Left,
    #[default]
    Right,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaneOptions {
    pub left_price_scale: PriceScaleOptions,
    pub right_price_scale: PriceScaleOptions,
    pub overlay_price_scale: PriceScaleOptions,
    pub default_visible_price_scale: DefaultVisiblePriceScale,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaneEffect {
    RecalculateAllPanes,
    LightUpdate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PaneError {
    SeriesAlreadyAttached(SeriesId),
    UnknownSeries(SeriesId),
    UnknownPriceScale(PriceScalePosition),
}

pub type SeriesHandle<I, O, M = ()> = Rc<RefCell<Series<I, O, M>>>;

/// Long-lived PriceScale adapter. It owns only a Series handle and a
/// Pane-managed visible-range snapshot; neither object refers back to Pane.
struct PaneSeriesPriceSource<I, O, M = ()> {
    series: SeriesHandle<I, O, M>,
    visible: PriceScaleVisibleRange,
}

impl<I, O, M> PriceScaleDataSource for PaneSeriesPriceSource<I, O, M>
where
    I: Clone + 'static,
    O: Clone + 'static,
    M: Clone + 'static,
{
    fn z_order(&self) -> i32 {
        self.series.borrow().z_order()
    }
    fn set_z_order(&mut self, z_order: i32) {
        self.series.borrow_mut().set_z_order(z_order);
    }
    fn visible(&self) -> bool {
        self.series.borrow().visible()
    }
    fn first_value(&self) -> Option<FirstValue> {
        let visible = self.visible.get();
        self.series.borrow().first_value(visible.as_ref())
    }
    fn format_price(&self, price: f64) -> String {
        self.series.borrow().format_price(price)
    }
    fn base(&self) -> f64 {
        self.series.borrow().base()
    }
    fn autoscale_info(
        &self,
        start: TimePointIndex,
        end: TimePointIndex,
    ) -> Option<AutoscaleInfoImpl> {
        self.series.borrow().autoscale_info(start, end)
    }
    fn update_all_views(&mut self) {}
}

struct AttachedSource {
    id: SeriesId,
    source: PriceSourceHandle,
    position: PriceScalePosition,
}

impl ZOrdered for AttachedSource {
    fn z_order(&self) -> i32 {
        self.source.borrow().z_order()
    }
}

struct OrderedSource {
    id: SeriesId,
    z_order: i32,
}

impl ZOrdered for OrderedSource {
    fn z_order(&self) -> i32 {
        self.z_order
    }
}

/// Model-only Pane core. Rendering, primitives, subscriptions, and Iced stay
/// outside this ownership boundary.
pub struct Pane<I = (), O = (), M = ()> {
    index: usize,
    options: PaneOptions,
    width: f64,
    height: f64,
    stretch_factor: f64,
    preserve_empty_pane: bool,
    layout_font_size: f64,
    left_price_scale: PriceScale,
    right_price_scale: PriceScale,
    overlay_price_scales: BTreeMap<String, PriceScale>,
    grid: Grid,
    grid_price_marks_generation: Option<u64>,
    attached_sources: Vec<AttachedSource>,
    ordered_source_ids_cache: Option<Vec<SeriesId>>,
    visible_range: PriceScaleVisibleRange,
    invalidation_generation: u64,
    marker: PhantomData<(I, O, M)>,
}

impl<I, O, M> Pane<I, O, M>
where
    I: Clone + 'static,
    O: Clone + 'static,
    M: Clone + 'static,
{
    pub fn new(index: usize, options: PaneOptions, layout: &LayoutOptions) -> Self {
        Self {
            index,
            left_price_scale: PriceScale::new(
                LEFT_PRICE_SCALE_ID,
                options.left_price_scale.clone(),
                layout,
            ),
            right_price_scale: PriceScale::new(
                RIGHT_PRICE_SCALE_ID,
                options.right_price_scale.clone(),
                layout,
            ),
            options,
            width: 0.0,
            height: 0.0,
            stretch_factor: DEFAULT_STRETCH_FACTOR,
            preserve_empty_pane: false,
            layout_font_size: layout.font_size,
            overlay_price_scales: BTreeMap::new(),
            grid: Grid::new(),
            grid_price_marks_generation: None,
            attached_sources: vec![],
            ordered_source_ids_cache: None,
            visible_range: PriceScaleVisibleRange::default(),
            invalidation_generation: 0,
            marker: PhantomData,
        }
    }

    pub const fn index(&self) -> usize {
        self.index
    }
    pub(crate) fn set_index(&mut self, index: usize) {
        self.index = index;
    }
    pub const fn width(&self) -> f64 {
        self.width
    }
    pub const fn height(&self) -> f64 {
        self.height
    }
    pub const fn stretch_factor(&self) -> f64 {
        self.stretch_factor
    }
    pub const fn preserve_empty_pane(&self) -> bool {
        self.preserve_empty_pane
    }
    pub const fn invalidation_generation(&self) -> u64 {
        self.invalidation_generation
    }

    pub fn set_stretch_factor(&mut self, factor: f64) -> Vec<PaneEffect> {
        if !factor.is_finite() || factor <= 0.0 || self.stretch_factor == factor {
            return vec![];
        }
        self.stretch_factor = factor;
        self.invalidate()
    }

    pub fn set_preserve_empty_pane(&mut self, preserve: bool) -> Vec<PaneEffect> {
        if self.preserve_empty_pane == preserve {
            return vec![];
        }
        self.preserve_empty_pane = preserve;
        self.invalidate()
    }

    pub fn set_width(&mut self, width: f64) -> Vec<PaneEffect> {
        let width = width.max(0.0);
        if self.width == width {
            return vec![];
        }
        self.width = width;
        self.invalidate()
    }

    pub fn set_height(&mut self, height: f64) -> Vec<PaneEffect> {
        let height = height.max(0.0);
        if self.height == height {
            return vec![];
        }
        self.height = height;
        self.left_price_scale.set_height(height);
        self.right_price_scale.set_height(height);
        for scale in self.overlay_price_scales.values_mut() {
            scale.set_height(height);
        }
        self.invalidate()
    }

    pub fn left_price_scale(&self) -> &PriceScale {
        &self.left_price_scale
    }
    pub fn right_price_scale(&self) -> &PriceScale {
        &self.right_price_scale
    }
    pub(crate) fn axis_price_scale_mut(
        &mut self,
        side: crate::model::axis_snapshots::PriceAxisSide,
    ) -> &mut PriceScale {
        match side {
            crate::model::axis_snapshots::PriceAxisSide::Left => &mut self.left_price_scale,
            crate::model::axis_snapshots::PriceAxisSide::Right => &mut self.right_price_scale,
        }
    }
    pub(crate) fn set_font_size(&mut self, font_size: f64) {
        self.layout_font_size = font_size;
        self.left_price_scale.set_font_size(font_size);
        self.right_price_scale.set_font_size(font_size);
        for scale in self.overlay_price_scales.values_mut() {
            scale.set_font_size(font_size);
        }
        self.invalidate_grid();
    }
    pub fn grid(&self) -> &Grid {
        &self.grid
    }
    pub fn grid_mut(&mut self) -> &mut Grid {
        &mut self.grid
    }
    pub fn invalidate_grid(&mut self) {
        self.grid.pane_view_mut().update(UpdateType::Other);
    }

    /// Refreshes the Grid's lazy render snapshot. The chart owner supplies
    /// TimeScale marks because Pane deliberately does not own TimeScale.
    pub fn refresh_grid(&mut self, options: &GridOptions, time_marks: &[TimeMark]) {
        let position = self.default_price_scale_position();
        let (price_marks, marks_generation) = {
            let scale = self
                .price_scale_mut_existing(&position)
                .expect("the selected default price scale always exists");
            (scale.marks().to_vec(), scale.marks_generation())
        };
        if self.grid_price_marks_generation != Some(marks_generation) {
            self.grid.pane_view_mut().update(UpdateType::Data);
            self.grid_price_marks_generation = Some(marks_generation);
        }
        self.grid
            .pane_view_mut()
            .refresh(options, &price_marks, time_marks);
    }

    pub fn price_scale_by_id(&self, id: &str) -> Option<&PriceScale> {
        match id {
            LEFT_PRICE_SCALE_ID => Some(&self.left_price_scale),
            RIGHT_PRICE_SCALE_ID => Some(&self.right_price_scale),
            _ => self.overlay_price_scales.get(id),
        }
    }

    pub(crate) fn price_scale_for_source(&self, id: SeriesId) -> Option<&PriceScale> {
        self.price_scale_existing(self.source_position(id)?)
    }

    pub fn default_visible_price_scale(&self) -> Option<&PriceScale> {
        let (primary, secondary) = self.default_price_scale_positions();
        if self
            .price_scale_existing(&primary)
            .is_some_and(|scale| scale.options().visible)
        {
            return self.price_scale_existing(&primary);
        }
        if self
            .price_scale_existing(&secondary)
            .is_some_and(|scale| scale.options().visible)
        {
            return self.price_scale_existing(&secondary);
        }
        None
    }

    pub fn default_price_scale(&self) -> &PriceScale {
        let position = self.default_price_scale_position();
        self.price_scale_existing(&position)
            .expect("the selected default price scale always exists")
    }

    fn default_price_scale_position(&self) -> PriceScalePosition {
        let (primary, secondary) = self.default_price_scale_positions();
        if self.attached_to(&primary)
            && self
                .price_scale_existing(&primary)
                .is_some_and(|scale| scale.options().visible)
        {
            return primary;
        }
        if self.attached_to(&secondary)
            && self
                .price_scale_existing(&secondary)
                .is_some_and(|scale| scale.options().visible)
        {
            return secondary;
        }
        if let Some(source) = self.attached_sources.first() {
            return source.position.clone();
        }
        if self
            .price_scale_existing(&primary)
            .is_some_and(|scale| scale.options().visible)
        {
            primary
        } else if self
            .price_scale_existing(&secondary)
            .is_some_and(|scale| scale.options().visible)
        {
            secondary
        } else {
            primary
        }
    }

    pub fn attached_source_ids(&self) -> Vec<SeriesId> {
        self.attached_sources
            .iter()
            .map(|attached| attached.id)
            .collect()
    }

    pub fn ordered_source_ids(&mut self) -> &[SeriesId] {
        if self.ordered_source_ids_cache.is_none() {
            let mut sources = self
                .attached_sources
                .iter()
                .map(|source| OrderedSource {
                    id: source.id,
                    z_order: source.z_order(),
                })
                .collect::<Vec<_>>();
            sort_sources(&mut sources);
            self.ordered_source_ids_cache =
                Some(sources.into_iter().map(|source| source.id).collect());
        }
        self.ordered_source_ids_cache
            .as_deref()
            .expect("the cache was initialized above")
    }

    pub fn ordered_sources(&mut self) -> Vec<PriceSourceHandle> {
        let ids = self.ordered_source_ids().to_vec();
        ids.into_iter()
            .filter_map(|id| {
                self.source_index(id)
                    .map(|index| self.attached_sources[index].source.clone())
            })
            .collect()
    }

    pub fn attach_series(
        &mut self,
        series: SeriesHandle<I, O, M>,
        position: PriceScalePosition,
    ) -> Result<Vec<PaneEffect>, PaneError> {
        let id = series.borrow().id();
        if self.contains_source(id) {
            return Err(PaneError::SeriesAlreadyAttached(id));
        }
        series
            .borrow_mut()
            .set_z_order(self.attached_sources.len() as i32);
        let source: PriceSourceHandle = Rc::new(RefCell::new(PaneSeriesPriceSource {
            series: series.clone(),
            visible: self.visible_range.clone(),
        }));
        self.price_scale_mut(&position)
            .add_data_source(source.clone());
        self.attached_sources.push(AttachedSource {
            id,
            source,
            position: position.clone(),
        });
        self.invalidate_source_order_cache();
        let mut effects = self.invalidate();
        let visible = self.visible_range.get();
        effects.extend(self.recalculate_price_scale(position, visible.as_ref()));
        Ok(deduplicate_effects(effects))
    }

    pub fn attach_custom_series<D>(
        &mut self,
        series: CustomSeriesHandle<I, O, D, M>,
        position: PriceScalePosition,
    ) -> Result<Vec<PaneEffect>, PaneError>
    where
        D: 'static,
    {
        let id = series.borrow().id();
        if self.contains_source(id) {
            return Err(PaneError::SeriesAlreadyAttached(id));
        }
        series
            .borrow_mut()
            .set_z_order(self.attached_sources.len() as i32);
        let source: PriceSourceHandle = Rc::new(RefCell::new(CustomSeriesPriceScaleSource::new(
            series,
            self.visible_range.clone(),
        )));
        self.price_scale_mut(&position)
            .add_data_source(source.clone());
        self.attached_sources.push(AttachedSource {
            id,
            source,
            position: position.clone(),
        });
        self.invalidate_source_order_cache();
        let mut effects = self.invalidate();
        let visible = self.visible_range.get();
        effects.extend(self.recalculate_price_scale(position, visible.as_ref()));
        Ok(deduplicate_effects(effects))
    }

    pub fn detach_series(&mut self, id: SeriesId) -> Result<Vec<PaneEffect>, PaneError> {
        self.detach_source(id)
    }

    pub fn detach_custom_series(&mut self, id: SeriesId) -> Result<Vec<PaneEffect>, PaneError> {
        self.detach_source(id)
    }

    pub fn move_series_to_scale(
        &mut self,
        id: SeriesId,
        position: PriceScalePosition,
    ) -> Result<Vec<PaneEffect>, PaneError> {
        self.move_source_to_scale(id, position)
    }

    pub fn move_custom_series_to_scale(
        &mut self,
        id: SeriesId,
        position: PriceScalePosition,
    ) -> Result<Vec<PaneEffect>, PaneError> {
        self.move_source_to_scale(id, position)
    }

    pub fn start_scale_price(
        &mut self,
        position: PriceScalePosition,
        pointer: Coordinate,
    ) -> Result<Vec<PaneEffect>, PaneError> {
        let scale = self
            .price_scale_mut_existing(&position)
            .ok_or_else(|| PaneError::UnknownPriceScale(position.clone()))?;
        scale.start_scale(pointer);
        Ok(vec![])
    }

    pub fn scale_price_to(
        &mut self,
        position: PriceScalePosition,
        pointer: Coordinate,
    ) -> Result<Vec<PaneEffect>, PaneError> {
        let changed = self
            .price_scale_mut_existing(&position)
            .ok_or_else(|| PaneError::UnknownPriceScale(position.clone()))?
            .scale_to(pointer);
        Ok(self.update_after_price_interaction(changed))
    }

    pub fn end_scale_price(
        &mut self,
        position: PriceScalePosition,
    ) -> Result<Vec<PaneEffect>, PaneError> {
        let scale = self
            .price_scale_mut_existing(&position)
            .ok_or_else(|| PaneError::UnknownPriceScale(position.clone()))?;
        scale.end_scale();
        Ok(vec![])
    }

    pub fn start_scroll_price(
        &mut self,
        position: PriceScalePosition,
        pointer: Coordinate,
    ) -> Result<Vec<PaneEffect>, PaneError> {
        let scale = self
            .price_scale_mut_existing(&position)
            .ok_or_else(|| PaneError::UnknownPriceScale(position.clone()))?;
        scale.start_scroll(pointer);
        Ok(vec![])
    }

    pub fn scroll_price_to(
        &mut self,
        position: PriceScalePosition,
        pointer: Coordinate,
    ) -> Result<Vec<PaneEffect>, PaneError> {
        let changed = self
            .price_scale_mut_existing(&position)
            .ok_or_else(|| PaneError::UnknownPriceScale(position.clone()))?
            .scroll_to(pointer);
        Ok(self.update_after_price_interaction(changed))
    }

    pub fn end_scroll_price(
        &mut self,
        position: PriceScalePosition,
    ) -> Result<Vec<PaneEffect>, PaneError> {
        let scale = self
            .price_scale_mut_existing(&position)
            .ok_or_else(|| PaneError::UnknownPriceScale(position.clone()))?;
        scale.end_scroll();
        Ok(vec![])
    }

    pub fn reset_price_scale(
        &mut self,
        position: PriceScalePosition,
    ) -> Result<Vec<PaneEffect>, PaneError> {
        let visible = self.visible_range.get();
        let changed = {
            let scale = self
                .price_scale_mut_existing(&position)
                .ok_or_else(|| PaneError::UnknownPriceScale(position.clone()))?;
            let enabled_auto_scale = !scale
                .set_mode(PriceScaleStatePatch {
                    auto_scale: Some(true),
                    ..PriceScaleStatePatch::default()
                })
                .old
                .auto_scale;
            let recalculated = visible
                .as_ref()
                .is_some_and(|range| scale.recalculate_price_range(range));
            enabled_auto_scale || recalculated
        };
        Ok(self.update_after_price_interaction(changed))
    }

    pub fn momentary_auto_scale(&mut self) -> Vec<PaneEffect> {
        let visible = self.visible_range.get();
        let Some(visible) = visible.as_ref() else {
            return vec![];
        };
        let left_changed = self
            .left_price_scale
            .recalculate_price_range_forced(visible);
        let right_changed = self
            .right_price_scale
            .recalculate_price_range_forced(visible);
        self.update_after_price_interaction(left_changed || right_changed)
    }

    pub fn recalculate(&mut self, visible: Option<&RangeImpl<TimePointIndex>>) -> Vec<PaneEffect> {
        self.set_visible_range(visible);
        let mut positions = vec![PriceScalePosition::Left, PriceScalePosition::Right];
        positions.extend(
            self.overlay_price_scales
                .keys()
                .cloned()
                .map(PriceScalePosition::Overlay),
        );
        let mut effects = vec![];
        for position in positions {
            effects.extend(self.recalculate_price_scale(position, visible));
        }
        deduplicate_effects(effects)
    }

    pub fn recalculate_price_scale(
        &mut self,
        position: PriceScalePosition,
        visible: Option<&RangeImpl<TimePointIndex>>,
    ) -> Vec<PaneEffect> {
        self.set_visible_range(visible);
        let before = match self.price_scale_mut_existing(&position) {
            Some(scale) => scale.price_range(),
            None => return vec![],
        };
        let has_sources = self.attached_to(&position);
        let scale = self
            .price_scale_mut_existing(&position)
            .expect("the checked price scale remains attached");
        match visible {
            Some(visible) if has_sources => {
                scale.recalculate_price_range(visible);
            }
            _ => scale.set_price_range(None),
        }
        if scale.price_range() == before {
            vec![]
        } else {
            self.invalidate()
        }
    }

    fn set_visible_range(&self, visible: Option<&RangeImpl<TimePointIndex>>) {
        self.visible_range.set(visible);
    }

    fn update_after_price_interaction(&mut self, changed: bool) -> Vec<PaneEffect> {
        if !changed {
            return vec![];
        }
        for source in &self.attached_sources {
            source.source.borrow_mut().update_all_views();
        }
        vec![PaneEffect::LightUpdate]
    }

    pub fn set_series_order(
        &mut self,
        id: SeriesId,
        requested_order: usize,
    ) -> Result<Vec<PaneEffect>, PaneError> {
        let index = self.source_index(id).ok_or(PaneError::UnknownSeries(id))?;
        let target = requested_order.min(self.attached_sources.len().saturating_sub(1));
        if index == target {
            return Ok(vec![]);
        }
        let source = self.attached_sources.remove(index);
        self.attached_sources.insert(target, source);
        self.renumber_source_z_orders();
        self.refresh_formatter_sources();
        self.invalidate_source_order_cache();
        Ok(vec![PaneEffect::LightUpdate])
    }

    fn detach_source(&mut self, id: SeriesId) -> Result<Vec<PaneEffect>, PaneError> {
        let index = self.source_index(id).ok_or(PaneError::UnknownSeries(id))?;
        let attached = self.attached_sources.remove(index);
        let position = attached.position.clone();
        if let Some(scale) = self.price_scale_mut_existing(&position) {
            scale.remove_data_source(&attached.source);
        }
        self.renumber_source_z_orders();
        self.invalidate_source_order_cache();
        self.remove_empty_overlay(&position);
        let mut effects = self.invalidate();
        let visible = self.visible_range.get();
        effects.extend(self.recalculate_price_scale(position, visible.as_ref()));
        Ok(deduplicate_effects(effects))
    }

    fn move_source_to_scale(
        &mut self,
        id: SeriesId,
        position: PriceScalePosition,
    ) -> Result<Vec<PaneEffect>, PaneError> {
        let index = self.source_index(id).ok_or(PaneError::UnknownSeries(id))?;
        let old_position = self.attached_sources[index].position.clone();
        if old_position == position {
            return Ok(vec![]);
        }
        let source = self.attached_sources[index].source.clone();
        if let Some(scale) = self.price_scale_mut_existing(&old_position) {
            scale.remove_data_source(&source);
        }
        self.attached_sources[index].position = position.clone();
        self.invalidate_source_order_cache();
        self.remove_empty_overlay(&old_position);
        self.price_scale_mut(&position).add_data_source(source);
        let mut effects = self.invalidate();
        let visible = self.visible_range.get();
        effects.extend(self.recalculate_price_scale(old_position, visible.as_ref()));
        effects.extend(self.recalculate_price_scale(position, visible.as_ref()));
        Ok(deduplicate_effects(effects))
    }

    fn source_index(&self, id: SeriesId) -> Option<usize> {
        self.attached_sources
            .iter()
            .position(|attached| attached.id == id)
    }

    fn source_position(&self, id: SeriesId) -> Option<&PriceScalePosition> {
        self.source_index(id)
            .map(|index| &self.attached_sources[index].position)
    }

    fn contains_source(&self, id: SeriesId) -> bool {
        self.source_position(id).is_some()
    }

    fn attached_to(&self, position: &PriceScalePosition) -> bool {
        self.attached_sources
            .iter()
            .any(|attached| &attached.position == position)
    }

    pub(crate) fn refresh_formatter_sources(&mut self) {
        self.left_price_scale.refresh_formatter_source();
        self.right_price_scale.refresh_formatter_source();
        for scale in self.overlay_price_scales.values_mut() {
            scale.refresh_formatter_source();
        }
    }

    fn invalidate_source_order_cache(&mut self) {
        self.ordered_source_ids_cache = None;
    }

    fn renumber_source_z_orders(&mut self) {
        for (index, source) in self.attached_sources.iter().enumerate() {
            source.source.borrow_mut().set_z_order(index as i32);
        }
    }

    fn default_price_scale_positions(&self) -> (PriceScalePosition, PriceScalePosition) {
        match self.options.default_visible_price_scale {
            DefaultVisiblePriceScale::Left => (PriceScalePosition::Left, PriceScalePosition::Right),
            DefaultVisiblePriceScale::Right => {
                (PriceScalePosition::Right, PriceScalePosition::Left)
            }
        }
    }

    fn price_scale_existing(&self, position: &PriceScalePosition) -> Option<&PriceScale> {
        match position {
            PriceScalePosition::Left => Some(&self.left_price_scale),
            PriceScalePosition::Right => Some(&self.right_price_scale),
            PriceScalePosition::Overlay(id) => self.overlay_price_scales.get(id),
        }
    }

    fn price_scale_mut_existing(
        &mut self,
        position: &PriceScalePosition,
    ) -> Option<&mut PriceScale> {
        match position {
            PriceScalePosition::Left => Some(&mut self.left_price_scale),
            PriceScalePosition::Right => Some(&mut self.right_price_scale),
            PriceScalePosition::Overlay(id) => self.overlay_price_scales.get_mut(id),
        }
    }

    fn price_scale_mut(&mut self, position: &PriceScalePosition) -> &mut PriceScale {
        match position {
            PriceScalePosition::Left => &mut self.left_price_scale,
            PriceScalePosition::Right => &mut self.right_price_scale,
            PriceScalePosition::Overlay(id) => {
                let options = self.options.overlay_price_scale.clone();
                let height = self.height;
                let font_size = self.layout_font_size;
                self.overlay_price_scales
                    .entry(id.clone())
                    .or_insert_with(|| {
                        let mut scale = PriceScale::new_with_font_size(id, options, font_size);
                        scale.set_height(height);
                        scale
                    })
            }
        }
    }

    fn remove_empty_overlay(&mut self, position: &PriceScalePosition) {
        let PriceScalePosition::Overlay(id) = position else {
            return;
        };
        if !self.attached_to(position) {
            self.overlay_price_scales.remove(id);
        }
    }

    fn invalidate(&mut self) -> Vec<PaneEffect> {
        self.invalidate_grid();
        self.invalidation_generation = self.invalidation_generation.wrapping_add(1);
        vec![PaneEffect::RecalculateAllPanes, PaneEffect::LightUpdate]
    }
}

fn deduplicate_effects(effects: Vec<PaneEffect>) -> Vec<PaneEffect> {
    let mut unique = vec![];
    for effect in effects {
        if !unique.contains(&effect) {
            unique.push(effect);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::{
            autoscale_info_impl::AutoscaleInfo,
            layout_options::{Background, ColorSpace, LayoutPanesOptions},
            plot_data::PlotRow,
            price_range_impl::PriceRange,
            series::CustomSeries,
            series_data::{CustomPlotRow, HistogramPlotRow, LinePlotRow, SeriesPlotRow},
            series_options::{
                AutoscaleInfoProvider, CustomSeriesOptions, CustomStyleOptions,
                HistogramStyleOptions, LastPriceAnimationMode, LineStyleOptions,
                PriceAxisLastValueMode, PriceFormat, PriceFormatBuiltIn, PriceFormatBuiltInType,
                PriceLineSource, SeriesOptions, SeriesOptionsCommon, SeriesOptionsMap, SeriesType,
            },
        },
        renderers::draw_line::{LineStyle, LineType, LineWidth},
    };

    fn layout() -> LayoutOptions {
        LayoutOptions {
            background: Background::Solid {
                color: String::new(),
            },
            text_color: String::new(),
            font_size: 12.0,
            font_family: String::new(),
            panes: LayoutPanesOptions {
                enable_resize: true,
                separator_color: String::new(),
                separator_hover_color: String::new(),
            },
            attribution_logo: false,
            color_space: ColorSpace::Srgb,
            color_parsers: vec![],
        }
    }

    fn common() -> SeriesOptionsCommon {
        SeriesOptionsCommon {
            last_value_visible: true,
            title: String::new(),
            price_scale_id: None,
            series_last_value_mode: Some(PriceAxisLastValueMode::LastValueAccordingToScale),
            visible: true,
            hit_test_tolerance: 3.0,
            price_line_visible: true,
            price_line_source: PriceLineSource::LastBar,
            price_line_width: LineWidth::One,
            price_line_color: String::new(),
            price_line_style: LineStyle::Solid,
            price_format: PriceFormat::BuiltIn(PriceFormatBuiltIn {
                kind: PriceFormatBuiltInType::Price,
                precision: 2,
                min_move: 0.01,
                base: Some(100.0),
            }),
            base_line_visible: true,
            base_line_color: String::new(),
            base_line_width: LineWidth::One,
            base_line_style: LineStyle::Solid,
            autoscale_info_provider: None,
            conflation_threshold_factor: None,
        }
    }

    fn line_options() -> SeriesOptionsMap {
        SeriesOptionsMap::Line(SeriesOptions {
            common: common(),
            style: LineStyleOptions {
                color: String::new(),
                line_style: LineStyle::Solid,
                line_width: LineWidth::One,
                line_type: LineType::Simple,
                line_visible: true,
                point_markers_visible: false,
                point_markers_radius: None,
                crosshair_marker_visible: true,
                crosshair_marker_radius: 4.0,
                crosshair_marker_border_color: String::new(),
                crosshair_marker_background_color: String::new(),
                crosshair_marker_border_width: 2.0,
                last_price_animation: LastPriceAnimationMode::Disabled,
            },
        })
    }

    fn histogram_options(base: f64) -> SeriesOptionsMap {
        SeriesOptionsMap::Histogram(SeriesOptions {
            common: common(),
            style: HistogramStyleOptions {
                color: String::new(),
                base,
            },
        })
    }

    fn custom_options() -> CustomSeriesOptions {
        SeriesOptions {
            common: common(),
            style: CustomStyleOptions {
                color: String::new(),
            },
        }
    }

    fn line_handle(id: u64, close: f64) -> SeriesHandle<(), (), ()> {
        let mut series = Series::new(SeriesId::new(id), SeriesType::Line, line_options()).unwrap();
        series.set_data(
            vec![SeriesPlotRow::Line(LinePlotRow {
                base: PlotRow {
                    index: 0.0.into(),
                    time: (),
                    original_time: (),
                    value: [0.0, 100.0, -100.0, close],
                    custom_values: None,
                    original_data_count: None,
                },
                color: None,
            })],
            None,
        );
        Rc::new(RefCell::new(series))
    }

    fn histogram_handle(id: u64, base: f64, close: f64) -> SeriesHandle<(), (), ()> {
        let mut series = Series::new(
            SeriesId::new(id),
            SeriesType::Histogram,
            histogram_options(base),
        )
        .unwrap();
        series.set_data(
            vec![SeriesPlotRow::Histogram(HistogramPlotRow {
                base: PlotRow {
                    index: 0.0.into(),
                    time: (),
                    original_time: (),
                    value: [0.0, 100.0, -100.0, close],
                    custom_values: None,
                    original_data_count: None,
                },
                color: None,
            })],
            None,
        );
        Rc::new(RefCell::new(series))
    }

    fn custom_handle(id: u64, values: [f64; 4]) -> CustomSeriesHandle<(), (), &'static str, ()> {
        let mut series = CustomSeries::new(SeriesId::new(id), custom_options());
        series.set_data(
            vec![CustomPlotRow {
                base: PlotRow {
                    index: 0.0.into(),
                    time: (),
                    original_time: (),
                    value: values,
                    custom_values: None,
                    original_data_count: None,
                },
                data: "typed payload",
                color: None,
            }],
            None,
        );
        Rc::new(RefCell::new(series))
    }

    fn pane_options(
        preferred: DefaultVisiblePriceScale,
        left_visible: bool,
        right_visible: bool,
    ) -> PaneOptions {
        let mut options = PaneOptions {
            default_visible_price_scale: preferred,
            ..PaneOptions::default()
        };
        options.left_price_scale.visible = left_visible;
        options.right_price_scale.visible = right_visible;
        options
    }

    #[test]
    fn creates_resizes_attaches_moves_and_removes_sources() {
        let mut pane = Pane::new(2, PaneOptions::default(), &layout());
        assert_eq!(pane.index(), 2);
        assert_eq!(pane.stretch_factor(), DEFAULT_STRETCH_FACTOR);
        assert!(pane.set_width(0.0).is_empty());
        pane.set_height(120.0);
        assert_eq!(pane.left_price_scale().height(), 120.0);

        let series = line_handle(1, 10.0);
        assert_eq!(
            pane.attach_series(series.clone(), PriceScalePosition::Left)
                .unwrap(),
            vec![PaneEffect::RecalculateAllPanes, PaneEffect::LightUpdate]
        );
        assert!(matches!(
            pane.attach_series(series, PriceScalePosition::Left),
            Err(PaneError::SeriesAlreadyAttached(_))
        ));
        pane.move_series_to_scale(
            SeriesId::new(1),
            PriceScalePosition::Overlay("study".into()),
        )
        .unwrap();
        assert!(pane.price_scale_by_id("study").is_some());
        assert_eq!(pane.price_scale_by_id("study").unwrap().height(), 120.0);
        pane.move_series_to_scale(SeriesId::new(1), PriceScalePosition::Right)
            .unwrap();
        assert!(pane.price_scale_by_id("study").is_none());
        pane.detach_series(SeriesId::new(1)).unwrap();
        assert!(pane.attached_source_ids().is_empty());
    }

    #[test]
    fn recalculates_default_and_overlay_ranges_from_the_visible_interval() {
        let mut pane = Pane::new(0, PaneOptions::default(), &layout());
        pane.set_height(100.0);
        pane.attach_series(line_handle(1, 10.0), PriceScalePosition::Left)
            .unwrap();
        pane.attach_series(
            histogram_handle(2, 5.0, 20.0),
            PriceScalePosition::Overlay("h".into()),
        )
        .unwrap();
        let visible = RangeImpl::new(0.0.into(), 0.0.into());
        assert_eq!(
            pane.recalculate(Some(&visible)),
            vec![PaneEffect::RecalculateAllPanes, PaneEffect::LightUpdate]
        );
        assert_eq!(
            pane.left_price_scale().price_range(),
            Some(crate::model::price_range_impl::PriceRangeImpl::new(
                10.0, 10.0
            ))
        );
        assert_eq!(
            pane.price_scale_by_id("h").unwrap().price_range(),
            Some(crate::model::price_range_impl::PriceRangeImpl::new(
                5.0, 20.0
            ))
        );
        assert_eq!(
            pane.recalculate(None),
            vec![PaneEffect::RecalculateAllPanes, PaneEffect::LightUpdate]
        );
        assert_eq!(pane.left_price_scale().price_range(), None);
        assert_eq!(pane.price_scale_by_id("h").unwrap().price_range(), None);
    }

    #[test]
    fn provider_range_flows_through_the_pane_source_adapter() {
        let provider: AutoscaleInfoProvider = Box::new(|_| {
            Some(AutoscaleInfo {
                price_range: Some(PriceRange {
                    min_value: -2.0,
                    max_value: 8.0,
                }),
                margins: None,
            })
        });
        let mut options = line_options();
        let SeriesOptionsMap::Line(line) = &mut options else {
            unreachable!()
        };
        line.common.autoscale_info_provider = Some(provider);
        let mut series =
            Series::<(), (), ()>::new(SeriesId::new(1), SeriesType::Line, options).unwrap();
        series.set_data(
            vec![SeriesPlotRow::Line(LinePlotRow {
                base: PlotRow {
                    index: 0.0.into(),
                    time: (),
                    original_time: (),
                    value: [1.0; 4],
                    custom_values: None,
                    original_data_count: None,
                },
                color: None,
            })],
            None,
        );
        let mut pane = Pane::new(0, PaneOptions::default(), &layout());
        pane.attach_series(Rc::new(RefCell::new(series)), PriceScalePosition::Left)
            .unwrap();
        pane.recalculate(Some(&RangeImpl::new(0.0.into(), 0.0.into())));
        assert_eq!(
            pane.left_price_scale().price_range(),
            Some(crate::model::price_range_impl::PriceRangeImpl::new(
                -2.0, 8.0
            ))
        );
    }

    #[test]
    fn custom_sources_attach_merge_move_and_clean_up_overlays() {
        let mut pane = Pane::new(0, PaneOptions::default(), &layout());
        let built_in = line_handle(1, 10.0);
        let custom = custom_handle(2, [30.0, 50.0, -2.0, 30.0]);
        pane.attach_series(built_in, PriceScalePosition::Left)
            .unwrap();
        pane.attach_custom_series(custom.clone(), PriceScalePosition::Left)
            .unwrap();

        let visible = RangeImpl::new(0.0.into(), 0.0.into());
        pane.recalculate(Some(&visible));
        assert_eq!(
            pane.left_price_scale().price_range(),
            Some(crate::model::price_range_impl::PriceRangeImpl::new(
                -2.0, 50.0
            ))
        );
        assert_eq!(
            pane.attached_source_ids(),
            vec![SeriesId::new(1), SeriesId::new(2)]
        );
        assert!(matches!(
            pane.attach_custom_series(custom.clone(), PriceScalePosition::Right),
            Err(PaneError::SeriesAlreadyAttached(_))
        ));
        assert!(matches!(
            pane.attach_custom_series(
                custom_handle(1, [1.0, 1.0, 1.0, 1.0]),
                PriceScalePosition::Right,
            ),
            Err(PaneError::SeriesAlreadyAttached(_))
        ));

        pane.move_custom_series_to_scale(
            SeriesId::new(2),
            PriceScalePosition::Overlay("custom".into()),
        )
        .unwrap();
        assert_eq!(
            pane.price_scale_by_id("custom").unwrap().price_range(),
            Some(crate::model::price_range_impl::PriceRangeImpl::new(
                -2.0, 50.0
            ))
        );
        pane.detach_custom_series(SeriesId::new(2)).unwrap();
        assert!(pane.price_scale_by_id("custom").is_none());
    }

    #[test]
    fn custom_provider_range_reaches_the_price_scale() {
        let provider: AutoscaleInfoProvider = Box::new(|_| {
            Some(AutoscaleInfo {
                price_range: Some(PriceRange {
                    min_value: -7.0,
                    max_value: 9.0,
                }),
                margins: None,
            })
        });
        let mut options = custom_options();
        options.common.autoscale_info_provider = Some(provider);
        let mut series = CustomSeries::<(), (), (), ()>::new(SeriesId::new(2), options);
        series.set_data(
            vec![CustomPlotRow {
                base: PlotRow {
                    index: 0.0.into(),
                    time: (),
                    original_time: (),
                    value: [2.0, 5.0, 1.0, 2.0],
                    custom_values: None,
                    original_data_count: None,
                },
                data: (),
                color: None,
            }],
            None,
        );

        let mut pane = Pane::new(0, PaneOptions::default(), &layout());
        pane.attach_custom_series(Rc::new(RefCell::new(series)), PriceScalePosition::Right)
            .unwrap();
        pane.recalculate(Some(&RangeImpl::new(0.0.into(), 0.0.into())));
        assert_eq!(
            pane.right_price_scale().price_range(),
            Some(crate::model::price_range_impl::PriceRangeImpl::new(
                -7.0, 9.0
            ))
        );
    }

    #[test]
    fn orders_mixed_sources_and_renumbers_concrete_series() {
        let mut pane = Pane::new(0, PaneOptions::default(), &layout());
        let first = line_handle(1, 1.0);
        let custom = custom_handle(2, [2.0; 4]);
        let last = line_handle(3, 3.0);
        pane.attach_series(first.clone(), PriceScalePosition::Left)
            .unwrap();
        pane.attach_custom_series(custom.clone(), PriceScalePosition::Right)
            .unwrap();
        pane.attach_series(last.clone(), PriceScalePosition::Left)
            .unwrap();

        assert_eq!(
            pane.ordered_source_ids(),
            &[SeriesId::new(1), SeriesId::new(2), SeriesId::new(3)]
        );
        assert_eq!(pane.ordered_sources().len(), 3);
        assert_eq!(
            pane.set_series_order(SeriesId::new(3), 0).unwrap(),
            vec![PaneEffect::LightUpdate]
        );
        assert_eq!(
            pane.ordered_source_ids(),
            &[SeriesId::new(3), SeriesId::new(1), SeriesId::new(2)]
        );
        assert_eq!(last.borrow().z_order(), 0);
        assert_eq!(first.borrow().z_order(), 1);
        assert_eq!(custom.borrow().z_order(), 2);

        pane.set_series_order(SeriesId::new(1), usize::MAX).unwrap();
        assert_eq!(
            pane.ordered_source_ids(),
            &[SeriesId::new(3), SeriesId::new(2), SeriesId::new(1)]
        );
        assert_eq!(first.borrow().z_order(), 2);
        assert!(matches!(
            pane.set_series_order(SeriesId::new(99), 0),
            Err(PaneError::UnknownSeries(_))
        ));
    }

    #[test]
    fn formatter_leader_changes_invalidate_only_the_affected_price_scale_marks() {
        let mut pane = Pane::new(0, PaneOptions::default(), &layout());
        pane.set_height(160.0);
        pane.attach_series(line_handle(1, 10.0), PriceScalePosition::Left)
            .unwrap();
        pane.attach_series(line_handle(2, 20.0), PriceScalePosition::Left)
            .unwrap();
        let visible = RangeImpl::new(0.0.into(), 0.0.into());
        pane.recalculate(Some(&visible));
        assert!(!pane.left_price_scale.marks().is_empty());
        let generation = pane.left_price_scale.marks_generation();

        assert_eq!(
            pane.set_series_order(SeriesId::new(2), 0).unwrap(),
            vec![PaneEffect::LightUpdate]
        );
        assert!(pane.left_price_scale.marks_generation() > generation);
        pane.left_price_scale.marks();
        let generation = pane.left_price_scale.marks_generation();

        pane.detach_series(SeriesId::new(2)).unwrap();
        assert!(pane.left_price_scale.marks_generation() > generation);
    }

    #[test]
    fn grid_snapshot_uses_default_scale_marks_and_is_invalidated_by_resize() {
        let mut pane = Pane::new(0, PaneOptions::default(), &layout());
        pane.set_height(120.0);
        pane.attach_series(histogram_handle(1, 0.0, 20.0), PriceScalePosition::Right)
            .unwrap();
        let visible = RangeImpl::new(0.0.into(), 0.0.into());
        pane.recalculate(Some(&visible));
        let expected_price_marks = pane.right_price_scale.marks().to_vec();
        let time_marks = [TimeMark {
            need_align_coordinate: false,
            coordinate: 42.0,
            label: "ignored by grid".into(),
            weight: crate::model::time_data::TickMarkWeightValue::new(0),
        }];
        pane.refresh_grid(&GridOptions::default(), &time_marks);
        let data = pane.grid().pane_view().renderer().data().unwrap();
        assert_eq!(data.price_marks, expected_price_marks);
        assert_eq!(data.time_marks[0].coordinate, Coordinate::new(42.0));
        let refresh_generation = pane.grid().pane_view().refresh_generation();

        pane.set_width(200.0);
        assert!(pane.grid().pane_view().is_invalidated());
        pane.refresh_grid(&GridOptions::default(), &time_marks);
        assert!(pane.grid().pane_view().refresh_generation() > refresh_generation);
    }

    #[test]
    fn source_order_cache_refreshes_after_move_and_detach() {
        let mut pane = Pane::new(0, PaneOptions::default(), &layout());
        let first = line_handle(1, 1.0);
        let custom = custom_handle(2, [2.0; 4]);
        pane.attach_series(first.clone(), PriceScalePosition::Left)
            .unwrap();
        pane.attach_custom_series(custom.clone(), PriceScalePosition::Right)
            .unwrap();
        assert_eq!(
            pane.ordered_source_ids(),
            &[SeriesId::new(1), SeriesId::new(2)]
        );

        pane.move_custom_series_to_scale(
            SeriesId::new(2),
            PriceScalePosition::Overlay("study".into()),
        )
        .unwrap();
        assert_eq!(
            pane.ordered_source_ids(),
            &[SeriesId::new(1), SeriesId::new(2)]
        );
        pane.detach_series(SeriesId::new(1)).unwrap();
        assert_eq!(pane.ordered_source_ids(), &[SeriesId::new(2)]);
        assert_eq!(custom.borrow().z_order(), 0);
    }

    #[test]
    fn selects_default_scales_from_preference_visibility_and_sources() {
        let mut right_first = Pane::new(
            0,
            pane_options(DefaultVisiblePriceScale::Right, true, true),
            &layout(),
        );
        right_first
            .attach_series(line_handle(1, 1.0), PriceScalePosition::Left)
            .unwrap();
        right_first
            .attach_series(line_handle(2, 2.0), PriceScalePosition::Right)
            .unwrap();
        assert_eq!(
            right_first.default_visible_price_scale().unwrap().id(),
            "right"
        );
        assert_eq!(right_first.default_price_scale().id(), "right");

        let mut left_first = Pane::new(
            0,
            pane_options(DefaultVisiblePriceScale::Left, true, true),
            &layout(),
        );
        left_first
            .attach_series(line_handle(1, 1.0), PriceScalePosition::Left)
            .unwrap();
        left_first
            .attach_series(line_handle(2, 2.0), PriceScalePosition::Right)
            .unwrap();
        assert_eq!(
            left_first.default_visible_price_scale().unwrap().id(),
            "left"
        );
        assert_eq!(left_first.default_price_scale().id(), "left");

        let mut secondary_with_source = Pane::new(
            0,
            pane_options(DefaultVisiblePriceScale::Right, true, true),
            &layout(),
        );
        secondary_with_source
            .attach_series(line_handle(1, 1.0), PriceScalePosition::Left)
            .unwrap();
        assert_eq!(secondary_with_source.default_price_scale().id(), "left");
    }

    #[test]
    fn falls_back_to_overlay_or_preferred_empty_scale() {
        let mut overlay = Pane::new(
            0,
            pane_options(DefaultVisiblePriceScale::Right, false, false),
            &layout(),
        );
        overlay
            .attach_custom_series(
                custom_handle(1, [1.0; 4]),
                PriceScalePosition::Overlay("indicator".into()),
            )
            .unwrap();
        assert!(overlay.default_visible_price_scale().is_none());
        assert_eq!(overlay.default_price_scale().id(), "indicator");

        let empty = Pane::<(), (), ()>::new(
            0,
            pane_options(DefaultVisiblePriceScale::Right, false, false),
            &layout(),
        );
        assert!(empty.default_visible_price_scale().is_none());
        assert_eq!(empty.default_price_scale().id(), "right");
    }

    #[test]
    fn forwards_scale_and_scroll_interactions_without_a_ui_runtime() {
        let mut pane = Pane::new(0, PaneOptions::default(), &layout());
        pane.set_height(100.0);
        pane.attach_series(histogram_handle(1, 0.0, 20.0), PriceScalePosition::Left)
            .unwrap();
        let visible = RangeImpl::new(0.0.into(), 0.0.into());
        pane.recalculate(Some(&visible));
        let initial_range = pane.left_price_scale().price_range();

        assert!(
            pane.start_scale_price(PriceScalePosition::Left, Coordinate::new(50.0))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            pane.scale_price_to(PriceScalePosition::Left, Coordinate::new(75.0))
                .unwrap(),
            vec![PaneEffect::LightUpdate]
        );
        assert_ne!(pane.left_price_scale().price_range(), initial_range);
        assert!(
            pane.end_scale_price(PriceScalePosition::Left)
                .unwrap()
                .is_empty()
        );

        assert!(
            pane.start_scroll_price(PriceScalePosition::Left, Coordinate::new(10.0))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            pane.scroll_price_to(PriceScalePosition::Left, Coordinate::new(20.0))
                .unwrap(),
            vec![PaneEffect::LightUpdate]
        );
        assert!(
            pane.end_scroll_price(PriceScalePosition::Left)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn reset_and_momentary_auto_scale_use_the_visible_snapshot() {
        let mut pane = Pane::new(0, PaneOptions::default(), &layout());
        pane.set_height(100.0);
        pane.attach_series(histogram_handle(1, 0.0, 10.0), PriceScalePosition::Left)
            .unwrap();
        let visible = RangeImpl::new(0.0.into(), 0.0.into());
        pane.recalculate(Some(&visible));
        pane.start_scale_price(PriceScalePosition::Left, Coordinate::new(50.0))
            .unwrap();
        pane.scale_price_to(PriceScalePosition::Left, Coordinate::new(75.0))
            .unwrap();
        assert!(!pane.left_price_scale().is_auto_scale());
        assert_eq!(
            pane.reset_price_scale(PriceScalePosition::Left).unwrap(),
            vec![PaneEffect::LightUpdate]
        );
        assert!(pane.left_price_scale().is_auto_scale());

        let mut options = PaneOptions::default();
        options.left_price_scale.auto_scale = false;
        let mut manual = Pane::new(0, options, &layout());
        manual.set_height(100.0);
        manual
            .attach_series(line_handle(2, 25.0), PriceScalePosition::Left)
            .unwrap();
        manual.recalculate(Some(&visible));
        assert!(manual.left_price_scale().price_range().is_none());
        assert_eq!(manual.momentary_auto_scale(), vec![PaneEffect::LightUpdate]);
        assert_eq!(
            manual.left_price_scale().price_range(),
            Some(crate::model::price_range_impl::PriceRangeImpl::new(
                25.0, 25.0
            ))
        );
        assert!(!manual.left_price_scale().is_auto_scale());
    }

    #[test]
    fn interaction_requests_for_missing_overlays_are_explicit_errors() {
        let mut pane = Pane::<(), (), ()>::new(0, PaneOptions::default(), &layout());
        let position = PriceScalePosition::Overlay("missing".into());
        assert_eq!(
            pane.start_scale_price(position.clone(), Coordinate::new(1.0)),
            Err(PaneError::UnknownPriceScale(position))
        );
    }

    #[test]
    fn mode_transitions_preserve_pane_interaction_rules() {
        let mut pane = Pane::new(0, PaneOptions::default(), &layout());
        pane.set_height(100.0);
        pane.attach_series(histogram_handle(1, 0.0, 20.0), PriceScalePosition::Left)
            .unwrap();
        let visible = RangeImpl::new(0.0.into(), 0.0.into());
        pane.recalculate(Some(&visible));

        pane.left_price_scale.set_mode(PriceScaleStatePatch {
            mode: Some(crate::model::price_scale::PriceScaleMode::Percentage),
            ..PriceScaleStatePatch::default()
        });
        assert!(pane.left_price_scale().is_auto_scale());
        assert!(
            pane.start_scale_price(PriceScalePosition::Left, Coordinate::new(50.0))
                .unwrap()
                .is_empty()
        );

        pane.left_price_scale.set_mode(PriceScaleStatePatch {
            mode: Some(crate::model::price_scale::PriceScaleMode::Normal),
            ..PriceScaleStatePatch::default()
        });
        assert!(
            pane.start_scale_price(PriceScalePosition::Left, Coordinate::new(50.0))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            pane.scale_price_to(PriceScalePosition::Left, Coordinate::new(75.0))
                .unwrap(),
            vec![PaneEffect::LightUpdate]
        );
        pane.end_scale_price(PriceScalePosition::Left).unwrap();
    }
}
