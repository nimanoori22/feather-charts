use crate::model::bar::BarPrice;
pub type PriceFormatterFn = Box<dyn Fn(BarPrice) -> String>;
pub type TickmarksPriceFormatterFn = Box<dyn Fn(&[BarPrice]) -> Vec<String>>;
