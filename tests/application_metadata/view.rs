use super::*;
use feather_charts::model::{coordinate::Coordinate, icustom_series::*};
use iced::widget::canvas::Frame;
use std::marker::PhantomData;
#[derive(Clone)]
pub struct Datum<T> {
    pub time: T,
    pub value: f64,
    pub payload: Payload,
}
impl<T> TimedData for Datum<T> {
    type Item = T;
    fn time(&self) -> &T {
        &self.time
    }
    fn time_mut(&mut self) -> &mut T {
        &mut self.time
    }
}
impl<T: Clone> CustomData for Datum<T> {}
pub struct Renderer<T> {
    data: Option<Rc<PaneRendererCustomData<Datum<T>>>>,
}
impl<T> Clone for Renderer<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}
impl<T> Default for Renderer<T> {
    fn default() -> Self {
        Self { data: None }
    }
}
impl<T> CustomSeriesPaneRenderer<Frame> for Renderer<T> {
    type HitTestData = ();
    fn draw(&self, _: &mut Frame, _: &PriceToCoordinateConverter<'_>, _: bool, _: Option<&()>) {}
    fn hit_test(
        &self,
        _: Coordinate,
        _: Coordinate,
        _: &PriceToCoordinateConverter<'_>,
    ) -> Option<CustomSeriesHitTestResult<()>> {
        None
    }
}
// The live view deliberately has no Clone implementation.
pub struct View<T, I> {
    renderer: Renderer<T>,
    marker: PhantomData<I>,
}
impl<T, I> Default for View<T, I> {
    fn default() -> Self {
        Self {
            renderer: Default::default(),
            marker: PhantomData,
        }
    }
}
impl<T: Clone, I: Clone> CustomSeriesPaneView<Frame> for View<T, I> {
    type Item = T;
    type InternalTime = I;
    type Data = Datum<T>;
    type Metadata = Payload;
    type Options = ();
    type Renderer = Renderer<T>;
    fn renderer(&self) -> &Self::Renderer {
        &self.renderer
    }
    fn update(&mut self, data: PaneRendererCustomData<Datum<T>>, _: &()) {
        self.renderer.data = Some(Rc::new(data));
    }
    fn price_value_builder(&self, d: &Datum<T>) -> CustomSeriesPricePlotValues {
        vec![d.value.into()]
    }
    fn is_whitespace(&self, d: &CustomSeriesDataItem<Datum<T>, T, Payload>) -> bool {
        matches!(d, CustomSeriesDataItem::Whitespace(_))
    }
    fn default_options(&self) {}
    fn conflate(
        &self,
        a: &CustomConflationContext<Datum<T>, T, I>,
        b: &CustomConflationContext<Datum<T>, T, I>,
    ) -> Option<Datum<T>> {
        Some(Datum {
            time: a.original_time.clone(),
            value: b.data.value,
            payload: b.data.payload.clone(),
        })
    }
}
