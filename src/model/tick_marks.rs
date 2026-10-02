//! Cached horizontal-axis tick-mark selection.

use crate::model::time_data::{TickMarkWeightValue, TimePointIndex};
use std::collections::{BTreeMap, BTreeSet};

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
}

impl<InternalItem, OriginalItem> Default for TickMarks<InternalItem, OriginalItem> {
    fn default() -> Self {
        Self {
            marks_by_weight: BTreeMap::new(),
            cache: None,
            uniform_distribution: false,
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
        let cache_is_current = self.cache.as_ref().is_some_and(|cache| {
            cache.max_indexes_per_mark == max_indexes_per_mark
                && cache.indices_with_data_id == indices_with_data_id
                && cache.check_indices_for_data == check_indices_for_data
        });

        if !cache_is_current {
            let marks = self.build_marks_impl(
                max_indexes_per_mark,
                check_indices_for_data,
                indices_with_data,
            );
            self.cache = Some(MarksCache {
                max_indexes_per_mark,
                indices_with_data_id,
                check_indices_for_data,
                marks,
            });
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
