use crate::model::{
    data_source::PriceScaleHandle, idata_source::IDataSource, text_width_cache::TextMeasurer,
};
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

pub trait IPriceDataSource<Target: TextMeasurer>:
    IDataSource<Target> + PriceScaleDataSource
{
    fn price_line_color(&self, last_bar_color: &str) -> String;
    fn model_id(&self) -> &str;
    fn attached_price_scale(&self) -> Option<PriceScaleHandle> {
        self.price_scale()
    }
}
