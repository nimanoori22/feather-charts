use crate::model::coordinate::Coordinate;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PrimitivePaneViewZOrder {
    Bottom,
    #[default]
    Normal,
    Top,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveHoveredItem {
    pub distance: Option<f64>,
    pub external_id: String,
    pub z_order: PrimitivePaneViewZOrder,
    pub cursor_style: Option<String>,
    pub is_background: bool,
}
pub trait PrimitiveHitTestSource {
    fn primitive_hit_test(&self, _x: Coordinate, _y: Coordinate) -> Vec<PrimitiveHoveredItem> {
        Vec::new()
    }
}
