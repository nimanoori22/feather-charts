//! The supported public chart families share one synchronous owner.
use super::ChartError;
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
