use super::*;
use crate::model::{
    autoscale_info_impl::{AutoScaleMargins, AutoscaleInfoImpl},
    iprice_data_source::FirstValue,
};
use std::rc::Weak;

struct Source {
    calls: Rc<Cell<usize>>,
    calculate: Box<dyn Fn(TimePointIndex, TimePointIndex) -> AutoscaleInfoImpl>,
}
impl PriceScaleDataSource for Source {
    fn z_order(&self) -> i32 {
        0
    }
    fn set_z_order(&mut self, _: i32) {}
    fn visible(&self) -> bool {
        true
    }
    fn first_value(&self) -> Option<FirstValue> {
        Some(FirstValue {
            value: 1.,
            time_point: 0.0.into(),
        })
    }
    fn format_price(&self, price: f64) -> String {
        price.to_string()
    }
    fn base(&self) -> f64 {
        100.
    }
    fn autoscale_info(
        &self,
        start: TimePointIndex,
        end: TimePointIndex,
    ) -> Option<AutoscaleInfoImpl> {
        self.calls.set(self.calls.get() + 1);
        Some((self.calculate)(start, end))
    }
    fn update_all_views(&mut self) {}
}
fn scale() -> PriceScale {
    let mut scale = PriceScale::new_with_font_size("right", PriceScaleOptions::default(), 12.);
    scale.set_height(200.);
    scale.set_price_range(Some(PriceRangeImpl::new(0., 10.)));
    scale
}
fn interval() -> RangeImpl<TimePointIndex> {
    RangeImpl::new(0.0.into(), 10.0.into())
}

#[test]
fn nested_range_read_uses_settled_value_without_reentering_provider() {
    let scale = Rc::new(RefCell::new(scale()));
    let weak: Weak<RefCell<PriceScale>> = Rc::downgrade(&scale);
    let calls = Rc::new(Cell::new(0));
    scale
        .borrow_mut()
        .add_data_source(Rc::new(RefCell::new(Source {
            calls: calls.clone(),
            calculate: Box::new(move |_, _| {
                assert_eq!(
                    weak.upgrade().unwrap().borrow().price_range(),
                    Some(PriceRangeImpl::new(0., 10.))
                );
                AutoscaleInfoImpl::new(Some(PriceRangeImpl::new(0., 100.)), None)
            }),
        })));
    scale.borrow_mut().invalidate_price_range(interval());
    assert_eq!(calls.get(), 0);
    assert_eq!(
        scale.borrow().price_range(),
        Some(PriceRangeImpl::new(0., 100.))
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn lazy_range_and_margins_invalidate_cached_marks_once() {
    let mut scale = scale();
    let calls = Rc::new(Cell::new(0));
    scale.add_data_source(Rc::new(RefCell::new(Source {
        calls: calls.clone(),
        calculate: Box::new(|_, _| {
            AutoscaleInfoImpl::new(
                Some(PriceRangeImpl::new(0., 100.)),
                Some(AutoScaleMargins {
                    above: 10.,
                    below: 20.,
                }),
            )
        }),
    })));
    let old = scale.marks().to_vec();
    let generation = scale.marks_generation();
    scale.invalidate_price_range(interval());
    assert_eq!(calls.get(), 0);
    assert!((scale.internal_height() - 110.).abs() < 1e-9);
    assert_eq!(scale.marks_generation(), generation + 1);
    assert_ne!(scale.marks(), old);
    assert_eq!(scale.marks_generation(), generation + 1);
    assert_eq!(calls.get(), 1);
}

#[test]
fn manual_transition_settles_pending_range_and_forced_calculation_remains_eager() {
    let mut scale = scale();
    let calls = Rc::new(Cell::new(0));
    let high = Rc::new(Cell::new(30.));
    let state = high.clone();
    scale.add_data_source(Rc::new(RefCell::new(Source {
        calls: calls.clone(),
        calculate: Box::new(move |_, _| {
            AutoscaleInfoImpl::new(Some(PriceRangeImpl::new(0., state.get())), None)
        }),
    })));
    scale.invalidate_price_range(interval());
    scale.set_auto_scale(false);
    assert_eq!(calls.get(), 1);
    assert_eq!(scale.price_range(), Some(PriceRangeImpl::new(0., 30.)));
    high.set(50.);
    scale.invalidate_price_range(interval());
    assert_eq!(scale.price_range(), Some(PriceRangeImpl::new(0., 30.)));
    assert_eq!(calls.get(), 1);
    assert!(scale.recalculate_price_range_forced(&interval()));
    assert_eq!(calls.get(), 2);
    assert_eq!(scale.price_range(), Some(PriceRangeImpl::new(0., 50.)));
}

#[test]
fn removal_and_drop_do_not_invoke_stale_pending_sources() {
    let mut scale = scale();
    let calls = Rc::new(Cell::new(0));
    let source: PriceSourceHandle = Rc::new(RefCell::new(Source {
        calls: calls.clone(),
        calculate: Box::new(|_, _| panic!("removed source queried")),
    }));
    scale.add_data_source(source.clone());
    scale.invalidate_price_range(interval());
    scale.remove_data_source(&source);
    assert_eq!(scale.price_range(), None);
    assert_eq!(calls.get(), 0);
    scale.add_data_source(source);
    scale.invalidate_price_range(interval());
    drop(scale);
    assert_eq!(calls.get(), 0);
}

#[test]
fn latest_viewport_wins_and_marks_invalidation_is_not_emitted_twice() {
    let observed = Rc::new(Cell::new((0., 0.)));
    let result = observed.clone();
    let mut scale = scale();
    let calls = Rc::new(Cell::new(0));
    scale.add_data_source(Rc::new(RefCell::new(Source {
        calls: calls.clone(),
        calculate: Box::new(move |start, end| {
            result.set((start.value(), end.value()));
            AutoscaleInfoImpl::new(Some(PriceRangeImpl::new(0., 100.)), None)
        }),
    })));
    scale.marks();
    let generation = scale.marks_generation();
    scale.invalidate_price_range(interval());
    scale.invalidate_price_range(RangeImpl::new(2.0.into(), 7.0.into()));

    scale.price_range();
    assert_eq!(observed.get(), (2., 7.));
    assert_eq!(scale.marks_generation(), generation + 1);
    scale.set_height(300.);
    assert_eq!(scale.marks_generation(), generation + 1);
    assert!(!scale.marks().is_empty());
    assert_eq!(calls.get(), 1);
}
