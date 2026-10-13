//! Synchronous chart ownership and the reusable Iced component.
mod chart;
mod defaults;
mod markers;
pub use chart::{Chart, ChartError};
mod interaction;
mod widget;
pub use interaction::{ChartEvent, ChartMessage, PointerEvent};
pub use widget::{ChartProgram, ChartState};

pub use crate::model::price_line::{PriceLineId, PriceLineOptions};
pub use crate::model::series_markers::{
    MarkerCollectionId, MarkerHit, MarkerPosition, MarkerShape, MarkerZOrder, SeriesMarker,
    SeriesMarkersOptions,
};

pub use crate::model::hit_test::{ChartHit, HitTarget, PriceLineKind};

pub(crate) mod primitives;
pub use primitives::{
    DrawingPrimitive, PreparedPrimitive, PrimitiveAxisLabel, PrimitiveContext, PrimitiveHit,
    PrimitiveHitCandidate, PrimitiveHitTester, PrimitiveId, PrimitiveLayer, PrimitiveOwner,
    PrimitivePaneId, PrimitiveRenderer, PrimitiveView,
};
