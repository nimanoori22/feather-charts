//! Owned public identity, separate from renderer distance and optional external IDs.
use super::{data_layer::SeriesId, price_line::PriceLineId, series_markers::MarkerHit};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PriceLineKind {
    LastValue,
    RelativeBase,
    Custom(PriceLineId),
}
#[derive(Clone, Debug, PartialEq)]
pub enum HitTarget {
    CustomSeries {
        hit_type: super::icustom_series::CustomSeriesHitTestType,
        object_id: Option<String>,
        cursor_style: Option<String>,
    },
    SeriesPoint,
    SeriesLine,
    SeriesRange,
    PriceLine {
        line: PriceLineKind,
        external_id: Option<String>,
    },
    Marker(MarkerHit),
}
#[derive(Clone, Debug, PartialEq)]
pub struct ChartHit {
    pub series: SeriesId,
    pub target: HitTarget,
}
