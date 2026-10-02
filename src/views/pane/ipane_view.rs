use crate::{
    model::{coordinate::Coordinate, internal_hit_test::InternalHitTestCandidate, pane::Pane},
    renderers::ipane_renderer::PaneRenderer,
};
pub trait PaneView<Target> {
    fn renderer(&self, pane: &Pane, add_anchors: bool) -> Option<Box<dyn PaneRenderer<Target>>>;
    fn hit_test(
        &self,
        _x: Coordinate,
        _y: Coordinate,
        _pane: Option<&Pane>,
    ) -> Option<InternalHitTestCandidate> {
        None
    }
}
