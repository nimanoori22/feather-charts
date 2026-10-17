//! Price horizontal scales reuse the ordinary chart owner without support series.
use super::{Chart, ChartBehavior, ChartError};
use crate::model::{
    chart_model::ChartModelOptions,
    data_consumer::TimedData,
    data_layer::DataLayerError,
    horz_scale_behavior_price::{
        HorizontalPrice, HorzScaleBehaviorPrice, OptionsChartOptions, PriceConverter,
    },
    ihorz_scale_behavior::HorzScaleBehavior,
    localization_options::{LocalizationOptions, TimeFormatterFn},
    time_scale_options::HorzScaleOptions,
};

/// An ordinary chart whose horizontal items are finite prices or strikes.
pub type OptionsChart = Chart<HorzScaleBehaviorPrice>;
impl ChartBehavior for HorzScaleBehaviorPrice {
    fn conversion_error(error: Self::Error) -> ChartError {
        ChartError::OptionsChart(error)
    }
    fn data_error(error: DataLayerError<Self::Error>) -> ChartError {
        ChartError::OptionsData(error)
    }
    fn validation_converter<D: TimedData<Item = Self::Item>>(
        &self,
        _: &[D],
    ) -> Result<Self::Converter, Self::Error> {
        Ok(PriceConverter)
    }
}
impl Chart<HorzScaleBehaviorPrice> {
    /// Uses ordinary chart/axis defaults; omitted horizontal precision is zero.
    pub fn new_options(options: OptionsChartOptions) -> Result<Self, ChartError> {
        Self::with_options_chart_options(
            options,
            ChartModelOptions::default(),
            HorzScaleOptions::default(),
            LocalizationOptions::new("en-US", ""),
        )
    }
    /// Creates a price-based chart with explicit chart, scale and localization options.
    pub fn with_options_chart_options(
        options: OptionsChartOptions,
        chart_options: ChartModelOptions,
        scale_options: HorzScaleOptions,
        localization: LocalizationOptions<HorizontalPrice>,
    ) -> Result<Self, ChartError> {
        options.validate().map_err(ChartError::OptionsChart)?;
        let mut data = HorzScaleBehaviorPrice::default();
        data.set_options(options);
        let mut scale = HorzScaleBehaviorPrice::default();
        scale.set_options(options);
        Ok(Self::with_behavior(
            chart_options,
            data,
            scale,
            scale_options,
            localization,
        ))
    }
    pub fn options_chart_options(&self) -> OptionsChartOptions {
        *self.layer.behavior().options()
    }
    pub fn replace_options_chart_options(
        &mut self,
        options: OptionsChartOptions,
    ) -> Result<(), ChartError> {
        options.validate().map_err(ChartError::OptionsChart)?;
        if self.options_chart_options() != options {
            self.model
                .configure_horizontal_behavior(&mut self.layer, || options);
        }
        Ok(())
    }
    /// Overrides crosshair/item labels only; decimal axis ticks retain precision.
    pub fn set_horizontal_price_formatter(
        &mut self,
        formatter: Option<TimeFormatterFn<HorizontalPrice>>,
    ) {
        self.model.set_time_formatter(&mut self.layer, formatter);
    }
}
