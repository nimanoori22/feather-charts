//! Cached horizontal-axis tick-mark selection.

use crate::model::time_data::{TickMarkWeightValue, TimePointIndex};
use std::collections::{BTreeMap, BTreeSet};

fn suffix_start<T>(items: &[T], since: usize, mut index: impl FnMut(&T) -> f64) -> usize {
    if since == 0 {
        return 0;
    }
    // Append invalidation touches the tail; avoid cold historical cache lines.
    items
        .iter()
        .rposition(|item| index(item) < since as f64)
        .map_or(0, |at| at + 1)
}

/// A selectable horizontal-axis tick mark.
#[derive(Clone, Debug, PartialEq)]
pub struct TickMark<InternalItem, OriginalItem> {
    pub index: TimePointIndex,
    pub time: InternalItem,
    pub weight: TickMarkWeightValue,
    pub original_time: OriginalItem,
}

#[derive(Clone, Debug)]
struct MarksCache<InternalItem, OriginalItem> {
    max_indexes_per_mark: usize,
    indices_with_data_id: u64,
    check_indices_for_data: bool,
    marks: Vec<TickMark<InternalItem, OriginalItem>>,
    stages: BTreeMap<TickMarkWeightValue, Vec<usize>>,
}

/// Incrementally maintained tick marks, selected by weight and available
/// horizontal label spacing.
///
/// Time-scale points are ordered by index. Updating from an index discards
/// only the affected suffix of each weight bucket, which matches the data
/// layer's incremental time-point updates.
#[derive(Clone, Debug)]
pub struct TickMarks<InternalItem, OriginalItem> {
    marks_by_weight: BTreeMap<TickMarkWeightValue, Vec<TickMark<InternalItem, OriginalItem>>>,
    cache: Option<MarksCache<InternalItem, OriginalItem>>,
    uniform_distribution: bool,
    dirty_since: Option<usize>,
}

impl<InternalItem, OriginalItem> Default for TickMarks<InternalItem, OriginalItem> {
    fn default() -> Self {
        Self {
            marks_by_weight: BTreeMap::new(),
            cache: None,
            uniform_distribution: false,
            dirty_since: None,
        }
    }
}

