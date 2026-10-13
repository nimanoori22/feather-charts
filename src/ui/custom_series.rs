//! Frozen custom renderers contain no live chart, model, or pane-view borrow.
use crate::model::{
    bar::BarPrice, coordinate::Coordinate, icustom_series::*, iprice_data_source::FirstValue,
    price_scale::PriceCoordinateTransform, range_impl::RangeImpl,
    series::line_pane_view::LinePaneView, time_data::TimePointIndex,
};
use iced::widget::canvas::Frame;
use std::rc::Rc;

pub(crate) struct CustomPreparation<'a> {
    pub visible: Option<RangeImpl<TimePointIndex>>,
    pub coordinate: &'a dyn Fn(TimePointIndex) -> Coordinate,
    pub bar_spacing: f64,
    pub pixel_ratio: f64,
    pub price: Option<PriceCoordinateTransform>,
}
pub(crate) trait CustomPreparationAdapter {
    fn hit_tolerance(&self) -> f64;
    fn data_at(&self, index: TimePointIndex) -> Option<crate::api::CustomSeriesEventData>;
    fn first_value(&self, range: Option<&RangeImpl<TimePointIndex>>) -> Option<FirstValue>;
    fn prepare(&mut self, context: CustomPreparation<'_>) -> Option<PreparedCustomSeries>;
}
/// Opaque application pane view owned by chart composition.
pub struct CustomPaneView {
    pub(crate) adapter: Box<dyn CustomPreparationAdapter>,
    pub(crate) empty: LinePaneView,
}
impl CustomPaneView {
    pub(crate) fn new(adapter: impl CustomPreparationAdapter + 'static) -> Self {
        Self {
            adapter: Box::new(adapter),
            empty: Default::default(),
        }
    }
}

use crate::model::{
    hit_test::HitTarget,
    internal_hit_test::{HitTestPriority, InternalHitTestCandidate},
};
pub(crate) struct CustomRendererHit {
    pub target: HitTarget,
    pub candidate: InternalHitTestCandidate,
    pub data: Option<crate::api::CustomHitData>,
}
trait FrozenRenderer {
    fn draw(&self, frame: &mut Frame, hovered: bool, data: Option<&crate::api::CustomHitData>);
    fn hit(&self, x: f64, y: f64) -> Option<CustomRendererHit>;
}
struct Frozen<R> {
    renderer: R,
    price: PriceCoordinateTransform,
}
impl<R> FrozenRenderer for Frozen<R>
where
    R: CustomSeriesPaneRenderer<Frame>,
    R::HitTestData: 'static,
{
    fn draw(&self, frame: &mut Frame, hovered: bool, data: Option<&crate::api::CustomHitData>) {
        let converter = |price: BarPrice| {
            let y = self.price.price_to_coordinate(price.value());
            y.value().is_finite().then_some(y)
        };
        self.renderer.draw(
            frame,
            &converter,
            hovered,
            data.and_then(|data| data.0.downcast_ref()),
        );
    }
    fn hit(&self, x: f64, y: f64) -> Option<CustomRendererHit> {
        let converter = |price: BarPrice| {
            let y = self.price.price_to_coordinate(price.value());
            y.value().is_finite().then_some(y)
        };
        let hit = self.renderer.hit_test(x.into(), y.into(), &converter)?;
        let priority = match hit.hit_type {
            CustomSeriesHitTestType::Point => HitTestPriority::Point,
            CustomSeriesHitTestType::Range => HitTestPriority::Range,
            CustomSeriesHitTestType::Line | CustomSeriesHitTestType::Custom => {
                HitTestPriority::Line
            }
        };
        Some(CustomRendererHit {
            target: HitTarget::CustomSeries {
                hit_type: hit.hit_type,
                object_id: hit.object_id.clone(),
                cursor_style: hit.cursor_style.clone(),
            },
            candidate: InternalHitTestCandidate {
                distance: hit.distance,
                priority,
                cursor_style: hit.cursor_style,
                external_id: hit.object_id,
            },
            data: hit
                .hit_test_data
                .map(|data| crate::api::CustomHitData(Rc::new(data))),
        })
    }
}
/// Immutable custom geometry and copied owner-price conversion.
#[derive(Clone)]
pub struct PreparedCustomSeries {
    renderer: Rc<dyn FrozenRenderer>,
    fallback: Vec<crate::renderers::range_hit_test::RangePoint>,
    spacing: f64,
}
impl std::fmt::Debug for PreparedCustomSeries {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedCustomSeries")
            .finish_non_exhaustive()
    }
}
impl PreparedCustomSeries {
    pub(crate) fn new<R>(
        renderer: R,
        price: PriceCoordinateTransform,
        fallback: Vec<crate::renderers::range_hit_test::RangePoint>,
        spacing: f64,
    ) -> Self
    where
        R: CustomSeriesPaneRenderer<Frame> + 'static,
        R::HitTestData: 'static,
    {
        Self {
            renderer: Rc::new(Frozen { renderer, price }),
            fallback,
            spacing,
        }
    }
    pub(crate) fn draw(
        &self,
        frame: &mut Frame,
        hovered: bool,
        data: Option<&crate::api::CustomHitData>,
    ) {
        self.renderer.draw(frame, hovered, data);
    }
    pub(crate) fn hit(&self, x: f64, y: f64, tolerance: f64) -> Option<CustomRendererHit> {
        if let Some(hit) = self.renderer.hit(x, y) {
            return (hit.candidate.distance.is_finite() && hit.candidate.distance >= 0.)
                .then_some(hit);
        }
        let distance = crate::renderers::range_hit_test::range_hit(
            &self.fallback,
            x,
            y,
            self.spacing,
            tolerance,
        )?;
        Some(CustomRendererHit {
            target: HitTarget::SeriesLine,
            data: None,
            candidate: InternalHitTestCandidate {
                distance,
                priority: HitTestPriority::Line,
                cursor_style: None,
                external_id: None,
            },
        })
    }
}
