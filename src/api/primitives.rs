//! Application extensions prepare owned geometry; drawing never reads live state.
use super::{Chart, ChartBehavior, ChartError};
use crate::model::{
    autoscale_info_impl::AutoscaleInfo, chart_model::ChartModel, data_layer::SeriesId,
    horz_scale_behavior_time::horz_scale_behavior_time::HorzScaleBehaviorTime,
    ipane_primitive::PrimitivePaneViewZOrder, time_data::TimePointIndex,
};
use iced::{Point, Size, widget::canvas::Frame};
use std::{
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

pub use crate::model::axis_snapshots::PrimitiveAxisLabel;
pub use crate::model::ipane_primitive::PrimitivePaneViewZOrder as PrimitiveLayer;
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PrimitiveId(u64);
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PrimitivePaneId(u64);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimitiveOwner {
    Series(SeriesId),
    Pane(PrimitivePaneId),
}
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_PANE: AtomicU64 = AtomicU64::new(1);
pub(crate) fn pane_id() -> PrimitivePaneId {
    PrimitivePaneId(
        NEXT_PANE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .expect("pane identity exhausted"),
    )
}

/// Borrowed only during preparation. A pane attachment has no price converter.
pub struct PrimitiveContext<'a, B: ChartBehavior = HorzScaleBehaviorTime> {
    pub owner: PrimitiveOwner,
    pub pane: usize,
    pub size: Size,
    pub pixel_ratio: f32,
    pub model: &'a ChartModel<B>,
    pub visible_range: Option<crate::model::range_impl::RangeImpl<TimePointIndex>>,
}
/// Copyable, start-of-gesture conversions; contains no live Chart references.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrimitiveCoordinateTransform {
    x_origin: f64,
    spacing: f64,
    price: Option<crate::model::price_scale::PriceCoordinateTransform>,
}
impl PrimitiveCoordinateTransform {
    pub fn logical_to_coordinate(&self, logical: f64) -> Option<f64> {
        let x = self.x_origin + logical * self.spacing;
        (logical.is_finite() && x.is_finite()).then_some(x)
    }
    pub fn coordinate_to_logical(&self, x: f64) -> Option<f64> {
        let logical = (x - self.x_origin) / self.spacing;
        (x.is_finite() && logical.is_finite()).then_some(logical)
    }
    pub fn price_to_coordinate(&self, price: f64) -> Option<f64> {
        let y = self.price?.price_to_coordinate(price).value();
        (price.is_finite() && y.is_finite()).then_some(y)
    }
    pub fn coordinate_to_price(&self, y: f64) -> Option<f64> {
        let price = self.price?.coordinate_to_price(y.into());
        (y.is_finite() && price.is_finite()).then_some(price)
    }
}
impl<B: ChartBehavior> PrimitiveContext<'_, B> {
    pub fn coordinate_transform(&self) -> Option<PrimitiveCoordinateTransform> {
        let price = match self.owner {
            PrimitiveOwner::Series(id) => self
                .model
                .panes()
                .get(self.pane)?
                .price_scale_for_source(id)?
                .coordinate_transform(
                    self.model
                        .projection(id)?
                        .first_value(self.visible_range.as_ref())?
                        .value,
                )
                .into(),
            PrimitiveOwner::Pane(_) => None,
        };
        Some(PrimitiveCoordinateTransform {
            x_origin: self.logical_to_coordinate(0.)?,
            spacing: self.model.time_scale().bar_spacing(),
            price,
        })
    }
    /// Continuous drawing positions; unlike the source index API, no rounding.
    pub fn continuous_logical_to_coordinate(&self, logical: f64) -> Option<f64> {
        let scale = self.model.time_scale();
        let anchor = self.logical_to_coordinate(0.)?;
        let x = anchor + logical * scale.bar_spacing();
        (logical.is_finite() && x.is_finite()).then_some(x)
    }
    pub fn coordinate_to_logical(&self, x: f64) -> Option<f64> {
        let scale = self.model.time_scale();
        let logical = (x - self.logical_to_coordinate(0.)?) / scale.bar_spacing();
        (x.is_finite() && logical.is_finite()).then_some(logical)
    }
    /// Uses the owning series' actual scale and visible first value.
    pub fn coordinate_to_price(&self, y: f64) -> Option<f64> {
        let PrimitiveOwner::Series(id) = self.owner else {
            return None;
        };
        let scale = self
            .model
            .panes()
            .get(self.pane)?
            .price_scale_for_source(id)?;
        let first = self
            .model
            .projection(id)?
            .first_value(self.visible_range.as_ref())?;
        let price = scale.coordinate_to_price(y.into(), first.value);
        (y.is_finite() && price.is_finite()).then_some(price)
    }
    pub fn time_to_coordinate(&self, time: &B::Item) -> Option<f64> {
        let time = self.model.time_scale().behavior().to_internal(time).ok()?;
        let index = self.model.time_scale().time_to_index(&time, false)?;
        self.logical_to_coordinate(index.value())
    }
    /// Matches the source scale: fractional indices map to zero. Interpolate
    /// from an integer index and `bar_spacing` for continuous positions.
    pub fn logical_to_coordinate(&self, logical: f64) -> Option<f64> {
        if !logical.is_finite() || self.model.time_scale().is_empty() {
            return None;
        }
        let coordinate = self
            .model
            .time_scale()
            .index_to_coordinate(TimePointIndex::new(logical))
            .value();
        coordinate.is_finite().then_some(coordinate)
    }
    pub fn price_to_coordinate(&self, price: f64) -> Option<f64> {
        let PrimitiveOwner::Series(id) = self.owner else {
            return None;
        };
        if !price.is_finite() {
            return None;
        }
        let pane = self.model.panes().get(self.pane)?;
        let scale = pane.price_scale_for_source(id)?;
        let first = self
            .model
            .projection(id)?
            .first_value(self.visible_range.as_ref())?;
        let y = scale.price_to_coordinate(price, first.value).value();
        y.is_finite().then_some(y)
    }
}