impl<InternalItem, OriginalItem> TickMarks<InternalItem, OriginalItem>
where
    InternalItem: Clone,
    OriginalItem: Clone,
{
    /// When enabled, a weight is retained only if all of its eligible marks
    /// fit; otherwise the previously selected, higher-weight marks are kept.
    pub fn set_uniform_distribution(&mut self, value: bool) {
        self.uniform_distribution = value;
        self.cache = None;
    }

    /// Replaces marks at and after `first_changed_point_index` with marks
    /// generated from the updated time-scale point suffix.
    pub fn set_time_scale_points(
        &mut self,
        points: &[crate::model::time_data::TimeScalePoint<InternalItem, OriginalItem>],
        first_changed_point_index: usize,
    ) {
        self.remove_marks_since_index(first_changed_point_index);
        self.cache = None;

        for (index, point) in points.iter().enumerate().skip(first_changed_point_index) {
            self.marks_by_weight
                .entry(point.time_weight)
                .or_default()
                .push(TickMark {
                    index: TimePointIndex::new(index as f64),
                    time: point.time.clone(),
                    weight: point.time_weight,
                    original_time: point.original_time.clone(),
                });
        }
    }

    pub fn append_point(
        &mut self,
        index: usize,
        point: &crate::model::time_data::TimeScalePoint<InternalItem, OriginalItem>,
    ) {
        self.marks_by_weight
            .entry(point.time_weight)
            .or_default()
            .push(TickMark {
                index: TimePointIndex::new(index as f64),
                time: point.time.clone(),
                weight: point.time_weight,
                original_time: point.original_time.clone(),
            });
        self.invalidate_index(index);
    }

    pub fn update_membership(&mut self, index: usize, version: u64) {
        if let Some(cache) = &mut self.cache {
            cache.indices_with_data_id = version;
        }
        self.invalidate_index(index);
    }

    pub fn invalidate_index(&mut self, index: usize) {
        self.dirty_since = Some(self.dirty_since.map_or(index, |old| old.min(index)));
    }

    /// Builds the currently drawable marks. Higher-weight marks take
    /// precedence, and every returned pair is separated by at least
    /// `ceil(max_label_width / bar_spacing)` logical indexes.
    ///
    /// `indices_with_data` contains integral time-point indexes that are not
    /// whitespace. It is consulted only when `check_indices_for_data` is true.
    pub fn build(
        &mut self,
        bar_spacing: f64,
        max_label_width: f64,
        check_indices_for_data: bool,
        indices_with_data: &BTreeSet<usize>,
        indices_with_data_id: u64,
    ) -> &[TickMark<InternalItem, OriginalItem>] {
        assert!(
            bar_spacing.is_finite() && bar_spacing > 0.0,
            "bar spacing must be finite and positive"
        );
        assert!(
            max_label_width.is_finite() && max_label_width >= 0.0,
            "maximum label width must be finite and non-negative"
        );

        let max_indexes_per_mark = (max_label_width / bar_spacing).ceil() as usize;
        let same_layout = self.cache.as_ref().is_some_and(|cache| {
            cache.max_indexes_per_mark == max_indexes_per_mark
                && cache.check_indices_for_data == check_indices_for_data
        });
        let same_membership = self
            .cache
            .as_ref()
            .is_some_and(|cache| cache.indices_with_data_id == indices_with_data_id);
        if !same_layout || !same_membership || self.dirty_since.is_some() {
            if self.uniform_distribution {
                self.cache = Some(MarksCache {
                    max_indexes_per_mark,
                    indices_with_data_id,
                    check_indices_for_data,
                    marks: self.build_marks_impl(
                        max_indexes_per_mark,
                        check_indices_for_data,
                        indices_with_data,
                    ),
                    stages: BTreeMap::new(),
                });
            } else {
                let incremental = same_layout && same_membership && self.dirty_since.is_some();
                let mut cache = self
                    .cache
                    .take()
                    .filter(|_| incremental)
                    .unwrap_or_else(|| MarksCache {
                        max_indexes_per_mark,
                        indices_with_data_id,
                        check_indices_for_data,
                        marks: Vec::new(),
                        stages: BTreeMap::new(),
                    });
                let mut since = if incremental {
                    self.dirty_since.unwrap()
                } else {
                    0
                };
                let mut previous = Vec::new();
                let mut previous_weight = None;
                for (&weight, candidates) in self.marks_by_weight.iter().rev() {
                    // A higher-priority change can displace this group's marks
                    // only within one label spacing to its left.
                    since = since.saturating_sub(max_indexes_per_mark);
                    let old = cache.stages.remove(&weight);
                    if old.is_none() {
                        since = 0;
                    }
                    let mut selected = old.unwrap_or_default();
                    selected.truncate(suffix_start(&selected, since, |&index| index as f64));
                    let mut left = selected
                        .last()
                        .map_or(f64::NEG_INFINITY, |&index| index as f64);
                    let mut at = suffix_start(&previous, since, |&index| index as f64);
                    for mark in
                        &candidates[suffix_start(candidates, since, |mark| mark.index.value())..]
                    {
                        let index = mark.index.value() as usize;
                        while at < previous.len() && previous[at] < index {
                            left = previous[at] as f64;
                            selected.push(previous[at]);
                            at += 1;
                        }
                        let right = previous
                            .get(at)
                            .map_or(f64::INFINITY, |&index| index as f64);
                        if right - index as f64 >= max_indexes_per_mark as f64
                            && index as f64 - left >= max_indexes_per_mark as f64
                            && (!check_indices_for_data || indices_with_data.contains(&index))
                        {
                            selected.push(index);
                            left = index as f64;
                        }
                    }
                    selected.extend_from_slice(&previous[at..]);
                    if let Some(weight) = previous_weight {
                        cache.stages.insert(weight, previous);
                    }
                    previous = selected;
                    previous_weight = Some(weight);
                }
                cache
                    .marks
                    .truncate(suffix_start(&cache.marks, since, |mark| mark.index.value()));
                for &index in &previous[suffix_start(&previous, since, |&index| index as f64)..] {
                    let mark = self
                        .marks_by_weight
                        .values()
                        .find_map(|bucket| {
                            if since > 0 {
                                let at = suffix_start(bucket, index, |mark| mark.index.value());
                                bucket
                                    .get(at)
                                    .filter(|mark| mark.index.value() == index as f64)
                            } else {
                                bucket
                                    .binary_search_by_key(&index, |mark| {
                                        mark.index.value() as usize
                                    })
                                    .ok()
                                    .map(|at| &bucket[at])
                            }
                        })
                        .expect("selected index must have a tick mark");
                    cache.marks.push(mark.clone());
                }
                if let Some(weight) = previous_weight {
                    cache.stages.insert(weight, previous);
                }
                cache.indices_with_data_id = indices_with_data_id;
                self.cache = Some(cache);
            }
            self.dirty_since = None;
        }

        &self
            .cache
            .as_ref()
            .expect("cache was just initialized")
            .marks
    }

    fn remove_marks_since_index(&mut self, since_index: usize) {
        if since_index == 0 {
            self.marks_by_weight.clear();
            return;
        }

        self.marks_by_weight.retain(|_, marks| {
            if marks
                .first()
                .is_some_and(|mark| mark.index.value() >= since_index as f64)
            {
                return false;
            }

            let cut = marks.partition_point(|mark| mark.index.value() < since_index as f64);
            marks.truncate(cut);
            !marks.is_empty()
        });
    }

    fn build_marks_impl(
        &self,
        max_indexes_per_mark: usize,
        check_indices_for_data: bool,
        indices_with_data: &BTreeSet<usize>,
    ) -> Vec<TickMark<InternalItem, OriginalItem>> {
        let can_be_included = |mark: &TickMark<InternalItem, OriginalItem>| {
            !check_indices_for_data
                || (mark.index.value().is_finite()
                    && mark.index.value() >= 0.0
                    && mark.index.value().fract() == 0.0
                    && indices_with_data.contains(&(mark.index.value() as usize)))
        };

        let mut marks: Vec<TickMark<InternalItem, OriginalItem>> = Vec::new();
        for current_weight_marks in self.marks_by_weight.values().rev() {
            let previous_marks = marks;
            marks = Vec::new();
            let mut previous_index = 0;
            let mut left_index = f64::NEG_INFINITY;
            let mut right_index = f64::INFINITY;

            for mark in current_weight_marks {
                let current_index = mark.index.value();

                while previous_index < previous_marks.len() {
                    let previous_mark = &previous_marks[previous_index];
                    let previous_mark_index = previous_mark.index.value();
                    if previous_mark_index < current_index && can_be_included(previous_mark) {
                        previous_index += 1;
                        marks.push(previous_mark.clone());
                        left_index = previous_mark_index;
                        right_index = f64::INFINITY;
                    } else {
                        right_index = previous_mark_index;
                        break;
                    }
                }

                let spacing = max_indexes_per_mark as f64;
                if right_index - current_index >= spacing
                    && current_index - left_index >= spacing
                    && can_be_included(mark)
                {
                    marks.push(mark.clone());
                    left_index = current_index;
                } else if self.uniform_distribution {
                    return previous_marks;
                }
            }

            for previous_mark in previous_marks.iter().skip(previous_index) {
                if can_be_included(previous_mark) {
                    marks.push(previous_mark.clone());
                }
            }
        }

        marks
    }
}

