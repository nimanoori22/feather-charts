use crate::model::bar::BarPrice;
use std::rc::Rc;

pub type PriceFormatterFn = Rc<dyn Fn(BarPrice) -> String>;
/// Must return exactly one label per input value; malformed callback output panics.
pub type TickmarksPriceFormatterFn = Rc<dyn Fn(&[BarPrice]) -> Vec<String>>;
pub type PercentageFormatterFn = Rc<dyn Fn(f64) -> String>;
/// Must return exactly one label per input value; malformed callback output panics.
pub type TickmarksPercentageFormatterFn = Rc<dyn Fn(&[f64]) -> Vec<String>>;

pub(crate) fn checked_tickmarks(expected: usize, labels: Vec<String>) -> Vec<String> {
    assert_eq!(
        labels.len(),
        expected,
        "tickmarks formatter must return one label per input value"
    );
    labels
}
