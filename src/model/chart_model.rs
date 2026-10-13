//! Synchronous chart ownership and effect processing, independent of the UI.

mod markers;
mod price_lines;
mod primitives;

use crate::model::{
    axis_snapshots::{
        self, AxisSnapshotError, AxisSnapshots, PriceAxisOptionsError, PriceAxisSide,
    },
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
    price_scale::PriceScaleOptionsPatch,
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
use crate::renderers::price_axis_renderer_options_provider::PriceAxisRendererOptionsProvider;
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
    InvalidCrosshairOptions,
    InvalidPriceLineOptions,
    UnknownPriceLine(crate::model::price_line::PriceLineId),
    PriceLineIdExhausted,
    DuplicateSeries(SeriesId),
    UnknownSeries(SeriesId),
    InvalidPane(usize),
    InvalidDimensions,
    InvalidTypography,
    InvalidPriceInteraction,
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
    projection: Rc<RefCell<dyn crate::model::series::projection::SeriesProjection<O>>>,
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
    crosshair: crate::model::crosshair::Crosshair,
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
            crosshair: Default::default(),
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
    pub fn crosshair(&self) -> &crate::model::crosshair::Crosshair {
        &self.crosshair
    }
    pub fn crosshair_markers(
        &mut self,
        pane: usize,
    ) -> Vec<crate::model::crosshair::CrosshairMarker> {
        use crate::model::crosshair::CrosshairMarker;
        use crate::renderers::line_renderer::Point;
        let Some(position) = self.crosshair.position else {
            return vec![];
        };
        let visible = self.time_scale.visible_strict_range();
        let Some(state) = self.panes.get_mut(pane) else {
            return vec![];
        };
        let ids = state.ordered_source_ids().to_vec();
        let mut markers = vec![];
        let mut parser = crate::model::colors::ColorParser::default();
        for id in ids {
            let Some(handle) = self.series.get(&id).and_then(|entry| entry.built_in()) else {
                continue;
            };
            let series = handle.borrow();
            macro_rules! marker_options {
                ($options:expr) => {{
                    let o = &$options.style;
                    (
                        o.crosshair_marker_visible,
                        o.crosshair_marker_radius,
                        o.crosshair_marker_border_width,
                        &o.crosshair_marker_background_color,
                        &o.crosshair_marker_border_color,
                    )
                }};
            }
            let (enabled, radius, border_width, background, border) = match series.options() {
                SeriesOptionsMap::Line(o) => marker_options!(o),
                SeriesOptionsMap::Area(o) => marker_options!(o),
                SeriesOptionsMap::Baseline(o) => marker_options!(o),
                _ => continue,
            };
            if !series.visible() || !enabled {
                continue;
            }
            let Some(row) = series.bars().value_at(position.index) else {
                continue;
            };
            let Some(first) = series.first_value(visible.as_ref()) else {
                continue;
            };
            let Some(scale) = self.panes[pane]
                .price_scale_for_source(id)
                .filter(|scale| !scale.is_empty())
            else {
                continue;
            };
            let y = scale
                .price_to_coordinate(
                    crate::model::plot_data::PlotRowLike::values(row)[3],
                    first.value,
                )
                .value();
            if !y.is_finite()
                || !radius.is_finite()
                || radius < 0.
                || !border_width.is_finite()
                || border_width < 0.
            {
                continue;
            }
            let border_color = if border.is_empty() {
                match &self.options.layout.background {
                    crate::model::layout_options::Background::Solid { color } => color.clone(),
                    crate::model::layout_options::Background::VerticalGradient {
                        top_color,
                        bottom_color,
                    } => parser
                        .gradient_color_at_percent(
                            top_color,
                            bottom_color,
                            (y / scale.height()).clamp(0., 1.) as f32,
                        )
                        .unwrap_or_else(|_| top_color.clone()),
                }
            } else {
                border.clone()
            };
            markers.push(CrosshairMarker {
                point: Point { x: position.x, y },
                radius,
                border_width,
                color: if background.is_empty() {
                    series.bar_color(row).unwrap_or_default().to_owned()
                } else {
                    background.clone()
                },
                border_color,
            });
        }
        markers
    }
    pub fn set_crosshair_options(
        &mut self,
        options: crate::model::crosshair::CrosshairOptions,
    ) -> Result<(), ChartModelError> {
        let mut parser = crate::model::colors::ColorParser::default();
        for line in [&options.vertical, &options.horizontal] {
            if parser
                .generate_contrast_colors(&line.label_background_color)
                .is_err()
                || parser.generate_contrast_colors(&line.color).is_err()
            {
                return Err(ChartModelError::InvalidCrosshairOptions);
            }
        }
        self.crosshair.options = options;
        self.refresh_crosshair();
        self.full_update();
        Ok(())
    }
    pub fn set_crosshair_position(
        &mut self,
        pane: usize,
        point: Option<crate::renderers::line_renderer::Point>,
    ) -> Result<(), ChartModelError> {
        if pane >= self.panes.len() {
            return Err(ChartModelError::InvalidPane(pane));
        }
        self.crosshair.origin = point
            .filter(|p| p.x.is_finite() && p.y.is_finite())
            .map(|point| crate::model::crosshair::CrosshairOrigin { pane, point });
        self.refresh_crosshair();
        self.queue_invalidation(InvalidateMask::new(InvalidationLevel::Cursor));
        Ok(())
    }
    fn refresh_crosshair(&mut self) {
        use crate::model::{
            coordinate::Coordinate,
            crosshair::{CrosshairMode, CrosshairPosition},
            time_data::TimePointIndex,
        };
        self.crosshair.position = None;
        let Some(origin) = self.crosshair.origin else {
            return;
        };
        if self.crosshair.options.mode == CrosshairMode::Hidden || self.time_scale.is_empty() {
            return;
        }
        let Some(pane) = self.panes.get_mut(origin.pane) else {
            self.crosshair.origin = None;
            return;
        };
        let ordered_ids = pane.ordered_source_ids().to_vec();
        if origin.point.x < 0.
            || origin.point.y < 0.
            || origin.point.x >= self.time_scale.width()
            || origin.point.y >= pane.height()
        {
            return;
        }
        let mut index = self
            .time_scale
            .coordinate_to_index(Coordinate::new(origin.point.x), true);
        let visible = self.time_scale.visible_strict_range();
        if let Some(range) = &visible {
            index = TimePointIndex::new(
                index
                    .value()
                    .max(range.left().value())
                    .min(range.right().value()),
            );
        }
        if self.crosshair.options.do_not_snap_to_hidden_series_indices {
            use crate::model::plot_list::MismatchDirection;
            let mut left: Option<TimePointIndex> = None;
            let mut right: Option<TimePointIndex> = None;
            for entry in self.series.values() {
                let handle = &entry.projection;
                let series = handle.borrow();
                if !series.visible() {
                    continue;
                }
                if let Some(row) = series.row(index, MismatchDirection::NearestLeft) {
                    use crate::model::plot_data::PlotRowLike;
                    let at = row.index();
                    if left.is_none_or(|old| at > old) {
                        left = Some(at);
                    }
                }
                if let Some(row) = series.row(index, MismatchDirection::NearestRight) {
                    use crate::model::plot_data::PlotRowLike;
                    let at = row.index();
                    if right.is_none_or(|old| at < old) {
                        right = Some(at);
                    }
                }
            }
            let x = self.time_scale.index_to_coordinate(index).value();
            let mut distance = f64::INFINITY;
            for candidate in [left, right].into_iter().flatten() {
                let next = (self.time_scale.index_to_coordinate(candidate).value() - x).abs();
                if next < distance {
                    index = candidate;
                    distance = next;
                }
            }
        }
        let scale = pane.default_price_scale();
        let mut y = scale
            .first_value()
            .filter(|_| !scale.is_empty())
            .map(|_| origin.point.y);
        if self.crosshair.options.mode != CrosshairMode::Normal
            && let Some(raw_y) = y
        {
            use crate::model::plot_data::PlotRowLike;
            let mut candidates = vec![];
            for id in ordered_ids {
                let handle = &self.series[&id].projection;
                let series = handle.borrow();
                let Some(price_scale) = pane.price_scale_for_source(id) else {
                    continue;
                };
                if !series.visible()
                    || price_scale.is_empty()
                    || !matches!(price_scale.id(), "left" | "right")
                {
                    continue;
                }
                if let (Some(row), Some(first)) = (
                    series.row(index, crate::model::plot_list::MismatchDirection::None),
                    series.first_value(visible.as_ref()),
                ) {
                    let keys: &[usize] = if self.crosshair.options.mode == CrosshairMode::MagnetOhlc
                    {
                        &[0, 1, 2, 3]
                    } else {
                        &[3]
                    };
                    for key in keys {
                        candidates.push(
                            price_scale
                                .price_to_coordinate(row.values()[*key], first.value)
                                .value(),
                        );
                    }
                }
            }
            y = Some(crate::model::magnet::nearest_coordinate(raw_y, candidates));
        }
        self.crosshair.position = Some(CrosshairPosition {
            pane: origin.pane,
            index,
            x: self.time_scale.index_to_coordinate(index).value(),
            y,
        });
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
    pub(crate) fn projection(
        &self,
        id: SeriesId,
    ) -> Option<Ref<'_, dyn crate::model::series::projection::SeriesProjection<B::Item>>> {
        Some(self.series.get(&id)?.projection.borrow())
    }
    pub fn pane_for_series(&self, id: SeriesId) -> Option<usize> {
        self.series.get(&id).map(|entry| entry.pane)
    }

    /// Materializes lazy marks only; does not drain invalidation, change layout,
    /// or measure text. All returned values survive subsequent model updates.
    pub fn prepare_axis_snapshots(
        &mut self,
        pane: usize,
    ) -> Result<AxisSnapshots, AxisSnapshotError> {
        if pane >= self.panes.len() {
            return Err(AxisSnapshotError::InvalidPane(pane));
        }
        let decorations = self.price_decorations(pane)?;
        let layout = &self.options.layout;
        let pane_state = &mut self.panes[pane];
        let has_width = pane_state.width() > 0.;
        let mut provider = PriceAxisRendererOptionsProvider::new();
        let mut left = axis_snapshots::price_snapshot(
            pane_state.axis_price_scale_mut(PriceAxisSide::Left),
            PriceAxisSide::Left,
            layout,
            has_width,
            &mut provider,
        );
        let mut right = axis_snapshots::price_snapshot(
            pane_state.axis_price_scale_mut(PriceAxisSide::Right),
            PriceAxisSide::Right,
            layout,
            has_width,
            &mut provider,
        );
        left.labels = decorations.left;
        right.labels = decorations.right;
        left.label_center = decorations.left_center;
        right.label_center = decorations.right_center;
        for axis in [&mut left, &mut right] {
            if axis.align_labels {
                crate::renderers::price_axis_labels::align_labels(
                    &mut axis.labels,
                    axis.label_center.unwrap_or(pane_state.height() / 2.),
                    pane_state.height(),
                    f64::from(
                        axis.renderer_options.font_size
                            + axis.renderer_options.padding_top
                            + axis.renderer_options.padding_bottom,
                    ),
                );
            }
        }
        let marks = self
            .time_scale
            .marks(TimeScaleLayoutContext {
                font_size: layout.font_size,
            })
            .unwrap_or_default()
            .to_vec();
        // Grid and axes share the same refreshed marks, including font changes.
        pane_state.refresh_grid(&self.options.grid, &marks);
        let mut time = axis_snapshots::time_snapshot(
            marks,
            self.time_scale.behavior(),
            self.time_scale.options(),
            layout,
        );
        use crate::model::{
            axis_snapshots::FloatingAxisLabel, coordinate::Coordinate, crosshair::CrosshairMode,
        };
        let options = &self.crosshair.options;
        if options.mode != CrosshairMode::Hidden {
            let position = self.crosshair.position.filter(|p| p.pane == pane);
            let mut parser = crate::model::colors::ColorParser::default();
            if options.horizontal.label_visible {
                let colors = parser
                    .generate_contrast_colors(&options.horizontal.label_background_color)
                    .expect("validated crosshair color");
                for (snapshot, scale) in [
                    (&mut left, pane_state.left_price_scale()),
                    (&mut right, pane_state.right_price_scale()),
                ] {
                    if !scale.is_empty()
                        && let Some(base) = scale.first_value()
                    {
                        if options.horizontal.visible {
                            let top = scale.coordinate_to_price(Coordinate::new(1.), base);
                            let bottom = scale.coordinate_to_price(
                                Coordinate::new(pane_state.height() - 2.),
                                base,
                            );
                            for price in [
                                top.min(bottom).floor() + 0.11111111111111,
                                top.max(bottom).ceil() - 0.11111111111111,
                            ] {
                                if price.is_finite() {
                                    snapshot
                                        .crosshair_width_labels
                                        .push(scale.format_price(price, base));
                                }
                            }
                        }
                        if let Some(y) = position.and_then(|p| p.y) {
                            let price = scale.coordinate_to_price(Coordinate::new(y), base);
                            if price.is_finite() {
                                snapshot.crosshair_label = Some(FloatingAxisLabel {
                                    coordinate: y,
                                    text: scale.format_price(price, base),
                                    background: colors.background.clone(),
                                    color: colors.foreground.clone(),
                                });
                            }
                        }
                    }
                }
            }
            if options.vertical.label_visible
                && let Some(position) = self.crosshair.position
                && let Some(text) = self.time_scale.format_index(position.index)
            {
                let colors = parser
                    .generate_contrast_colors(&options.vertical.label_background_color)
                    .expect("validated crosshair color");
                time.crosshair_label = Some(FloatingAxisLabel {
                    coordinate: position.x,
                    text,
                    background: colors.background,
                    color: colors.foreground,
                });
            }
        }
        Ok(AxisSnapshots { left, right, time })
    }

    pub fn set_layout_typography(
        &mut self,
        font_size: f64,
        font_family: impl Into<String>,
    ) -> Result<(), ChartModelError> {
        if !font_size.is_finite() || font_size <= 0. || font_size > f64::from(f32::MAX) {
            return Err(ChartModelError::InvalidTypography);
        }
        self.options.layout.font_size = font_size;
        self.options.layout.font_family = font_family.into();
        for pane in &mut self.panes {
            pane.set_font_size(font_size);
        }
        self.full_update();
        self.flush_effects();
        Ok(())
    }

    pub fn apply_price_axis_options(
        &mut self,
        pane: usize,
        side: PriceAxisSide,
        patch: PriceScaleOptionsPatch,
    ) -> Result<(), PriceAxisOptionsError> {
        let pane_state = self
            .panes
            .get_mut(pane)
            .ok_or(PriceAxisOptionsError::InvalidPane(pane))?;
        pane_state
            .axis_price_scale_mut(side)
            .apply_options(patch)
            .map_err(PriceAxisOptionsError::Options)?;
        self.recalculate_requested = true;
        self.full_update();
        self.flush_effects();
        Ok(())
    }

    pub fn start_scale_price(
        &mut self,
        pane: usize,
        side: PriceAxisSide,
        y: f64,
    ) -> Result<(), ChartModelError> {
        if !y.is_finite() {
            return Err(ChartModelError::InvalidPriceInteraction);
        }
        self.panes
            .get_mut(pane)
            .ok_or(ChartModelError::InvalidPane(pane))?
            .start_scale_price(Self::axis_position(side), y.into())
            .map_err(ChartModelError::Pane)?;
        Ok(())
    }

    pub fn scale_price_to(
        &mut self,
        pane: usize,
        side: PriceAxisSide,
        y: f64,
    ) -> Result<(), ChartModelError> {
        if !y.is_finite() {
            return Err(ChartModelError::InvalidPriceInteraction);
        }
        let effects = self
            .panes
            .get_mut(pane)
            .ok_or(ChartModelError::InvalidPane(pane))?
            .scale_price_to(Self::axis_position(side), y.into())
            .map_err(ChartModelError::Pane)?;
        self.consume_pane_effects(effects);
        self.flush_effects();
        Ok(())
    }

    pub fn end_scale_price(
        &mut self,
        pane: usize,
        side: PriceAxisSide,
    ) -> Result<(), ChartModelError> {
        self.panes
            .get_mut(pane)
            .ok_or(ChartModelError::InvalidPane(pane))?
            .end_scale_price(Self::axis_position(side))
            .map_err(ChartModelError::Pane)?;
        Ok(())
    }

    pub fn reset_price_scale(
        &mut self,
        pane: usize,
        side: PriceAxisSide,
    ) -> Result<(), ChartModelError> {
        let state = self
            .panes
            .get_mut(pane)
            .ok_or(ChartModelError::InvalidPane(pane))?;
        state
            .end_scale_price(Self::axis_position(side))
            .map_err(ChartModelError::Pane)?;
        let effects = state
            .reset_price_scale(Self::axis_position(side))
            .map_err(ChartModelError::Pane)?;
        self.consume_pane_effects(effects);
        self.flush_effects();
        Ok(())
    }

    /// Wheel extension: positive normalized delta narrows the price range.
    pub fn zoom_price_scale(
        &mut self,
        pane: usize,
        side: PriceAxisSide,
        delta: f64,
    ) -> Result<(), ChartModelError> {
        if !delta.is_finite() {
            return Err(ChartModelError::InvalidPriceInteraction);
        }
        let scale = self
            .panes
            .get_mut(pane)
            .ok_or(ChartModelError::InvalidPane(pane))?
            .axis_price_scale_mut(side);
        if delta == 0. || scale.height() <= 1. {
            return Ok(());
        }
        let height = scale.height();
        let center = height * 0.5;
        if !scale.start_scale(center.into()) {
            return Ok(());
        }
        let padding = (height - 1.) * 0.2;
        let factor = (-delta.clamp(-1., 1.) * 0.2).exp();
        let y = height + padding - (height - center + padding) / factor;
        self.scale_price_to(pane, side, y)?;
        self.end_scale_price(pane, side)
    }

    fn axis_position(side: PriceAxisSide) -> PriceScalePosition {
        match side {
            PriceAxisSide::Left => PriceScalePosition::Left,
            PriceAxisSide::Right => PriceScalePosition::Right,
        }
    }

    /// Begin manual price panning; the caller chooses when to leave autoscale.
    pub fn start_scroll_price(
        &mut self,
        pane: usize,
        side: PriceAxisSide,
        y: f64,
    ) -> Result<(), ChartModelError> {
        use crate::model::price_scale::PriceScaleStatePatch;
        if !y.is_finite() {
            return Err(ChartModelError::InvalidPriceInteraction);
        }
        let state = self
            .panes
            .get_mut(pane)
            .ok_or(ChartModelError::InvalidPane(pane))?;
        let scale = state.axis_price_scale_mut(side);
        if scale.is_empty() || scale.is_percentage() || scale.is_indexed_to_100() {
            return Ok(());
        }
        scale.set_mode(PriceScaleStatePatch {
            auto_scale: Some(false),
            ..Default::default()
        });
        state
            .start_scroll_price(Self::axis_position(side), y.into())
            .map_err(ChartModelError::Pane)?;
        self.light_update();
        self.flush_effects();
        Ok(())
    }

    pub fn scroll_price_to(
        &mut self,
        pane: usize,
        side: PriceAxisSide,
        y: f64,
    ) -> Result<(), ChartModelError> {
        if !y.is_finite() {
            return Err(ChartModelError::InvalidPriceInteraction);
        }
        let effects = self
            .panes
            .get_mut(pane)
            .ok_or(ChartModelError::InvalidPane(pane))?
            .scroll_price_to(Self::axis_position(side), y.into())
            .map_err(ChartModelError::Pane)?;
        self.consume_pane_effects(effects);
        self.flush_effects();
        Ok(())
    }

    pub fn end_scroll_price(
        &mut self,
        pane: usize,
        side: PriceAxisSide,
    ) -> Result<(), ChartModelError> {
        self.panes
            .get_mut(pane)
            .ok_or(ChartModelError::InvalidPane(pane))?
            .end_scroll_price(Self::axis_position(side))
            .map_err(ChartModelError::Pane)?;
        Ok(())
    }

    pub fn prepare_line(
        &mut self,
        id: SeriesId,
    ) -> Result<
        Option<crate::renderers::line_renderer::LineRendererData>,
        crate::model::series::line_pane_view::LinePreparationError,
    > {
        use crate::model::series::line_pane_view::{self, LinePreparationError};
        let entry = self
            .series
            .get(&id)
            .ok_or(LinePreparationError::UnknownSeries(id))?;
        let handle = entry
            .built_in()
            .ok_or(LinePreparationError::NotLineSeries(id))?;
        let series = handle.borrow();
        let pane = &self.panes[entry.pane];
        let price = pane
            .price_scale_for_source(id)
            .ok_or(LinePreparationError::MissingPriceScale(id))?;
        line_pane_view::prepare(&series, &mut self.time_scale, price, pane.height())
    }

    pub fn prepare_baseline(
        &mut self,
        id: SeriesId,
    ) -> Result<
        Option<crate::renderers::baseline_renderer::BaselineRendererData>,
        crate::model::series::baseline_pane_view::BaselinePreparationError,
    > {
        use crate::model::series::baseline_pane_view::{self, BaselinePreparationError};
        let entry = self
            .series
            .get(&id)
            .ok_or(BaselinePreparationError::UnknownSeries(id))?;
        let handle = entry
            .built_in()
            .ok_or(BaselinePreparationError::NotBaselineSeries(id))?;
        let series = handle.borrow();
        let pane = &self.panes[entry.pane];
        let price = pane
            .price_scale_for_source(id)
            .ok_or(BaselinePreparationError::MissingPriceScale(id))?;
        baseline_pane_view::prepare(&series, &mut self.time_scale, price, pane.height())
    }

    pub fn prepare_area(
        &mut self,
        id: SeriesId,
    ) -> Result<
        Option<crate::renderers::area_renderer::AreaRendererData>,
        crate::model::series::area_pane_view::AreaPreparationError,
    > {
        use crate::model::series::area_pane_view::{self, AreaPreparationError};
        let entry = self
            .series
            .get(&id)
            .ok_or(AreaPreparationError::UnknownSeries(id))?;
        let handle = entry
            .built_in()
            .ok_or(AreaPreparationError::NotAreaSeries(id))?;
        let series = handle.borrow();
        let pane = &self.panes[entry.pane];
        let price = pane
            .price_scale_for_source(id)
            .ok_or(AreaPreparationError::MissingPriceScale(id))?;
        area_pane_view::prepare(&series, &mut self.time_scale, price, pane.height())
    }

    pub fn prepare_bars(
        &mut self,
        id: SeriesId,
    ) -> Result<
        Option<crate::renderers::bars_renderer::BarsRendererData>,
        crate::model::series::bars_pane_view::BarPreparationError,
    > {
        use crate::model::series::bars_pane_view::{self, BarPreparationError};
        let entry = self
            .series
            .get(&id)
            .ok_or(BarPreparationError::UnknownSeries(id))?;
        let handle = entry
            .built_in()
            .ok_or(BarPreparationError::NotBarSeries(id))?;
        let series = handle.borrow();
        let pane = &self.panes[entry.pane];
        let price = pane
            .price_scale_for_source(id)
            .ok_or(BarPreparationError::MissingPriceScale(id))?;
        bars_pane_view::prepare(&series, &mut self.time_scale, price, pane.height())
    }

    pub fn prepare_candlesticks(
        &mut self,
        id: SeriesId,
    ) -> Result<
        Option<crate::renderers::candlesticks_renderer::CandlestickRendererData>,
        crate::model::series::candlesticks_pane_view::CandlestickPreparationError,
    > {
        use crate::model::series::candlesticks_pane_view::{self, CandlestickPreparationError};
        let entry = self
            .series
            .get(&id)
            .ok_or(CandlestickPreparationError::UnknownSeries(id))?;
        let handle = entry
            .built_in()
            .ok_or(CandlestickPreparationError::NotCandlestickSeries(id))?;
        let series = handle.borrow();
        let pane = &self.panes[entry.pane];
        let price = pane
            .price_scale_for_source(id)
            .ok_or(CandlestickPreparationError::MissingPriceScale(id))?;
        candlesticks_pane_view::prepare(&series, &mut self.time_scale, price, pane.height())
    }

    pub fn prepare_histogram(
        &mut self,
        id: SeriesId,
    ) -> Result<
        Option<crate::renderers::histogram_renderer::HistogramRendererData>,
        crate::model::series::histogram_pane_view::HistogramPreparationError,
    > {
        use crate::model::series::histogram_pane_view::{self, HistogramPreparationError};
        let entry = self
            .series
            .get(&id)
            .ok_or(HistogramPreparationError::UnknownSeries(id))?;
        let handle = entry
            .built_in()
            .ok_or(HistogramPreparationError::NotHistogramSeries(id))?;
        let series = handle.borrow();
        let pane = &self.panes[entry.pane];
        let price = pane
            .price_scale_for_source(id)
            .ok_or(HistogramPreparationError::MissingPriceScale(id))?;
        histogram_pane_view::prepare(&series, &mut self.time_scale, price, pane.height())
    }

    pub fn replace_series_options(
        &mut self,
        id: SeriesId,
        options: SeriesOptionsMap,
    ) -> Result<(), ChartModelError> {
        let entry = self
            .series
            .get(&id)
            .ok_or(ChartModelError::UnknownSeries(id))?;
        let handle = entry
            .built_in()
            .ok_or(ChartModelError::WrongUpdateKind(id))?;
        handle
            .borrow_mut()
            .replace_options(options)
            .map_err(ChartModelError::Construction)?;
        self.panes[entry.pane].refresh_formatter_sources();
        self.recalculate_requested = true;
        self.full_update();
        self.flush_effects();
        Ok(())
    }

    pub(crate) fn custom_options_changed(&mut self, id: SeriesId) {
        let pane = self.series[&id].pane;
        self.panes[pane].refresh_formatter_sources();
        self.recalculate_requested = true;
        self.full_update();
        self.flush_effects();
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
                state: RegisteredSeriesState::BuiltIn(handle.clone()),
                projection: handle,
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
                state: RegisteredSeriesState::Custom(handle.clone()),
                projection: handle,
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
            self.crosshair.origin = None;
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
    pub fn set_pane_stretch_factor(
        &mut self,
        pane: usize,
        factor: f64,
    ) -> Result<(), ChartModelError> {
        if !factor.is_finite() || factor <= 0. {
            return Err(ChartModelError::InvalidDimensions);
        }
        let state = self
            .panes
            .get_mut(pane)
            .ok_or(ChartModelError::InvalidPane(pane))?;
        let effects = state.set_stretch_factor(factor);
        self.consume_pane_effects(effects);
        self.full_update();
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
    pub fn zoom_time(&mut self, point: crate::model::coordinate::Coordinate, scale: f64) {
        if self.time_scale.is_empty()
            || !point.value().is_finite()
            || !scale.is_finite()
            || scale == 0.
        {
            return;
        }
        self.end_scroll_time();
        self.stop_time_scale_animation();
        let point = crate::model::coordinate::Coordinate::new(
            point.value().max(1.).min(self.time_scale.width()),
        );
        let effects = self.time_scale.zoom(point, scale);
        self.consume_time_effects(effects);
        self.flush_effects();
    }
    pub fn start_scroll_time(&mut self, point: crate::model::coordinate::Coordinate) {
        if !point.value().is_finite() || self.time_scale.is_empty() {
            return;
        }
        self.stop_time_scale_animation();
        self.time_scale.start_scroll(point);
    }
    pub fn scroll_time_to(&mut self, point: crate::model::coordinate::Coordinate) {
        if !point.value().is_finite() || self.time_scale.is_empty() {
            self.end_scroll_time();
            return;
        }
        let effects = self.time_scale.scroll_to(point);
        self.consume_time_effects(effects);
        self.flush_effects();
    }
    pub fn end_scroll_time(&mut self) {
        self.time_scale.end_scroll();
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
        for entry in self.series.values() {
            entry
                .projection
                .borrow()
                .marker_spacing()
                .set(self.time_scale.bar_spacing());
        }
        if !std::mem::take(&mut self.recalculate_requested) {
            self.refresh_crosshair();
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
        self.refresh_crosshair();
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
