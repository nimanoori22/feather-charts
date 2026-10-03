//! Chart-owner update fan-out between DataLayer, TimeScale, and Series state.

use crate::model::{
    data_layer::{DataUpdateResponse, SeriesId},
    ihorz_scale_behavior::HorzScaleBehavior,
    series::SeriesUpdateTarget,
    time_data::TimePointIndex,
    time_scale::{TimeScale, TimeScaleUpdate},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartDataEffect {
    RecalculateAllPanes,
    LightUpdate,
}

/// Owns the heterogeneous Series registry at the chart boundary. Custom
/// Series are erased only here, after their typed DataLayer handoff; DataLayer
/// itself never stores a trait object or custom payload.
pub struct ChartDataCoordinator<I, O, M = ()> {
    targets: BTreeMap<SeriesId, Box<dyn SeriesUpdateTarget<I, O, M>>>,
    fulfilled_indices: BTreeSet<usize>,
    fulfilled_indices_version: u64,
}

impl<I, O, M> Default for ChartDataCoordinator<I, O, M> {
    fn default() -> Self {
        Self {
            targets: BTreeMap::new(),
            fulfilled_indices: BTreeSet::new(),
            fulfilled_indices_version: 0,
        }
    }
}

impl<I, O, M> ChartDataCoordinator<I, O, M>
where
    I: Clone + 'static,
    O: Clone + 'static,
    M: Clone + 'static,
{
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert_target(
        &mut self,
        id: SeriesId,
        target: impl SeriesUpdateTarget<I, O, M> + 'static,
    ) {
        self.targets.insert(id, Box::new(target));
    }
    pub fn remove_target(&mut self, id: SeriesId) -> bool {
        self.targets.remove(&id).is_some()
    }
    pub fn target_count(&self) -> usize {
        self.targets.len()
    }
    pub fn fulfilled_indices(&self) -> &BTreeSet<usize> {
        &self.fulfilled_indices
    }
    pub fn refresh_fulfilled_indices<B>(&mut self, time_scale: &mut TimeScale<B>)
    where
        B: HorzScaleBehavior<Item = O, InternalItem = I>,
        B::Key: PartialOrd,
        B::CacheKey: Eq + std::hash::Hash,
    {
        let next = self
            .targets
            .values()
            .flat_map(|target| target.fulfilled_indices())
            .filter_map(TimePointIndex::as_usize)
            .collect::<BTreeSet<_>>();
        if next != self.fulfilled_indices {
            self.fulfilled_indices = next;
            self.fulfilled_indices_version = self.fulfilled_indices_version.wrapping_add(1);
        }
        time_scale.set_indices_with_data(
            self.fulfilled_indices.clone(),
            self.fulfilled_indices_version,
        );
    }

    pub fn apply<B>(
        &mut self,
        time_scale: &mut TimeScale<B>,
        update: DataUpdateResponse<I, O, (), M>,
    ) -> Vec<ChartDataEffect>
    where
        B: HorzScaleBehavior<Item = O, InternalItem = I>,
        B::InternalItem: Clone,
        B::Item: Clone,
        B::Key: PartialOrd,
        B::CacheKey: Clone + Eq + std::hash::Hash,
    {
        self.apply_with_rows(time_scale, update, || {})
    }

    pub fn apply_with_rows<B>(
        &mut self,
        time_scale: &mut TimeScale<B>,
        update: DataUpdateResponse<I, O, (), M>,
        apply_custom_rows: impl FnOnce(),
    ) -> Vec<ChartDataEffect>
    where
        B: HorzScaleBehavior<Item = O, InternalItem = I>,
        B::Key: PartialOrd,
        B::CacheKey: Eq + std::hash::Hash,
    {
        // Must happen before TimeScale changes logical indexes.
        for id in update.series.keys().chain(update.custom.keys()) {
            if let Some(target) = self.targets.get_mut(id) {
                target.invalidate_pane_data();
            }
        }

        let old_first = time_scale.index_to_time(TimePointIndex::new(0.0)).cloned();
        let replaced_whitespace = update.time_scale.first_changed_point_index.is_none();
        if let (Some(points), Some(first_changed_point_index)) = (
            update.time_scale.points,
            update.time_scale.first_changed_point_index,
        ) {
            time_scale.update(TimeScaleUpdate {
                points,
                first_changed_point_index,
            });
        }
        let new_first = time_scale.index_to_time(TimePointIndex::new(0.0)).cloned();
        let current_base = time_scale.base_index().unwrap_or_default();
        let visible = time_scale.visible_strict_range();
        if let (Some(visible), Some(old_first), Some(new_first)) = (visible, old_first, new_first) {
            let added_to_right = update
                .time_scale
                .base_index
                .is_some_and(|base| base > current_base)
                && !matches!(
                    time_scale
                        .behavior()
                        .key(&old_first)
                        .partial_cmp(&time_scale.behavior().key(&new_first)),
                    Some(std::cmp::Ordering::Greater)
                );
            let options = time_scale.options();
            let should_shift = visible.contains(current_base)
                && (!replaced_whitespace
                    || options.allow_shift_visible_range_on_whitespace_replacement)
                && options.shift_visible_range_on_new_bar;
            if added_to_right && !should_shift {
                let shift = update.time_scale.base_index.unwrap().value() - current_base.value();
                time_scale.set_right_offset(time_scale.right_offset() - shift);
            }
        }
        time_scale.set_base_index(update.time_scale.base_index);

        for (id, changes) in update.series {
            if let Some(target) = self.targets.get_mut(&id) {
                target.apply_built_in_rows(changes.rows, changes.info);
            }
        }
        apply_custom_rows();
        for (id, changes) in update.custom {
            if let Some(target) = self.targets.get_mut(&id) {
                target.apply_custom_indices(&changes);
            }
        }

        self.refresh_fulfilled_indices(time_scale);

        vec![
            ChartDataEffect::RecalculateAllPanes,
            ChartDataEffect::LightUpdate,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        data_layer::{SeriesChanges, SeriesUpdateInfo, TimeScaleChanges},
        horz_scale_behavior_time::{
            horz_scale_behavior_time::HorzScaleBehaviorTime,
            types::{Time, TimePoint, UtcTimestamp},
        },
        localization_options::LocalizationOptions,
        time_data::{TickMarkWeightValue, TimeScalePoint},
        time_scale_options::HorzScaleOptions,
    };
    use std::{cell::RefCell, rc::Rc};

    struct Target {
        calls: Rc<RefCell<Vec<&'static str>>>,
        indexes: Vec<TimePointIndex>,
    }

    impl SeriesUpdateTarget<TimePoint, Time, ()> for Target {
        fn invalidate_pane_data(&mut self) {
            self.calls.borrow_mut().push("invalidate");
        }
        fn apply_built_in_rows(
            &mut self,
            _rows: Vec<crate::model::series_data::SeriesPlotRow<TimePoint, Time>>,
            _info: Option<SeriesUpdateInfo>,
        ) {
            self.calls.borrow_mut().push("rows");
        }
        fn fulfilled_indices(&self) -> Vec<TimePointIndex> {
            self.indexes.clone()
        }
    }

    #[test]
    fn preserves_source_order_updates_time_scale_and_returns_effects() {
        let id = SeriesId::new(1);
        let calls = Rc::new(RefCell::new(vec![]));
        let mut coordinator = ChartDataCoordinator::new();
        coordinator.insert_target(
            id,
            Target {
                calls: calls.clone(),
                indexes: vec![0.0.into()],
            },
        );
        let point = TimePoint {
            timestamp: UtcTimestamp::new(1.),
            business_day: None,
        };
        let mut time_scale = TimeScale::new(
            HorzScaleBehaviorTime::default(),
            HorzScaleOptions::default(),
            LocalizationOptions::new("en-US", "dd MMM 'yy"),
        );
        let update = DataUpdateResponse {
            series: BTreeMap::from([(
                id,
                SeriesChanges {
                    rows: vec![],
                    info: None,
                },
            )]),
            custom: BTreeMap::new(),
            time_scale: TimeScaleChanges {
                points: Some(vec![TimeScalePoint {
                    time_weight: TickMarkWeightValue::default(),
                    time: point,
                    original_time: Time::from(UtcTimestamp::new(1.)),
                }]),
                first_changed_point_index: Some(0),
                base_index: Some(0.0.into()),
            },
        };
        assert_eq!(
            coordinator.apply(&mut time_scale, update),
            vec![
                ChartDataEffect::RecalculateAllPanes,
                ChartDataEffect::LightUpdate
            ]
        );
        assert_eq!(calls.borrow().as_slice(), ["invalidate", "rows"]);
        assert!(time_scale.has_points());
    }
}
