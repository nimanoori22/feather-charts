use crate::model::{
    chart_model::{ChartModel, ChartModelError, ChartModelOptions},
    data_consumer::{BuiltInSeriesDataItem, TimedData},
    data_layer::{DataLayer, DataLayerError, SeriesId},
    get_series_plot_row_creator::create_builtin_plot_row,
    horz_scale_behavior_time::{
        horz_scale_behavior_time::HorzScaleBehaviorTime,
        time_utils::{TimeConversionError, select_time_converter},
        types::Time,
    },
    localization_options::LocalizationOptions,
    pane::PriceScalePosition,
    plot_data::MutablePlotRow,
    series_options::{SeriesOptionsMap, SeriesType},
    time_data::TimePointIndex,
    time_scale::TimeScale,
    time_scale_options::HorzScaleOptions,
};
use crate::renderers::{grid_renderer::PixelRatio, price_axis_renderer::AxisBounds};
use crate::ui::{
    chart_frame::{ChartFrameError, IcedAxisMeasurer, IcedChartFrame},
    multi_pane_frame::{
        IcedMultiPaneChartFrame, MultiPaneChartFrameInput, MultiPaneChartFrameOwner,
        MultiPaneChartFrameSnapshot, SeriesPaneView,
    },
    price_axis::AxisFontResolver,
};
use crate::views::time_scale_animation::TimeScaleAnimationController;
use std::time::Instant;
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Debug)]
pub enum ChartError {
    UnknownPrimitive(super::PrimitiveId),
    Model(ChartModelError),
    Frame(ChartFrameError),
    PriceAxis(crate::model::axis_snapshots::PriceAxisOptionsError),
    Data(DataLayerError<TimeConversionError>),
    Time(TimeConversionError),
    UnknownSeries(SeriesId),
    UnsupportedSeries,
    HistoricalUpdateWithConflation(SeriesId),
    InvalidData { index: usize },
    UnorderedData { index: usize },
    IdExhausted,
    InvalidInteraction,
    InvalidMarker { index: usize },
    UnknownMarkerCollection(super::MarkerCollectionId),
}
impl std::fmt::Display for ChartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ChartError {}
impl From<ChartModelError> for ChartError {
    fn from(e: ChartModelError) -> Self {
        Self::Model(e)
    }
}
impl From<DataLayerError<TimeConversionError>> for ChartError {
    fn from(e: DataLayerError<TimeConversionError>) -> Self {
        Self::Data(e)
    }
}
pub(super) static NEXT_SERIES: AtomicU64 = AtomicU64::new(1);

