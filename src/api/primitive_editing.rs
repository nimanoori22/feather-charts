//! Optional synchronous gesture capture. Applications own drawing geometry.
use super::{Chart, ChartError, PrimitiveHit, PrimitiveId, PrimitiveOwner};
use crate::model::{price_range_impl::PriceRangeImpl, price_scale::PriceScaleOptions};
use iced::{Point, Size};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrimitiveEditId(u64);
impl PrimitiveEditId {
    pub(super) fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(
            NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
                .expect("primitive edit identity exhausted"),
        )
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimitiveEditCancelReason {
    Escape,
    FocusLost,
    WindowLeft,
    ViewportChanged,
    ExternalChange,
    OwnerRemoved,
}
impl PrimitiveEditCancelReason {
    /// External replacement/removal must never be overwritten by stale rollback.
    pub fn restores_bounds(self) -> bool {
        !matches!(self, Self::ExternalChange | Self::OwnerRemoved)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimitiveEditPhase {
    Begin,
    Update,
    Commit,
    Cancel(PrimitiveEditCancelReason),
}
#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveEditEvent {
    pub id: PrimitiveEditId,
    pub hit: PrimitiveHit,
    pub pane: usize,
    /// Plot-local logical pixels, unsnapped and allowed outside the plot after press.
    pub start: Point,
    pub position: Point,
    pub phase: PrimitiveEditPhase,
}
#[derive(PartialEq)]
struct EditViewport {
    pane: usize,
    size: Size,
    plot_size: Size,
    stretches: Vec<f64>,
    ratio: f32,
    x_origin: f64,
    spacing: f64,
    first: Option<f64>,
    scale: Option<(String, PriceScaleOptions, Option<PriceRangeImpl>)>,
    revision: u64,
}
pub(super) struct EditSession {
    pub event: PrimitiveEditEvent,
    viewport: EditViewport,
    auto_scale: Option<bool>,
}
impl Chart {
    fn edit_viewport(&mut self, id: PrimitiveId) -> Option<EditViewport> {
        let revision = self.revision;
        let size = self.size;
        let ratio = self.ratio;
        let stretches = self
            .model
            .panes()
            .iter()
            .map(|p| p.stretch_factor())
            .collect();
        let c = self.primitive_context(id).ok()?;
        let (first, scale) = match c.owner {
            PrimitiveOwner::Series(series) => {
                let scale = c.model.panes()[c.pane].price_scale_for_source(series)?;
                let first = c
                    .model
                    .series(series)?
                    .first_value(c.visible_range.as_ref())?
                    .value;
                if scale.is_empty() {
                    return None;
                }
                (
                    Some(first),
                    Some((
                        scale.id().to_owned(),
                        scale.options().clone(),
                        scale.price_range(),
                    )),
                )
            }
            PrimitiveOwner::Pane(_) => (None, None),
        };
        Some(EditViewport {
            pane: c.pane,
            size,
            plot_size: c.size,
            stretches,
            ratio,
            x_origin: c.continuous_logical_to_coordinate(0.)?,
            spacing: c.model.time_scale().bar_spacing(),
            first,
            scale,
            revision,
        })
    }
    pub(super) fn primitive_edit_id(&self) -> Option<PrimitiveEditId> {
        self.primitive_edit.as_ref().map(|e| e.event.id)
    }
    /// Notification for the active editor's preview; no lifecycle hooks are run.
    pub fn preview_primitive_update(&mut self, id: PrimitiveEditId) -> Result<(), ChartError> {
        self.check_primitive_edit(self.size, self.ratio);
        if self.primitive_edit_id() != Some(id) {
            return Err(ChartError::InvalidInteraction);
        }
        self.model.full_update();
        Ok(())
    }
    pub(super) fn check_primitive_edit(&mut self, size: Size, ratio: f32) {
        let Some(edit) = self.primitive_edit.as_ref() else {
            return;
        };
        let id = edit.event.hit.primitive;
        let current = self.edit_viewport(id);
        if self.primitive_edit.as_ref().is_some_and(|e| {
            size != self.size || ratio != self.ratio || current.as_ref() != Some(&e.viewport)
        }) {
            self.cancel_primitive_edit(PrimitiveEditCancelReason::ViewportChanged);
        }
    }
    fn finish_primitive_edit(&mut self) -> Option<PrimitiveEditEvent> {
        let edit = self.primitive_edit.take()?;
        self.owner.freeze_axis_sizes(false);
        if let PrimitiveOwner::Series(series) = edit.event.hit.owner
            && let Some(auto) = edit.auto_scale
        {
            // Restore only our temporary policy, not externally changed scale options.
            let unchanged = self
                .model
                .pane_for_series(series)
                .and_then(|pane| self.model.panes()[pane].price_scale_for_source(series))
                .is_some_and(|scale| {
                    edit.viewport
                        .scale
                        .as_ref()
                        .is_some_and(|(id, options, _)| {
                            scale.id() == id && scale.options() == options
                        })
                });
            if unchanged {
                self.model.primitive_scale_auto(series, auto);
            }
        }
        self.model.full_update();
        Some(edit.event)
    }
    pub(super) fn cancel_primitive_edit(&mut self, reason: PrimitiveEditCancelReason) {
        if let Some(mut event) = self.finish_primitive_edit() {
            event.phase = PrimitiveEditPhase::Cancel(reason);
            self.pending_primitive_edits.push(event);
        }
    }
    pub(super) fn cancel_primitive_edit_for(
        &mut self,
        id: PrimitiveId,
        reason: PrimitiveEditCancelReason,
    ) {
        if self
            .primitive_edit
            .as_ref()
            .is_some_and(|e| e.event.hit.primitive == id)
        {
            self.cancel_primitive_edit(reason);
        }
    }
    pub(super) fn apply_primitive_edit(
        &mut self,
        event: PrimitiveEditEvent,
    ) -> Option<PrimitiveEditEvent> {
        match event.phase {
            PrimitiveEditPhase::Begin => {
                if self.primitive_edit.is_some()
                    || self.primitive_hit(event.pane, event.start).as_ref() != Some(&event.hit)
                    || !event.hit.editable
                    || event.position != event.start
                    || self.edit_viewport(event.hit.primitive).is_none()
                {
                    return None;
                }
                self.cancel_viewport_interaction();
                self.stop_time_scale_animation();
                self.controller.cancel();
                let auto_scale = match event.hit.owner {
                    PrimitiveOwner::Series(series) => {
                        self.model.primitive_scale_auto(series, false)
                    }
                    PrimitiveOwner::Pane(_) => None,
                };
                let viewport = self.edit_viewport(event.hit.primitive)?;
                self.owner.freeze_axis_sizes(true);
                self.primitive_edit = Some(EditSession {
                    event: event.clone(),
                    viewport,
                    auto_scale,
                });
            }
            _ => {
                let edit = self.primitive_edit.as_mut()?;
                if edit.event.id != event.id
                    || edit.event.hit != event.hit
                    || edit.event.start != event.start
                    || edit.event.pane != event.pane
                {
                    return None;
                }
                edit.event = event.clone();
                if matches!(
                    event.phase,
                    PrimitiveEditPhase::Commit | PrimitiveEditPhase::Cancel(_)
                ) {
                    self.finish_primitive_edit();
                }
            }
        }
        Some(event)
    }
}
