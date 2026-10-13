//! Owned annotations. Attachment identity is distinct from optional external IDs.
use super::{autoscale_info_impl::AutoScaleMargins, data_layer::SeriesId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MarkerCollectionId(pub(crate) u64);
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MarkerPosition {
    AboveBar,
    BelowBar,
    InBar,
    AtPriceTop(f64),
    AtPriceBottom(f64),
    AtPriceMiddle(f64),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkerShape {
    Circle,
    Square,
    ArrowUp,
    ArrowDown,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MarkerZOrder {
    #[default]
    Normal,
    AboveSeries,
    Top,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeriesMarkersOptions {
    pub auto_scale: bool,
    pub z_order: MarkerZOrder,
}
impl Default for SeriesMarkersOptions {
    fn default() -> Self {
        Self {
            auto_scale: true,
            z_order: MarkerZOrder::Normal,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct SeriesMarker<T> {
    pub time: T,
    pub position: MarkerPosition,
    pub shape: MarkerShape,
    pub color: String,
    pub size: f64,
    pub id: Option<String>,
    pub text: Option<String>,
}
impl<T> SeriesMarker<T> {
    pub fn new(
        time: T,
        position: MarkerPosition,
        shape: MarkerShape,
        color: impl Into<String>,
    ) -> Self {
        Self {
            time,
            position,
            shape,
            color: color.into(),
            size: 1.,
            id: None,
            text: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkerHit {
    pub series: SeriesId,
    pub collection: MarkerCollectionId,
    pub index: usize,
    pub external_id: Option<String>,
}
#[derive(Clone, Debug)]
pub(crate) struct MarkerCollection<T> {
    pub id: MarkerCollectionId,
    pub markers: Vec<SeriesMarker<T>>,
    pub options: SeriesMarkersOptions,
}
pub fn ceiled_odd(value: f64) -> f64 {
    let n = value.ceil();
    if n % 2. == 0. { n + 1. } else { n }
}
pub fn shape_height(spacing: f64) -> f64 {
    ceiled_odd(spacing.clamp(12., 30.)) + 1.
}
pub fn shape_margin(spacing: f64) -> f64 {
    ceiled_odd(spacing.clamp(12., 30.) * 0.1).max(3.)
}
pub fn shape_size(shape: MarkerShape, size: f64) -> f64 {
    ceiled_odd(
        size.max(12.)
            * match shape {
                MarkerShape::Circle => 0.8,
                MarkerShape::Square => 0.7,
                _ => 1.,
            },
    )
}
impl<T> MarkerCollection<T> {
    pub fn margins(&self, spacing: f64) -> Option<AutoScaleMargins> {
        if !self.options.auto_scale || self.markers.is_empty() {
            return None;
        }
        let margin = shape_height(spacing) * 1.5 + shape_margin(spacing) * 2.;
        let in_bar = self
            .markers
            .iter()
            .any(|m| m.position == MarkerPosition::InBar);
        let adjusted = |side| {
            if self.markers.iter().any(|m| m.position == side) {
                margin
            } else if in_bar {
                (margin / 2.).ceil()
            } else {
                0.
            }
        };
        Some(AutoScaleMargins {
            above: adjusted(MarkerPosition::AboveBar),
            below: adjusted(MarkerPosition::BelowBar),
        })
    }
}
