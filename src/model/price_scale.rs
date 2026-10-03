//! Synchronous price-to-coordinate conversion and source autoscaling.
use crate::helpers::delegate::Delegate;
use crate::{
    formatters::{percentage_formatter::PercentageFormatter, price_formatter::PriceFormatter},
    model::{
        coordinate::Coordinate, iprice_data_source::PriceScaleDataSource,
        layout_options::LayoutOptions, price_range_impl::PriceRangeImpl,
        price_scale_conversions::*, price_tick_mark_builder::PriceTickMarkBuilder,
        range_impl::RangeImpl, time_data::TimePointIndex,
    },
};
use std::{cell::RefCell, rc::Rc};
pub type PriceSourceHandle = Rc<RefCell<dyn PriceScaleDataSource>>;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PriceScaleMode {
    #[default]
    Normal,
    Logarithmic,
    Percentage,
    IndexedTo100,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PriceScaleState {
    pub auto_scale: bool,
    pub inverted: bool,
    pub mode: PriceScaleMode,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PriceScaleStatePatch {
    pub auto_scale: Option<bool>,
    pub inverted: Option<bool>,
    pub mode: Option<PriceScaleMode>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PriceScaleModeChange {
    pub old: PriceScaleState,
    pub new: PriceScaleState,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PriceScaleMarginsPatch {
    pub top: Option<f64>,
    pub bottom: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PriceScaleOptionsPatch {
    pub auto_scale: Option<bool>,
    pub mode: Option<PriceScaleMode>,
    pub invert_scale: Option<bool>,
    pub align_labels: Option<bool>,
    pub scale_margins: Option<PriceScaleMarginsPatch>,
    pub entire_text_only: Option<bool>,
    pub visible: Option<bool>,
    pub border_visible: Option<bool>,
    pub border_color: Option<String>,
    /// `Some(None)` restores the layout text color.
    pub text_color: Option<Option<String>>,
    pub ticks_visible: Option<bool>,
    pub minimum_width: Option<f64>,
    pub ensure_edge_tick_marks_visible: Option<bool>,
    pub tick_mark_density: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PriceScaleOptionsError {
    InvalidTopMargin(f64),
    InvalidBottomMargin(f64),
    InvalidMarginsSum(f64),
    NonFiniteMargin,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PriceScaleMargins {
    pub top: f64,
    pub bottom: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PriceScaleOptions {
    pub auto_scale: bool,
    pub mode: PriceScaleMode,
    pub invert_scale: bool,
    pub align_labels: bool,
    pub scale_margins: PriceScaleMargins,
    pub entire_text_only: bool,
    pub visible: bool,
    pub border_visible: bool,
    pub border_color: String,
    pub text_color: Option<String>,
    pub ticks_visible: bool,
    pub minimum_width: f64,
    pub ensure_edge_tick_marks_visible: bool,
    pub tick_mark_density: f64,
}
impl Default for PriceScaleOptions {
    fn default() -> Self {
        Self {
            auto_scale: true,
            mode: PriceScaleMode::Normal,
            invert_scale: false,
            align_labels: true,
            scale_margins: PriceScaleMargins {
                top: 0.2,
                bottom: 0.1,
            },
            entire_text_only: false,
            visible: true,
            border_visible: true,
            border_color: "#2B2B43".into(),
            text_color: None,
            ticks_visible: false,
            minimum_width: 0.0,
            ensure_edge_tick_marks_visible: false,
            tick_mark_density: 2.5,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PriceMark {
    pub coord: Coordinate,
    pub label: String,
    pub logical: f64,
}
pub struct PriceScale {
    id: String,
    options: PriceScaleOptions,
    height: f64,
    range: Option<PriceRangeImpl>,
    sources: Vec<PriceSourceHandle>,
    selected_formatter_source: Option<PriceSourceHandle>,
    mark_builder: PriceTickMarkBuilder,
    marks_cache: Option<Vec<PriceMark>>,
    marks_generation: u64,
    marks_changed: Delegate<()>,
    log_formula: LogFormula,
    layout_font_size: f64,
    scale_start: Option<Coordinate>,
    scroll_start: Option<Coordinate>,
    price_range_snapshot: Option<PriceRangeImpl>,
    is_custom_price_range: bool,
    mode_changed: Delegate<PriceScaleModeChange>,
}
impl PriceScale {
    pub fn new(id: impl Into<String>, options: PriceScaleOptions, layout: &LayoutOptions) -> Self {
        Self::new_with_font_size(id, options, layout.font_size)
    }
    pub fn new_with_font_size(
        id: impl Into<String>,
        options: PriceScaleOptions,
        layout_font_size: f64,
    ) -> Self {
        let mark_builder =
            PriceTickMarkBuilder::new(100.0, layout_font_size, options.tick_mark_density);
        Self {
            id: id.into(),
            options,
            height: 0.,
            range: None,
            sources: vec![],
            selected_formatter_source: None,
            mark_builder,
            marks_cache: None,
            marks_generation: 0,
            marks_changed: Delegate::new(),
            log_formula: DEFAULT_LOG_FORMULA,
            layout_font_size,
            scale_start: None,
            scroll_start: None,
            price_range_snapshot: None,
            is_custom_price_range: false,
            mode_changed: Delegate::new(),
        }
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn options(&self) -> &PriceScaleOptions {
        &self.options
    }
    pub fn apply_options(
        &mut self,
        patch: PriceScaleOptionsPatch,
    ) -> Result<PriceScaleModeChange, PriceScaleOptionsError> {
        let margins = self.margins_after_patch(patch.scale_margins.as_ref())?;
        let marks_changed = patch.scale_margins.is_some()
            || patch.entire_text_only.is_some()
            || patch.tick_mark_density.is_some()
            || patch.ensure_edge_tick_marks_visible.is_some();
        if let Some(value) = patch.align_labels {
            self.options.align_labels = value;
        }
        if let Some(value) = patch.entire_text_only {
            self.options.entire_text_only = value;
        }
        if let Some(value) = patch.visible {
            self.options.visible = value;
        }
        if let Some(value) = patch.border_visible {
            self.options.border_visible = value;
        }
        if let Some(value) = patch.border_color {
            self.options.border_color = value;
        }
        if let Some(value) = patch.text_color {
            self.options.text_color = value;
        }
        if let Some(value) = patch.ticks_visible {
            self.options.ticks_visible = value;
        }
        if let Some(value) = patch.minimum_width {
            self.options.minimum_width = value;
        }
        if let Some(value) = patch.ensure_edge_tick_marks_visible {
            self.options.ensure_edge_tick_marks_visible = value;
        }
        if let Some(value) = patch.tick_mark_density {
            self.options.tick_mark_density = value;
        }
        if let Some(margins) = margins {
            self.options.scale_margins = margins;
        }
        let change = self.set_mode(PriceScaleStatePatch {
            auto_scale: patch.auto_scale,
            inverted: patch.invert_scale,
            mode: patch.mode,
        });
        if marks_changed {
            self.rebuild_tick_mark_builder();
            self.invalidate_marks();
        }
        Ok(change)
    }

    pub fn replace_options(
        &mut self,
        options: PriceScaleOptions,
    ) -> Result<PriceScaleModeChange, PriceScaleOptionsError> {
        self.apply_options(PriceScaleOptionsPatch {
            auto_scale: Some(options.auto_scale),
            mode: Some(options.mode),
            invert_scale: Some(options.invert_scale),
            align_labels: Some(options.align_labels),
            scale_margins: Some(PriceScaleMarginsPatch {
                top: Some(options.scale_margins.top),
                bottom: Some(options.scale_margins.bottom),
            }),
            entire_text_only: Some(options.entire_text_only),
            visible: Some(options.visible),
            border_visible: Some(options.border_visible),
            border_color: Some(options.border_color),
            text_color: Some(options.text_color),
            ticks_visible: Some(options.ticks_visible),
            minimum_width: Some(options.minimum_width),
            ensure_edge_tick_marks_visible: Some(options.ensure_edge_tick_marks_visible),
            tick_mark_density: Some(options.tick_mark_density),
        })
    }
    pub fn height(&self) -> f64 {
        self.height
    }
    pub fn set_height(&mut self, height: f64) {
        let height = height.max(0.);
        if self.height != height {
            self.height = height;
            self.invalidate_marks();
        }
    }
    pub fn internal_height(&self) -> f64 {
        self.height * (1. - self.options.scale_margins.top - self.options.scale_margins.bottom)
    }
    pub fn font_size(&self) -> f64 {
        self.layout_font_size
    }
    pub(crate) fn set_font_size(&mut self, font_size: f64) {
        if self.layout_font_size != font_size {
            self.layout_font_size = font_size;
            self.rebuild_tick_mark_builder();
            self.invalidate_marks();
        }
    }
    pub fn is_auto_scale(&self) -> bool {
        self.options.auto_scale
    }
    pub fn mode(&self) -> PriceScaleState {
        PriceScaleState {
            auto_scale: self.options.auto_scale,
            inverted: self.options.invert_scale,
            mode: self.options.mode,
        }
    }
    pub fn mode_changed(&self) -> &Delegate<PriceScaleModeChange> {
        &self.mode_changed
    }
    pub fn marks_changed(&self) -> &Delegate<()> {
        &self.marks_changed
    }
    pub fn marks_generation(&self) -> u64 {
        self.marks_generation
    }
    pub fn is_custom_price_range(&self) -> bool {
        self.is_custom_price_range
    }
    pub fn set_auto_scale(&mut self, enabled: bool) -> bool {
        if self.options.auto_scale == enabled {
            return false;
        }
        self.options.auto_scale = enabled;
        self.invalidate_marks();
        true
    }
    pub fn is_log(&self) -> bool {
        self.options.mode == PriceScaleMode::Logarithmic
    }
    pub fn is_percentage(&self) -> bool {
        self.options.mode == PriceScaleMode::Percentage
    }
    pub fn is_indexed_to_100(&self) -> bool {
        self.options.mode == PriceScaleMode::IndexedTo100
    }
    pub fn price_range(&self) -> Option<PriceRangeImpl> {
        self.range
    }
    pub fn is_empty(&self) -> bool {
        self.height <= 0.0 || self.range.is_none_or(|range| range.is_empty())
    }
    pub fn is_inverted(&self) -> bool {
        self.options.invert_scale
    }
    pub fn set_price_range(&mut self, range: Option<PriceRangeImpl>) {
        if self.range != range {
            self.range = range;
            self.invalidate_marks();
        }
    }
    pub fn set_custom_price_range(&mut self, range: Option<PriceRangeImpl>) {
        self.set_price_range(range);
        self.is_custom_price_range = range.is_some();
    }
    pub fn set_mode(&mut self, patch: PriceScaleStatePatch) -> PriceScaleModeChange {
        let old = self.mode();
        if let Some(auto_scale) = patch.auto_scale {
            self.options.auto_scale = auto_scale;
        }

        let mode_changed = patch.mode.is_some_and(|mode| mode != old.mode);
        if mode_changed && old.mode == PriceScaleMode::Logarithmic {
            if can_convert_price_range_from_log(self.range, self.log_formula) {
                self.range = convert_price_range_from_log(self.range, self.log_formula);
            } else {
                self.options.auto_scale = true;
            }
        }

        if let Some(mode) = patch.mode {
            self.options.mode = mode;
            if matches!(
                mode,
                PriceScaleMode::Percentage | PriceScaleMode::IndexedTo100
            ) {
                self.options.auto_scale = true;
            }
        }

        if mode_changed && self.options.mode == PriceScaleMode::Logarithmic {
            self.range = convert_price_range_to_log(self.range, self.log_formula);
        }
        if let Some(inverted) = patch.inverted {
            self.options.invert_scale = inverted;
        }

        let change = PriceScaleModeChange {
            old,
            new: self.mode(),
        };
        if change.old != change.new {
            self.rebuild_tick_mark_builder();
            self.invalidate_marks();
        }
        self.mode_changed.emit(&change);
        change
    }
    pub fn add_data_source(&mut self, source: PriceSourceHandle) {
        if !self.sources.iter().any(|s| Rc::ptr_eq(s, &source)) {
            self.sources.push(source);
            self.update_formatter_source();
        }
    }
    pub fn remove_data_source(&mut self, source: &PriceSourceHandle) -> bool {
        let before = self.sources.len();
        self.sources.retain(|s| !Rc::ptr_eq(s, source));
        let changed = before != self.sources.len();
        if changed {
            self.update_formatter_source();
        }
        changed
    }
    /// Re-evaluates the formatter leader after Pane source ordering or source
    /// visibility changes. Returns true only when the selected source changed.
    pub fn refresh_formatter_source(&mut self) -> bool {
        self.update_formatter_source()
    }
    pub fn first_value(&self) -> Option<f64> {
        self.sources
            .iter()
            .filter_map(|s| s.borrow().first_value())
            .min_by(|a, b| {
                a.time_point
                    .partial_cmp(&b.time_point)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|v| v.value)
    }
    pub fn recalculate_price_range(&mut self, visible: &RangeImpl<TimePointIndex>) -> bool {
        self.calculate_price_range(visible, true)
    }
    pub fn recalculate_price_range_forced(&mut self, visible: &RangeImpl<TimePointIndex>) -> bool {
        self.calculate_price_range(visible, false)
    }
    fn calculate_price_range(
        &mut self,
        visible: &RangeImpl<TimePointIndex>,
        respect_auto_scale: bool,
    ) -> bool {
        if self.is_custom_price_range && !self.options.auto_scale {
            return false;
        }
        if respect_auto_scale && !self.options.auto_scale {
            return false;
        }
        let old_range = self.range;
        let old_margins = self.options.scale_margins;
        let mut range = None;
        let mut above: f64 = 0.0;
        let mut below: f64 = 0.0;
        for source in &self.sources {
            let source = source.borrow();
            if !source.visible() {
                continue;
            }
            let Some(first) = source.first_value() else {
                continue;
            };
            let Some(info) = source.autoscale_info(visible.left(), visible.right()) else {
                continue;
            };
            if let Some(mut r) = info.price_range() {
                r = match self.options.mode {
                    PriceScaleMode::Logarithmic => {
                        convert_price_range_to_log(Some(r), self.log_formula).unwrap()
                    }
                    PriceScaleMode::Percentage => to_percent_range(r, first.value),
                    PriceScaleMode::IndexedTo100 => to_indexed_to_100_range(r, first.value),
                    PriceScaleMode::Normal => r,
                };
                range = Some(range.map_or(r, |old: PriceRangeImpl| old.merge(Some(&r))));
            }
            if let Some(m) = info.margins() {
                above = above.max(m.above);
                below = below.max(m.below)
            }
        }
        self.range = range;
        if above != 0. || below != 0. {
            self.options.scale_margins.top = above / self.height.max(1.);
            self.options.scale_margins.bottom = below / self.height.max(1.);
        }
        let changed = self.range != old_range || self.options.scale_margins != old_margins;
        if changed {
            self.rebuild_tick_mark_builder();
            self.invalidate_marks();
        }
        changed
    }

    fn margins_after_patch(
        &self,
        patch: Option<&PriceScaleMarginsPatch>,
    ) -> Result<Option<PriceScaleMargins>, PriceScaleOptionsError> {
        let Some(patch) = patch else {
            return Ok(None);
        };
        let margins = PriceScaleMargins {
            top: patch.top.unwrap_or(self.options.scale_margins.top),
            bottom: patch.bottom.unwrap_or(self.options.scale_margins.bottom),
        };
        if !margins.top.is_finite() || !margins.bottom.is_finite() {
            return Err(PriceScaleOptionsError::NonFiniteMargin);
        }
        if !(0.0..=1.0).contains(&margins.top) {
            return Err(PriceScaleOptionsError::InvalidTopMargin(margins.top));
        }
        if !(0.0..=1.0).contains(&margins.bottom) {
            return Err(PriceScaleOptionsError::InvalidBottomMargin(margins.bottom));
        }
        let sum = margins.top + margins.bottom;
        if sum > 1.0 {
            return Err(PriceScaleOptionsError::InvalidMarginsSum(sum));
        }
        Ok(Some(margins))
    }

    pub fn start_scale(&mut self, pointer: Coordinate) -> bool {
        if self.is_percentage()
            || self.is_indexed_to_100()
            || self.is_empty()
            || self.scale_start.is_some()
            || self.price_range_snapshot.is_some()
        {
            return false;
        }
        let Some(range) = self.range else {
            return false;
        };
        self.scale_start = Some(Coordinate::new(self.height - pointer.value()));
        self.price_range_snapshot = Some(range);
        true
    }

    pub fn scale_to(&mut self, pointer: Coordinate) -> bool {
        if self.is_percentage() || self.is_indexed_to_100() {
            return false;
        }
        let Some(scale_start) = self.scale_start else {
            return false;
        };
        let Some(snapshot) = self.price_range_snapshot else {
            return false;
        };

        // The source routes this transition through `setMode`, so subscribers
        // observe an auto-scale change that originates from manual scaling.
        let was_auto_scale = self.options.auto_scale;
        self.set_mode(PriceScaleStatePatch {
            auto_scale: Some(false),
            ..PriceScaleStatePatch::default()
        });
        let inverted_pointer = (self.height - pointer.value()).max(0.0);
        let padding = (self.height - 1.0) * 0.2;
        let denominator = inverted_pointer + padding;
        if !denominator.is_finite() || denominator <= 0.0 {
            return was_auto_scale;
        }
        let coefficient = ((scale_start.value() + padding) / denominator).max(0.1);
        if !coefficient.is_finite() {
            return was_auto_scale;
        }
        let mut range = snapshot;
        range.scale_around_center(coefficient);
        let changed = self.range != Some(range);
        self.range = Some(range);
        if changed {
            self.invalidate_marks();
        }
        was_auto_scale || changed
    }

    pub fn end_scale(&mut self) -> bool {
        if self.is_percentage() || self.is_indexed_to_100() {
            return false;
        }
        let changed = self.scale_start.is_some() || self.price_range_snapshot.is_some();
        self.scale_start = None;
        self.price_range_snapshot = None;
        changed
    }

    pub fn start_scroll(&mut self, pointer: Coordinate) -> bool {
        if self.is_auto_scale()
            || self.scroll_start.is_some()
            || self.price_range_snapshot.is_some()
        {
            return false;
        }
        if self.is_empty() {
            return false;
        }
        let range = self.range.expect("a non-empty price scale has a range");
        self.scroll_start = Some(pointer);
        self.price_range_snapshot = Some(range);
        true
    }

    pub fn scroll_to(&mut self, pointer: Coordinate) -> bool {
        if self.is_auto_scale() {
            return false;
        }
        let Some(scroll_start) = self.scroll_start else {
            return false;
        };
        let Some(snapshot) = self.price_range_snapshot else {
            return false;
        };
        let usable_height = self.internal_height() - 1.0;
        if !usable_height.is_finite() || usable_height <= 0.0 {
            return false;
        }
        let price_units_per_pixel = snapshot.length() / usable_height;
        if !price_units_per_pixel.is_finite() {
            return false;
        }
        let mut pixel_delta = pointer.value() - scroll_start.value();
        if self.is_inverted() {
            pixel_delta *= -1.0;
        }
        let mut range = snapshot;
        range.shift(pixel_delta * price_units_per_pixel);
        let changed = self.range != Some(range);
        self.range = Some(range);
        if changed {
            self.invalidate_marks();
        }
        changed
    }

    pub fn end_scroll(&mut self) -> bool {
        if self.is_auto_scale() || self.scroll_start.is_none() {
            return false;
        }
        self.scroll_start = None;
        self.price_range_snapshot = None;
        true
    }
    fn logical(&self, price: f64, base: f64) -> f64 {
        match self.options.mode {
            PriceScaleMode::Normal => price,
            PriceScaleMode::Logarithmic => to_log(price, self.log_formula),
            PriceScaleMode::Percentage => to_percent(price, base),
            PriceScaleMode::IndexedTo100 => to_indexed_to_100(price, base),
        }
    }
    fn unlogical(&self, value: f64, base: f64) -> f64 {
        match self.options.mode {
            PriceScaleMode::Normal => value,
            PriceScaleMode::Logarithmic => from_log(value, self.log_formula),
            PriceScaleMode::Percentage => from_percent(value, base),
            PriceScaleMode::IndexedTo100 => from_indexed_to_100(value, base),
        }
    }
    pub fn price_to_coordinate(&self, price: f64, base: f64) -> Coordinate {
        let Some(range) = self.range else {
            return Coordinate::new(f64::NAN);
        };
        let h =
            self.height * (1. - self.options.scale_margins.top - self.options.scale_margins.bottom);
        if h <= 0. || range.length() == 0. {
            return Coordinate::new(f64::NAN);
        }
        let ratio = (self.logical(price, base) - range.min_value()) / range.length();
        let y = self.height * (1. - self.options.scale_margins.bottom) - ratio * h;
        Coordinate::new(if self.options.invert_scale {
            self.height - y
        } else {
            y
        })
    }
    pub fn coordinate_to_price(&self, coord: Coordinate, base: f64) -> f64 {
        let Some(range) = self.range else {
            return f64::NAN;
        };
        let h =
            self.height * (1. - self.options.scale_margins.top - self.options.scale_margins.bottom);
        let mut y = coord.value();
        if self.options.invert_scale {
            y = self.height - y
        }
        let ratio = (self.height * (1. - self.options.scale_margins.bottom) - y) / h;
        self.unlogical(range.min_value() + ratio * range.length(), base)
    }
    pub fn format_price(&self, price: f64, base: f64) -> String {
        match self.options.mode {
            PriceScaleMode::Percentage => {
                PercentageFormatter::default().format(to_percent(price, base))
            }
            PriceScaleMode::IndexedTo100 => {
                Self::indexed_formatter().format(to_indexed_to_100(price, base))
            }
            PriceScaleMode::Normal | PriceScaleMode::Logarithmic => self.format_absolute(price),
        }
    }
    pub fn format_logical(&self, logical: f64) -> String {
        match self.options.mode {
            PriceScaleMode::Percentage => PercentageFormatter::default().format(logical),
            PriceScaleMode::IndexedTo100 => Self::indexed_formatter().format(logical),
            PriceScaleMode::Logarithmic => self.format_absolute(logical),
            PriceScaleMode::Normal => self.format_absolute(logical),
        }
    }
    pub fn format_logical_tickmarks(&self, logicals: &[f64]) -> Vec<String> {
        logicals
            .iter()
            .map(|&logical| self.format_logical(logical))
            .collect()
    }
    pub fn has_visible_edge_marks(&self) -> bool {
        self.options.ensure_edge_tick_marks_visible && self.is_auto_scale()
    }
    pub fn edge_marks_padding(&self) -> f64 {
        self.font_size() / 2.0
    }
    pub fn update_all_views(&self) {
        for source in &self.sources {
            source.borrow_mut().update_all_views();
        }
    }
    pub fn marks(&mut self) -> &[PriceMark] {
        if self.marks_cache.is_none() {
            self.marks_cache = Some(self.build_marks());
        }
        self.marks_cache
            .as_deref()
            .expect("marks cache was initialized above")
    }

    fn build_marks(&self) -> Vec<PriceMark> {
        let (Some(range), Some(first)) = (self.range, self.first_value()) else {
            return Vec::new();
        };
        if self.is_empty() || self.internal_height() <= 0.0 {
            return Vec::new();
        }
        let bottom = self.coordinate_to_tick_logical(self.height - 1.0, first);
        let top = self.coordinate_to_tick_logical(0.0, first);
        let high = bottom.max(top);
        let low = bottom.min(top);
        if !high.is_finite() || !low.is_finite() || high == low {
            return Vec::new();
        }
        let mut marks = self.mark_builder.build(
            first,
            self.height,
            low,
            high,
            |logical, base| self.logical_to_coordinate(logical, base),
            |logical| self.format_logical(logical),
        );
        if self.options.entire_text_only {
            let padding = self.font_size() / 2.0;
            marks.retain(|mark| {
                mark.coord.value() >= padding && mark.coord.value() <= self.height - 1.0 - padding
            });
        }
        if self.has_visible_edge_marks() {
            self.apply_edge_marks(&mut marks, first, range, low, high);
        }
        marks
    }

    fn formatter_source(&self) -> Option<PriceSourceHandle> {
        let mut selected = None;
        let mut selected_order = i32::MAX;
        for source in &self.sources {
            let source_ref = source.borrow();
            if source_ref.visible() && source_ref.z_order() < selected_order {
                selected_order = source_ref.z_order();
                selected = Some(source.clone());
            }
        }
        selected
    }

    fn update_formatter_source(&mut self) -> bool {
        let selected = self.formatter_source();
        let changed = match (&self.selected_formatter_source, &selected) {
            (Some(old), Some(new)) => !Rc::ptr_eq(old, new),
            (None, None) => false,
            _ => true,
        };
        if changed {
            self.selected_formatter_source = selected;
            self.rebuild_tick_mark_builder();
            self.invalidate_marks();
        }
        changed
    }

    fn formatter_base(&self) -> f64 {
        if self.is_percentage() || self.is_indexed_to_100() {
            return 100.0;
        }
        self.selected_formatter_source
            .as_ref()
            .map(|source| source.borrow().base().round())
            .filter(|base| base.is_finite() && *base > 0.0)
            .unwrap_or(100.0)
    }

    fn rebuild_tick_mark_builder(&mut self) {
        self.mark_builder = PriceTickMarkBuilder::new(
            self.formatter_base(),
            self.font_size(),
            self.options.tick_mark_density,
        );
    }

    fn invalidate_marks(&mut self) {
        if self.marks_cache.take().is_some() {
            self.marks_generation = self.marks_generation.wrapping_add(1);
            self.marks_changed.emit(&());
        }
    }

    fn format_absolute(&self, price: f64) -> String {
        self.selected_formatter_source
            .as_ref()
            .map(|source| source.borrow().format_price(price))
            .unwrap_or_else(|| PriceFormatter::default().format(price))
    }

    fn indexed_formatter() -> PriceFormatter {
        PriceFormatter::new(Some(100.0), Some(1.0))
            .expect("the indexed-to-100 formatter configuration is valid")
    }

    fn apply_edge_marks(
        &self,
        marks: &mut Vec<PriceMark>,
        first_value: f64,
        range: PriceRangeImpl,
        low: f64,
        high: f64,
    ) {
        let span = self.mark_builder.tick_span(high, low, self.height);
        if !span.is_finite() || span <= 0.0 {
            return;
        }
        let range = if self.is_log() {
            convert_price_range_from_log(Some(range), self.log_formula)
                .expect("a present logarithmic range remains present after conversion")
        } else {
            range
        };
        if range.min_value() - low >= span || high - range.max_value() >= span {
            return;
        }
        let padding = self.edge_marks_padding();
        let min_coord = if self.options.entire_text_only {
            padding
        } else {
            0.0
        };
        let max_coord = if self.options.entire_text_only {
            self.height - 1.0 - padding
        } else {
            self.height - 1.0
        };
        let top = self.boundary_mark(first_value, min_coord, padding, padding * 2.0, span);
        let bottom = self.boundary_mark(first_value, max_coord, -padding * 2.0, -padding, span);
        let span_px = (self.logical_to_coordinate(0.0, first_value).value()
            - self.logical_to_coordinate(span, first_value).value())
        .abs();
        if let Some(top) = top
            && !marks
                .iter()
                .any(|mark| (mark.logical - top.logical).abs() < f64::EPSILON)
        {
            if marks
                .first()
                .is_some_and(|mark| (mark.coord.value() - top.coord.value()).abs() < span_px / 2.0)
            {
                marks.remove(0);
            }
            marks.insert(0, top);
        }
        if let Some(bottom) = bottom
            && !marks
                .iter()
                .any(|mark| (mark.logical - bottom.logical).abs() < f64::EPSILON)
        {
            if marks.last().is_some_and(|mark| {
                (mark.coord.value() - bottom.coord.value()).abs() < span_px / 2.0
            }) {
                marks.pop();
            }
            marks.push(bottom);
        }
    }

    fn boundary_mark(
        &self,
        first_value: f64,
        coord: f64,
        min_padding: f64,
        max_padding: f64,
        span: f64,
    ) -> Option<PriceMark> {
        let average_padding = (min_padding + max_padding) / 2.0;
        let value = self.coordinate_to_tick_logical(coord + average_padding, first_value);
        if !value.is_finite() {
            return None;
        }
        let logical = value - value.rem_euclid(span);
        let coord = self.logical_to_coordinate(logical, first_value);
        if !coord.value().is_finite() {
            return None;
        }
        Some(PriceMark {
            coord,
            label: self.format_logical(logical),
            logical,
        })
    }

    fn logical_to_coordinate(&self, logical: f64, first_value: f64) -> Coordinate {
        // Log tick values are absolute prices, not already transformed values.
        // Applying from_log here a second time can overflow for large prices.
        let price = match self.options.mode {
            PriceScaleMode::Normal | PriceScaleMode::Logarithmic => logical,
            PriceScaleMode::Percentage | PriceScaleMode::IndexedTo100 => {
                self.unlogical(logical, first_value)
            }
        };
        self.price_to_coordinate(price, first_value)
    }

    fn coordinate_to_tick_logical(&self, coordinate: f64, first_value: f64) -> f64 {
        let price = self.coordinate_to_price(Coordinate::new(coordinate), first_value);
        match self.options.mode {
            PriceScaleMode::Normal | PriceScaleMode::Logarithmic => price,
            PriceScaleMode::Percentage | PriceScaleMode::IndexedTo100 => {
                self.logical(price, first_value)
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        autoscale_info_impl::AutoscaleInfoImpl,
        iprice_data_source::FirstValue,
        layout_options::{Background, ColorSpace, LayoutPanesOptions},
    };

    struct StaticSource {
        range: PriceRangeImpl,
        z_order: i32,
    }

    impl PriceScaleDataSource for StaticSource {
        fn z_order(&self) -> i32 {
            self.z_order
        }
        fn set_z_order(&mut self, z_order: i32) {
            self.z_order = z_order;
        }
        fn visible(&self) -> bool {
            true
        }
        fn first_value(&self) -> Option<FirstValue> {
            Some(FirstValue {
                value: self.range.min_value(),
                time_point: 0.0.into(),
            })
        }
        fn format_price(&self, price: f64) -> String {
            price.to_string()
        }
        fn base(&self) -> f64 {
            1.0
        }
        fn autoscale_info(
            &self,
            _start: TimePointIndex,
            _end: TimePointIndex,
        ) -> Option<AutoscaleInfoImpl> {
            Some(AutoscaleInfoImpl::new(Some(self.range), None))
        }
        fn update_all_views(&mut self) {}
    }

    struct FormattingSource {
        range: PriceRangeImpl,
        z_order: i32,
        visible: bool,
        prefix: &'static str,
        base: f64,
    }

    impl PriceScaleDataSource for FormattingSource {
        fn z_order(&self) -> i32 {
            self.z_order
        }
        fn set_z_order(&mut self, z_order: i32) {
            self.z_order = z_order;
        }
        fn visible(&self) -> bool {
            self.visible
        }
        fn first_value(&self) -> Option<FirstValue> {
            Some(FirstValue {
                value: self.range.min_value(),
                time_point: 0.0.into(),
            })
        }
        fn format_price(&self, price: f64) -> String {
            format!("{}:{price:.2}", self.prefix)
        }
        fn base(&self) -> f64 {
            self.base
        }
        fn autoscale_info(
            &self,
            _start: TimePointIndex,
            _end: TimePointIndex,
        ) -> Option<AutoscaleInfoImpl> {
            Some(AutoscaleInfoImpl::new(Some(self.range), None))
        }
        fn update_all_views(&mut self) {}
    }

    fn formatting_source(prefix: &'static str, z_order: i32, visible: bool) -> PriceSourceHandle {
        Rc::new(RefCell::new(FormattingSource {
            range: PriceRangeImpl::new(0.0, 100.0),
            z_order,
            visible,
            prefix,
            base: 100.0,
        }))
    }

    fn layout() -> LayoutOptions {
        LayoutOptions {
            background: Background::Solid {
                color: "white".into(),
            },
            text_color: "black".into(),
            font_size: 12.,
            font_family: "sans".into(),
            panes: LayoutPanesOptions {
                enable_resize: true,
                separator_color: "".into(),
                separator_hover_color: "".into(),
            },
            attribution_logo: true,
            color_space: ColorSpace::Srgb,
            color_parsers: vec![],
        }
    }
    #[test]
    fn coordinates_round_trip() {
        let mut s = PriceScale::new("right", PriceScaleOptions::default(), &layout());
        s.set_height(100.);
        s.set_price_range(Some(PriceRangeImpl::new(0., 100.)));
        let y = s.price_to_coordinate(25., 1.);
        assert!((s.coordinate_to_price(y, 1.) - 25.).abs() < 1e-9);
    }

    fn scale_with_range(range: PriceRangeImpl) -> PriceScale {
        let mut scale = PriceScale::new("right", PriceScaleOptions::default(), &layout());
        scale.set_height(100.0);
        scale.set_price_range(Some(range));
        scale
    }

    #[test]
    fn scales_from_a_snapshot_and_clamps_the_coefficient() {
        let mut scale = scale_with_range(PriceRangeImpl::new(0.0, 100.0));
        assert!(scale.start_scale(50.0.into()));
        assert!(!scale.start_scroll(50.0.into()));
        assert!(scale.scale_to(75.0.into()));
        assert!(!scale.is_auto_scale());
        assert!(scale.end_scale());
        assert!(!scale.end_scale());

        let mut minimum = scale_with_range(PriceRangeImpl::new(0.0, 100.0));
        assert!(minimum.start_scale(200.0.into()));
        assert!(minimum.scale_to(0.0.into()));
        assert_eq!(minimum.price_range(), Some(PriceRangeImpl::new(45.0, 55.0)));
    }

    #[test]
    fn rejects_scale_interactions_for_special_modes_and_empty_ranges() {
        for mode in [PriceScaleMode::Percentage, PriceScaleMode::IndexedTo100] {
            let mut options = PriceScaleOptions::default();
            options.mode = mode;
            let mut scale = PriceScale::new("right", options, &layout());
            scale.set_height(100.0);
            scale.set_price_range(Some(PriceRangeImpl::new(0.0, 100.0)));
            assert!(!scale.start_scale(20.0.into()));
            assert!(!scale.scale_to(30.0.into()));
            assert!(!scale.end_scale());
        }

        let mut empty = PriceScale::new("right", PriceScaleOptions::default(), &layout());
        assert!(empty.is_empty());
        assert!(!empty.start_scale(20.0.into()));
        assert!(!empty.start_scroll(20.0.into()));
    }

    #[test]
    fn scrolls_in_normal_and_inverted_directions_and_respects_lifecycle() {
        let mut normal = scale_with_range(PriceRangeImpl::new(0.0, 70.0));
        normal.set_auto_scale(false);
        assert!(normal.start_scroll(10.0.into()));
        assert!(!normal.start_scale(10.0.into()));
        assert!(normal.scroll_to(20.0.into()));
        let normal_range = normal.price_range().unwrap();
        let delta = 70.0 / 69.0 * 10.0;
        assert!((normal_range.min_value() - delta).abs() < 1e-9);
        assert!((normal_range.max_value() - (70.0 + delta)).abs() < 1e-9);
        assert!(normal.end_scroll());

        let mut inverted_options = PriceScaleOptions::default();
        inverted_options.invert_scale = true;
        let mut inverted = PriceScale::new("right", inverted_options, &layout());
        inverted.set_height(100.0);
        inverted.set_price_range(Some(PriceRangeImpl::new(0.0, 70.0)));
        inverted.set_auto_scale(false);
        assert!(inverted.start_scroll(10.0.into()));
        assert!(inverted.scroll_to(20.0.into()));
        let inverted_range = inverted.price_range().unwrap();
        assert!((inverted_range.min_value() + delta).abs() < 1e-9);
        assert!((inverted_range.max_value() - (70.0 - delta)).abs() < 1e-9);
    }

    #[test]
    fn auto_scale_blocks_scrolling_and_forced_recalculation_preserves_manual_mode() {
        let mut scale = scale_with_range(PriceRangeImpl::new(0.0, 1.0));
        assert!(!scale.start_scroll(10.0.into()));
        assert!(scale.set_auto_scale(false));
        assert!(scale.start_scroll(10.0.into()));
        assert!(scale.end_scroll());
        assert!(scale.set_auto_scale(true));
        assert!(!scale.start_scroll(10.0.into()));

        scale.set_auto_scale(false);
        let source: PriceSourceHandle = Rc::new(RefCell::new(StaticSource {
            range: PriceRangeImpl::new(10.0, 20.0),
            z_order: 0,
        }));
        scale.add_data_source(source);
        let visible = RangeImpl::new(0.0.into(), 0.0.into());
        assert!(scale.recalculate_price_range_forced(&visible));
        assert!(!scale.is_auto_scale());
        assert_eq!(scale.price_range(), Some(PriceRangeImpl::new(10.0, 20.0)));
    }

    #[test]
    fn mode_state_and_custom_ranges_are_explicit() {
        let mut scale = scale_with_range(PriceRangeImpl::new(10.0, 20.0));
        assert_eq!(
            scale.mode(),
            PriceScaleState {
                auto_scale: true,
                inverted: false,
                mode: PriceScaleMode::Normal,
            }
        );
        scale.set_custom_price_range(Some(PriceRangeImpl::new(3.0, 7.0)));
        assert!(scale.is_custom_price_range());
        assert_eq!(scale.price_range(), Some(PriceRangeImpl::new(3.0, 7.0)));
        scale.set_custom_price_range(None);
        assert!(!scale.is_custom_price_range());
        assert_eq!(scale.price_range(), None);
    }

    #[test]
    fn mode_transitions_convert_ranges_and_special_modes_enable_auto_scale() {
        let mut scale = scale_with_range(PriceRangeImpl::new(10.0, 100.0));
        let original = scale.price_range();
        scale.set_auto_scale(false);
        scale.set_mode(PriceScaleStatePatch {
            mode: Some(PriceScaleMode::Logarithmic),
            ..PriceScaleStatePatch::default()
        });
        assert_ne!(scale.price_range(), original);
        assert_eq!(scale.mode().mode, PriceScaleMode::Logarithmic);
        scale.set_mode(PriceScaleStatePatch {
            mode: Some(PriceScaleMode::Normal),
            ..PriceScaleStatePatch::default()
        });
        let restored = scale.price_range().unwrap();
        let original = original.unwrap();
        assert!((restored.min_value() - original.min_value()).abs() < 1e-8);
        assert!((restored.max_value() - original.max_value()).abs() < 1e-8);

        scale.set_auto_scale(false);
        for mode in [PriceScaleMode::Percentage, PriceScaleMode::IndexedTo100] {
            scale.set_mode(PriceScaleStatePatch {
                mode: Some(mode),
                ..PriceScaleStatePatch::default()
            });
            assert!(scale.is_auto_scale());
            scale.set_auto_scale(false);
        }
    }

    #[test]
    fn invalid_log_range_falls_back_to_auto_scale_when_leaving_log_mode() {
        let mut scale = scale_with_range(PriceRangeImpl::new(f64::NAN, 10.0));
        scale.set_mode(PriceScaleStatePatch {
            mode: Some(PriceScaleMode::Logarithmic),
            ..PriceScaleStatePatch::default()
        });
        scale.set_auto_scale(false);
        scale.set_mode(PriceScaleStatePatch {
            mode: Some(PriceScaleMode::Normal),
            ..PriceScaleStatePatch::default()
        });
        assert!(scale.is_auto_scale());
    }

    #[test]
    fn patches_are_transactional_for_margins_and_change_only_named_fields() {
        let mut scale = scale_with_range(PriceRangeImpl::new(0.0, 10.0));
        let before = scale.options().clone();
        assert_eq!(
            scale.apply_options(PriceScaleOptionsPatch {
                scale_margins: Some(PriceScaleMarginsPatch {
                    top: Some(-0.1),
                    bottom: None,
                }),
                ..PriceScaleOptionsPatch::default()
            }),
            Err(PriceScaleOptionsError::InvalidTopMargin(-0.1))
        );
        assert_eq!(scale.options(), &before);

        scale
            .apply_options(PriceScaleOptionsPatch {
                visible: Some(false),
                ..PriceScaleOptionsPatch::default()
            })
            .unwrap();
        assert!(!scale.options().visible);
        assert_eq!(scale.options().scale_margins, before.scale_margins);
        assert_eq!(scale.options().mode, before.mode);
    }

    #[test]
    fn inversion_retains_the_range_and_mode_delegate_fires_for_every_set_mode_call() {
        use std::{cell::RefCell, rc::Rc};

        let mut scale = scale_with_range(PriceRangeImpl::new(2.0, 8.0));
        let received = Rc::new(RefCell::new(Vec::new()));
        let events = Rc::clone(&received);
        let subscription = scale.mode_changed().subscribe(move |event| {
            events.borrow_mut().push(*event);
        });
        let range = scale.price_range();
        scale.set_mode(PriceScaleStatePatch {
            inverted: Some(true),
            ..PriceScaleStatePatch::default()
        });
        scale.set_mode(PriceScaleStatePatch::default());
        assert_eq!(scale.price_range(), range);
        assert!(scale.is_inverted());
        assert_eq!(received.borrow().len(), 2);
        assert!(received.borrow()[0].new.inverted);
        drop(subscription);
        scale.set_mode(PriceScaleStatePatch::default());
        assert_eq!(received.borrow().len(), 2);
    }

    #[test]
    fn forced_recalculation_keeps_a_manual_custom_range_intact() {
        let mut scale = scale_with_range(PriceRangeImpl::new(0.0, 1.0));
        scale.set_auto_scale(false);
        scale.set_custom_price_range(Some(PriceRangeImpl::new(3.0, 7.0)));
        let source: PriceSourceHandle = Rc::new(RefCell::new(StaticSource {
            range: PriceRangeImpl::new(10.0, 20.0),
            z_order: 0,
        }));
        scale.add_data_source(source);
        let visible = RangeImpl::new(0.0.into(), 0.0.into());
        assert!(!scale.recalculate_price_range_forced(&visible));
        assert_eq!(scale.price_range(), Some(PriceRangeImpl::new(3.0, 7.0)));
        assert!(!scale.is_auto_scale());
        assert!(scale.is_custom_price_range());
    }

    #[test]
    fn formatter_leader_uses_visible_lowest_z_order_and_stable_ties() {
        let mut scale = scale_with_range(PriceRangeImpl::new(0.0, 100.0));
        scale.add_data_source(formatting_source("first", 2, true));
        scale.add_data_source(formatting_source("hidden", 0, false));
        scale.add_data_source(formatting_source("leader", 1, true));
        assert_eq!(scale.format_logical(12.0), "leader:12.00");

        let mut ties = scale_with_range(PriceRangeImpl::new(0.0, 100.0));
        ties.add_data_source(formatting_source("first", 1, true));
        ties.add_data_source(formatting_source("second", 1, true));
        assert_eq!(ties.format_logical(12.0), "first:12.00");
    }

    #[test]
    fn logical_formatting_follows_each_price_scale_mode() {
        let mut scale = scale_with_range(PriceRangeImpl::new(0.0, 100.0));
        scale.add_data_source(formatting_source("source", 0, true));
        assert_eq!(scale.format_logical(12.0), "source:12.00");

        scale.set_mode(PriceScaleStatePatch {
            mode: Some(PriceScaleMode::Percentage),
            ..PriceScaleStatePatch::default()
        });
        assert_eq!(scale.format_logical(12.0), "12.00%");

        scale.set_mode(PriceScaleStatePatch {
            mode: Some(PriceScaleMode::IndexedTo100),
            ..PriceScaleStatePatch::default()
        });
        assert_eq!(scale.format_logical(12.0), "12.00");

        scale.set_mode(PriceScaleStatePatch {
            mode: Some(PriceScaleMode::Normal),
            ..PriceScaleStatePatch::default()
        });
        scale.set_mode(PriceScaleStatePatch {
            mode: Some(PriceScaleMode::Logarithmic),
            ..PriceScaleStatePatch::default()
        });
        assert_eq!(scale.format_logical(12.0), "source:12.00");
    }

    #[test]
    fn marks_cache_invalidates_for_geometry_tick_options_and_formatter_changes() {
        let mut scale = scale_with_range(PriceRangeImpl::new(0.0, 100.0));
        scale.add_data_source(formatting_source("first", 1, true));
        scale.add_data_source(formatting_source("second", 2, true));
        let first_ptr = scale.marks().as_ptr();
        assert!(!scale.marks().is_empty());
        assert_eq!(scale.marks().as_ptr(), first_ptr);
        let generation = scale.marks_generation();

        scale.set_height(200.0);
        assert!(scale.marks_generation() > generation);
        assert!(!scale.marks().is_empty());
        let generation = scale.marks_generation();
        scale
            .apply_options(PriceScaleOptionsPatch {
                tick_mark_density: Some(4.0),
                ..PriceScaleOptionsPatch::default()
            })
            .unwrap();
        assert!(scale.marks_generation() > generation);

        let generation = scale.marks_generation();
        scale
            .apply_options(PriceScaleOptionsPatch {
                visible: Some(false),
                ..PriceScaleOptionsPatch::default()
            })
            .unwrap();
        assert_eq!(scale.marks_generation(), generation);

        let second = scale.sources[1].clone();
        second.borrow_mut().set_z_order(0);
        assert!(scale.refresh_formatter_source());
        assert_eq!(scale.format_logical(10.0), "second:10.00");
    }

    #[test]
    fn edge_marks_are_deduplicated_and_marks_notifications_follow_cache_lifecycle() {
        use std::{cell::Cell, rc::Rc};

        let mut scale = scale_with_range(PriceRangeImpl::new(0.0, 100.0));
        scale.add_data_source(formatting_source("source", 0, true));
        let notifications = Rc::new(Cell::new(0));
        let observed = Rc::clone(&notifications);
        let subscription = scale.marks_changed().subscribe(move |_| {
            observed.set(observed.get() + 1);
        });
        let ordinary = scale.marks().to_vec();
        assert!(!ordinary.is_empty());
        scale
            .apply_options(PriceScaleOptionsPatch {
                ensure_edge_tick_marks_visible: Some(true),
                ..PriceScaleOptionsPatch::default()
            })
            .unwrap();
        assert_eq!(notifications.get(), 1);
        assert!(scale.has_visible_edge_marks());
        assert_eq!(scale.edge_marks_padding(), scale.font_size() / 2.0);
        let with_edges = scale.marks().to_vec();
        assert!(with_edges.len() >= ordinary.len());
        for (index, mark) in with_edges.iter().enumerate() {
            assert!(
                with_edges[..index]
                    .iter()
                    .all(|other| (other.logical - mark.logical).abs() >= f64::EPSILON)
            );
        }
        scale.set_price_range(Some(PriceRangeImpl::new(1.0, 101.0)));
        assert_eq!(notifications.get(), 2);
        drop(subscription);
        scale.marks();
        scale.set_price_range(Some(PriceRangeImpl::new(2.0, 102.0)));
        assert_eq!(notifications.get(), 2);
    }
}
