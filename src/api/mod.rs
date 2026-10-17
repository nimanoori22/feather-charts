//! Synchronous chart ownership and the reusable Iced component.
mod behavior;
mod chart;
pub use behavior::ChartBehavior;
mod conflation;
mod custom_series;
pub use custom_series::{CustomHitData, CustomSeriesEventData, CustomSeriesHandle};
mod defaults;
mod markers;
pub use chart::{Chart, ChartError};
mod interaction;
mod primitive_edit_input;
mod primitive_editing;
pub use primitive_editing::{
    PrimitiveEditCancelReason, PrimitiveEditEvent, PrimitiveEditId, PrimitiveEditPhase,
};
mod touch;
mod touch_input;
mod widget;
pub use interaction::{ChartEvent, ChartMessage, PointerEvent};
pub use touch::{
    KineticScrollOptions, TouchAction, TouchInteraction, TouchNavigationOptions, TrackingExitMode,
};
pub use widget::{ChartProgram, ChartState};

pub use crate::model::price_line::{PriceLineId, PriceLineOptions};
pub use crate::model::series_markers::{
    MarkerCollectionId, MarkerHit, MarkerPosition, MarkerShape, MarkerZOrder, SeriesMarker,
    SeriesMarkersOptions,
};

pub use crate::model::hit_test::{ChartHit, HitTarget, PriceLineKind};

pub(crate) mod primitives;
pub use primitives::{
    DrawingPrimitive, PreparedPrimitive, PrimitiveAxisLabel, PrimitiveContext,
    PrimitiveCoordinateTransform, PrimitiveHit, PrimitiveHitCandidate, PrimitiveHitTester,
    PrimitiveId, PrimitiveLayer, PrimitiveOwner, PrimitivePaneId, PrimitiveRenderer, PrimitiveView,
};

mod text_watermark;
pub use text_watermark::{
    TextWatermark, TextWatermarkError, TextWatermarkLine, TextWatermarkOptions,
};

mod image_watermark;
pub use image_watermark::{ImageWatermark, ImageWatermarkError, ImageWatermarkOptions};

mod up_down_markers;
pub use up_down_markers::{MarkerSign, UpDownMarker, UpDownMarkersId, UpDownMarkersOptions};

mod yield_curve;
pub use crate::model::yield_curve_horz_scale_behavior::{
    Maturity, YieldCurveError, YieldCurveOptions,
};
pub use yield_curve::YieldCurveChart;
