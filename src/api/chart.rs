use crate::model::{
    chart_model::{ChartModel, ChartModelError, ChartModelOptions},
    data_consumer::{BuiltInSeriesDataItem, TimedData},
    data_layer::{DataLayer, DataLayerError, SeriesId},
    get_series_plot_row_creator::create_builtin_plot_row,
    horz_scale_behavior_time::{
        horz_scale_behavior_time::HorzScaleBehaviorTime,
        time_utils::{TimeConversionError, select_time_converter},
        types::Time,
    },
    localization_options::LocalizationOptions,
    pane::PriceScalePosition,
    plot_data::MutablePlotRow,
    series_options::{SeriesOptionsMap, SeriesType},
    time_data::TimePointIndex,
    time_scale::TimeScale,
    time_scale_options::HorzScaleOptions,
};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Debug)]
pub enum ChartError {
    Model(ChartModelError),
    Data(DataLayerError<TimeConversionError>),
    Time(TimeConversionError),
    UnknownSeries(SeriesId),
    UnsupportedSeries,
    InvalidData { index: usize },
    UnorderedData { index: usize },
    IdExhausted,
}
impl std::fmt::Display for ChartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ChartError {}
impl From<ChartModelError> for ChartError {
    fn from(e: ChartModelError) -> Self {
        Self::Model(e)
    }
}
impl From<DataLayerError<TimeConversionError>> for ChartError {
    fn from(e: DataLayerError<TimeConversionError>) -> Self {
        Self::Data(e)
    }
}
static NEXT_SERIES: AtomicU64 = AtomicU64::new(1);

/// Owns data normalization and model mutation together. IDs are not reused.
/// Data operations are synchronous; prepare a frame after a batch of operations.
pub struct Chart {
    pub(crate) model: ChartModel<HorzScaleBehaviorTime>,
    layer: DataLayer<HorzScaleBehaviorTime>,
    kinds: BTreeMap<SeriesId, SeriesType>,
}
impl Default for Chart {
    fn default() -> Self {
        Self::new(ChartModelOptions::default())
    }
}
impl Chart {
    pub fn new(options: ChartModelOptions) -> Self {
        Self {
            model: ChartModel::new(
                TimeScale::new(
                    HorzScaleBehaviorTime::default(),
                    HorzScaleOptions::default(),
                    LocalizationOptions::new("en-US", "dd MMM 'yy"),
                ),
                options,
            ),
            layer: DataLayer::new(HorzScaleBehaviorTime::default()),
            kinds: BTreeMap::new(),
        }
    }
    /// Read-only engine inspection; data mutations must pass through this owner.
    pub fn model(&self) -> &ChartModel<HorzScaleBehaviorTime> {
        &self.model
    }
    pub fn add_series(
        &mut self,
        options: SeriesOptionsMap,
        pane: usize,
        position: PriceScalePosition,
    ) -> Result<SeriesId, ChartError> {
        let kind = match &options {
            SeriesOptionsMap::Line(_) => SeriesType::Line,
            SeriesOptionsMap::Candlestick(_) => SeriesType::Candlestick,
            SeriesOptionsMap::Histogram(_) => SeriesType::Histogram,
            _ => return Err(ChartError::UnsupportedSeries),
        };
        let id = NEXT_SERIES
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map(SeriesId::new)
            .map_err(|_| ChartError::IdExhausted)?;
        self.model
            .register_series(id, kind, options, pane, position)?;
        self.kinds.insert(id, kind);
        // Establish DataLayer membership so update() also works before set_data().
        let update = self.layer.set_series_data(id, kind, vec![])?;
        self.model.apply_data_update(update)?;
        Ok(id)
    }
    fn kind(&self, id: SeriesId) -> Result<SeriesType, ChartError> {
        self.kinds
            .get(&id)
            .copied()
            .ok_or(ChartError::UnknownSeries(id))
    }
    /// Replace all data. Times must be strictly increasing; errors preserve data.
    pub fn set_data(
        &mut self,
        id: SeriesId,
        data: Vec<BuiltInSeriesDataItem<Time>>,
    ) -> Result<(), ChartError> {
        let kind = self.kind(id)?;
        validate_data(kind, &data, true)?;
        let update = self.layer.set_series_data(id, kind, data)?;
        self.model.apply_data_update(update)?;
        Ok(())
    }
    /// Replace the latest bar or append. Historical updates require an existing time.
    pub fn update_data(
        &mut self,
        id: SeriesId,
        item: BuiltInSeriesDataItem<Time>,
        historical: bool,
    ) -> Result<(), ChartError> {
        validate_data(self.kind(id)?, std::slice::from_ref(&item), false)?;
        let update = self.layer.update_series_data(id, item, historical)?;
        self.model.apply_data_update(update)?;
        Ok(())
    }
    pub fn remove_series(&mut self, id: SeriesId) -> Result<(), ChartError> {
        self.kind(id)?;
        let update = self.layer.remove_series(id)?;
        self.model.remove_series(id, update)?;
        self.kinds.remove(&id);
        Ok(())
    }
    pub fn replace_series_options(
        &mut self,
        id: SeriesId,
        options: SeriesOptionsMap,
    ) -> Result<(), ChartError> {
        self.kind(id)?;
        self.model.replace_series_options(id, options)?;
        Ok(())
    }
    pub fn add_pane(&mut self) -> usize {
        self.model.add_pane()
    }
    pub fn apply_time_scale_options(
        &mut self,
        patch: crate::model::time_scale_options::HorzScaleOptionsPatch,
    ) {
        self.model.apply_time_scale_options(&mut self.layer, patch);
    }
}

// Validate the entire replacement before DataLayer removes previous memberships.
fn validate_data(
    kind: SeriesType,
    data: &[BuiltInSeriesDataItem<Time>],
    ordered: bool,
) -> Result<(), ChartError> {
    if data.is_empty() {
        return Ok(());
    }
    let converter = select_time_converter(data).map_err(ChartError::Time)?;
    let mut previous = None;
    for (index, item) in data.iter().enumerate() {
        let time = converter.convert(item.time()).map_err(ChartError::Time)?;
        let timestamp = time.timestamp.seconds();
        if ordered && previous.is_some_and(|p| timestamp <= p) {
            return Err(ChartError::UnorderedData { index });
        }
        previous = Some(timestamp);
        let row = create_builtin_plot_row(
            kind,
            time,
            TimePointIndex::default(),
            item.clone(),
            item.time().clone(),
        )
        .map_err(|e| ChartError::Data(DataLayerError::PlotRow(e)))?;
        if row.values().is_some_and(|values| {
            values
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 90_071_992_547_409.9)
        }) {
            return Err(ChartError::InvalidData { index });
        }
    }
    Ok(())
}
