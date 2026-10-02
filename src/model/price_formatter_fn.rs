use crate::model::bar::BarPrice;
use std::rc::Rc;

pub type PriceFormatterFn = Rc<dyn Fn(BarPrice) -> String>;
pub type TickmarksPriceFormatterFn = Rc<dyn Fn(&[BarPrice]) -> Vec<String>>;
pub type PercentageFormatterFn = Rc<dyn Fn(f64) -> String>;
pub type TickmarksPercentageFormatterFn = Rc<dyn Fn(&[f64]) -> Vec<String>>;
