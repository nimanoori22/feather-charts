//! The supported public chart families share one synchronous owner.
use super::{Chart, ChartError};
use crate::model::{
    data_consumer::TimedData,
    data_layer::DataLayerError,
    horz_scale_behavior_time::{
        horz_scale_behavior_time::HorzScaleBehaviorTime,
        time_utils::{TimeConversionError, select_time_converter},
    },
    ihorz_scale_behavior::{HorzScaleBehavior, HorzScaleItemConverter},
};
use std::{fmt::Debug, hash::Hash};

/// Adapter used by the chart owner; custom series retain their existing API.
pub trait ChartBehavior:
    HorzScaleBehavior<
        Item: Debug + PartialEq + 'static,
        InternalItem: Debug + 'static,
        Key: Ord + Debug,
        CacheKey: Eq + Hash,
        Converter: HorzScaleItemConverter<Self::Item, Self::InternalItem, Error = Self::Error>,
    > + 'static
{
    fn conversion_error(error: Self::Error) -> ChartError;
    fn data_error(error: DataLayerError<Self::Error>) -> ChartError;
    fn validation_converter<D: TimedData<Item = Self::Item>>(
        &self,
        data: &[D],
    ) -> Result<Self::Converter, Self::Error>;
}
impl ChartBehavior for HorzScaleBehaviorTime {
    fn conversion_error(error: TimeConversionError) -> ChartError {
        ChartError::Time(error)
    }
    fn data_error(error: DataLayerError<TimeConversionError>) -> ChartError {
        ChartError::Data(error)
    }
    fn validation_converter<D: TimedData<Item = Self::Item>>(
        &self,
        data: &[D],
    ) -> Result<Self::Converter, Self::Error> {
        select_time_converter(data)
    }
}

impl ChartBehavior for crate::model::yield_curve_horz_scale_behavior::YieldCurveHorzScaleBehavior {
    fn conversion_error(error: Self::Error) -> ChartError {
        ChartError::YieldCurve(error)
    }
    fn data_error(error: DataLayerError<Self::Error>) -> ChartError {
        ChartError::YieldData(error)
    }
    fn validation_converter<D: TimedData<Item = Self::Item>>(
        &self,
        data: &[D],
    ) -> Result<Self::Converter, Self::Error> {
        for row in data {
            self.options().scaffold(row.time().months())?;
        }
        Ok(crate::model::yield_curve_horz_scale_behavior::MaturityConverter)
    }
}

impl<B: ChartBehavior, M: Clone + 'static> Chart<B, M> {
    /// Creates a chart with an application-defined horizontal domain.
    ///
    /// On valid construction, calls the factory twice, then drops it. Each behavior must independently
    /// own equivalent options/conversion/key policies and mutable caches. Neither
    /// B nor its options needs Clone. Keys must be stable and totally ordered;
    /// cache keys must identify the same internal point. Domains use logical
    /// indices, not continuous numeric distance. Existing constructors are unchanged.
    pub fn with_custom_behavior_and_metadata(
        mut make_behavior: impl FnMut() -> B,
        chart_options: crate::model::chart_model::ChartModelOptions,
        scale_options: crate::model::time_scale_options::HorzScaleOptions,
        localization: crate::model::localization_options::LocalizationOptions<B::Item>,
    ) -> Result<Self, ChartError> {
        chart_options.validate_color_space()?;
        if !crate::model::chart_configuration::valid_horizontal_options(&scale_options) {
            return Err(
                crate::model::chart_model::ChartModelError::InvalidChartOptions("horizontal scale")
                    .into(),
            );
        }
        let mut data = make_behavior();
        data.update_scale_options(&scale_options);
        data.update_formatter(&localization);
        let chart = Self::with_behavior(
            chart_options,
            data,
            make_behavior(),
            scale_options,
            localization,
        );
        chart.model.preview_runtime_options(&Default::default())?;
        Ok(chart)
    }
    pub fn horizontal_behavior_options(&self) -> &B::Options {
        self.layer.behavior().options()
    }
    /// Builds equivalent application options twice without requiring Clone.
    /// Formatting/cache changes apply immediately. To change key interpretation
    /// or stored weighting rules, clear all shared series before configuration
    /// and reload them afterward, or create a new chart. The option factory is not retained. This does not cancel gestures.
    pub fn configure_horizontal_behavior(&mut self, make_options: impl FnMut() -> B::Options) {
        self.model
            .configure_horizontal_behavior(&mut self.layer, make_options);
        self.revision = self.revision.wrapping_add(1);
    }
}

impl<B: ChartBehavior> Chart<B> {
    pub fn with_custom_behavior(
        make_behavior: impl FnMut() -> B,
        chart_options: crate::model::chart_model::ChartModelOptions,
        scale_options: crate::model::time_scale_options::HorzScaleOptions,
        localization: crate::model::localization_options::LocalizationOptions<B::Item>,
    ) -> Result<Self, ChartError> {
        Self::with_custom_behavior_and_metadata(
            make_behavior,
            chart_options,
            scale_options,
            localization,
        )
    }
}