/// Return renderers owning copied geometry. Do not capture mutable live state in them.
pub trait DrawingPrimitive<B: ChartBehavior = HorzScaleBehaviorTime>:
    std::fmt::Debug + 'static
{
    fn prepare(&self, context: &PrimitiveContext<'_, B>) -> PreparedPrimitive;
    fn autoscale_info(
        &self,
        _start: TimePointIndex,
        _end: TimePointIndex,
    ) -> Option<AutoscaleInfo> {
        None
    }
    fn attached(&self, _owner: PrimitiveOwner) {}
    fn detached(&self) {}
}
#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveHitCandidate {
    /// Opt into mouse editing; `external_id` names the application's edit target.
    pub editable: bool,
    pub external_id: String,
    pub distance: f64,
    pub priority: crate::model::internal_hit_test::HitTestPriority,
    pub layer: PrimitiveLayer,
    pub is_background: bool,
    pub cursor: Option<iced::mouse::Interaction>,
}
impl PrimitiveHitCandidate {
    pub fn new(external_id: impl Into<String>) -> Self {
        Self {
            editable: false,
            external_id: external_id.into(),
            distance: 0.,
            priority: Default::default(),
            layer: PrimitiveLayer::Normal,
            is_background: false,
            cursor: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveHit {
    pub editable: bool,
    pub primitive: PrimitiveId,
    pub owner: PrimitiveOwner,
    pub external_id: String,
    pub cursor: Option<iced::mouse::Interaction>,
}
/// Optional independent hit geometry, owned by the same preparation revision.
pub trait PrimitiveHitTester: std::fmt::Debug + 'static {
    fn hit_test(&self, point: Point) -> Option<PrimitiveHitCandidate>;
}
/// Iced is the adapter boundary; the same snapshot renders on TinySkia and WGPU.
pub trait PrimitiveRenderer: std::fmt::Debug + 'static {
    fn draw(&self, target: &mut Frame);
    fn draw_background(&self, _target: &mut Frame) {}
    fn hit_test(&self, _point: Point) -> Option<PrimitiveHitCandidate> {
        None
    }
}
#[derive(Clone, Debug)]
pub struct PrimitiveView {
    pub layer: PrimitiveLayer,
    pub renderer: Rc<dyn PrimitiveRenderer>,
}
impl PrimitiveView {
    pub fn new(layer: PrimitiveLayer, renderer: impl PrimitiveRenderer) -> Self {
        Self {
            layer,
            renderer: Rc::new(renderer),
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct PreparedPrimitive {
    pub hit_tester: Option<Rc<dyn PrimitiveHitTester>>,
    pub price_labels: Vec<PrimitiveAxisLabel>,
    pub time_labels: Vec<PrimitiveAxisLabel>,
    pub price_views: Vec<PrimitiveView>,
    pub time_views: Vec<PrimitiveView>,
    pub views: Vec<PrimitiveView>,
}
#[derive(Clone, Debug)]
pub struct PreparedAttachment {
    pub order: u64,
    pub id: PrimitiveId,
    pub owner: PrimitiveOwner,
    pub output: PreparedPrimitive,
    pub price_side: Option<crate::model::axis_snapshots::PriceAxisSide>,
}
pub(crate) struct Attachment<B: ChartBehavior = HorzScaleBehaviorTime> {
    pub order: u64,
    pub id: PrimitiveId,
    pub owner: PrimitiveOwner,
    pub primitive: Rc<dyn DrawingPrimitive<B>>,
}
impl<B: ChartBehavior> Drop for Attachment<B> {
    fn drop(&mut self) {
        self.primitive.detached();
    }
}
impl<B: super::ChartBehavior> Chart<B> {
    pub fn primitive_context(
        &mut self,
        id: PrimitiveId,
    ) -> Result<PrimitiveContext<'_, B>, ChartError> {
        let owner = self.primitive_owner(id)?;
        let pane = match owner {
            PrimitiveOwner::Series(series) => self.model.pane_for_series(series),
            PrimitiveOwner::Pane(key) => self.primitive_panes.iter().position(|p| *p == key),
        }
        .ok_or(ChartError::UnknownPrimitive(id))?;
        let visible_range = self.model.visible_strict_range();
        let state = &self.model.panes()[pane];
        Ok(PrimitiveContext {
            owner,
            pane,
            size: Size::new(state.width() as f32, state.height() as f32),
            pixel_ratio: self.ratio,
            model: &self.model,
            visible_range,
        })
    }
    /// Notify after application-owned state changes. Preparation still copies geometry.
    pub fn request_primitive_update(&mut self, id: PrimitiveId) -> Result<(), ChartError> {
        self.primitive_owner(id)?;
        self.cancel_primitive_edit_for(id, super::PrimitiveEditCancelReason::ExternalChange);
        self.model.full_update();
        Ok(())
    }
    pub(crate) fn next_extension_order(&mut self) -> u64 {
        let order = self.extension_order;
        self.extension_order = self
            .extension_order
            .checked_add(1)
            .expect("extension order exhausted");
        order
    }
    pub fn primitive_hit(&self, pane: usize, point: Point) -> Option<PrimitiveHit> {
        self.frame_for_pane(pane)?.plot.primitive_hit(point)
    }
    fn sync_primitive_autoscale(&mut self, id: PrimitiveId) {
        let attachment = self
            .primitives
            .iter()
            .find(|a| a.id == id)
            .expect("validated attachment");
        if let PrimitiveOwner::Series(series) = attachment.owner {
            let primitive = attachment.primitive.clone();
            self.model.set_primitive_autoscale(
                series,
                id.0,
                Some(Rc::new(move |start, end| {
                    let info = primitive.autoscale_info(start, end)?;
                    if info.price_range.is_some_and(|r| {
                        !r.min_value.is_finite()
                            || !r.max_value.is_finite()
                            || r.min_value > r.max_value
                    }) || info.margins.is_some_and(|m| {
                        !m.above.is_finite() || !m.below.is_finite() || m.above < 0. || m.below < 0.
                    }) {
                        return None;
                    }
                    Some(info)
                })),
            );
        }
    }
    fn attach_primitive(
        &mut self,
        owner: PrimitiveOwner,
        primitive: impl DrawingPrimitive<B>,
    ) -> Result<PrimitiveId, ChartError> {
        let id = NEXT_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map(PrimitiveId)
            .map_err(|_| ChartError::IdExhausted)?;
        primitive.attached(owner);
        let order = self.next_extension_order();
        self.primitives.push(Attachment {
            order,
            id,
            owner,
            primitive: Rc::new(primitive),
        });
        self.sync_primitive_autoscale(id);
        self.model.full_update();
        Ok(id)
    }
    pub fn add_series_primitive(
        &mut self,
        series: SeriesId,
        primitive: impl DrawingPrimitive<B>,
    ) -> Result<PrimitiveId, ChartError> {
        if self.model.projection(series).is_none() {
            return Err(ChartError::UnknownSeries(series));
        }
        self.attach_primitive(PrimitiveOwner::Series(series), primitive)
    }
    pub fn add_pane_primitive(
        &mut self,
        pane: usize,
        primitive: impl DrawingPrimitive<B>,
    ) -> Result<PrimitiveId, ChartError> {
        let owner = *self.primitive_panes.get(pane).ok_or(
            crate::model::chart_model::ChartModelError::InvalidPane(pane),
        )?;
        self.attach_primitive(PrimitiveOwner::Pane(owner), primitive)
    }
    /// Replace application state without sharing a mutable Chart reference.
    pub fn replace_primitive(
        &mut self,
        id: PrimitiveId,
        primitive: impl DrawingPrimitive<B>,
    ) -> Result<(), ChartError> {
        self.primitive_owner(id)?;
        self.up_down.remove(&id);
        self.cancel_primitive_edit_for(id, super::PrimitiveEditCancelReason::ExternalChange);
        let attachment = self
            .primitives
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or(ChartError::UnknownPrimitive(id))?;
        let owner = attachment.owner;
        let order = attachment.order;
        let previous = std::mem::replace(
            attachment,
            Attachment {
                order,
                id,
                owner,
                primitive: Rc::new(primitive),
            },
        );
        drop(previous);
        attachment.primitive.attached(owner);
        self.sync_primitive_autoscale(id);
        self.model.full_update();
        Ok(())
    }
    pub fn remove_primitive(&mut self, id: PrimitiveId) -> Result<(), ChartError> {
        self.primitive_owner(id)?;
        self.cancel_primitive_edit_for(id, super::PrimitiveEditCancelReason::OwnerRemoved);
        let index = self
            .primitives
            .iter()
            .position(|a| a.id == id)
            .ok_or(ChartError::UnknownPrimitive(id))?;
        let owner = self.primitives[index].owner;
        if let PrimitiveOwner::Series(series) = owner {
            self.model.set_primitive_autoscale(series, id.0, None);
        }
        self.primitives.remove(index);
        self.up_down.remove(&id);
        self.model.full_update();
        Ok(())
    }
    pub fn primitive_owner(&self, id: PrimitiveId) -> Result<PrimitiveOwner, ChartError> {
        self.primitives
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.owner)
            .ok_or(ChartError::UnknownPrimitive(id))
    }
}

impl crate::ui::line_chart::PlotSnapshot {
    pub(crate) fn draw_primitive_layer(&self, frame: &mut Frame, layer: PrimitiveLayer) {
        for background in [true, false] {
            self.draw_primitives(frame, layer, None, background);
            let raised = self.hovered_series.filter(|_| self.hovered_series_on_top);
            let ids = || {
                std::iter::once(self.series_id)
                    .chain(self.series_layers.iter().map(|s| s.series_id))
                    .enumerate()
                    .filter_map(|(layer, id)| id.map(|id| (layer, id)))
            };
            for (series_layer, id) in ids()
                .filter(|(_, id)| Some(*id) != raised)
                .chain(ids().filter(|(_, id)| Some(*id) == raised))
            {
                if layer == PrimitiveLayer::Top && !background {
                    for animation in self.last_price_animations.iter().filter(|a| a.series == id) {
                        animation.draw(frame);
                    }
                    self.draw_annotations(
                        frame,
                        crate::model::series_markers::MarkerZOrder::Top,
                        Some(series_layer),
                    );
                }
                self.draw_primitives(frame, layer, Some(id), background);
            }
        }
    }
    pub(crate) fn draw_primitives(
        &self,
        frame: &mut Frame,
        layer: PrimitivePaneViewZOrder,
        owner: Option<SeriesId>,
        background: bool,
    ) {
        for attachment in &self.primitives {
            let matches = match attachment.owner {
                PrimitiveOwner::Pane(_) => owner.is_none(),
                PrimitiveOwner::Series(id) => owner == Some(id),
            };
            if matches {
                for view in &attachment.output.views {
                    if view.layer == layer {
                        // Restore transforms before drawing the next view.
                        frame.with_save(|frame| {
                            if background {
                                view.renderer.draw_background(frame);
                            } else {
                                view.renderer.draw(frame);
                            }
                        });
                    }
                }
            }
        }
    }
}

pub(crate) fn prepare_axes<B: ChartBehavior>(
    model: &mut ChartModel<B>,
    axes: &mut [crate::model::axis_snapshots::AxisSnapshots],
    attachments: &[Attachment<B>],
    pane_keys: &[PrimitivePaneId],
    ratio: f32,
) -> Result<Vec<Vec<PreparedAttachment>>, crate::ui::chart_frame::ChartFrameError> {
    use crate::model::{
        axis_snapshots::{PriceAxisSide, SeriesAxisLabel},
        pane::PriceScalePosition,
    };
    if attachments.is_empty() {
        return Ok(vec![]);
    }
    let visible_range = model.visible_strict_range();
    let mut output = vec![vec![]; axes.len()];
    let mut time_labels = vec![];
    for (pane, snapshots) in axes.iter_mut().enumerate() {
        let source_ids = model.ordered_pane_sources(pane);
        let state = &model.panes()[pane];
        let mut ordered = attachments
            .iter()
            .filter(|a| match a.owner {
                PrimitiveOwner::Series(id) => model.pane_for_series(id) == Some(pane),
                PrimitiveOwner::Pane(id) => pane_keys.get(pane) == Some(&id),
            })
            .collect::<Vec<_>>();
        ordered.sort_by_key(|a| match a.owner {
            PrimitiveOwner::Pane(_) => 0,
            PrimitiveOwner::Series(id) => 1 + source_ids.iter().position(|s| *s == id).unwrap_or(0),
        });
        for a in ordered {
            let size = Size::new(state.width() as f32, state.height() as f32);
            let mut prepared = a.primitive.prepare(&PrimitiveContext {
                owner: a.owner,
                pane,
                size,
                pixel_ratio: ratio,
                model,
                visible_range,
            });
            let price_side = match a.owner {
                PrimitiveOwner::Series(id) => match state.source_position(id) {
                    Some(PriceScalePosition::Left) => Some(PriceAxisSide::Left),
                    Some(PriceScalePosition::Right) => Some(PriceAxisSide::Right),
                    _ => None,
                },
                _ => None,
            };
            if let PrimitiveOwner::Series(id) = a.owner {
                let label_side = price_side.or_else(|| {
                    state.default_visible_price_scale().map(|s| {
                        if s.id() == "left" {
                            PriceAxisSide::Left
                        } else {
                            PriceAxisSide::Right
                        }
                    })
                });
                if let Some(side) = label_side {
                    let axis = match side {
                        PriceAxisSide::Left => &mut snapshots.left,
                        PriceAxisSide::Right => &mut snapshots.right,
                    };
                    for label in prepared
                        .price_labels
                        .iter()
                        .filter(|l| l.visible && !l.axis.text.is_empty())
                    {
                        axis.labels.push(SeriesAxisLabel {
                            source: Some(id),
                            axis: label.axis.clone(),
                            pane_text: String::new(),
                            render_coordinate: label
                                .fixed_coordinate
                                .unwrap_or(label.axis.coordinate),
                            fixed_coordinate: label.fixed_coordinate,
                            tick_visible: Some(label.tick_visible),
                            additional_padding: axis.renderer_options.font_size / 3.,
                        });
                    }
                }
                time_labels.extend(
                    prepared
                        .time_labels
                        .iter()
                        .filter(|l| l.visible && !l.axis.text.is_empty())
                        .cloned(),
                );
            } else {
                prepared.price_labels.clear();
                prepared.time_labels.clear();
                prepared.price_views.clear();
                prepared.time_views.clear();
            }
            if let Some(side) = price_side {
                let axis = match side {
                    PriceAxisSide::Left => &mut snapshots.left,
                    PriceAxisSide::Right => &mut snapshots.right,
                };
                axis.primitive_graphics |= !prepared.price_views.is_empty();
            }
            output[pane].push(PreparedAttachment {
                order: a.order,
                id: a.id,
                owner: a.owner,
                output: prepared,
                price_side,
            });
        }
        for axis in [&mut snapshots.left, &mut snapshots.right] {
            axis.labels.sort_by_key(|label| {
                label
                    .source
                    .and_then(|id| source_ids.iter().position(|s| *s == id))
                    .unwrap_or(0)
            });
        }
    }
    for axis in axes {
        axis.time.primitive_labels = time_labels.clone();
        axis.time.primitive_graphics = output
            .iter()
            .flatten()
            .any(|a| !a.output.time_views.is_empty());
    }
    Ok(output)
}

impl crate::ui::line_chart::PlotSnapshot {
    pub(crate) fn draw_axis_views(
        &self,
        frame: &mut Frame,
        layer: PrimitiveLayer,
        side: Option<crate::model::axis_snapshots::PriceAxisSide>,
    ) {
        let attachments = if side.is_some() {
            &self.primitives
        } else {
            &self.time_primitives
        };
        for background in [true, false] {
            for a in attachments {
                if side.is_some() && a.price_side != side {
                    continue;
                }
                let views = if side.is_some() {
                    &a.output.price_views
                } else {
                    &a.output.time_views
                };
                for v in views.iter().filter(|v| v.layer == layer) {
                    frame.with_save(|frame| {
                        if background {
                            v.renderer.draw_background(frame)
                        } else {
                            v.renderer.draw(frame)
                        }
                    });
                }
            }
        }
    }
}