#[cfg(test)]
mod tests {
    use super::TickMarks;
    use crate::model::time_data::{TickMarkWeightValue, TimeScalePoint};
    use std::collections::BTreeSet;

    fn point(index: i32, weight: i32) -> TimeScalePoint<i32, i32> {
        TimeScalePoint {
            time_weight: TickMarkWeightValue::new(weight),
            time: index,
            original_time: index,
        }
    }

    #[test]
    fn appended_selection_matches_global_reference_across_weights_and_membership_changes() {
        for uniform in [false, true] {
            for spacing in [0., 1., 3., 13.] {
                for filter in [false, true] {
                    let mut marks = TickMarks::default();
                    marks.set_uniform_distribution(uniform);
                    let mut members = BTreeSet::new();
                    let mut random = 17_u64;
                    for index in 0..300 {
                        random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                        marks
                            .append_point(index, &point(index as i32, ((random >> 32) % 7) as i32));
                        if random & 3 != 0 {
                            members.insert(index);
                        }
                        if index > 20 && index % 11 == 0 {
                            let changed = index - 17;
                            if !members.remove(&changed) {
                                members.insert(changed);
                            }
                            marks.invalidate_index(changed);
                        }
                        marks.update_membership(index, index as u64);
                        let expected = marks.build_marks_impl(spacing as usize, filter, &members);
                        assert_eq!(
                            marks.build(1., spacing, filter, &members, index as u64),
                            expected,
                            "uniform={uniform} spacing={spacing} filter={filter} index={index}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn unknown_membership_version_rebuilds_the_prefix_even_with_a_pending_append() {
        let mut marks = TickMarks::default();
        let points = (0..100).map(|index| point(index, 1)).collect::<Vec<_>>();
        marks.set_time_scale_points(&points, 0);
        let mut members = (0..100).collect::<BTreeSet<_>>();
        marks.build(1., 10., true, &members, 0);
        marks.append_point(100, &point(100, 1));
        members.remove(&0);
        members.insert(100);
        let expected = marks.build_marks_impl(10, true, &members);
        assert_eq!(marks.build(1., 10., true, &members, 1), expected);
    }

    #[test]
    fn suffix_bound_search_visits_only_the_changed_tail() {
        for count in [1200, 100_000] {
            let items = (0..count).collect::<Vec<_>>();
            let mut visits = 0;
            let at = super::suffix_start(&items, count - 10, |&index| {
                visits += 1;
                index as f64
            });
            assert_eq!(at, count - 10);
            assert!(visits <= 12, "{count}: visited {visits} retained indexes");
            for since in [0, 1, count / 2, count, count + 1] {
                assert_eq!(
                    super::suffix_start(&items, since, |&index| index as f64),
                    items.partition_point(|&index| index < since)
                );
            }
        }
    }

    #[test]
    fn suffix_bounds_match_binary_search_for_empty_and_sparse_buckets() {
        assert_eq!(
            super::suffix_start::<usize>(&[], 10, |&index| index as f64),
            0
        );
        let items = [2, 5, 20, 200];
        for since in 0..=201 {
            assert_eq!(
                super::suffix_start(&items, since, |&index| index as f64),
                items.partition_point(|&index| index < since)
            );
        }
    }

    #[test]
    fn selects_higher_weight_marks_before_lower_weight_marks() {
        let mut marks = TickMarks::default();
        marks.set_time_scale_points(&[point(0, 1), point(1, 10), point(2, 1), point(3, 1)], 0);

        let selected = marks.build(1.0, 2.0, false, &BTreeSet::new(), 0);

        assert_eq!(
            selected
                .iter()
                .map(|mark| mark.index.value())
                .collect::<Vec<_>>(),
            vec![1.0, 3.0]
        );
        assert_eq!(selected[0].weight, TickMarkWeightValue::new(10));
    }

    #[test]
    fn filters_whitespace_indices_and_rebuilds_when_the_data_id_changes() {
        let mut marks = TickMarks::default();
        marks.set_time_scale_points(&[point(0, 1), point(1, 1), point(2, 1)], 0);

        let with_data = BTreeSet::from([0_usize, 2]);
        let selected = marks.build(1.0, 1.0, true, &with_data, 1);
        assert_eq!(
            selected
                .iter()
                .map(|mark| mark.index.value())
                .collect::<Vec<_>>(),
            vec![0.0, 2.0]
        );

        let selected = marks.build(1.0, 1.0, true, &BTreeSet::from([1_usize]), 2);
        assert_eq!(
            selected
                .iter()
                .map(|mark| mark.index.value())
                .collect::<Vec<_>>(),
            vec![1.0]
        );
    }

    #[test]
    fn uniform_distribution_keeps_the_previous_weight_group_when_one_mark_cannot_fit() {
        let mut marks = TickMarks::default();
        marks.set_uniform_distribution(true);
        marks.set_time_scale_points(&[point(0, 10), point(1, 1), point(2, 1)], 0);

        let selected = marks.build(1.0, 2.0, false, &BTreeSet::new(), 0);

        assert_eq!(
            selected
                .iter()
                .map(|mark| mark.index.value())
                .collect::<Vec<_>>(),
            vec![0.0]
        );
    }

    #[test]
    fn replacing_a_point_suffix_removes_stale_marks() {
        let mut marks = TickMarks::default();
        marks.set_time_scale_points(&[point(0, 1), point(1, 1), point(2, 1)], 0);
        marks.set_time_scale_points(&[point(0, 1), point(1, 10)], 1);

        let selected = marks.build(1.0, 0.0, false, &BTreeSet::new(), 0);
        assert_eq!(
            selected
                .iter()
                .map(|mark| mark.index.value())
                .collect::<Vec<_>>(),
            vec![0.0, 1.0]
        );
        assert_eq!(selected[1].weight, TickMarkWeightValue::new(10));
    }
}
