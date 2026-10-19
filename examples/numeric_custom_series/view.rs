//! Example-owned interval renderer; engine contracts remain backend independent.
use feather_charts::model::{coordinate::Coordinate, data_consumer::TimedData, icustom_series::*};
use iced::{
    Color, Point, Size,
    widget::canvas::{Frame, Path},
};
use std::rc::Rc;
#[derive(Clone, Debug)]
pub struct Datum<T> {
    pub time: T,
    pub low: f64,
    pub high: f64,
    pub value: f64,
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
#[derive(Clone, Copy)]
pub struct Options {
    pub color: Color,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            color: Color::from_rgba(0.55, 0.2, 0.75, 0.55),
        }
    }
}
pub struct Renderer<T> {
    data: Option<Rc<PaneRendererCustomData<Datum<T>>>>,
    options: Options,
}
impl<T> Clone for Renderer<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            options: self.options,
        }
    }
}
impl<T> Default for Renderer<T> {
    fn default() -> Self {
        Self {
            data: None,
            options: Default::default(),
        }
    }
}
impl<T> CustomSeriesPaneRenderer<Frame> for Renderer<T> {
    type HitTestData = f64;
    fn draw(
        &self,
        f: &mut Frame,
        convert: &PriceToCoordinateConverter<'_>,
        hovered: bool,
        hit: Option<&f64>,
    ) {
        let Some(data) = &self.data else { return };
        let Some(range) = &data.visible_range else {
            return;
        };
        let width = (data.bar_spacing * data.conflation_factor * 0.6).clamp(2., 30.) as f32;
        for bar in &data.bars[range.from..range.to] {
            let d = &bar.original_data;
            if let Some(((top, bottom), middle)) = convert(d.high.into())
                .zip(convert(d.low.into()))
                .zip(convert(d.value.into()))
            {
                let x = bar.x.value() as f32;
                let y = top.value().min(bottom.value()) as f32;
                let height = (top.value() - bottom.value()).abs() as f32;
                f.fill_rectangle(
                    Point::new(x - width / 2., y),
                    Size::new(width, height),
                    self.options.color,
                );
                f.fill_rectangle(
                    Point::new(x - width / 2., middle.value() as f32 - 1.),
                    Size::new(width, 2.),
                    Color::from_rgb(0.25, 0.1, 0.4),
                );
                if hovered && hit == Some(&d.value) {
                    f.fill(
                        &Path::circle(Point::new(x, middle.value() as f32), 4.),
                        Color::from_rgb(0.95, 0.4, 0.1),
                    );
                }
            }
        }
    }
    fn hit_test(
        &self,
        x: Coordinate,
        y: Coordinate,
        convert: &PriceToCoordinateConverter<'_>,
    ) -> Option<CustomSeriesHitTestResult<f64>> {
        let data = self.data.as_ref()?;
        let range = data.visible_range.as_ref()?;
        let half = (data.bar_spacing * data.conflation_factor * 0.3).clamp(2., 15.);
        data.bars[range.from..range.to].iter().find_map(|bar| {
            let top = convert(bar.original_data.high.into())?.value();
            let bottom = convert(bar.original_data.low.into())?.value();
            ((x.value() - bar.x.value()).abs() <= half
                && y.value() >= top.min(bottom)
                && y.value() <= top.max(bottom))
            .then(|| CustomSeriesHitTestResult {
                distance: 0.,
                object_id: Some("interval".into()),
                hit_type: CustomSeriesHitTestType::Range,
                cursor_style: Some("crosshair".into()),
                hit_test_data: Some(bar.original_data.value),
            })
        })
    }
}
pub struct View<T, I = T, M = ()> {
    renderer: Renderer<T>,
    internal: std::marker::PhantomData<(I, M)>,
}
impl<T, I, M> Default for View<T, I, M> {
    fn default() -> Self {
        Self {
            renderer: Default::default(),
            internal: std::marker::PhantomData,
        }
    }
}
impl<T: Clone, I: Clone, M> CustomSeriesPaneView<Frame> for View<T, I, M> {
    type Item = T;
    type InternalTime = I;
    type Data = Datum<T>;
    type Metadata = M;
    type Options = Options;
    type Renderer = Renderer<T>;
    fn renderer(&self) -> &Self::Renderer {
        &self.renderer
    }
    fn update(&mut self, data: PaneRendererCustomData<Datum<T>>, options: &Options) {
        self.renderer.data = Some(Rc::new(data));
        self.renderer.options = *options;
    }
    fn price_value_builder(&self, d: &Datum<T>) -> CustomSeriesPricePlotValues {
        [d.low, d.high, d.value].map(Into::into).to_vec()
    }
    fn is_whitespace(&self, d: &CustomSeriesDataItem<Datum<T>, T, M>) -> bool {
        matches!(d, CustomSeriesDataItem::Whitespace(_))
    }
    fn default_options(&self) -> Options {
        Default::default()
    }
    fn conflate(
        &self,
        a: &CustomConflationContext<Datum<T>, T, I>,
        b: &CustomConflationContext<Datum<T>, T, I>,
    ) -> Option<Datum<T>> {
        Some(Datum {
            time: a.original_time.clone(),
            low: a.data.low.min(b.data.low),
            high: a.data.high.max(b.data.high),
            value: b.data.value,
        })
    }
}
