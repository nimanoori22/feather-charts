//! Synchronous Pane ownership and visible-range price-scale recalculation.

use crate::model::{
    autoscale_info_impl::AutoscaleInfoImpl,
    data_layer::SeriesId,
    iprice_data_source::{FirstValue, PriceScaleDataSource},
    layout_options::LayoutOptions,
    price_scale::{PriceScale, PriceScaleOptions, PriceSourceHandle},
    price_scale_visible_range::PriceScaleVisibleRange,
    range_impl::RangeImpl,
    series::{CustomSeriesHandle, CustomSeriesPriceScaleSource, Series},
    time_data::TimePointIndex,
};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaneOptions {
    pub left_price_scale: PriceScaleOptions,
    pub right_price_scale: PriceScaleOptions,
    pub overlay_price_scale: PriceScaleOptions,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaneEffect {
    RecalculateAllPanes,
    LightUpdate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaneError {
    SeriesAlreadyAttached(SeriesId),
    UnknownSeries(SeriesId),
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

struct AttachedSeries<I, O, M = ()> {
    series: SeriesHandle<I, O, M>,
    source: PriceSourceHandle,
    position: PriceScalePosition,
}

struct AttachedCustomSource {
    id: SeriesId,
    source: PriceSourceHandle,
    position: PriceScalePosition,
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
    attached_series: Vec<AttachedSeries<I, O, M>>,
    attached_custom_sources: Vec<AttachedCustomSource>,
    visible_range: PriceScaleVisibleRange,
    invalidation_generation: u64,
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
            attached_series: vec![],
            attached_custom_sources: vec![],
            visible_range: PriceScaleVisibleRange::default(),
            invalidation_generation: 0,
        }
    }

    pub const fn index(&self) -> usize {
        self.index
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

    pub fn price_scale_by_id(&self, id: &str) -> Option<&PriceScale> {
        match id {
            LEFT_PRICE_SCALE_ID => Some(&self.left_price_scale),
            RIGHT_PRICE_SCALE_ID => Some(&self.right_price_scale),
            _ => self.overlay_price_scales.get(id),
        }
    }

    pub fn attached_series_ids(&self) -> Vec<SeriesId> {
        self.attached_series
            .iter()
            .map(|attached| attached.series.borrow().id())
            .collect()
    }

    pub fn attached_custom_series_ids(&self) -> Vec<SeriesId> {
        self.attached_custom_sources
            .iter()
            .map(|attached| attached.id)
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
            .set_z_order(self.attached_series.len() as i32);
        let source: PriceSourceHandle = Rc::new(RefCell::new(PaneSeriesPriceSource {
            series: series.clone(),
            visible: self.visible_range.clone(),
        }));
        self.price_scale_mut(&position)
            .add_data_source(source.clone());
        self.attached_series.push(AttachedSeries {
            series,
            source,
            position: position.clone(),
        });
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
            .set_z_order((self.attached_series.len() + self.attached_custom_sources.len()) as i32);
        let source: PriceSourceHandle = Rc::new(RefCell::new(CustomSeriesPriceScaleSource::new(
            series,
            self.visible_range.clone(),
        )));
        self.price_scale_mut(&position)
            .add_data_source(source.clone());
        self.attached_custom_sources.push(AttachedCustomSource {
            id,
            source,
            position: position.clone(),
        });
        let mut effects = self.invalidate();
        let visible = self.visible_range.get();
        effects.extend(self.recalculate_price_scale(position, visible.as_ref()));
        Ok(deduplicate_effects(effects))
    }

    pub fn detach_series(&mut self, id: SeriesId) -> Result<Vec<PaneEffect>, PaneError> {
        let index = self
            .attached_series
            .iter()
            .position(|attached| attached.series.borrow().id() == id)
            .ok_or(PaneError::UnknownSeries(id))?;
        let attached = self.attached_series.remove(index);
        let position = attached.position.clone();
        if let Some(scale) = self.price_scale_mut_existing(&position) {
            scale.remove_data_source(&attached.source);
        }
        self.remove_empty_overlay(&position);
        let mut effects = self.invalidate();
        let visible = self.visible_range.get();
        effects.extend(self.recalculate_price_scale(position, visible.as_ref()));
        Ok(deduplicate_effects(effects))
    }

    pub fn detach_custom_series(&mut self, id: SeriesId) -> Result<Vec<PaneEffect>, PaneError> {
        let index = self
            .attached_custom_sources
            .iter()
            .position(|attached| attached.id == id)
            .ok_or(PaneError::UnknownSeries(id))?;
        let attached = self.attached_custom_sources.remove(index);
        let position = attached.position.clone();
        if let Some(scale) = self.price_scale_mut_existing(&position) {
            scale.remove_data_source(&attached.source);
        }
        self.remove_empty_overlay(&position);
        let mut effects = self.invalidate();
        let visible = self.visible_range.get();
        effects.extend(self.recalculate_price_scale(position, visible.as_ref()));
        Ok(deduplicate_effects(effects))
    }

    pub fn move_series_to_scale(
        &mut self,
        id: SeriesId,
        position: PriceScalePosition,
    ) -> Result<Vec<PaneEffect>, PaneError> {
        let index = self
            .attached_series
            .iter()
            .position(|attached| attached.series.borrow().id() == id)
            .ok_or(PaneError::UnknownSeries(id))?;
        let old_position = self.attached_series[index].position.clone();
        if old_position == position {
            return Ok(vec![]);
        }
        let source = self.attached_series[index].source.clone();
        if let Some(scale) = self.price_scale_mut_existing(&old_position) {
            scale.remove_data_source(&source);
        }
        self.attached_series[index].position = position.clone();
        self.remove_empty_overlay(&old_position);
        self.price_scale_mut(&position).add_data_source(source);
        let mut effects = self.invalidate();
        let visible = self.visible_range.get();
        effects.extend(self.recalculate_price_scale(old_position, visible.as_ref()));
        effects.extend(self.recalculate_price_scale(position, visible.as_ref()));
        Ok(deduplicate_effects(effects))
    }

    pub fn move_custom_series_to_scale(
        &mut self,
        id: SeriesId,
        position: PriceScalePosition,
    ) -> Result<Vec<PaneEffect>, PaneError> {
        let index = self
            .attached_custom_sources
            .iter()
            .position(|attached| attached.id == id)
            .ok_or(PaneError::UnknownSeries(id))?;
        let old_position = self.attached_custom_sources[index].position.clone();
        if old_position == position {
            return Ok(vec![]);
        }
        let source = self.attached_custom_sources[index].source.clone();
        if let Some(scale) = self.price_scale_mut_existing(&old_position) {
            scale.remove_data_source(&source);
        }
        self.attached_custom_sources[index].position = position.clone();
        self.remove_empty_overlay(&old_position);
        self.price_scale_mut(&position).add_data_source(source);
        let mut effects = self.invalidate();
        let visible = self.visible_range.get();
        effects.extend(self.recalculate_price_scale(old_position, visible.as_ref()));
        effects.extend(self.recalculate_price_scale(position, visible.as_ref()));
        Ok(deduplicate_effects(effects))
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
        let has_sources = self
            .attached_series
            .iter()
            .any(|attached| attached.position == position)
            || self
                .attached_custom_sources
                .iter()
                .any(|attached| attached.position == position);
        let scale = self
            .price_scale_mut_existing(&position)
            .expect("the checked price scale remains attached");
        match visible {
            Some(visible) if has_sources => scale.recalculate_price_range(visible),
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

    fn series_position(&self, id: SeriesId) -> Option<&PriceScalePosition> {
        self.attached_series
            .iter()
            .find(|attached| attached.series.borrow().id() == id)
            .map(|attached| &attached.position)
    }

    fn contains_source(&self, id: SeriesId) -> bool {
        self.series_position(id).is_some()
            || self
                .attached_custom_sources
                .iter()
                .any(|attached| attached.id == id)
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
        if !self
            .attached_series
            .iter()
            .any(|attached| attached.position == *position)
            && !self
                .attached_custom_sources
                .iter()
                .any(|attached| attached.position == *position)
        {
            self.overlay_price_scales.remove(id);
        }
    }

    fn invalidate(&mut self) -> Vec<PaneEffect> {
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
        assert!(pane.attached_series_ids().is_empty());
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
        assert_eq!(pane.attached_custom_series_ids(), vec![SeriesId::new(2)]);
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
}
