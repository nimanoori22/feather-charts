use crate::model::{coordinate::Coordinate, internal_hit_test::InternalHitTestCandidate};
/// Backend-neutral pane renderer. `Target` is supplied by the Iced adapter later.
pub trait PaneRenderer<Target> {
    fn draw(&self, target: &mut Target, is_hovered: bool);
    fn draw_background(&self, _target: &mut Target, _is_hovered: bool) {}
    fn hit_test(&self, _x: Coordinate, _y: Coordinate) -> Option<InternalHitTestCandidate> {
        None
    }
}
pub trait AxisRenderer<Target> {
    fn draw(&self, target: &mut Target, is_hovered: bool);
    fn draw_background(&self, _target: &mut Target, _is_hovered: bool) {}
}
pub trait TimeAxisRenderer<Target>: AxisRenderer<Target> {}
impl<Target, T: AxisRenderer<Target>> TimeAxisRenderer<Target> for T {}