/// Owns data normalization and model mutation together. IDs are not reused.
/// Data operations are synchronous; prepare a frame after a batch of operations.
pub struct Chart {
    pub(super) primitive_edit: Option<super::primitive_editing::EditSession>,
    pub(super) pending_primitive_edits: Vec<super::PrimitiveEditEvent>,
    pub(crate) extension_order: u64,
    pub(crate) marker_orders: BTreeMap<super::MarkerCollectionId, u64>,
    pub(super) last_primitive: Option<super::PrimitiveHit>,
    empty_frame: IcedChartFrame,
    pub(crate) primitives: Vec<super::primitives::Attachment>,
    pub(crate) primitive_panes: Vec<super::PrimitivePaneId>,
    hovered_series_on_top: bool,
    pub(super) last_hit: Option<super::ChartHit>,
    pub(crate) model: ChartModel<HorzScaleBehaviorTime>,
    pub(super) layer: DataLayer<HorzScaleBehaviorTime>,
    pub(crate) size: iced::Size,
    pub(crate) ratio: f32,
    pub(crate) plot_pan: Option<super::interaction::PlotPan>,
    pub(super) touch_options: super::TouchNavigationOptions,
    pub(super) kinetic_options: super::KineticScrollOptions,
    pub(super) kinetic: Option<crate::model::kinetic_animation::KineticSampler>,
    pub(super) touch_pending_pan: Option<(iced::Point, std::time::Instant)>,
    pub(super) touch_tracking: bool,
    pub(super) touch_epoch: u64,
    pub(crate) active_price_axis: Option<(usize, crate::model::axis_snapshots::PriceAxisSide)>,
    pub(super) kinds: BTreeMap<SeriesId, SeriesType>,
    pub(super) views: Vec<Vec<(SeriesId, SeriesPaneView)>>,
    pub(super) owner: MultiPaneChartFrameOwner,
    pub(crate) controller: TimeScaleAnimationController,
    frame: IcedMultiPaneChartFrame,
    fonts: AxisFontResolver,
    background: String,
    separator: String,
    separator_hover: String,
    pub(super) hover_color: iced::Color,
    pub(crate) resize_panes: bool,
    pub(crate) revision: u64,
}
impl Default for Chart {
    fn default() -> Self {
        Self::new(ChartModelOptions::default())
    }
}
impl Chart {
    pub fn new(options: ChartModelOptions) -> Self {
        let background = match &options.layout.background {
            crate::model::layout_options::Background::Solid { color } => color.clone(),
            crate::model::layout_options::Background::VerticalGradient { top_color, .. } => {
                top_color.clone()
            }
        };
        let separator = options.layout.panes.separator_color.clone();
        let resize_panes = options.layout.panes.enable_resize;
        let separator_hover = options.layout.panes.separator_hover_color.clone();
        Self {
            primitive_edit: None,
            pending_primitive_edits: vec![],
            extension_order: 0,
            marker_orders: BTreeMap::new(),
            last_primitive: None,
            empty_frame: Default::default(),
            primitives: vec![],
            primitive_panes: (0..usize::from(options.add_default_pane))
                .map(|_| super::primitives::pane_id())
                .collect(),
            hovered_series_on_top: true,
            last_hit: None,
            size: iced::Size::ZERO,
            ratio: 1.,
            plot_pan: None,
            touch_options: Default::default(),
            kinetic_options: Default::default(),
            kinetic: None,
            touch_pending_pan: None,
            touch_tracking: false,
            touch_epoch: 0,
            active_price_axis: None,
            views: (0..usize::from(options.add_default_pane))
                .map(|_| vec![])
                .collect(),
            owner: Default::default(),
            controller: Default::default(),
            frame: Default::default(),
            fonts: Default::default(),
            background,
            separator,
            separator_hover,
            hover_color: iced::Color::TRANSPARENT,
            resize_panes,
            revision: 0,
            model: ChartModel::new(
                TimeScale::new(
                    HorzScaleBehaviorTime::default(),
                    HorzScaleOptions::default(),
                    LocalizationOptions::new("en-US", "dd MMM 'yy"),
                ),
                options,
            ),
            layer: DataLayer::new(HorzScaleBehaviorTime::default()),
            kinds: BTreeMap::new(),
        }
    }
    /// Read-only engine inspection; data mutations must pass through this owner.
    pub fn model(&self) -> &ChartModel<HorzScaleBehaviorTime> {
        &self.model
    }
    pub fn add_series(
        &mut self,
        options: SeriesOptionsMap,
        pane: usize,
        position: PriceScalePosition,
    ) -> Result<SeriesId, ChartError> {
        let kind = match &options {
            SeriesOptionsMap::Line(_) => SeriesType::Line,
            SeriesOptionsMap::Area(_) => SeriesType::Area,
            SeriesOptionsMap::Bar(_) => SeriesType::Bar,
            SeriesOptionsMap::Baseline(_) => SeriesType::Baseline,
            SeriesOptionsMap::Candlestick(_) => SeriesType::Candlestick,
            SeriesOptionsMap::Histogram(_) => SeriesType::Histogram,
            _ => return Err(ChartError::UnsupportedSeries),
        };
        let id = NEXT_SERIES
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map(SeriesId::new)
            .map_err(|_| ChartError::IdExhausted)?;
        self.model
            .register_series(id, kind, options, pane, position)?;
        self.kinds.insert(id, kind);
        let view = match kind {
            SeriesType::Line => SeriesPaneView::Line(Default::default()),
            SeriesType::Area => SeriesPaneView::Area(Default::default()),
            SeriesType::Bar => SeriesPaneView::Bars(Default::default()),
            SeriesType::Baseline => SeriesPaneView::Baseline(Default::default()),
            SeriesType::Candlestick => SeriesPaneView::Candlesticks(Default::default()),
            SeriesType::Histogram => SeriesPaneView::Histogram(Default::default()),
            _ => unreachable!(),
        };
        self.views[pane].push((id, view));
        self.revision += 1;
        // Establish DataLayer membership so update() also works before set_data().
        let update = self.layer.set_series_data(id, kind, vec![])?;
        self.model.apply_data_update(update)?;
        Ok(id)
    }
    pub fn series_ids(&self) -> impl Iterator<Item = SeriesId> + '_ {
        self.kinds.keys().copied()
    }
    fn kind(&self, id: SeriesId) -> Result<SeriesType, ChartError> {
        self.kinds
            .get(&id)
            .copied()
            .ok_or(ChartError::UnknownSeries(id))
    }
    /// Replace all data. Times must be strictly increasing; errors preserve data.
    pub fn set_data(
        &mut self,
        id: SeriesId,
        data: Vec<BuiltInSeriesDataItem<Time>>,
    ) -> Result<(), ChartError> {
        let kind = self.kind(id)?;
        validate_data(kind, &data, true)?;
        if data.is_empty() {
            self.cancel_interaction();
        }
        let update = self.layer.set_series_data(id, kind, data)?;
        self.model.apply_data_update(update)?;
        Ok(())
    }
    /// Replace the latest bar or append. Historical updates require an existing time.
    pub fn update_data(
        &mut self,
        id: SeriesId,
        item: BuiltInSeriesDataItem<Time>,
        historical: bool,
    ) -> Result<(), ChartError> {
        validate_data(self.kind(id)?, std::slice::from_ref(&item), false)?;
        let threshold = self
            .model
            .series(id)
            .ok_or(ChartError::UnknownSeries(id))?
            .common_options()
            .conflation_threshold_factor;
        self.check_historical_conflation(id, historical, threshold)?;
        let update = self.layer.update_series_data(id, item, historical)?;
        self.model.apply_data_update(update)?;
        Ok(())
    }
    pub(super) fn check_historical_conflation(
        &self,
        id: SeriesId,
        historical: bool,
        threshold: Option<f64>,
    ) -> Result<(), ChartError> {
        if historical
            && self
                .model
                .time_scale()
                .effective_conflation_factor(threshold)
                > 1
        {
            return Err(ChartError::HistoricalUpdateWithConflation(id));
        }
        Ok(())
    }
    pub fn remove_series(&mut self, id: SeriesId) -> Result<(), ChartError> {
        self.kind(id)?;
        let pane = self
            .model
            .pane_for_series(id)
            .ok_or(ChartError::UnknownSeries(id))?;
        self.cancel_interaction();
        let update = self.layer.remove_series(id)?;
        self.model.remove_series(id, update)?;
        self.marker_orders
            .retain(|key, _| self.model.marker_collection(*key).is_some());
        self.kinds.remove(&id);
        self.primitives
            .retain(|a| a.owner != super::PrimitiveOwner::Series(id));
        for views in &mut self.views {
            views.retain(|(other, _)| *other != id);
        }
        if self.views.len() != self.model.panes().len() {
            // Model removes an unpreserved pane when its final series is removed.
            self.views.remove(pane);
            let removed = self.primitive_panes.remove(pane);
            self.primitives
                .retain(|a| a.owner != super::PrimitiveOwner::Pane(removed));
        }
        self.revision += 1;
        Ok(())
    }
    pub fn replace_series_options(
        &mut self,
        id: SeriesId,
        options: SeriesOptionsMap,
    ) -> Result<(), ChartError> {
        self.kind(id)?;
        self.model.replace_series_options(id, options)?;
        Ok(())
    }
    pub fn add_price_line(
        &mut self,
        series: SeriesId,
        options: super::PriceLineOptions,
    ) -> Result<super::PriceLineId, ChartError> {
        self.kind(series)?;
        Ok(self.model.add_price_line(series, options)?)
    }
    pub fn price_line_options(
        &self,
        line: super::PriceLineId,
    ) -> Result<super::PriceLineOptions, ChartError> {
        Ok(self.model.price_line_options(line)?)
    }
    pub fn replace_price_line_options(
        &mut self,
        line: super::PriceLineId,
        options: super::PriceLineOptions,
    ) -> Result<(), ChartError> {
        Ok(self.model.replace_price_line_options(line, options)?)
    }
    pub fn remove_price_line(&mut self, line: super::PriceLineId) -> Result<(), ChartError> {
        Ok(self.model.remove_price_line(line)?)
    }
    pub fn add_pane(&mut self) -> usize {
        self.cancel_interaction();
        self.views.push(vec![]);
        self.primitive_panes.push(super::primitives::pane_id());
        self.revision += 1;
        self.model.add_pane()
    }
    pub fn apply_time_scale_options(
        &mut self,
        patch: crate::model::time_scale_options::HorzScaleOptionsPatch,
    ) {
        self.cancel_primitive_edit(super::PrimitiveEditCancelReason::ViewportChanged);
        self.model.apply_time_scale_options(&mut self.layer, patch);
    }
    /// Settle shared layout and geometry without converting glyphs. Primarily
    /// useful for hosts with their own frame conversion/inspection.
    pub fn prepare_snapshot(
        &mut self,
        size: iced::Size,
        ratio: f32,
        now: Instant,
    ) -> Result<MultiPaneChartFrameSnapshot, ChartError> {
        self.precompute_conflation_step();
        self.check_primitive_edit(size, ratio);
        let series = self
            .views
            .iter()
            .map(|v| v.first().map(|(id, _)| *id))
            .collect();
        let mut primitive_output = vec![];
        let mut decorator =
            |model: &mut ChartModel<HorzScaleBehaviorTime>,
             axes: &mut [crate::model::axis_snapshots::AxisSnapshots]| {
                primitive_output = super::primitives::prepare_axes(
                    model,
                    axes,
                    &self.primitives,
                    &self.primitive_panes,
                    ratio,
                )?;
                Ok(())
            };
        self.owner
            .prepare_composed_decorated(
                &mut self.model,
                &mut self.controller,
                &mut self.views,
                &MultiPaneChartFrameInput {
                    series,
                    bounds: AxisBounds {
                        width: f64::from(size.width),
                        height: f64::from(size.height),
                    },
                    pixel_ratio: PixelRatio {
                        horizontal: ratio,
                        vertical: ratio,
                    },
                    background: self.background.clone(),
                },
                (&mut IcedAxisMeasurer { fonts: &self.fonts }, &mut decorator),
                now,
            )
            .map(|mut snapshot| {
                let origin = self.model.crosshair().origin();
                let time_primitives = primitive_output
                    .iter()
                    .flatten()
                    .filter(|a| matches!(a.owner, super::PrimitiveOwner::Series(_)))
                    .cloned()
                    .collect::<Vec<_>>();
                for (index, pane) in snapshot.panes.iter_mut().enumerate() {
                    pane.plot.marker_orders = self.marker_orders.clone();
                    pane.plot.primitives = primitive_output.get(index).cloned().unwrap_or_default();
                    if index == 0 {
                        pane.plot.time_primitives = time_primitives.clone();
                    }
                    pane.plot.prepare_hover(
                        origin
                            .filter(|o| o.pane == index)
                            .map(|o| iced::Point::new(o.point.x as f32, o.point.y as f32)),
                        self.hovered_series_on_top,
                    );
                }
                snapshot
            })
            .map_err(ChartError::Frame)
    }
    /// Prepare an immutable frame after a batch of updates. Failed preparation
    /// clears the displayed geometry instead of retaining a stale chart.
    pub fn prepare(
        &mut self,
        size: iced::Size,
        ratio: f32,
        now: Instant,
    ) -> Result<(), ChartError> {
        if self.size != size || self.ratio != ratio {
            self.cancel_viewport_interaction();
        }
        self.size = size;
        self.ratio = ratio;
        let result = self
            .prepare_snapshot(size, ratio, now)
            .and_then(|snapshot| {
                self.hover_color = crate::ui::line_chart::color(&self.separator_hover)
                    .map_err(|error| ChartError::Frame(ChartFrameError::Plot(error)))?;
                IcedMultiPaneChartFrame::from_snapshot(snapshot, &self.fonts, &self.separator)
                    .map_err(ChartError::Frame)
            });
        match result {
            Ok(frame) => {
                self.frame = frame;
                Ok(())
            }
            Err(error) => {
                self.cancel_primitive_edit(super::PrimitiveEditCancelReason::ViewportChanged);
                self.last_hit = None;
                self.last_primitive = None;
                self.frame = Default::default();
                let mut pane = IcedChartFrame::default();
                pane.layout.available = AxisBounds {
                    width: f64::from(size.width),
                    height: f64::from(size.height),
                };
                self.frame.panes.push(pane);
                Err(error)
            }
        }
    }
    pub(crate) fn frame_for_pane(&self, pane: usize) -> Option<&IcedChartFrame> {
        self.frame.panes.get(pane)
    }
    pub fn frame(&self) -> &IcedChartFrame {
        self.frame.panes.first().unwrap_or(&self.empty_frame)
    }
    /// Source default is enabled. Changes drawing order without changing selection order.
    pub fn set_hovered_series_on_top(&mut self, enabled: bool) {
        self.hovered_series_on_top = enabled;
        for pane in &mut self.frame.panes {
            pane.plot.hovered_series_on_top = enabled;
        }
        self.revision = self.revision.wrapping_add(1);
    }
    pub fn hovered_series_on_top(&self) -> bool {
        self.hovered_series_on_top
    }
    pub fn hit_test(&self, pane: usize, point: iced::Point) -> Option<super::ChartHit> {
        self.frame.panes.get(pane)?.plot.hit_test(point)
    }
    pub fn marker_hit(&self, pane: usize, point: iced::Point) -> Option<super::MarkerHit> {
        self.frame.panes.get(pane)?.plot.marker_hit(point)
    }
    pub fn multi_frame(&self) -> Option<&IcedMultiPaneChartFrame> {
        (self.frame.panes.len() > 1).then_some(&self.frame)
    }
    pub fn is_animating(&self) -> bool {
        use crate::ui::primitive_hit_testing::PaneHit;
        let selection = self.model.crosshair().origin().and_then(|o| {
            self.frame_for_pane(o.pane)?
                .plot
                .selection(iced::Point::new(o.point.x as f32, o.point.y as f32))
        });
        let changed = match selection {
            Some(PaneHit::BuiltIn(h, _)) => {
                self.last_hit.as_ref() != Some(&h) || self.last_primitive.is_some()
            }
            Some(PaneHit::Primitive(h)) => {
                self.last_primitive.as_ref() != Some(&h) || self.last_hit.is_some()
            }
            None => self.last_hit.is_some() || self.last_primitive.is_some(),
        };
        self.controller.is_active()
            || changed
            || !self.pending_primitive_edits.is_empty()
            || self.has_pending_conflation()
    }
    pub fn fit_content(&mut self) {
        self.cancel_viewport_interaction();
        self.model.fit_content();
    }
    pub fn set_logical_range(&mut self, range: crate::model::time_data::LogicalRange) {
        self.cancel_viewport_interaction();
        self.model.set_logical_range(range);
    }
    pub fn set_right_offset(&mut self, offset: f64) {
        self.cancel_viewport_interaction();
        self.model.set_right_offset(offset);
    }
    pub fn scroll_to_offset_animated(&mut self, offset: f64, duration: std::time::Duration) {
        self.cancel_viewport_interaction();
        self.model.scroll_to_offset_animated(offset, duration);
    }
    pub fn stop_time_scale_animation(&mut self) {
        self.controller.cancel();
        self.kinetic = None;
        self.model.stop_time_scale_animation();
    }
    pub fn set_pane_stretch_factor(&mut self, pane: usize, factor: f64) -> Result<(), ChartError> {
        if !factor.is_finite() || factor <= 0. {
            return Err(crate::model::chart_model::ChartModelError::InvalidDimensions.into());
        }
        if pane >= self.model.panes().len() {
            return Err(crate::model::chart_model::ChartModelError::InvalidPane(pane).into());
        }
        self.cancel_viewport_interaction();
        self.model
            .set_pane_stretch_factor(pane, factor)
            .map_err(Into::into)
    }
    pub fn set_preserve_empty_pane(
        &mut self,
        pane: usize,
        preserve: bool,
    ) -> Result<(), ChartError> {
        self.model
            .set_preserve_empty_pane(pane, preserve)
            .map_err(Into::into)
    }
    pub fn apply_price_axis_options(
        &mut self,
        pane: usize,
        side: crate::model::axis_snapshots::PriceAxisSide,
        patch: crate::model::price_scale::PriceScaleOptionsPatch,
    ) -> Result<(), ChartError> {
        use crate::model::axis_snapshots::{PriceAxisOptionsError, PriceAxisSide};
        let pane_state = self.model.panes().get(pane).ok_or(ChartError::PriceAxis(
            PriceAxisOptionsError::InvalidPane(pane),
        ))?;
        let scale = match side {
            PriceAxisSide::Left => pane_state.left_price_scale(),
            PriceAxisSide::Right => pane_state.right_price_scale(),
        };
        scale
            .margins_after_patch(patch.scale_margins.as_ref())
            .map_err(|e| ChartError::PriceAxis(PriceAxisOptionsError::Options(e)))?;
        self.cancel_viewport_interaction();
        self.model
            .apply_price_axis_options(pane, side, patch)
            .map_err(ChartError::PriceAxis)
    }
    pub fn set_crosshair_options(
        &mut self,
        options: crate::model::crosshair::CrosshairOptions,
    ) -> Result<(), ChartError> {
        self.model
            .set_crosshair_options(options)
            .map_err(Into::into)
    }
}

