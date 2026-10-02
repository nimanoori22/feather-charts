//! Reusable state for model data sources.
use crate::model::price_scale::PriceScale;
use std::{cell::RefCell, rc::Rc};
pub type PriceScaleHandle = Rc<RefCell<PriceScale>>;
#[derive(Clone, Default)]
pub struct DataSourceState {
    price_scale: Option<PriceScaleHandle>,
    z_order: i32,
}
impl DataSourceState {
    pub fn z_order(&self) -> i32 {
        self.z_order
    }
    pub fn set_z_order(&mut self, value: i32) {
        self.z_order = value
    }
    pub fn price_scale(&self) -> Option<PriceScaleHandle> {
        self.price_scale.clone()
    }
    pub fn set_price_scale(&mut self, scale: Option<PriceScaleHandle>) {
        self.price_scale = scale
    }
    pub const fn visible(&self) -> bool {
        true
    }
}
