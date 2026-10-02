//! Sorted plot-row storage and range queries.

use crate::model::{
    plot_data::{PlotRowLike, PlotRowValueIndex},
    time_data::TimePointIndex,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MismatchDirection {
    NearestLeft,
    #[default]
    None,
    NearestRight,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinMax {
    pub min: f64,
    pub max: f64,
}

#[derive(Clone, Debug)]
pub struct PlotList<Row> {
    items: Vec<Row>,
    indices: Vec<TimePointIndex>,
}

impl<Row> Default for PlotList<Row> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            indices: Vec::new(),
        }
    }
}

impl<Row: PlotRowLike + Clone> PlotList<Row> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn last(&self) -> Option<&Row> {
        self.items.last()
    }
    pub fn first_index(&self) -> Option<TimePointIndex> {
        self.indices.first().copied()
    }
    pub fn last_index(&self) -> Option<TimePointIndex> {
        self.indices.last().copied()
    }
    pub fn size(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    pub fn contains(&self, index: TimePointIndex) -> bool {
        self.position(index, MismatchDirection::None).is_some()
    }
    pub fn value_at(&self, index: TimePointIndex) -> Option<&Row> {
        self.search(index, MismatchDirection::None)
    }
    pub fn search(&self, index: TimePointIndex, direction: MismatchDirection) -> Option<&Row> {
        self.position(index, direction).map(|i| &self.items[i])
    }
    pub fn rows(&self) -> &[Row] {
        &self.items
    }
    pub fn indices(&self) -> &[TimePointIndex] {
        &self.indices
    }
    pub fn set_data(&mut self, rows: Vec<Row>) {
        self.indices = rows.iter().map(PlotRowLike::index).collect();
        self.items = rows;
    }
    pub fn min_max_on_range_cached(
        &self,
        start: TimePointIndex,
        end: TimePointIndex,
        plots: &[PlotRowValueIndex],
    ) -> Option<MinMax> {
        let mut result: Option<MinMax> = None;
        for row in &self.items {
            if row.index() < start || row.index() > end {
                continue;
            }
            for plot in plots {
                let value = row.values()[*plot as usize];
                if value.is_nan() {
                    continue;
                }
                result = Some(match result {
                    None => MinMax {
                        min: value,
                        max: value,
                    },
                    Some(old) => MinMax {
                        min: old.min.min(value),
                        max: old.max.max(value),
                    },
                });
            }
        }
        result
    }
    fn position(&self, index: TimePointIndex, direction: MismatchDirection) -> Option<usize> {
        let lower = self.indices.partition_point(|candidate| *candidate < index);
        if lower < self.indices.len() && self.indices[lower] == index {
            return Some(lower);
        }
        match direction {
            MismatchDirection::None => None,
            MismatchDirection::NearestLeft => lower.checked_sub(1),
            MismatchDirection::NearestRight => self.indices.get(lower).map(|_| lower),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::plot_data::PlotRow;
    fn row(i: f64, values: [f64; 4]) -> PlotRow<(), ()> {
        PlotRow {
            index: i.into(),
            time: (),
            original_time: (),
            value: values,
            custom_values: None,
            original_data_count: None,
        }
    }
    #[test]
    fn searches_exact_and_neighbors_and_skips_nan() {
        let mut list = PlotList::new();
        list.set_data(vec![
            row(0.0, [1.; 4]),
            row(2.0, [f64::NAN, 3., 2., 4.]),
            row(4.0, [9.; 4]),
        ]);
        assert!(list.contains(2.0.into()));
        assert_eq!(
            list.search(3.0.into(), MismatchDirection::NearestLeft)
                .unwrap()
                .index()
                .value(),
            2.0
        );
        assert_eq!(
            list.search(3.0.into(), MismatchDirection::NearestRight)
                .unwrap()
                .index()
                .value(),
            4.0
        );
        assert_eq!(
            list.min_max_on_range_cached(
                0.0.into(),
                2.0.into(),
                &[PlotRowValueIndex::Open, PlotRowValueIndex::Close]
            ),
            Some(MinMax { min: 1., max: 4. })
        );
    }
}
