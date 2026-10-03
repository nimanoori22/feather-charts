//! Synchronous normalization, indexing, and update fan-out for series data.

use crate::model::{
    data_consumer::{BuiltInSeriesDataItem, TimedData},
    get_series_plot_row_creator::{
        CustomPlotRowBuilder, PlotRowCreationError, create_builtin_plot_row, create_custom_plot_row,
    },
    icustom_series::{CustomData, CustomSeriesDataItem},
    ihorz_scale_behavior::{HorzScaleBehavior, HorzScaleItemConverter},
    plot_data::MutablePlotRow,
    series_data::{CustomPlotRow, SeriesDataRow, SeriesPlotRow},
    series_options::SeriesType,
    time_data::{TickMarkWeightValue, TimePointIndex, TimeScalePoint},
};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct SeriesId(u64);
impl SeriesId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SeriesUpdateInfo {
    pub historical_update: bool,
    pub last_bar_updated_or_new_bars_added_to_the_right: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeriesChanges<I, O, D = (), M = ()> {
    pub rows: Vec<SeriesPlotRow<I, O, D, M>>,
    pub info: Option<SeriesUpdateInfo>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimeScaleChanges<I, O> {
    pub points: Option<Vec<TimeScalePoint<I, O>>>,
    pub first_changed_point_index: Option<usize>,
    pub base_index: Option<TimePointIndex>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DataUpdateResponse<I, O, D = (), M = ()> {
    pub series: BTreeMap<SeriesId, SeriesChanges<I, O, D, M>>,
    pub custom: BTreeMap<SeriesId, CustomSeriesChanges<I, O>>,
    pub time_scale: TimeScaleChanges<I, O>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomSeriesChanges<I, O> {
    /// Fulfilled custom-row indexes in time order. The typed custom Series
    /// applies these to the rows it owns after a shared time-scale reindex.
    pub indices: Vec<TimePointIndex>,
    pub info: Option<SeriesUpdateInfo>,
    marker: PhantomData<fn() -> (I, O)>,
}

impl<I, O> CustomSeriesChanges<I, O> {
    pub fn new(indices: Vec<TimePointIndex>, info: Option<SeriesUpdateInfo>) -> Self {
        Self {
            indices,
            info,
            marker: PhantomData,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomDataUpdateResponse<I, O, D, M = ()> {
    /// Built-in rows reindexed by a custom-series change to the shared time axis.
    pub series: BTreeMap<SeriesId, SeriesChanges<I, O, (), M>>,
    /// Typed rows for the custom Series that initiated this operation.
    pub custom_rows: Vec<CustomPlotRow<I, O, D, M>>,
    /// Index updates for every affected custom Series, including the caller.
    pub custom: BTreeMap<SeriesId, CustomSeriesChanges<I, O>>,
    pub time_scale: TimeScaleChanges<I, O>,
    pub is_full_replacement: bool,
    pub changed_index: Option<TimePointIndex>,
}

/// The built-in-series response shape. Custom rows gain their own typed
/// adapter rather than forcing heterogeneous custom data into this core.
pub type BuiltInDataUpdate<I, O, M = ()> = DataUpdateResponse<I, O, (), M>;
pub type DataLayerResult<T, E> = Result<T, DataLayerError<E>>;
pub type BuiltInPopResult<I, O, M, E> =
    DataLayerResult<(Vec<SeriesPlotRow<I, O, (), M>>, BuiltInDataUpdate<I, O, M>), E>;
pub type CustomDataUpdate<I, O, D, M, E> = DataLayerResult<CustomDataUpdateResponse<I, O, D, M>, E>;
pub type CustomPopResult<I, O, M, E> =
    DataLayerResult<(Vec<TimePointIndex>, BuiltInDataUpdate<I, O, M>), E>;

/// Typed bridge owned by one future `CustomSeries<D>`. It owns no chart data:
/// its generic `D` crosses DataLayer only in method inputs and results, while
/// DataLayer stores the projection required for indexing and autoscaling.
pub struct CustomDataLayerAdapter<D, Builder> {
    series: SeriesId,
    builder: Builder,
    marker: PhantomData<fn() -> D>,
}

impl<D, Builder> CustomDataLayerAdapter<D, Builder> {
    pub fn new(series: SeriesId, builder: Builder) -> Self {
        Self {
            series,
            builder,
            marker: PhantomData,
        }
    }

    pub const fn series(&self) -> SeriesId {
        self.series
    }

    pub fn builder(&self) -> &Builder {
        &self.builder
    }

    pub fn set_data<B, M>(
        &self,
        layer: &mut DataLayer<B, M>,
        data: Vec<CustomSeriesDataItem<D, B::Item, M>>,
    ) -> CustomDataUpdate<B::InternalItem, B::Item, D, M, B::Error>
    where
        B: HorzScaleBehavior,
        B::Converter: HorzScaleItemConverter<B::Item, B::InternalItem, Error = B::Error>,
        B::InternalItem: Clone,
        B::Item: Clone,
        B::Key: PartialOrd,
        D: CustomData<Item = B::Item>,
        M: Clone,
        Builder: CustomPlotRowBuilder<D, B::Item, M>,
    {
        layer.set_custom_series_data(self.series, data, &self.builder)
    }

    pub fn update_data<B, M>(
        &self,
        layer: &mut DataLayer<B, M>,
        item: CustomSeriesDataItem<D, B::Item, M>,
        historical_update: bool,
    ) -> CustomDataUpdate<B::InternalItem, B::Item, D, M, B::Error>
    where
        B: HorzScaleBehavior,
        B::Converter: HorzScaleItemConverter<B::Item, B::InternalItem, Error = B::Error>,
        B::InternalItem: Clone,
        B::Item: Clone,
        B::Key: PartialOrd,
        D: CustomData<Item = B::Item>,
        M: Clone,
        Builder: CustomPlotRowBuilder<D, B::Item, M>,
    {
        layer.update_custom_series_data(self.series, item, &self.builder, historical_update)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct TimePointData<I, O, M> {
    index: TimePointIndex,
    time: I,
    original_time: O,
    weight: TickMarkWeightValue,
    rows: BTreeMap<SeriesId, SeriesDataRow<I, O, (), M>>,
    custom_rows: BTreeMap<SeriesId, CustomMembership<I, O, M>>,
}

/// The common projection a DataLayer needs for custom data. In particular it
/// never stores `D`; the typed Series retains that payload for its pane view.
#[derive(Clone, Debug, PartialEq)]
struct CustomMembership<I, O, M> {
    index: TimePointIndex,
    time: I,
    original_time: O,
    fulfilled: bool,
    values: Option<[f64; 4]>,
    custom_values: Option<M>,
}

fn custom_membership_from_row<I: Clone, O: Clone, D, M: Clone>(
    row: &SeriesDataRow<I, O, D, M>,
) -> CustomMembership<I, O, M> {
    match row {
        SeriesDataRow::Value(SeriesPlotRow::Custom(row)) => CustomMembership {
            index: row.base.index,
            time: row.base.time.clone(),
            original_time: row.base.original_time.clone(),
            fulfilled: true,
            values: Some(row.base.value),
            custom_values: row.base.custom_values.clone(),
        },
        SeriesDataRow::Whitespace(row) => CustomMembership {
            index: row.index,
            time: row.time.clone(),
            original_time: row.original_time.clone(),
            fulfilled: false,
            values: None,
            custom_values: row.custom_values.clone(),
        },
        SeriesDataRow::Value(_) => unreachable!("custom row creator only produces Custom rows"),
    }
}

#[derive(Debug)]
pub enum DataLayerError<E> {
    Behavior(E),
    UnknownSeries(SeriesId),
    OutOfOrderUpdate,
    HistoricalUpdateMissingPoint,
    PlotRow(PlotRowCreationError),
}

/// DataLayer owns normalized time points and per-series rows. A point owns its
/// member rows, so reindexing has one exclusive owner and needs no shared
/// mutable references or object identity.
pub struct DataLayer<B, M = ()>
where
    B: HorzScaleBehavior,
{
    behavior: B,
    points: Vec<TimePointData<B::InternalItem, B::Item, M>>,
    series_types: BTreeMap<SeriesId, SeriesType>,
    custom_series: BTreeSet<SeriesId>,
    last_time_by_series: BTreeMap<SeriesId, B::InternalItem>,
}

impl<B, M> DataLayer<B, M>
where
    B: HorzScaleBehavior,
    B::Converter: HorzScaleItemConverter<B::Item, B::InternalItem, Error = B::Error>,
    B::InternalItem: Clone,
    B::Item: Clone,
    B::Key: PartialOrd,
    M: Clone,
{
    pub fn new(behavior: B) -> Self {
        Self {
            behavior,
            points: vec![],
            series_types: BTreeMap::new(),
            custom_series: BTreeSet::new(),
            last_time_by_series: BTreeMap::new(),
        }
    }
    pub fn behavior(&self) -> &B {
        &self.behavior
    }
    pub fn behavior_mut(&mut self) -> &mut B {
        &mut self.behavior
    }

    pub fn set_series_data(
        &mut self,
        series: SeriesId,
        series_type: SeriesType,
        mut data: Vec<BuiltInSeriesDataItem<B::Item, M>>,
    ) -> DataLayerResult<BuiltInDataUpdate<B::InternalItem, B::Item, M>, B::Error> {
        let old_keys = self.keys();
        let previous_bounds = self.series_time_bounds(series);
        // A behavior is allowed to normalize the public input in place (for
        // example, a business-day string becomes a BusinessDay). Plot rows
        // must still retain the caller's original time for formatting/API use.
        let original_times = data
            .iter()
            .map(|item| item.time().clone())
            .collect::<Vec<_>>();
        self.remove_series_memberships(series);
        self.cleanup_empty_points();
        let mut last = None;
        if !data.is_empty() {
            self.behavior
                .preprocess_data(&mut data)
                .map_err(DataLayerError::Behavior)?;
            let mut converter = self
                .behavior
                .create_converter_to_internal(&data)
                .map_err(DataLayerError::Behavior)?;
            for (item, original_time) in data.into_iter().zip(original_times) {
                let internal_time = converter
                    .convert(item.time())
                    .map_err(DataLayerError::Behavior)?;
                let point_index = self.ensure_point(internal_time.clone(), original_time.clone());
                let row = create_builtin_plot_row(
                    series_type,
                    internal_time.clone(),
                    TimePointIndex::default(),
                    item,
                    original_time,
                )
                .map_err(DataLayerError::PlotRow)?;
                self.points[point_index].rows.insert(series, row);
                last = Some(internal_time);
            }
        }
        self.series_types.insert(series, series_type);
        if let Some(last) = last {
            self.last_time_by_series.insert(series, last);
        } else {
            self.last_time_by_series.remove(&series);
        }
        let info = self.series_update_info(previous_bounds, self.series_time_bounds(series));
        self.finish_mutation(old_keys, series, info)
    }

    pub fn set_custom_series_data<D, Builder>(
        &mut self,
        series: SeriesId,
        mut data: Vec<CustomSeriesDataItem<D, B::Item, M>>,
        builder: &Builder,
    ) -> CustomDataUpdate<B::InternalItem, B::Item, D, M, B::Error>
    where
        D: CustomData<Item = B::Item>,
        Builder: CustomPlotRowBuilder<D, B::Item, M>,
    {
        let old_keys = self.keys();
        let previous_bounds = self.series_time_bounds(series);
        let original_times = data
            .iter()
            .map(|item| item.time().clone())
            .collect::<Vec<_>>();
        self.remove_series_memberships(series);
        self.cleanup_empty_points();
        let mut custom_rows = vec![];
        let mut last = None;
        if !data.is_empty() {
            self.behavior
                .preprocess_data(&mut data)
                .map_err(DataLayerError::Behavior)?;
            let mut converter = self
                .behavior
                .create_converter_to_internal(&data)
                .map_err(DataLayerError::Behavior)?;
            for (item, original_time) in data.into_iter().zip(original_times) {
                let internal_time = converter
                    .convert(item.time())
                    .map_err(DataLayerError::Behavior)?;
                let point_index = self.ensure_point(internal_time.clone(), original_time.clone());
                let row = create_custom_plot_row(
                    internal_time.clone(),
                    TimePointIndex::default(),
                    item,
                    original_time,
                    builder,
                )
                .map_err(DataLayerError::PlotRow)?;
                let membership = custom_membership_from_row(&row);
                if let SeriesDataRow::Value(SeriesPlotRow::Custom(row)) = row {
                    custom_rows.push(row);
                }
                self.points[point_index]
                    .custom_rows
                    .insert(series, membership);
                last = Some(internal_time);
            }
        }
        self.custom_series.insert(series);
        if let Some(last) = last {
            self.last_time_by_series.insert(series, last);
        } else {
            self.last_time_by_series.remove(&series);
        }
        let info = self.series_update_info(previous_bounds, self.series_time_bounds(series));
        let response = self.finish_mutation(old_keys, series, info)?;
        self.sync_custom_row_indexes(&mut custom_rows);
        Ok(CustomDataUpdateResponse {
            series: response.series,
            custom_rows,
            custom: response.custom,
            time_scale: response.time_scale,
            is_full_replacement: true,
            changed_index: None,
        })
    }

    pub fn update_custom_series_data<D, Builder>(
        &mut self,
        series: SeriesId,
        mut item: CustomSeriesDataItem<D, B::Item, M>,
        builder: &Builder,
        historical_update: bool,
    ) -> CustomDataUpdate<B::InternalItem, B::Item, D, M, B::Error>
    where
        D: CustomData<Item = B::Item>,
        Builder: CustomPlotRowBuilder<D, B::Item, M>,
    {
        if !self.custom_series.contains(&series) {
            return Err(DataLayerError::UnknownSeries(series));
        }
        let original_time = item.time().clone();
        self.behavior
            .preprocess_data(std::slice::from_mut(&mut item))
            .map_err(DataLayerError::Behavior)?;
        let mut converter = self
            .behavior
            .create_converter_to_internal(std::slice::from_ref(&item))
            .map_err(DataLayerError::Behavior)?;
        let internal_time = converter
            .convert(item.time())
            .map_err(DataLayerError::Behavior)?;
        if !historical_update
            && self
                .last_time_by_series
                .get(&series)
                .is_some_and(|last| self.compare_time(&internal_time, last) == Ordering::Less)
        {
            return Err(DataLayerError::OutOfOrderUpdate);
        }
        let old_keys = self.keys();
        let point_index = self.find_point(&internal_time);
        if historical_update && point_index.is_none() {
            return Err(DataLayerError::HistoricalUpdateMissingPoint);
        }
        let affects_time_scale = point_index.is_none();
        let point_index = point_index
            .unwrap_or_else(|| self.ensure_point(internal_time.clone(), original_time.clone()));
        let row = create_custom_plot_row(
            internal_time.clone(),
            TimePointIndex::default(),
            item,
            original_time,
            builder,
        )
        .map_err(DataLayerError::PlotRow)?;
        let membership = custom_membership_from_row(&row);
        let fulfilled = membership.fulfilled;
        let mut custom_rows = match row {
            SeriesDataRow::Value(SeriesPlotRow::Custom(row)) => vec![row],
            SeriesDataRow::Whitespace(_) => vec![],
            SeriesDataRow::Value(_) => unreachable!("custom row creator only produces Custom rows"),
        };
        self.points[point_index]
            .custom_rows
            .insert(series, membership);
        if !historical_update {
            self.last_time_by_series.insert(series, internal_time);
        }
        let info = Some(SeriesUpdateInfo {
            historical_update,
            last_bar_updated_or_new_bars_added_to_the_right: fulfilled,
        });
        let response = if affects_time_scale {
            self.finish_mutation(old_keys, series, info)?
        } else {
            self.reindex();
            self.response_for(series, None, info)
        };
        self.sync_custom_row_indexes(&mut custom_rows);
        Ok(CustomDataUpdateResponse {
            series: response.series,
            custom_rows,
            custom: response.custom,
            time_scale: response.time_scale,
            is_full_replacement: false,
            changed_index: Some(self.points[point_index].index),
        })
    }

    pub fn remove_series(
        &mut self,
        series: SeriesId,
    ) -> DataLayerResult<BuiltInDataUpdate<B::InternalItem, B::Item, M>, B::Error> {
        if let Some(series_type) = self.series_types.get(&series).copied() {
            return self.set_series_data(series, series_type, vec![]);
        }
        if !self.custom_series.contains(&series) {
            return Ok(self.empty_response());
        }
        let old_keys = self.keys();
        self.remove_series_memberships(series);
        self.cleanup_empty_points();
        self.finish_mutation(old_keys, series, None)
    }

    pub fn update_series_data(
        &mut self,
        series: SeriesId,
        mut item: BuiltInSeriesDataItem<B::Item, M>,
        historical_update: bool,
    ) -> DataLayerResult<BuiltInDataUpdate<B::InternalItem, B::Item, M>, B::Error> {
        let series_type = *self
            .series_types
            .get(&series)
            .ok_or(DataLayerError::UnknownSeries(series))?;
        let original_time = item.time().clone();
        self.behavior
            .preprocess_data(std::slice::from_mut(&mut item))
            .map_err(DataLayerError::Behavior)?;
        let mut converter = self
            .behavior
            .create_converter_to_internal(std::slice::from_ref(&item))
            .map_err(DataLayerError::Behavior)?;
        let internal_time = converter
            .convert(item.time())
            .map_err(DataLayerError::Behavior)?;
        if !historical_update
            && self
                .last_time_by_series
                .get(&series)
                .is_some_and(|last| self.compare_time(&internal_time, last) == Ordering::Less)
        {
            return Err(DataLayerError::OutOfOrderUpdate);
        }
        let old_keys = self.keys();
        let point_index = self.find_point(&internal_time);
        if historical_update && point_index.is_none() {
            return Err(DataLayerError::HistoricalUpdateMissingPoint);
        }
        let affects_time_scale = point_index.is_none();
        let point_index = point_index
            .unwrap_or_else(|| self.ensure_point(internal_time.clone(), original_time.clone()));
        let row = create_builtin_plot_row(
            series_type,
            internal_time.clone(),
            TimePointIndex::default(),
            item,
            original_time,
        )
        .map_err(DataLayerError::PlotRow)?;
        let fulfilled = row.is_fulfilled();
        self.points[point_index].rows.insert(series, row);
        if !historical_update {
            self.last_time_by_series.insert(series, internal_time);
        }
        let info = SeriesUpdateInfo {
            historical_update,
            last_bar_updated_or_new_bars_added_to_the_right: fulfilled,
        };
        if affects_time_scale {
            self.finish_mutation(old_keys, series, Some(info))
        } else {
            self.reindex();
            Ok(self.response_for(series, None, Some(info)))
        }
    }

    pub fn pop_series_data(
        &mut self,
        series: SeriesId,
        count: usize,
    ) -> BuiltInPopResult<B::InternalItem, B::Item, M, B::Error> {
        if count == 0 || !self.series_types.contains_key(&series) {
            return Ok((vec![], self.empty_response()));
        }
        let old_keys = self.keys();
        let mut removed = vec![];
        for point in self.points.iter_mut().rev() {
            if removed.len() == count {
                break;
            }
            if point
                .rows
                .get(&series)
                .is_some_and(SeriesDataRow::is_fulfilled)
            {
                let row = point.rows.remove(&series).expect("checked membership");
                removed.push(row.into_value().expect("fulfilled row"));
            }
        }
        self.cleanup_empty_points();
        self.refresh_last_fulfilled_time(series);
        let response = self.finish_mutation(
            old_keys,
            series,
            Some(SeriesUpdateInfo {
                historical_update: false,
                last_bar_updated_or_new_bars_added_to_the_right: false,
            }),
        )?;
        Ok((removed, response))
    }

    /// Removes final fulfilled custom memberships. The typed `D` values are
    /// intentionally not present here: the owning `CustomSeries<D>` removes
    /// its matching rows after applying the returned indexes.
    pub fn pop_custom_series_data(
        &mut self,
        series: SeriesId,
        count: usize,
    ) -> CustomPopResult<B::InternalItem, B::Item, M, B::Error> {
        if count == 0 || !self.custom_series.contains(&series) {
            return Ok((vec![], self.empty_response()));
        }
        let old_keys = self.keys();
        let mut removed = vec![];
        for point in self.points.iter_mut().rev() {
            if removed.len() == count {
                break;
            }
            if point
                .custom_rows
                .get(&series)
                .is_some_and(|row| row.fulfilled)
            {
                point.custom_rows.remove(&series);
                removed.push(point.index);
            }
        }
        self.cleanup_empty_points();
        self.refresh_last_fulfilled_time(series);
        let response = self.finish_mutation(
            old_keys,
            series,
            Some(SeriesUpdateInfo {
                historical_update: false,
                last_bar_updated_or_new_bars_added_to_the_right: false,
            }),
        )?;
        Ok((removed, response))
    }

    fn ensure_point(&mut self, time: B::InternalItem, original_time: B::Item) -> usize {
        if let Some(index) = self.find_point(&time) {
            return index;
        }
        let index = self
            .points
            .partition_point(|point| self.compare_time(&point.time, &time) == Ordering::Less);
        self.points.insert(
            index,
            TimePointData {
                index: TimePointIndex::default(),
                time,
                original_time,
                weight: TickMarkWeightValue::default(),
                rows: BTreeMap::new(),
                custom_rows: BTreeMap::new(),
            },
        );
        index
    }
    fn find_point(&self, time: &B::InternalItem) -> Option<usize> {
        let index = self
            .points
            .partition_point(|point| self.compare_time(&point.time, time) == Ordering::Less);
        self.points
            .get(index)
            .filter(|point| self.compare_time(&point.time, time) == Ordering::Equal)
            .map(|_| index)
    }
    fn compare_time(&self, left: &B::InternalItem, right: &B::InternalItem) -> Ordering {
        self.behavior
            .key(left)
            .partial_cmp(&self.behavior.key(right))
            .unwrap_or(Ordering::Equal)
    }
    fn keys(&self) -> Vec<B::Key> {
        self.points
            .iter()
            .map(|point| self.behavior.key(&point.time))
            .collect()
    }
    fn remove_series_memberships(&mut self, series: SeriesId) {
        for point in &mut self.points {
            point.rows.remove(&series);
            point.custom_rows.remove(&series);
        }
        self.last_time_by_series.remove(&series);
    }
    fn cleanup_empty_points(&mut self) {
        self.points
            .retain(|point| !point.rows.is_empty() || !point.custom_rows.is_empty());
    }
    fn first_changed_index(&self, old: &[B::Key]) -> Option<usize> {
        let common = old.len().min(self.points.len());
        for (index, old_key) in old.iter().take(common).enumerate() {
            if old_key.partial_cmp(&self.behavior.key(&self.points[index].time))
                != Some(Ordering::Equal)
            {
                return Some(index);
            }
        }
        (old.len() != self.points.len()).then_some(common)
    }
    fn reindex(&mut self) {
        for (index, point) in self.points.iter_mut().enumerate() {
            point.index = TimePointIndex::new(index as f64);
            for row in point.rows.values_mut() {
                row.set_index(point.index);
            }
            for row in point.custom_rows.values_mut() {
                row.index = point.index;
            }
        }
    }
    fn refill_weights(&mut self, start: usize) {
        let mut scale_points = self
            .points
            .iter()
            .map(|point| TimeScalePoint {
                time_weight: point.weight,
                time: point.time.clone(),
                original_time: point.original_time.clone(),
            })
            .collect::<Vec<_>>();
        self.behavior
            .fill_weights_for_points(&mut scale_points, start);
        for (point, weighted) in self.points.iter_mut().zip(scale_points) {
            point.weight = weighted.time_weight;
        }
    }
    fn finish_mutation(
        &mut self,
        old_keys: Vec<B::Key>,
        updated: SeriesId,
        info: Option<SeriesUpdateInfo>,
    ) -> DataLayerResult<BuiltInDataUpdate<B::InternalItem, B::Item, M>, B::Error> {
        let changed = self.first_changed_index(&old_keys);
        self.reindex();
        if let Some(start) = changed {
            self.refill_weights(start);
        }
        Ok(self.response_for(updated, changed, info))
    }
    fn response_for(
        &self,
        updated: SeriesId,
        first_changed: Option<usize>,
        info: Option<SeriesUpdateInfo>,
    ) -> DataUpdateResponse<B::InternalItem, B::Item, (), M> {
        let mut series = BTreeMap::new();
        let mut custom = BTreeMap::new();
        let affected: Vec<SeriesId> = if first_changed.is_some() {
            self.series_types.keys().copied().collect()
        } else if self.series_types.contains_key(&updated) {
            vec![updated]
        } else {
            vec![]
        };
        for id in affected {
            series.insert(
                id,
                SeriesChanges {
                    rows: self.fulfilled_rows(id),
                    info: (id == updated).then_some(info).flatten(),
                },
            );
        }
        if !self.series_types.contains_key(&updated) && !self.custom_series.contains(&updated) {
            series.insert(updated, SeriesChanges { rows: vec![], info });
        }
        let affected_custom: Vec<SeriesId> = if first_changed.is_some() {
            self.custom_series.iter().copied().collect()
        } else if self.custom_series.contains(&updated) {
            vec![updated]
        } else {
            vec![]
        };
        for id in affected_custom {
            custom.insert(
                id,
                CustomSeriesChanges::new(
                    self.custom_indices(id),
                    (id == updated).then_some(info).flatten(),
                ),
            );
        }
        let points = first_changed.map(|_| self.time_scale_points());
        DataUpdateResponse {
            series,
            custom,
            time_scale: TimeScaleChanges {
                points,
                first_changed_point_index: first_changed,
                base_index: self.base_index(),
            },
        }
    }
    fn fulfilled_rows(
        &self,
        series: SeriesId,
    ) -> Vec<SeriesPlotRow<B::InternalItem, B::Item, (), M>> {
        self.points
            .iter()
            .filter_map(|point| point.rows.get(&series))
            .filter_map(|row| match row {
                SeriesDataRow::Value(row) => Some(row.clone()),
                SeriesDataRow::Whitespace(_) => None,
            })
            .collect()
    }
    fn custom_indices(&self, series: SeriesId) -> Vec<TimePointIndex> {
        self.points
            .iter()
            .filter_map(|point| point.custom_rows.get(&series))
            .filter(|row| row.fulfilled)
            .map(|row| row.index)
            .collect()
    }
    fn sync_custom_row_indexes<D>(
        &self,
        rows: &mut [CustomPlotRow<B::InternalItem, B::Item, D, M>],
    ) {
        for row in rows {
            if let Some(index) = self.find_point(&row.base.time) {
                row.base.index = self.points[index].index;
            }
        }
    }
    fn time_scale_points(&self) -> Vec<TimeScalePoint<B::InternalItem, B::Item>> {
        self.points
            .iter()
            .map(|point| TimeScalePoint {
                time_weight: point.weight,
                time: point.time.clone(),
                original_time: point.original_time.clone(),
            })
            .collect()
    }
    fn base_index(&self) -> Option<TimePointIndex> {
        // The base is the last value-bearing point, not trailing whitespace.
        self.points
            .iter()
            .rev()
            .find(|point| {
                point
                    .rows
                    .values()
                    .any(|row| matches!(row, SeriesDataRow::Value(_)))
                    || point.custom_rows.values().any(|row| row.fulfilled)
            })
            .map(|point| point.index)
            .or_else(|| (!self.last_time_by_series.is_empty()).then_some(TimePointIndex::default()))
    }
    fn refresh_last_fulfilled_time(&mut self, series: SeriesId) {
        let last = self
            .points
            .iter()
            .rev()
            .find(|point| self.series_is_fulfilled_at(point, series))
            .map(|point| point.time.clone());
        if let Some(last) = last {
            self.last_time_by_series.insert(series, last);
        } else {
            self.last_time_by_series.remove(&series);
        }
    }
    fn series_time_bounds(&self, series: SeriesId) -> Option<(B::InternalItem, B::InternalItem)> {
        let mut times = self
            .points
            .iter()
            .filter(|point| self.series_is_fulfilled_at(point, series))
            .map(|point| point.time.clone());
        let first = times.next()?;
        let last = times.last().unwrap_or_else(|| first.clone());
        Some((first, last))
    }
    fn series_is_fulfilled_at(
        &self,
        point: &TimePointData<B::InternalItem, B::Item, M>,
        series: SeriesId,
    ) -> bool {
        point
            .rows
            .get(&series)
            .is_some_and(SeriesDataRow::is_fulfilled)
            || point
                .custom_rows
                .get(&series)
                .is_some_and(|row| row.fulfilled)
    }
    fn series_update_info(
        &self,
        previous: Option<(B::InternalItem, B::InternalItem)>,
        current: Option<(B::InternalItem, B::InternalItem)>,
    ) -> Option<SeriesUpdateInfo> {
        let (previous_first, previous_last) = previous?;
        let (current_first, current_last) = current?;
        Some(SeriesUpdateInfo {
            historical_update: false,
            last_bar_updated_or_new_bars_added_to_the_right: self
                .compare_time(&current_last, &previous_last)
                != Ordering::Less
                && self.compare_time(&current_first, &previous_first) != Ordering::Less,
        })
    }
    fn empty_response(&self) -> DataUpdateResponse<B::InternalItem, B::Item, (), M> {
        DataUpdateResponse {
            series: BTreeMap::new(),
            custom: BTreeMap::new(),
            time_scale: TimeScaleChanges {
                points: None,
                first_changed_point_index: None,
                base_index: self.base_index(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        data_consumer::{LineData, LineDataItem, WhitespaceData},
        horz_scale_behavior_time::{
            horz_scale_behavior_time::HorzScaleBehaviorTime,
            types::{Time, UtcTimestamp},
        },
        localization_options::LocalizationOptions,
        time_scale::{TimeScale, TimeScaleUpdate},
        time_scale_options::HorzScaleOptions,
    };

    #[derive(Clone, Debug, PartialEq)]
    struct CustomDatum {
        time: Time,
        values: Vec<f64>,
        name: &'static str,
    }

    impl TimedData for CustomDatum {
        type Item = Time;

        fn time(&self) -> &Self::Item {
            &self.time
        }

        fn time_mut(&mut self) -> &mut Self::Item {
            &mut self.time
        }
    }

    impl CustomData for CustomDatum {}

    struct CustomBuilder;

    impl CustomPlotRowBuilder<CustomDatum, Time, ()> for CustomBuilder {
        fn values(&self, data: &CustomDatum) -> Vec<f64> {
            data.values.clone()
        }

        fn is_whitespace(&self, data: &CustomSeriesDataItem<CustomDatum, Time, ()>) -> bool {
            matches!(data, CustomSeriesDataItem::Whitespace(_))
        }
    }

    fn custom(
        time: f64,
        values: Vec<f64>,
        name: &'static str,
    ) -> CustomSeriesDataItem<CustomDatum, Time> {
        CustomSeriesDataItem::Data(CustomDatum {
            time: Time::from(UtcTimestamp::new(time)),
            values,
            name,
        })
    }

    fn custom_whitespace(time: f64) -> CustomSeriesDataItem<CustomDatum, Time> {
        CustomSeriesDataItem::Whitespace(crate::model::icustom_series::CustomSeriesWhitespaceData {
            time: Time::from(UtcTimestamp::new(time)),
            custom_values: None,
        })
    }
    fn line(time: f64, value: f64) -> BuiltInSeriesDataItem<Time> {
        BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time: Time::from(UtcTimestamp::new(time)),
            value,
            color: None,
            custom_values: None,
        }))
    }
    #[test]
    fn replacement_reindexes_overlapping_series_and_updates_time_scale() {
        let mut layer = DataLayer::new(HorzScaleBehaviorTime::default());
        let first = SeriesId::new(1);
        let second = SeriesId::new(2);
        let initial = layer
            .set_series_data(first, SeriesType::Line, vec![line(1., 1.), line(3., 3.)])
            .unwrap();
        assert_eq!(initial.time_scale.first_changed_point_index, Some(0));
        let update = layer
            .set_series_data(second, SeriesType::Line, vec![line(2., 2.), line(4., 4.)])
            .unwrap();
        assert_eq!(update.time_scale.base_index, Some(TimePointIndex::new(3.)));
        assert_eq!(
            update.series[&first]
                .rows
                .iter()
                .map(MutablePlotRow::index)
                .map(TimePointIndex::value)
                .collect::<Vec<_>>(),
            vec![0., 2.]
        );
        assert_eq!(update.time_scale.first_changed_point_index, Some(1));
    }
    #[test]
    fn incremental_and_pop_updates_preserve_whitespace_and_reject_invalid_history() {
        let mut layer = DataLayer::new(HorzScaleBehaviorTime::default());
        let id = SeriesId::new(1);
        layer
            .set_series_data(
                id,
                SeriesType::Line,
                vec![
                    line(1., 1.),
                    BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(WhitespaceData {
                        time: Time::from(UtcTimestamp::new(2.)),
                        custom_values: None,
                    })),
                    line(3., 3.),
                ],
            )
            .unwrap();
        let response = layer.update_series_data(id, line(4., 4.), false).unwrap();
        assert_eq!(response.time_scale.first_changed_point_index, Some(3));
        assert!(matches!(
            layer.update_series_data(id, line(2.5, 1.), false),
            Err(DataLayerError::OutOfOrderUpdate)
        ));
        let (popped, response) = layer.pop_series_data(id, 1).unwrap();
        assert_eq!(popped.len(), 1);
        assert_eq!(
            response.time_scale.base_index,
            Some(TimePointIndex::new(2.))
        );
    }

    #[test]
    fn replacement_preserves_original_business_day_strings_and_shared_removal() {
        let mut layer = DataLayer::new(HorzScaleBehaviorTime::default());
        let first = SeriesId::new(1);
        let second = SeriesId::new(2);
        let string_day = BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time: Time::from("2024-02-29"),
            value: 1.,
            color: None,
            custom_values: None,
        }));
        let first_response = layer
            .set_series_data(first, SeriesType::Line, vec![string_day])
            .unwrap();
        let original = first_response.series[&first].rows[0]
            .original_time()
            .clone();
        assert_eq!(original, Time::from("2024-02-29"));
        let second_response = layer
            .set_series_data(second, SeriesType::Line, vec![line(1_709_164_800., 2.)])
            .unwrap();
        assert!(second_response.time_scale.points.is_none());

        let response = layer.remove_series(first).unwrap();
        assert!(response.time_scale.points.is_none());
        assert_eq!(response.series[&first].rows, Vec::new());
        assert!(!response.series.contains_key(&second));
    }

    #[test]
    fn replacement_reports_directional_update_info_and_historical_updates_reuse_points() {
        let mut layer = DataLayer::new(HorzScaleBehaviorTime::default());
        let id = SeriesId::new(1);
        layer
            .set_series_data(id, SeriesType::Line, vec![line(2., 2.), line(4., 4.)])
            .unwrap();
        let replacement = layer
            .set_series_data(id, SeriesType::Line, vec![line(3., 3.), line(5., 5.)])
            .unwrap();
        assert_eq!(
            replacement.series[&id].info,
            Some(SeriesUpdateInfo {
                historical_update: false,
                last_bar_updated_or_new_bars_added_to_the_right: true,
            })
        );

        let other = SeriesId::new(2);
        layer
            .set_series_data(other, SeriesType::Line, vec![line(1., 10.)])
            .unwrap();
        let inserted = layer.update_series_data(id, line(1., 1.), true).unwrap();
        assert!(inserted.time_scale.points.is_none());
        assert_eq!(
            inserted.series[&id]
                .rows
                .iter()
                .map(MutablePlotRow::index)
                .map(TimePointIndex::value)
                .collect::<Vec<_>>(),
            vec![0., 1., 2.]
        );
    }

    #[test]
    fn time_scale_response_is_directly_consumable_by_the_viewport_model() {
        let mut layer = DataLayer::new(HorzScaleBehaviorTime::default());
        let response = layer
            .set_series_data(
                SeriesId::new(1),
                SeriesType::Line,
                vec![line(1., 1.), line(2., 2.)],
            )
            .unwrap();
        let mut scale = TimeScale::new(
            HorzScaleBehaviorTime::default(),
            HorzScaleOptions::default(),
            LocalizationOptions::new("en-US", "dd MMM 'yy"),
        );
        scale.update(TimeScaleUpdate {
            points: response.time_scale.points.unwrap(),
            first_changed_point_index: response.time_scale.first_changed_point_index.unwrap(),
        });
        scale.set_base_index(response.time_scale.base_index);
        assert!(scale.has_points());
    }

    #[test]
    fn custom_rows_remain_typed_while_shared_timestamps_stay_in_the_layer() {
        let mut layer = DataLayer::new(HorzScaleBehaviorTime::default());
        let custom_id = SeriesId::new(10);
        let built_in_id = SeriesId::new(11);
        let adapter = CustomDataLayerAdapter::new(custom_id, CustomBuilder);
        let response = adapter
            .set_data(
                &mut layer,
                vec![
                    custom(1., vec![2., 5., 1.], "first"),
                    custom_whitespace(2.),
                    custom(3., vec![4.], "third"),
                ],
            )
            .unwrap();
        assert_eq!(response.custom_rows.len(), 2);
        assert_eq!(response.custom_rows[0].data.name, "first");
        assert_eq!(response.custom_rows[0].base.value, [1., 5., 1., 1.]);
        assert_eq!(response.custom_rows[1].base.index, 2.0.into());
        assert_eq!(
            response.custom[&custom_id].indices,
            vec![0.0.into(), 2.0.into()]
        );

        let built_in = layer
            .set_series_data(built_in_id, SeriesType::Line, vec![line(3., 30.)])
            .unwrap();
        assert!(built_in.time_scale.points.is_none());

        let removed = layer.remove_series(custom_id).unwrap();
        assert_eq!(removed.time_scale.points.as_ref().map(Vec::len), Some(1));
        assert_eq!(removed.series[&built_in_id].rows.len(), 1);
    }

    #[test]
    fn custom_updates_validate_history_and_pop_without_storing_custom_payloads() {
        let mut layer = DataLayer::new(HorzScaleBehaviorTime::default());
        let id = SeriesId::new(20);
        let adapter = CustomDataLayerAdapter::new(id, CustomBuilder);
        adapter
            .set_data(
                &mut layer,
                vec![custom(1., vec![1.], "one"), custom(3., vec![3.], "three")],
            )
            .unwrap();
        let other = SeriesId::new(21);
        layer
            .set_series_data(other, SeriesType::Line, vec![line(2., 2.)])
            .unwrap();

        let historic = adapter
            .update_data(&mut layer, custom(2., vec![2., 8.], "two"), true)
            .unwrap();
        assert!(historic.time_scale.points.is_none());
        assert_eq!(historic.custom_rows[0].base.value, [8., 8., 2., 8.]);
        assert_eq!(
            historic.custom[&id].indices,
            vec![0.0.into(), 1.0.into(), 2.0.into()]
        );
        assert!(matches!(
            adapter.update_data(&mut layer, custom(4., vec![4.], "four"), true),
            Err(DataLayerError::HistoricalUpdateMissingPoint)
        ));

        let last = adapter
            .update_data(&mut layer, custom(3., vec![9.], "replacement"), false)
            .unwrap();
        assert!(last.time_scale.points.is_none());
        assert_eq!(last.custom_rows[0].data.name, "replacement");

        let (popped, response) = layer.pop_custom_series_data(id, 1).unwrap();
        assert_eq!(popped, vec![2.0.into()]);
        assert_eq!(response.custom[&id].indices, vec![0.0.into(), 1.0.into()]);
    }

    #[test]
    fn custom_empty_values_are_rejected_and_whitespace_remains_a_time_point() {
        let mut layer = DataLayer::new(HorzScaleBehaviorTime::default());
        let id = SeriesId::new(30);
        let adapter = CustomDataLayerAdapter::new(id, CustomBuilder);
        assert!(matches!(
            adapter.set_data(&mut layer, vec![custom(1., vec![], "empty")]),
            Err(DataLayerError::PlotRow(
                PlotRowCreationError::EmptyCustomValues
            ))
        ));
        let whitespace = adapter
            .set_data(&mut layer, vec![custom_whitespace(2.)])
            .unwrap();
        assert!(whitespace.custom_rows.is_empty());
        assert_eq!(whitespace.time_scale.points.as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn custom_series_share_points_and_append_with_a_time_scale_update() {
        let mut layer = DataLayer::new(HorzScaleBehaviorTime::default());
        let first = CustomDataLayerAdapter::new(SeriesId::new(40), CustomBuilder);
        let second = CustomDataLayerAdapter::new(SeriesId::new(41), CustomBuilder);
        first
            .set_data(&mut layer, vec![custom(1., vec![1.], "first")])
            .unwrap();
        let shared = second
            .set_data(&mut layer, vec![custom(1., vec![2.], "second")])
            .unwrap();
        assert!(shared.time_scale.points.is_none());

        let removed = layer.remove_series(first.series()).unwrap();
        assert!(removed.time_scale.points.is_none());
        let appended = second
            .update_data(&mut layer, custom(2., vec![3.], "appended"), false)
            .unwrap();
        assert_eq!(appended.time_scale.first_changed_point_index, Some(1));
        assert_eq!(appended.custom_rows[0].base.index, 1.0.into());
    }
}
