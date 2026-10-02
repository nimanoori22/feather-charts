//! Static horizontal-scale viewport state and coordinate conversion.

use crate::{
    helpers::delegate::{Delegate, Subscription},
    model::{
        coordinate::Coordinate,
        formatted_labels_cache::FormattedLabelsCache,
        ihorz_scale_behavior::HorzScaleBehavior,
        ihorz_scale_behavior::TimeMark,
        localization_options::LocalizationOptions,
        range_impl::{RangeImpl, are_ranges_equal},
        tick_marks::{TickMark, TickMarks},
        time_data::{
            Logical, LogicalRange, TimePointIndex, TimePointValue, TimeScalePoint, ValueRange,
        },
        time_scale_host::{TimeScaleEffect, viewport_changed_effects},
        time_scale_options::{HorzScaleOptions, HorzScaleOptionsPatch},
        time_scale_visible_range::TimeScaleVisibleRange,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    hash::Hash,
    time::Duration,
};

pub struct TimeScaleUpdate<Internal, Original> {
    pub points: Vec<TimeScalePoint<Internal, Original>>,
    pub first_changed_point_index: usize,
}

pub struct TimeScale<B: HorzScaleBehavior>
where
    B::InternalItem: Clone,
    B::Item: Clone,
    B::CacheKey: Clone + Eq + Hash,
{
    behavior: B,
    options: HorzScaleOptions,
    width: f64,
    base_index: Option<TimePointIndex>,
    points: Vec<TimeScalePoint<B::InternalItem, B::Item>>,
    bar_spacing: f64,
    right_offset: f64,
    tick_marks: TickMarks<B::InternalItem, B::Item>,
    visible_range: TimeScaleVisibleRange,
    visible_range_invalidated: bool,
    localization: LocalizationOptions<B::Item>,
    formatted_by_weight:
        BTreeMap<crate::model::time_data::TickMarkWeightValue, FormattedLabelsCache<B::CacheKey>>,
    time_marks_cache: Option<Vec<TimeMark>>,
    scale_start: Option<Coordinate>,
    scroll_start: Option<Coordinate>,
    transition_start: Option<TransitionState>,
    indices_with_data: BTreeSet<usize>,
    indices_with_data_version: u64,
    visible_bars_changed: Delegate<()>,
    logical_range_changed: Delegate<()>,
    options_applied: Delegate<()>,
    interactions_enabled: bool,
    conflation_factor: usize,
    device_pixel_ratio: f64,
}

#[derive(Clone, Copy)]
struct TransitionState {
    bar_spacing: f64,
    right_offset: f64,
}

impl<B> TimeScale<B>
where
    B: HorzScaleBehavior,
    B::InternalItem: Clone,
    B::Item: Clone,
    B::Key: PartialOrd,
    B::CacheKey: Clone + Eq + Hash,
{
    pub fn new(
        mut behavior: B,
        options: HorzScaleOptions,
        localization: LocalizationOptions<B::Item>,
    ) -> Self {
        let bar_spacing = options.bar_spacing;
        let right_offset = options
            .right_offset_pixels
            .map(|v| v / bar_spacing)
            .unwrap_or(options.right_offset);
        behavior.update_formatter(&localization);
        Self {
            behavior,
            options,
            width: 0.0,
            base_index: None,
            points: vec![],
            bar_spacing,
            right_offset,
            tick_marks: TickMarks::default(),
            visible_range: TimeScaleVisibleRange::invalid(),
            visible_range_invalidated: true,
            localization,
            formatted_by_weight: BTreeMap::new(),
            time_marks_cache: None,
            scale_start: None,
            scroll_start: None,
            transition_start: None,
            indices_with_data: BTreeSet::new(),
            indices_with_data_version: 0,
            visible_bars_changed: Delegate::new(),
            logical_range_changed: Delegate::new(),
            options_applied: Delegate::new(),
            interactions_enabled: true,
            conflation_factor: 1,
            device_pixel_ratio: 1.0,
        }
    }
    pub fn behavior(&self) -> &B {
        &self.behavior
    }
    pub fn options(&self) -> &HorzScaleOptions {
        &self.options
    }
    pub fn width(&self) -> f64 {
        self.width
    }
    pub fn bar_spacing(&self) -> f64 {
        self.bar_spacing
    }
    pub fn right_offset(&self) -> f64 {
        self.right_offset
    }
    pub fn is_empty(&self) -> bool {
        self.width == 0.0 || self.points.is_empty() || self.base_index.is_none()
    }
    pub fn has_points(&self) -> bool {
        !self.points.is_empty()
    }
    pub fn update(&mut self, update: TimeScaleUpdate<B::InternalItem, B::Item>) {
        self.visible_range_invalidated = true;
        self.time_marks_cache = None;
        self.tick_marks
            .set_time_scale_points(&update.points, update.first_changed_point_index);
        self.points = update.points;
        self.correct_offset();
        self.update_conflation_factor();
    }
    pub fn set_localization_options(&mut self, localization: LocalizationOptions<B::Item>) {
        self.localization = localization;
        self.behavior.update_formatter(&self.localization);
        self.formatted_by_weight.clear();
        self.time_marks_cache = None;
    }
    pub fn apply_options(&mut self, patch: HorzScaleOptionsPatch) -> Vec<TimeScaleEffect> {
        if patch
            .right_offset_pixels
            .is_some_and(|pixels| pixels.is_some_and(|v| !v.is_finite()))
        {
            return vec![];
        }
        let formatting = patch.time_visible.is_some() || patch.seconds_visible.is_some();
        let ticks = patch.uniform_distribution.is_some()
            || patch.tick_mark_max_character_length.is_some()
            || patch.ignore_whitespace_indices.is_some();
        let geometry = patch.bar_spacing.is_some()
            || patch.right_offset.is_some()
            || patch.right_offset_pixels.is_some()
            || patch.min_bar_spacing.is_some()
            || patch.max_bar_spacing.is_some()
            || patch.fix_left_edge.is_some()
            || patch.fix_right_edge.is_some();
        macro_rules! set {
            ($f:ident) => {
                if let Some(v) = patch.$f {
                    self.options.$f = v;
                }
            };
        }
        set!(min_bar_spacing);
        set!(max_bar_spacing);
        set!(fix_left_edge);
        set!(fix_right_edge);
        set!(lock_visible_time_range_on_resize);
        set!(right_bar_stays_on_scroll);
        set!(border_visible);
        set!(border_color);
        set!(visible);
        set!(time_visible);
        set!(seconds_visible);
        set!(shift_visible_range_on_new_bar);
        set!(allow_shift_visible_range_on_whitespace_replacement);
        set!(ticks_visible);
        set!(minimum_height);
        set!(allow_bold_labels);
        set!(ignore_whitespace_indices);
        set!(enable_conflation);
        set!(precompute_conflation_on_init);
        set!(precompute_conflation_priority);
        if let Some(v) = patch.tick_mark_max_character_length {
            self.options.tick_mark_max_character_length = v;
        }
        if let Some(v) = patch.conflation_threshold_factor {
            self.options.conflation_threshold_factor = v;
        }
        if let Some(v) = patch.uniform_distribution {
            self.options.uniform_distribution = v;
            self.tick_marks.set_uniform_distribution(v);
        }
        self.do_fix_left_edge();
        self.do_fix_right_edge();
        if let Some(spacing) = patch.bar_spacing {
            self.set_bar_spacing(spacing);
        }
        if let Some(offset) = patch.right_offset {
            self.options.right_offset = offset;
            if patch.right_offset_pixels.is_none() {
                self.set_right_offset(offset);
            }
        }
        if let Some(pixels) = patch.right_offset_pixels {
            self.options.right_offset_pixels = pixels;
            if let Some(p) = pixels {
                self.set_right_offset(p / self.bar_spacing);
            }
        }
        self.correct_bar_spacing();
        self.correct_offset();
        if geometry
            || patch.enable_conflation.is_some()
            || patch.conflation_threshold_factor.is_some()
        {
            self.update_conflation_factor();
        }
        if geometry || ticks {
            self.visible_range_invalidated = true;
            self.time_marks_cache = None;
        }
        if formatting {
            self.formatted_by_weight.clear();
            self.time_marks_cache = None;
        }
        let mut effects = if geometry {
            viewport_changed_effects()
        } else {
            vec![]
        };
        effects.push(TimeScaleEffect::OptionsApplied);
        self.options_applied.emit(&());
        effects
    }
    pub fn restore_default(&mut self) -> Vec<TimeScaleEffect> {
        let defaults = HorzScaleOptions::default();
        self.options.right_offset_pixels = defaults.right_offset_pixels;
        self.set_bar_spacing(defaults.bar_spacing);
        self.set_right_offset(
            defaults
                .right_offset_pixels
                .map(|p| p / self.bar_spacing)
                .unwrap_or(defaults.right_offset),
        );
        self.update_conflation_factor();
        let mut effects = viewport_changed_effects();
        effects.push(TimeScaleEffect::OptionsApplied);
        self.options_applied.emit(&());
        effects
    }
    pub fn set_indices_with_data(&mut self, indices: BTreeSet<usize>, version: u64) {
        if self.indices_with_data != indices || self.indices_with_data_version != version {
            self.indices_with_data = indices;
            self.indices_with_data_version = version;
            self.time_marks_cache = None;
        }
    }
    pub fn recalculate_indices_with_data(
        &mut self,
        host: &impl crate::model::time_scale_host::TimeScaleHost,
    ) -> bool {
        if !self.options.ignore_whitespace_indices {
            return false;
        }
        let indices = host.fulfilled_time_indices();
        if indices == self.indices_with_data {
            return false;
        }
        self.indices_with_data = indices;
        self.indices_with_data_version = self.indices_with_data_version.wrapping_add(1);
        self.time_marks_cache = None;
        true
    }
    pub fn set_base_index(&mut self, value: Option<TimePointIndex>) -> Vec<TimeScaleEffect> {
        self.base_index = value;
        self.visible_range_invalidated = true;
        self.correct_offset();
        viewport_changed_effects()
    }
    pub fn set_width(&mut self, width: f64) -> Vec<TimeScaleEffect> {
        if !width.is_finite() || width <= 0.0 || width == self.width {
            return vec![];
        }
        let previous_visible_range = self.visible_logical_range();
        let old = self.width;
        self.width = width;
        if self.options.lock_visible_time_range_on_resize && old != 0.0 {
            self.bar_spacing *= width / old;
        }
        if self.options.fix_left_edge
            && previous_visible_range.is_some_and(|range| range.left().value() <= 0.0)
        {
            self.right_offset -= ((old - width) / self.bar_spacing).round() + 1.0;
        }
        self.visible_range_invalidated = true;
        self.correct_bar_spacing();
        self.correct_offset();
        self.update_conflation_factor();
        viewport_changed_effects()
    }
    pub fn set_right_offset(&mut self, value: f64) -> Vec<TimeScaleEffect> {
        if !value.is_finite() {
            return vec![];
        }
        self.right_offset = value;
        self.visible_range_invalidated = true;
        self.correct_offset();
        viewport_changed_effects()
    }
    pub fn set_bar_spacing(&mut self, value: f64) -> Vec<TimeScaleEffect> {
        if !value.is_finite() || value <= 0.0 {
            return vec![];
        }
        let old = self.bar_spacing;
        self.bar_spacing = value;
        self.correct_bar_spacing();
        if self.options.right_offset_pixels.is_some() {
            self.right_offset *= old / self.bar_spacing;
        }
        self.correct_offset();
        self.update_conflation_factor();
        viewport_changed_effects()
    }
    pub fn set_visible_range(
        &mut self,
        range: RangeImpl<TimePointIndex>,
        apply_default_offset: bool,
    ) -> Vec<TimeScaleEffect> {
        if self.width <= 0.0 || !range.count().is_finite() || range.count() <= 0.0 {
            return vec![];
        }
        let pixel_offset = if apply_default_offset {
            self.options.right_offset_pixels.unwrap_or(0.0)
        } else {
            0.0
        };
        self.set_bar_spacing((self.width - pixel_offset) / range.count());
        self.right_offset = range.right().value() - self.base().value();
        if apply_default_offset {
            self.right_offset = self
                .options
                .right_offset_pixels
                .map(|pixels| pixels / self.bar_spacing)
                .unwrap_or(self.options.right_offset);
        }
        self.correct_offset();
        self.visible_range_invalidated = true;
        self.update_conflation_factor();
        viewport_changed_effects()
    }
    pub fn fit_content(&mut self) -> Vec<TimeScaleEffect> {
        let Some(last) = self.points.len().checked_sub(1) else {
            return vec![];
        };
        let right_offset = if self.options.right_offset_pixels.is_none() {
            self.options.right_offset
        } else {
            0.0
        };
        self.set_visible_range(
            RangeImpl::new(
                TimePointIndex::new(0.0),
                TimePointIndex::new(last as f64 + right_offset),
            ),
            true,
        )
    }
    pub fn set_logical_range(&mut self, range: LogicalRange) -> Vec<TimeScaleEffect> {
        self.set_visible_range(
            RangeImpl::new(
                TimePointIndex::new(range.from.value()),
                TimePointIndex::new(range.to.value()),
            ),
            false,
        )
    }
    pub fn index_to_time(&self, index: TimePointIndex) -> Option<&B::InternalItem> {
        self.point(index).map(|p| &p.time)
    }
    pub fn time_to_index(
        &self,
        time: &B::InternalItem,
        find_nearest: bool,
    ) -> Option<TimePointIndex> {
        let last = self.points.last()?;
        let key = self.behavior.key(time);
        if key > self.behavior.key(&last.time) {
            return find_nearest.then(|| TimePointIndex::new((self.points.len() - 1) as f64));
        }
        let index = self
            .points
            .partition_point(|point| self.behavior.key(&point.time) < key);
        let point = self.points.get(index)?;
        if key < self.behavior.key(&point.time) && !find_nearest {
            return None;
        }
        Some(TimePointIndex::new(index as f64))
    }
    pub fn logical_range_for_time_range(
        &self,
        range: ValueRange<B::InternalItem>,
    ) -> Option<LogicalRange> {
        Some(ValueRange {
            from: Logical::new(self.time_to_index(&range.from, true)?.value()),
            to: Logical::new(self.time_to_index(&range.to, true)?.value()),
        })
    }
    pub fn format_date_time(&self, point: &TimeScalePoint<B::InternalItem, B::Item>) -> String {
        self.localization.time_formatter.as_ref().map_or_else(
            || self.behavior.format_item(&point.time),
            |formatter| formatter(&point.original_time),
        )
    }
    pub fn index_to_coordinate(&self, index: TimePointIndex) -> Coordinate {
        if self.is_empty() || !index.is_integer() {
            return Coordinate::new(0.0);
        }
        Coordinate::new(
            self.width
                - (self.base().value() + self.right_offset - index.value() + 0.5)
                    * self.bar_spacing
                - 1.0,
        )
    }
    pub fn indexes_to_coordinates(
        &self,
        items: &mut [crate::model::time_data::TimedValue],
        range: Option<ValueRange<usize>>,
    ) {
        let r = range.unwrap_or(ValueRange {
            from: 0,
            to: items.len(),
        });
        let len = items.len();
        for item in &mut items[r.from.min(len)..r.to.min(len)] {
            item.x = self.index_to_coordinate(item.time);
        }
    }
    pub fn coordinate_to_index(
        &self,
        coordinate: Coordinate,
        consider_ignore_whitespace: bool,
    ) -> TimePointIndex {
        let index = TimePointIndex::new(self.float_index(coordinate).ceil());
        if !consider_ignore_whitespace || !self.options.ignore_whitespace_indices {
            return index;
        }
        let Some(start) = index.as_usize() else {
            return index;
        };
        for distance in 0..=self.points.len() {
            for candidate in [start.checked_sub(distance), start.checked_add(distance)] {
                if let Some(candidate) = candidate.filter(|v| self.indices_with_data.contains(v)) {
                    return TimePointIndex::new(candidate as f64);
                }
            }
        }
        index
    }
    pub fn visible_logical_range(&mut self) -> Option<RangeImpl<Logical>> {
        self.update_visible_range();
        self.visible_range.logical_range()
    }
    pub fn visible_strict_range(&mut self) -> Option<RangeImpl<TimePointIndex>> {
        self.update_visible_range();
        self.visible_range.strict_range()
    }
    pub fn visible_time_range(
        &mut self,
    ) -> Option<ValueRange<TimePointValue<B::InternalItem, B::Item>>> {
        let r = self.visible_strict_range()?;
        self.time_range_for_logical_range(ValueRange {
            from: Logical::new(r.left().value()),
            to: Logical::new(r.right().value()),
        })
    }
    pub fn marks(
        &mut self,
        layout: crate::model::time_scale_host::TimeScaleLayoutContext,
    ) -> Option<&[TimeMark]> {
        if self.is_empty() {
            return None;
        }
        if self.time_marks_cache.is_none() {
            let visible = self.visible_strict_range()?;
            let max_chars = self.options.tick_mark_max_character_length.unwrap_or(8) as f64;
            let max_width = (layout.font_size + 4.0) * 5.0 / 8.0 * max_chars;
            let selected = self
                .tick_marks
                .build(
                    self.bar_spacing,
                    max_width,
                    self.options.ignore_whitespace_indices,
                    &self.indices_with_data,
                    self.indices_with_data_version,
                )
                .to_vec();
            let mut result = Vec::new();
            let index_per_label = (max_width / self.bar_spacing).round();
            let earliest_second = index_per_label;
            let second_last = self.points.len().saturating_sub(1) as f64 - index_per_label;
            let left_fixed = self.options.fix_left_edge || !self.interactions_enabled;
            let right_fixed = self.options.fix_right_edge || !self.interactions_enabled;
            for mark in selected {
                if visible.contains(mark.index) {
                    let label = self.format_label(&mark);
                    result.push(TimeMark {
                        need_align_coordinate: if self.bar_spacing > max_width / 2.0
                            && self.interactions_enabled
                        {
                            false
                        } else {
                            (left_fixed && mark.index.value() <= earliest_second)
                                || (right_fixed && mark.index.value() >= second_last)
                        },
                        coordinate: self.index_to_coordinate(mark.index).value(),
                        label,
                        weight: mark.weight,
                    });
                }
            }
            self.time_marks_cache = Some(result);
        }
        self.time_marks_cache.as_deref()
    }
    pub fn time_range_for_logical_range(
        &self,
        range: LogicalRange,
    ) -> Option<ValueRange<TimePointValue<B::InternalItem, B::Item>>> {
        let first = self.point(TimePointIndex::new(range.from.value().round().max(0.0)))?;
        let last = self.point(TimePointIndex::new(
            range.to.value().round().min((self.points.len() - 1) as f64),
        ))?;
        Some(ValueRange {
            from: TimePointValue {
                time: first.time.clone(),
                original_time: first.original_time.clone(),
            },
            to: TimePointValue {
                time: last.time.clone(),
                original_time: last.original_time.clone(),
            },
        })
    }
    pub fn subscribe_visible_bars_changed(
        &self,
        callback: impl FnMut(&()) + 'static,
    ) -> Subscription<()> {
        self.visible_bars_changed.subscribe(callback)
    }
    pub fn subscribe_logical_range_changed(
        &self,
        callback: impl FnMut(&()) + 'static,
    ) -> Subscription<()> {
        self.logical_range_changed.subscribe(callback)
    }
    pub fn subscribe_options_applied(
        &self,
        callback: impl FnMut(&()) + 'static,
    ) -> Subscription<()> {
        self.options_applied.subscribe(callback)
    }
    pub fn set_interactions_enabled(&mut self, enabled: bool) {
        if self.interactions_enabled != enabled {
            self.interactions_enabled = enabled;
            self.time_marks_cache = None;
        }
    }
    pub fn conflation_factor(&self) -> usize {
        self.conflation_factor
    }
    pub fn possible_conflation_factors(&self) -> Vec<usize> {
        let threshold = 1.0 / self.device_pixel_ratio;
        if self.options.min_bar_spacing >= threshold {
            return vec![1];
        }
        let mut factors = vec![1];
        let mut level = 2;
        while level <= 512 {
            if self.options.min_bar_spacing < threshold / level as f64 {
                factors.push(level);
            }
            level *= 2;
        }
        factors
    }
    pub fn set_device_pixel_ratio(&mut self, ratio: f64) {
        if ratio.is_finite() && ratio > 0.0 && ratio != self.device_pixel_ratio {
            self.device_pixel_ratio = ratio;
            self.update_conflation_factor();
        }
    }
    fn base(&self) -> TimePointIndex {
        self.base_index.unwrap_or_default()
    }
    fn format_label(&mut self, mark: &TickMark<B::InternalItem, B::Item>) -> String {
        let key = self.behavior.cache_key(&mark.time);
        let behavior = &self.behavior;
        let localization = &self.localization;
        self.formatted_by_weight
            .entry(mark.weight)
            .or_default()
            .format(key, || behavior.format_tickmark(mark, localization))
    }
    pub fn zoom(&mut self, point: Coordinate, scale: f64) -> Vec<TimeScaleEffect> {
        let before = self.float_index(point);
        let effects = self.set_bar_spacing(self.bar_spacing + scale * self.bar_spacing / 10.0);
        self.set_right_offset(self.right_offset + before - self.float_index(point));
        effects
    }
    pub fn start_scale(&mut self, point: Coordinate) {
        if self.scroll_start.is_some() {
            self.end_scroll();
        }
        if self.is_empty() || self.transition_start.is_some() {
            return;
        }
        self.scale_start = Some(point);
        self.transition_start = Some(TransitionState {
            bar_spacing: self.bar_spacing,
            right_offset: self.right_offset,
        });
    }
    pub fn scale_to(&mut self, point: Coordinate) -> Vec<TimeScaleEffect> {
        let (Some(start), Some(state)) = (self.scale_start, self.transition_start) else {
            return vec![];
        };
        let a = (self.width - point.value()).clamp(0.0, self.width);
        let b = (self.width - start.value()).clamp(0.0, self.width);
        if a == 0.0 || b == 0.0 {
            return vec![];
        }
        self.set_bar_spacing(state.bar_spacing * a / b)
    }
    pub fn end_scale(&mut self) {
        self.scale_start = None;
        self.transition_start = None;
    }
    pub fn start_scroll(&mut self, point: Coordinate) {
        if self.scale_start.is_some() {
            self.end_scale();
        }
        if self.is_empty() || self.transition_start.is_some() {
            return;
        }
        self.scroll_start = Some(point);
        self.transition_start = Some(TransitionState {
            bar_spacing: self.bar_spacing,
            right_offset: self.right_offset,
        });
    }
    pub fn scroll_to(&mut self, point: Coordinate) -> Vec<TimeScaleEffect> {
        let (Some(start), Some(state)) = (self.scroll_start, self.transition_start) else {
            return vec![];
        };
        self.set_right_offset(
            state.right_offset + (start.value() - point.value()) / self.bar_spacing,
        )
    }
    pub fn end_scroll(&mut self) {
        self.scroll_start = None;
        self.transition_start = None;
    }
    pub fn scroll_to_real_time(&mut self) -> Vec<TimeScaleEffect> {
        self.scroll_to_offset_animated(self.options.right_offset, Duration::from_millis(400))
    }
    pub fn scroll_to_offset_animated(
        &mut self,
        target_offset: f64,
        duration: Duration,
    ) -> Vec<TimeScaleEffect> {
        if !target_offset.is_finite() || duration.is_zero() {
            return vec![];
        }
        vec![TimeScaleEffect::StartScrollAnimation(
            crate::model::time_scale_host::ScrollAnimation {
                start_offset: self.right_offset,
                target_offset,
                duration,
            },
        )]
    }
    fn point(&self, index: TimePointIndex) -> Option<&TimeScalePoint<B::InternalItem, B::Item>> {
        self.points.get(index.as_usize()?)
    }
    fn float_index(&self, coordinate: Coordinate) -> f64 {
        let index = self.base().value() + self.right_offset
            - (self.width - 1.0 - coordinate.value()) / self.bar_spacing;
        (index * 1_000_000.0).round() / 1_000_000.0
    }
    fn update_visible_range(&mut self) {
        if !self.visible_range_invalidated {
            return;
        }
        self.visible_range_invalidated = false;
        let old = self.visible_range;
        if self.is_empty() {
            self.visible_range = TimeScaleVisibleRange::invalid();
        } else {
            let right = self.right_offset + self.base().value();
            let left = right - self.width / self.bar_spacing + 1.0;
            self.visible_range = TimeScaleVisibleRange::new(Some(RangeImpl::new(
                Logical::new(left),
                Logical::new(right),
            )));
        }
        if !are_ranges_equal(old.strict_range(), self.visible_range.strict_range()) {
            self.visible_bars_changed.emit(&());
        }
        if !are_ranges_equal(old.logical_range(), self.visible_range.logical_range()) {
            self.logical_range_changed.emit(&());
        }
        self.time_marks_cache = None;
    }
    fn correct_bar_spacing(&mut self) {
        let min =
            if self.options.fix_left_edge && self.options.fix_right_edge && !self.points.is_empty()
            {
                self.width / self.points.len() as f64
            } else {
                self.options.min_bar_spacing
            };
        let max = if self.options.max_bar_spacing > 0.0 {
            self.options.max_bar_spacing
        } else {
            self.width * 0.5
        };
        let value = self.bar_spacing.clamp(min, max.max(min));
        if value != self.bar_spacing {
            self.bar_spacing = value;
            self.visible_range_invalidated = true;
        }
    }
    fn do_fix_left_edge(&mut self) {
        if !self.options.fix_left_edge || self.points.is_empty() {
            return;
        }
        let Some(visible) = self.visible_strict_range() else {
            return;
        };
        let delta = visible.left().value();
        if delta < 0.0 {
            self.right_offset -= delta + 1.0;
            self.visible_range_invalidated = true;
        }
        self.correct_bar_spacing();
    }
    fn do_fix_right_edge(&mut self) {
        self.correct_offset();
        self.correct_bar_spacing();
    }
    fn correct_offset(&mut self) {
        if self.points.is_empty() || self.base_index.is_none() {
            return;
        }
        let bars = if self.options.fix_left_edge {
            self.width / self.bar_spacing
        } else {
            2.0_f64.min(self.points.len() as f64)
        };
        let min = -self.base().value() - 1.0 + bars;
        let max = if self.options.fix_right_edge {
            0.0
        } else {
            self.width / self.bar_spacing - 2.0_f64.min(self.points.len() as f64)
        };
        let value = self.right_offset.clamp(min, max.max(min));
        if value != self.right_offset {
            self.right_offset = value;
            self.visible_range_invalidated = true;
        }
    }
    fn update_conflation_factor(&mut self) {
        if !self.options.enable_conflation {
            self.conflation_factor = 1;
            return;
        }
        let threshold =
            self.options.conflation_threshold_factor.unwrap_or(1.0) / self.device_pixel_ratio;
        if self.bar_spacing >= threshold {
            self.conflation_factor = 1;
            return;
        }
        let level = 2_f64.powf((threshold / self.bar_spacing).log2().floor()) as usize;
        self.conflation_factor = level.clamp(1, 512);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        horz_scale_behavior_time::{
            horz_scale_behavior_time::HorzScaleBehaviorTime,
            types::{Time, TimePoint, UtcTimestamp},
        },
        localization_options::LocalizationOptions,
        time_scale_host::{TimeScaleEffect, TimeScaleLayoutContext},
    };
    use std::{cell::Cell, collections::BTreeSet, rc::Rc};
    fn points() -> Vec<TimeScalePoint<TimePoint, Time>> {
        (0..10)
            .map(|i| {
                let t = UtcTimestamp::new(1_704_067_200.0 + i as f64);
                TimeScalePoint {
                    time_weight: Default::default(),
                    time: TimePoint {
                        timestamp: t,
                        business_day: None,
                    },
                    original_time: Time::from(t),
                }
            })
            .collect()
    }
    #[test]
    fn maps_coordinates_and_visible_range() {
        let mut s = TimeScale::new(
            HorzScaleBehaviorTime::default(),
            HorzScaleOptions {
                bar_spacing: 10.0,
                ..Default::default()
            },
            LocalizationOptions::new("en-US", "dd MMM 'yy"),
        );
        s.update(TimeScaleUpdate {
            points: points(),
            first_changed_point_index: 0,
        });
        s.set_width(100.0);
        s.set_base_index(Some(TimePointIndex::new(9.0)));
        let x = s.index_to_coordinate(TimePointIndex::new(9.0));
        assert_eq!(x.value(), 94.0);
        assert_eq!(s.coordinate_to_index(x, false).value(), 9.0);
        assert_eq!(s.visible_logical_range().unwrap().left().value(), 0.0);
        assert!(s.index_to_time(TimePointIndex::new(1.5)).is_none());
        let marks = s.marks(TimeScaleLayoutContext { font_size: 12.0 }).unwrap();
        assert!(marks.iter().all(|mark| mark.coordinate.is_finite()));
    }
    #[test]
    fn emits_effects_and_clamps_spacing() {
        let mut s = TimeScale::new(
            HorzScaleBehaviorTime::default(),
            HorzScaleOptions {
                bar_spacing: 10.0,
                ..Default::default()
            },
            LocalizationOptions::new("en-US", "dd MMM 'yy"),
        );
        s.update(TimeScaleUpdate {
            points: points(),
            first_changed_point_index: 0,
        });
        s.set_base_index(Some(TimePointIndex::new(9.0)));
        s.set_width(100.0);
        assert_eq!(
            s.set_right_offset(2.0),
            vec![
                TimeScaleEffect::RecalculateAllPanes,
                TimeScaleEffect::LightUpdate
            ]
        );
        s.set_bar_spacing(0.01);
        assert!(s.bar_spacing() >= s.options().min_bar_spacing);
    }
    #[test]
    fn applies_spacing_before_pixel_offset_and_emits_options_effect() {
        let mut s = TimeScale::new(
            HorzScaleBehaviorTime::default(),
            HorzScaleOptions::default(),
            LocalizationOptions::new("en-US", "dd MMM 'yy"),
        );
        s.update(TimeScaleUpdate {
            points: points(),
            first_changed_point_index: 0,
        });
        s.set_base_index(Some(TimePointIndex::new(9.0)));
        s.set_width(100.0);

        let effects = s.apply_options(HorzScaleOptionsPatch {
            bar_spacing: Some(20.0),
            right_offset_pixels: Some(Some(40.0)),
            ..Default::default()
        });

        assert_eq!(s.bar_spacing(), 20.0);
        assert_eq!(s.right_offset(), 2.0);
        assert!(effects.contains(&TimeScaleEffect::OptionsApplied));
    }
    #[test]
    fn fits_content_and_converts_ranges_using_nearest_time_points() {
        let mut s = TimeScale::new(
            HorzScaleBehaviorTime::default(),
            HorzScaleOptions::default(),
            LocalizationOptions::new("en-US", "dd MMM 'yy"),
        );
        s.update(TimeScaleUpdate {
            points: points(),
            first_changed_point_index: 0,
        });
        s.set_width(100.0);
        s.set_base_index(Some(TimePointIndex::new(9.0)));

        s.fit_content();
        assert_eq!(s.visible_strict_range().unwrap().left().value(), 0.0);
        assert_eq!(
            s.time_to_index(&points()[3].time, false),
            Some(TimePointIndex::new(3.0))
        );
        let between = TimePoint {
            timestamp: UtcTimestamp::new(1_704_067_203.5),
            business_day: None,
        };
        assert_eq!(s.time_to_index(&between, false), None);
        assert_eq!(
            s.time_to_index(&between, true),
            Some(TimePointIndex::new(4.0))
        );
        let range = s
            .logical_range_for_time_range(ValueRange {
                from: points()[1].time,
                to: points()[7].time,
            })
            .unwrap();
        assert_eq!((range.from.value(), range.to.value()), (1.0, 7.0));
    }
    #[test]
    fn notifications_are_ordered_and_silent_for_noop_visible_reads() {
        let mut s = TimeScale::new(
            HorzScaleBehaviorTime::default(),
            HorzScaleOptions::default(),
            LocalizationOptions::new("en-US", "dd MMM 'yy"),
        );
        let events = Rc::new(Cell::new(0));
        let strict_events = Rc::clone(&events);
        let _strict = s.subscribe_visible_bars_changed(move |_| {
            strict_events.set(strict_events.get() * 10 + 1)
        });
        let logical_events = Rc::clone(&events);
        let _logical = s.subscribe_logical_range_changed(move |_| {
            logical_events.set(logical_events.get() * 10 + 2)
        });
        s.update(TimeScaleUpdate {
            points: points(),
            first_changed_point_index: 0,
        });
        s.set_width(100.0);
        s.set_base_index(Some(TimePointIndex::new(9.0)));
        s.visible_logical_range();
        assert_eq!(events.get(), 12);
        s.visible_logical_range();
        assert_eq!(events.get(), 12);
    }
    #[test]
    fn whitespace_snapshots_and_conflation_only_change_when_their_inputs_change() {
        struct Host(BTreeSet<usize>);
        impl crate::model::time_scale_host::TimeScaleHost for Host {
            fn layout_context(&self) -> TimeScaleLayoutContext {
                TimeScaleLayoutContext { font_size: 12.0 }
            }
            fn fulfilled_time_indices(&self) -> BTreeSet<usize> {
                self.0.clone()
            }
            fn apply_time_scale_effects(&mut self, _: Vec<TimeScaleEffect>) {}
        }
        let mut s = TimeScale::new(
            HorzScaleBehaviorTime::default(),
            HorzScaleOptions {
                ignore_whitespace_indices: true,
                enable_conflation: true,
                min_bar_spacing: 0.01,
                ..Default::default()
            },
            LocalizationOptions::new("en-US", "dd MMM 'yy"),
        );
        s.update(TimeScaleUpdate {
            points: points(),
            first_changed_point_index: 0,
        });
        s.set_width(100.0);
        s.set_base_index(Some(TimePointIndex::new(9.0)));
        let host = Host(BTreeSet::from([1, 4, 8]));
        assert!(s.recalculate_indices_with_data(&host));
        let version = s.indices_with_data_version;
        assert!(!s.recalculate_indices_with_data(&host));
        assert_eq!(s.indices_with_data_version, version);
        s.set_bar_spacing(0.1);
        assert_eq!(s.conflation_factor(), 8);
        assert!(s.possible_conflation_factors().contains(&32));
    }
    #[test]
    fn options_delegate_and_edge_label_alignment_follow_model_state() {
        let mut s = TimeScale::new(
            HorzScaleBehaviorTime::default(),
            HorzScaleOptions {
                fix_left_edge: true,
                bar_spacing: 10.0,
                ..Default::default()
            },
            LocalizationOptions::new("en-US", "dd MMM 'yy"),
        );
        let options_events = Rc::new(Cell::new(0));
        let observed = Rc::clone(&options_events);
        let _subscription = s.subscribe_options_applied(move |_| observed.set(observed.get() + 1));
        s.update(TimeScaleUpdate {
            points: points(),
            first_changed_point_index: 0,
        });
        s.set_width(100.0);
        s.set_base_index(Some(TimePointIndex::new(9.0)));
        s.set_interactions_enabled(false);
        let marks = s.marks(TimeScaleLayoutContext { font_size: 12.0 }).unwrap();
        assert!(marks.iter().any(|mark| mark.need_align_coordinate));
        s.apply_options(HorzScaleOptionsPatch {
            border_visible: Some(false),
            ..Default::default()
        });
        assert_eq!(options_events.get(), 1);
    }
}
