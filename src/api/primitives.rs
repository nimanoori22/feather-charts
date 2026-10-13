//! Application extensions prepare owned geometry; drawing never reads live state.
use super::{Chart, ChartError};
use crate::model::{
    autoscale_info_impl::AutoscaleInfo, chart_model::ChartModel, data_layer::SeriesId,
    horz_scale_behavior_time::horz_scale_behavior_time::HorzScaleBehaviorTime,
    ipane_primitive::PrimitivePaneViewZOrder, time_data::TimePointIndex,
};
use iced::{Size, widget::canvas::Frame};
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
    PrimitivePaneId(NEXT_PANE.fetch_add(1, Ordering::Relaxed))
}

/// Borrowed only during preparation. A pane attachment has no price converter.
pub struct PrimitiveContext<'a> {
    pub owner: PrimitiveOwner,
    pub pane: usize,
    pub size: Size,
    pub pixel_ratio: f32,
    pub model: &'a ChartModel<HorzScaleBehaviorTime>,
    pub visible_range: Option<crate::model::range_impl::RangeImpl<TimePointIndex>>,
}
impl PrimitiveContext<'_> {
    pub fn logical_to_coordinate(&self, logical: f64) -> Option<f64> {
        if !logical.is_finite() || self.model.time_scale().is_empty() {
            return None;
        }
        Some(
            self.model
                .time_scale()
                .index_to_coordinate(TimePointIndex::new(logical))
                .value(),
        )
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
            .series(id)?
            .first_value(self.visible_range.as_ref())?;
        let y = scale.price_to_coordinate(price, first.value).value();
        y.is_finite().then_some(y)
    }
}

/// Return renderers owning copied geometry. Do not capture mutable live state in them.
pub trait DrawingPrimitive: std::fmt::Debug + 'static {
    fn prepare(&self, context: &PrimitiveContext<'_>) -> PreparedPrimitive;
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
/// Iced is the adapter boundary; the same snapshot renders on TinySkia and WGPU.
pub trait PrimitiveRenderer: std::fmt::Debug + 'static {
    fn draw(&self, target: &mut Frame);
    fn draw_background(&self, _target: &mut Frame) {}
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
    pub price_labels: Vec<PrimitiveAxisLabel>,
    pub time_labels: Vec<PrimitiveAxisLabel>,
    pub price_views: Vec<PrimitiveView>,
    pub time_views: Vec<PrimitiveView>,
    pub views: Vec<PrimitiveView>,
}
#[derive(Clone, Debug)]
pub struct PreparedAttachment {
    pub id: PrimitiveId,
    pub owner: PrimitiveOwner,
    pub output: PreparedPrimitive,
    pub price_side: Option<crate::model::axis_snapshots::PriceAxisSide>,
}
pub(crate) struct Attachment {
    pub id: PrimitiveId,
    pub owner: PrimitiveOwner,
    pub primitive: Rc<dyn DrawingPrimitive>,
}
impl Drop for Attachment {
    fn drop(&mut self) {
        self.primitive.detached();
    }
}
impl Chart {
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
        primitive: impl DrawingPrimitive,
    ) -> Result<PrimitiveId, ChartError> {
        let id = NEXT_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map(PrimitiveId)
            .map_err(|_| ChartError::IdExhausted)?;
        primitive.attached(owner);
        self.primitives.push(Attachment {
            id,
            owner,
            primitive: Rc::new(primitive),
        });
        self.sync_primitive_autoscale(id);
        self.model.full_update();
        self.revision = self.revision.wrapping_add(1);
        Ok(id)
    }
    pub fn add_series_primitive(
        &mut self,
        series: SeriesId,
        primitive: impl DrawingPrimitive,
    ) -> Result<PrimitiveId, ChartError> {
        if self.model.series(series).is_none() {
            return Err(ChartError::UnknownSeries(series));
        }
        self.attach_primitive(PrimitiveOwner::Series(series), primitive)
    }
    pub fn add_pane_primitive(
        &mut self,
        pane: usize,
        primitive: impl DrawingPrimitive,
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
        primitive: impl DrawingPrimitive,
    ) -> Result<(), ChartError> {
        let attachment = self
            .primitives
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or(ChartError::UnknownPrimitive(id))?;
        let owner = attachment.owner;
        primitive.attached(owner);
        *attachment = Attachment {
            id,
            owner,
            primitive: Rc::new(primitive),
        };
        self.sync_primitive_autoscale(id);
        self.model.full_update();
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }
    pub fn remove_primitive(&mut self, id: PrimitiveId) -> Result<(), ChartError> {
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
        self.model.full_update();
        self.revision = self.revision.wrapping_add(1);
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
            let ids = std::iter::once(self.series_id)
                .chain(self.series_layers.iter().map(|s| s.series_id));
            for id in ids.flatten().filter(|id| Some(*id) != raised).chain(raised) {
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
                        // A private draft prevents a plugin transform from leaking into later views.
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

pub(crate) fn prepare_axes(
    model: &mut ChartModel<HorzScaleBehaviorTime>,
    axes: &mut [crate::model::axis_snapshots::AxisSnapshots],
    attachments: &[Attachment],
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
