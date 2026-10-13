//! Upstream arbitration keeps stable source order separate from render order.
use super::line_chart::PlotSnapshot;
use crate::{
    api::{PrimitiveHit, PrimitiveHitCandidate, PrimitiveLayer, PrimitiveOwner},
    model::{
        data_layer::SeriesId,
        hit_test::{ChartHit, HitTarget},
        internal_hit_test::{HitTestPriority, InternalHitTestCandidate, is_better_hit},
    },
};
#[derive(Clone)]
pub(crate) enum PaneHit {
    BuiltIn(ChartHit),
    Primitive(PrimitiveHit),
}
impl PaneHit {
    pub(crate) fn series(&self) -> Option<SeriesId> {
        match self {
            Self::BuiltIn(h) => Some(h.series),
            Self::Primitive(h) => match h.owner {
                PrimitiveOwner::Series(id) => Some(id),
                _ => None,
            },
        }
    }
}
struct ExtensionHit {
    hit: PaneHit,
    candidate: InternalHitTestCandidate,
    layer: PrimitiveLayer,
    background: bool,
    order: u64,
}
fn rank(layer: PrimitiveLayer) -> u8 {
    match layer {
        PrimitiveLayer::Bottom => 0,
        PrimitiveLayer::Normal => 1,
        PrimitiveLayer::Top => 2,
    }
}
impl PlotSnapshot {
    pub fn hit_test(&self, point: iced::Point) -> Option<ChartHit> {
        match self.selection(point)? {
            PaneHit::BuiltIn(h) => Some(h),
            _ => None,
        }
    }
    pub fn primitive_hit(&self, point: iced::Point) -> Option<PrimitiveHit> {
        match self.selection(point)? {
            PaneHit::Primitive(h) => Some(h),
            _ => None,
        }
    }
    pub(crate) fn selection(&self, point: iced::Point) -> Option<PaneHit> {
        if self.primitives.is_empty() {
            return self.builtin_hit_test(point).map(PaneHit::BuiltIn);
        }
        if !point.x.is_finite()
            || !point.y.is_finite()
            || point.x < 0.
            || point.y < 0.
            || point.x >= self.size.width
            || point.y >= self.size.height
        {
            return None;
        }
        let ids = || {
            std::iter::once(self.series_id)
                .chain(self.series_layers.iter().map(|s| s.series_id))
                .rev()
                .flatten()
                .map(Some)
                .chain(std::iter::once(None))
        };
        let marker = self.marker_hit(point);
        let mut primitive: Option<ExtensionHit> = None;
        for source in ids() {
            let mut offer = |hit: ExtensionHit| {
                let better = primitive.as_ref().is_none_or(|best| {
                    rank(hit.layer) > rank(best.layer)
                        || (hit.layer == best.layer
                            && (is_better_hit(&hit.candidate, Some(&best.candidate))
                                || (hit.hit.series() == best.hit.series()
                                    && !is_better_hit(&best.candidate, Some(&hit.candidate))
                                    && hit.order < best.order)))
                });
                if better {
                    primitive = Some(hit);
                }
            };
            for attachment in &self.primitives {
                let owner = match attachment.owner {
                    PrimitiveOwner::Series(id) => Some(id),
                    _ => None,
                };
                if owner != source {
                    continue;
                }
                let mut offer_candidate = |c: PrimitiveHitCandidate| {
                    if !c.distance.is_finite() || c.distance < 0. {
                        return;
                    }
                    let candidate = InternalHitTestCandidate {
                        distance: c.distance,
                        priority: c.priority,
                        cursor_style: None,
                        external_id: Some(c.external_id.clone()),
                    };
                    offer(ExtensionHit {
                        candidate,
                        layer: c.layer,
                        background: c.is_background,
                        order: attachment.order,
                        hit: PaneHit::Primitive(PrimitiveHit {
                            editable: c.editable,
                            primitive: attachment.id,
                            owner: attachment.owner,
                            external_id: c.external_id,
                            cursor: c.cursor,
                        }),
                    });
                };
                if let Some(tester) = &attachment.output.hit_tester {
                    if let Some(c) = tester.hit_test(point) {
                        offer_candidate(c);
                    }
                } else {
                    for view in &attachment.output.views {
                        if let Some(c) = view.renderer.hit_test(point) {
                            offer_candidate(c);
                        }
                    }
                }
            }
            if let Some(m) = marker.as_ref().filter(|m| Some(m.series) == source) {
                offer(ExtensionHit {
                    hit: PaneHit::BuiltIn(ChartHit {
                        series: m.series,
                        target: HitTarget::Marker(m.clone()),
                    }),
                    candidate: InternalHitTestCandidate {
                        distance: 0.,
                        priority: HitTestPriority::Point,
                        cursor_style: None,
                        external_id: m.external_id.clone(),
                    },
                    layer: PrimitiveLayer::Normal,
                    background: false,
                    order: self.marker_orders.get(&m.collection).copied().unwrap_or(0),
                });
            }
        }
        if primitive
            .as_ref()
            .is_some_and(|h| h.layer == PrimitiveLayer::Top)
        {
            return primitive.map(|h| h.hit);
        }
        let mut best: Option<(ChartHit, InternalHitTestCandidate)> = None;
        let mut geometry = self.hit_series.iter().rev().peekable();
        for source in ids() {
            let owns = primitive
                .as_ref()
                .is_some_and(|h| h.hit.series() == source && h.layer != PrimitiveLayer::Bottom);
            if owns && primitive.as_ref().is_some_and(|h| !h.background) {
                return best
                    .map(|(h, _)| PaneHit::BuiltIn(h))
                    .or_else(|| primitive.map(|h| h.hit));
            }
            if let Some(id) = source {
                let mut local: Option<(ChartHit, InternalHitTestCandidate)> = None;
                let mut offer = |hit: Option<(ChartHit, InternalHitTestCandidate)>| {
                    if let Some(hit) = hit
                        && is_better_hit(&hit.1, local.as_ref().map(|(_, c)| c))
                    {
                        local = Some(hit);
                    }
                };
                let from = self.hit_price_lines.partition_point(|p| p.series < id);
                let to = self.hit_price_lines.partition_point(|p| p.series <= id);
                let prices = &self.hit_price_lines[from..to];
                for price in prices
                    .iter()
                    .filter(|p| p.kind == crate::api::PriceLineKind::RelativeBase)
                {
                    offer(price.hit(f64::from(point.y)));
                }
                offer(
                    (if geometry.peek().is_some_and(|s| s.series == id) {
                        geometry.next()
                    } else {
                        None
                    })
                    .and_then(|s| s.hit(f64::from(point.x), f64::from(point.y))),
                );
                for price in prices
                    .iter()
                    .filter(|p| p.kind != crate::api::PriceLineKind::RelativeBase)
                {
                    offer(price.hit(f64::from(point.y)));
                }
                if let Some(hit) = local
                    && is_better_hit(&hit.1, best.as_ref().map(|(_, c)| c))
                {
                    best = Some(hit);
                }
            }
            if owns {
                return best
                    .map(|(h, _)| PaneHit::BuiltIn(h))
                    .or_else(|| primitive.map(|h| h.hit));
            }
        }
        best.map(|(h, _)| PaneHit::BuiltIn(h))
            .or_else(|| primitive.map(|h| h.hit))
    }
}
