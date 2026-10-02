use crate::{
    formatters::iprice_formatter::PriceValueFormatter,
    model::{autoscale_info_impl::AutoscaleInfoImpl, time_data::TimePointIndex},
};
/// Narrow, synchronous source contract required by `PriceScale`.
pub trait PriceScaleDataSource {
    fn z_order(&self) -> i32;
    fn visible(&self) -> bool;
    fn first_value(&self) -> Option<FirstValue>;
    fn formatter(&self) -> &dyn PriceValueFormatter;
    fn base(&self) -> f64;
    fn autoscale_info(
        &self,
        start: TimePointIndex,
        end: TimePointIndex,
    ) -> Option<AutoscaleInfoImpl>;
    fn update_all_views(&mut self);
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FirstValue {
    pub value: f64,
    pub time_point: TimePointIndex,
}
