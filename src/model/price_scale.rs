//! Synchronous price-to-coordinate conversion and source autoscaling.
use crate::{
    formatters::{percentage_formatter::PercentageFormatter, price_formatter::PriceFormatter},
    model::{
        coordinate::Coordinate, iprice_data_source::PriceScaleDataSource,
        layout_options::LayoutOptions, price_range_impl::PriceRangeImpl,
        price_scale_conversions::*, range_impl::RangeImpl, time_data::TimePointIndex,
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
    log_formula: LogFormula,
    layout_font_size: f64,
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
        Self {
            id: id.into(),
            options,
            height: 0.,
            range: None,
            sources: vec![],
            log_formula: DEFAULT_LOG_FORMULA,
            layout_font_size,
        }
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn options(&self) -> &PriceScaleOptions {
        &self.options
    }
    pub fn apply_options(&mut self, options: PriceScaleOptions) {
        self.options = options;
        self.range = None;
    }
    pub fn height(&self) -> f64 {
        self.height
    }
    pub fn set_height(&mut self, height: f64) {
        self.height = height.max(0.);
    }
    pub fn font_size(&self) -> f64 {
        self.layout_font_size
    }
    pub fn is_auto_scale(&self) -> bool {
        self.options.auto_scale
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
    pub fn set_price_range(&mut self, range: Option<PriceRangeImpl>) {
        self.range = range;
    }
    pub fn add_data_source(&mut self, source: PriceSourceHandle) {
        if !self.sources.iter().any(|s| Rc::ptr_eq(s, &source)) {
            self.sources.push(source)
        }
    }
    pub fn remove_data_source(&mut self, source: &PriceSourceHandle) -> bool {
        let before = self.sources.len();
        self.sources.retain(|s| !Rc::ptr_eq(s, source));
        before != self.sources.len()
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
    pub fn recalculate_price_range(&mut self, visible: &RangeImpl<TimePointIndex>) {
        if !self.options.auto_scale {
            return;
        }
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
                PriceFormatter::default().format(to_indexed_to_100(price, base))
            }
            _ => {
                if let Some(source) = self.sources.iter().min_by_key(|s| s.borrow().z_order()) {
                    source.borrow().format_price(price)
                } else {
                    PriceFormatter::default().format(price)
                }
            }
        }
    }
    pub fn update_all_views(&self) {
        for source in &self.sources {
            source.borrow_mut().update_all_views();
        }
    }
    pub fn marks(&self) -> Vec<PriceMark> {
        let (Some(range), Some(first)) = (self.range, self.first_value()) else {
            return Vec::new();
        };
        let builder = crate::model::price_tick_mark_builder::PriceTickMarkBuilder::new(
            100.0,
            self.font_size(),
            self.options.tick_mark_density,
        );
        builder.build(
            first,
            self.height,
            range.min_value(),
            range.max_value(),
            |logical, base| self.price_to_coordinate(self.unlogical(logical, base), base),
            |logical| self.format_price(self.unlogical(logical, first), first),
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::layout_options::{Background, ColorSpace, LayoutPanesOptions};
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
}