// Validate the entire replacement before DataLayer removes previous memberships.
fn validate_data(
    kind: SeriesType,
    data: &[BuiltInSeriesDataItem<Time>],
    ordered: bool,
) -> Result<(), ChartError> {
    if data.is_empty() {
        return Ok(());
    }
    let converter = select_time_converter(data).map_err(ChartError::Time)?;
    let mut previous = None;
    for (index, item) in data.iter().enumerate() {
        let time = converter.convert(item.time()).map_err(ChartError::Time)?;
        let timestamp = time.timestamp.seconds();
        if ordered && previous.is_some_and(|p| timestamp <= p) {
            return Err(ChartError::UnorderedData { index });
        }
        previous = Some(timestamp);
        let row = create_builtin_plot_row(
            kind,
            time,
            TimePointIndex::default(),
            item.clone(),
            item.time().clone(),
        )
        .map_err(|e| ChartError::Data(DataLayerError::PlotRow(e)))?;
        if let crate::model::series_data::SeriesDataRow::Value(
            crate::model::series_data::SeriesPlotRow::Area(area),
        ) = &row
        {
            let mut parser = crate::model::colors::ColorParser::default();
            if [&area.line_color, &area.top_color, &area.bottom_color]
                .into_iter()
                .flatten()
                .any(|c| parser.parse_color(c).is_err())
            {
                return Err(ChartError::InvalidData { index });
            }
        }
        if let crate::model::series_data::SeriesDataRow::Value(
            crate::model::series_data::SeriesPlotRow::Bar(bar),
        ) = &row
        {
            let mut parser = crate::model::colors::ColorParser::default();
            if bar
                .color
                .as_ref()
                .is_some_and(|c| parser.parse_color(c).is_err())
            {
                return Err(ChartError::InvalidData { index });
            }
        }
        if let crate::model::series_data::SeriesDataRow::Value(
            crate::model::series_data::SeriesPlotRow::Baseline(r),
        ) = &row
        {
            let mut parser = crate::model::colors::ColorParser::default();
            if [
                &r.top_fill_color1,
                &r.top_fill_color2,
                &r.bottom_fill_color1,
                &r.bottom_fill_color2,
                &r.top_line_color,
                &r.bottom_line_color,
            ]
            .into_iter()
            .flatten()
            .any(|c| parser.parse_color(c).is_err())
            {
                return Err(ChartError::InvalidData { index });
            }
        }
        if row.values().is_some_and(|values| {
            values
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 90_071_992_547_409.9)
        }) {
            return Err(ChartError::InvalidData { index });
        }
    }
    Ok(())
}
