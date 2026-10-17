use feather_charts::model::{data_consumer::TimedData, icustom_series::*};
use iced::{
    Color, Point, Size,
    widget::canvas::{Frame, Path},
};
use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    rc::Rc,
};

#[derive(Debug, PartialEq)]
pub struct Datum<T> {
    pub time: T,
    pub low: f64,
    pub high: f64,
    pub value: f64,
    pub clones: Option<Rc<Cell<usize>>>,
}
impl<T: Clone> Clone for Datum<T> {
    fn clone(&self) -> Self {
        if let Some(c) = &self.clones {
            c.set(c.get() + 1);
        }
        Self {
            time: self.time.clone(),
            low: self.low,
            high: self.high,
            value: self.value,
            clones: self.clones.clone(),
        }
    }
}
impl<T: Clone> TimedData for Datum<T> {
    type Item = T;
    fn time(&self) -> &T {
        &self.time
    }
    fn time_mut(&mut self) -> &mut T {
        &mut self.time
    }
}
impl<T: Clone> CustomData for Datum<T> {}
pub fn datum<T: Clone>(time: T, value: f64) -> CustomSeriesDataItem<Datum<T>, T> {
    CustomSeriesDataItem::Data(Datum {
        time,
        low: value - 1.,
        high: value + 1.,
        value,
        clones: None,
    })
}
#[derive(Default)]
pub struct Renderer<T> {
    pub data: Option<Rc<PaneRendererCustomData<Datum<T>>>>,
    pub hover: bool,
}
impl<T> Clone for Renderer<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            hover: self.hover,
        }
    }
}
impl<T> CustomSeriesPaneRenderer<Frame> for Renderer<T> {
    type HitTestData = f64;
    fn draw(
        &self,
        f: &mut Frame,
        convert: &PriceToCoordinateConverter<'_>,
        _: bool,
        _: Option<&f64>,
    ) {
        let Some(d) = &self.data else { return };
        let Some(r) = &d.visible_range else { return };
        for bar in &d.bars[r.from..r.to] {
            if let Some((top, bottom)) =
                convert(bar.original_data.high.into()).zip(convert(bar.original_data.low.into()))
            {
                let p = Path::rectangle(
                    Point::new(
                        bar.x.value() as f32 - 2.,
                        top.value().min(bottom.value()) as f32,
                    ),
                    Size::new(4., (bottom.value() - top.value()).abs() as f32),
                );
                f.fill(&p, Color::from_rgba(0.5, 0.2, 0.8, 0.5));
            }
        }
    }
    fn hit_test(
        &self,
        x: feather_charts::model::coordinate::Coordinate,
        y: feather_charts::model::coordinate::Coordinate,
        convert: &PriceToCoordinateConverter<'_>,
    ) -> Option<CustomSeriesHitTestResult<f64>> {
        if !self.hover {
            return None;
        }
        self.data.as_ref()?.bars.iter().find_map(|bar| {
            let (top, bottom) = convert(bar.original_data.high.into())?
                .value()
                .minmax(convert(bar.original_data.low.into())?.value());
            ((bar.x.value() - x.value()).abs() <= 3. && y.value() >= top && y.value() <= bottom)
                .then(|| CustomSeriesHitTestResult {
                    distance: 0.,
                    object_id: Some("numeric-range".into()),
                    hit_type: CustomSeriesHitTestType::Range,
                    cursor_style: None,
                    hit_test_data: Some(bar.original_data.value),
                })
        })
    }
}
trait MinMax {
    fn minmax(self, b: f64) -> (f64, f64);
}
impl MinMax for f64 {
    fn minmax(self, b: f64) -> (f64, f64) {
        (self.min(b), self.max(b))
    }
}
type ObservedUpdates = Rc<RefCell<Vec<(f64, Vec<f64>)>>>;
pub struct View<T, I> {
    pub observed: ObservedUpdates,
    pub renderer: Renderer<T>,
    pub destroyed: Rc<Cell<usize>>,
    pub reductions: Rc<Cell<usize>>,
    pub seen: Rc<RefCell<Vec<(T, I)>>>,
    pub reduce: bool,
    pub order_sensitive: bool,
    pub count_during_reduce: bool,
    pub marker: PhantomData<I>,
}
impl<T, I> Default for View<T, I> {
    fn default() -> Self {
        Self {
            observed: Default::default(),
            renderer: Renderer {
                data: None,
                hover: true,
            },
            destroyed: Default::default(),
            reductions: Default::default(),
            seen: Default::default(),
            reduce: true,
            order_sensitive: false,
            count_during_reduce: false,
            marker: PhantomData,
        }
    }
}
impl<T: Clone, I: Clone> CustomSeriesPaneView<Frame> for View<T, I> {
    type Item = T;
    type InternalTime = I;
    type Data = Datum<T>;
    type Metadata = ();
    type Options = bool;
    type Renderer = Renderer<T>;
    fn renderer(&self) -> &Self::Renderer {
        &self.renderer
    }
    fn update(&mut self, d: PaneRendererCustomData<Datum<T>>, hover: &bool) {
        self.observed.borrow_mut().push((
            d.conflation_factor,
            d.bars.iter().map(|b| b.original_data.value).collect(),
        ));
        self.renderer.data = Some(Rc::new(d));
        self.renderer.hover = *hover;
    }
    fn price_value_builder(&self, d: &Datum<T>) -> Vec<feather_charts::model::bar::BarPrice> {
        vec![d.low.into(), d.high.into(), d.value.into()]
    }
    fn is_whitespace(&self, d: &CustomSeriesDataItem<Datum<T>, T>) -> bool {
        matches!(d, CustomSeriesDataItem::Whitespace(_))
    }
    fn default_options(&self) -> bool {
        true
    }
    fn destroy(&mut self) {
        self.destroyed.set(self.destroyed.get() + 1);
    }
    fn conflate(
        &self,
        a: &CustomConflationContext<Datum<T>, T, I>,
        b: &CustomConflationContext<Datum<T>, T, I>,
    ) -> Option<Datum<T>> {
        self.reductions.set(self.reductions.get() + 1);
        if self.count_during_reduce {
            self.seen
                .borrow_mut()
                .push((a.original_time.clone(), a.time.clone()));
        }
        self.reduce.then(|| Datum {
            time: a.original_time.clone(),
            low: a.data.low.min(b.data.low),
            high: a.data.high.max(b.data.high),
            value: if self.order_sensitive {
                a.data.value * 10. + b.data.value
            } else {
                (a.data.value + b.data.value) / 2.
            },
            clones: None,
        })
    }
}
