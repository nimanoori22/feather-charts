use crate::model::bar::BarPrice;
pub type PriceFormatterFn = Box<dyn Fn(BarPrice) -> String>;
pub type TickmarksPriceFormatterFn = Box<dyn Fn(&[BarPrice]) -> Vec<String>>;
pub type PercentageFormatterFn = Box<dyn Fn(f64) -> String>;
pub type TickmarksPercentageFormatterFn = Box<dyn Fn(&[f64]) -> Vec<String>>;
