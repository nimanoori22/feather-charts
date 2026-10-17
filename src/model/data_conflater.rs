//! Derived drawing summaries. Canonical rows and their styles remain with Series.
use crate::model::{
    plot_data::{PlotRowLike, PlotRowValue},
    series_data::SeriesPlotRow,
    time_data::TimePointIndex,
};
use std::collections::BTreeMap;

pub(crate) fn factor(spacing: f64, ratio: f64, threshold: f64) -> usize {
    let threshold = threshold / ratio;
    if spacing >= threshold {
        return 1;
    }
    (2_f64.powf((threshold / spacing).log2().floor()) as usize).clamp(1, 512)
}
pub(crate) fn normalize(level: usize) -> usize {
    if level <= 1 {
        1
    } else {
        level.min(512).next_power_of_two()
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Bar {
    pub first: usize,
    pub end: usize,
    pub index: TimePointIndex,
    pub value: PlotRowValue,
    pub count: usize,
}
impl PlotRowLike for Bar {
    fn index(&self) -> TimePointIndex {
        self.index
    }
    fn values(&self) -> &PlotRowValue {
        &self.value
    }
}
pub(crate) trait Input {
    fn bar(&self, position: usize) -> Bar;
}
impl Input for Bar {
    fn bar(&self, _: usize) -> Bar {
        self.clone()
    }
}
impl<I, O, D, M> Input for SeriesPlotRow<I, O, D, M> {
    fn bar(&self, position: usize) -> Bar {
        Bar {
            first: position,
            end: position + 1,
            index: self.index(),
            value: *self.values(),
            count: self.original_data_count().unwrap_or(1),
        }
    }
}
impl Bar {
    fn merge(&self, other: &Self) -> Self {
        Self {
            first: self.first,
            end: other.end,
            index: self.index,
            value: [
                self.value[0],
                self.value[1].max(other.value[1]),
                self.value[2].min(other.value[2]),
                other.value[3],
            ],
            count: self.count + other.count,
        }
    }
}
pub(crate) fn line_value(v: PlotRowValue, count: usize) -> f64 {
    if count <= 1 {
        v[3]
    } else if (v[1] - v[3]).abs() > (v[2] - v[3]).abs() {
        v[1]
    } else {
        v[2]
    }
}
#[derive(Default)]
struct Build {
    level: usize,
    position: usize,
    rows: Vec<Bar>,
}
#[derive(Default)]
pub(crate) struct ConflationCache {
    levels: BTreeMap<usize, Vec<Bar>>,
    build: Option<Build>,
    #[cfg(test)]
    pub work: usize,
}
impl ConflationCache {
    pub fn clear(&mut self) {
        self.levels.clear();
        self.build = None;
    }
    pub fn contains(&self, level: usize) -> bool {
        self.levels.contains_key(&normalize(level))
    }
    pub fn rows(&self, level: usize) -> &[Bar] {
        &self.levels[&normalize(level)]
    }
    pub fn ensure<R: Input>(&mut self, rows: &[R], level: usize) {
        while level > 1 && !self.contains(level) {
            self.step(rows, level, usize::MAX);
        }
    }
    /// One unit builds at most one pair; partial levels never become visible.
    pub fn step<R: Input>(&mut self, rows: &[R], target: usize, mut budget: usize) -> usize {
        let initial = budget;
        let target = normalize(target);
        if target <= 1 {
            return 0;
        }
        while budget > 0 && !self.contains(target) {
            let level = (1..=9)
                .map(|i| 1 << i)
                .find(|v| !self.levels.contains_key(v))
                .unwrap();
            let mut job = self
                .build
                .take()
                .filter(|b| b.level == level)
                .unwrap_or(Build {
                    level,
                    position: 0,
                    rows: Vec::new(),
                });
            let length = if level == 2 {
                rows.len()
            } else {
                self.levels[&(level / 2)].len()
            };
            let input = |i: usize| {
                if level == 2 {
                    rows[i].bar(i)
                } else {
                    self.levels[&(level / 2)][i].clone()
                }
            };
            while job.position < length && budget > 0 {
                let a = input(job.position);
                if job.position + 1 < length {
                    job.rows.push(a.merge(&input(job.position + 1)));
                } else if let Some(last) = job.rows.pop() {
                    job.rows.push(last.merge(&a));
                } else {
                    job.rows.push(a);
                }
                job.position += 2;
                budget -= 1;
                #[cfg(test)]
                {
                    self.work += 1;
                }
            }
            if job.position >= length {
                self.levels.insert(level, job.rows);
            } else {
                self.build = Some(job);
            }
        }
        initial - budget
    }
    pub fn update_last<R: Input>(&mut self, rows: &[R]) {
        self.build = None;
        for bars in self.levels.values_mut() {
            if let Some(last) = bars.last_mut() {
                let start = last.first;
                let mut merged = rows[start].bar(start);
                for (i, row) in rows.iter().enumerate().skip(start + 1) {
                    merged = merged.merge(&row.bar(i));
                }
                *last = merged;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_oracle() {
        for length in [0, 1, 2, 3, 7, 8, 9, 15, 16, 17, 31, 32, 33, 129] {
            let rows: Vec<_> = (0..length)
                .map(|i| Bar {
                    first: i,
                    end: i + 1,
                    index: (i as f64 * 2.).into(),
                    value: [i as f64, i as f64 + 5., i as f64 - 3., i as f64 + 1.],
                    count: i % 3 + 1,
                })
                .collect();
            for level in [1, 2, 3, 4, 5, 8, 16, 32, 64, 128, 256, 512, 4096] {
                let mut cache = ConflationCache::default();
                cache.ensure(&rows, level);
                let actual = if level == 1 {
                    &rows[..]
                } else {
                    cache.rows(level)
                };
                let encode = |bars: &[Bar]| {
                    bars.iter()
                        .map(|r| {
                            vec![
                                r.index.value(),
                                r.value[0],
                                r.value[1],
                                r.value[2],
                                r.value[3],
                                r.count as f64,
                            ]
                        })
                        .collect::<Vec<_>>()
                };
                println!(
                    "CONFLATION {{\"factor\":{level},\"rows\":{:?},\"actual\":{:?},\"line\":{:?}}}",
                    encode(&rows),
                    encode(actual),
                    actual
                        .iter()
                        .map(|r| line_value(r.value, r.count))
                        .collect::<Vec<_>>()
                );
            }
        }
    }

    #[test]
    fn bounded_builds_and_latest_repair_match_fresh_results() {
        for count in 1..50 {
            let mut rows: Vec<_> = (0..count)
                .map(|i| Bar {
                    first: i,
                    end: i + 1,
                    index: (i as f64).into(),
                    value: [i as f64, i as f64 + 5., i as f64 - 3., i as f64],
                    count: 1,
                })
                .collect();
            let mut cache = ConflationCache::default();
            while !cache.contains(512) {
                let before = cache.work;
                cache.step(&rows, 512, 3);
                assert!(cache.work - before <= 3);
            }
            let work = cache.work;
            cache.ensure(&rows, 512);
            assert_eq!(cache.work, work);
            rows.last_mut().unwrap().value = [3., 99., -8., 20.];
            cache.update_last(&rows);
            let mut fresh = ConflationCache::default();
            fresh.ensure(&rows, 512);
            for level in (1..=9).map(|i| 1 << i) {
                assert_eq!(
                    cache.rows(level),
                    fresh.rows(level),
                    "count={count}, level={level}"
                );
            }
        }
    }

    #[test]
    fn dense_counts_storage_and_factor_source_cases() {
        let rows: Vec<_> = (0..8192)
            .map(|i| Bar {
                first: i,
                end: i + 1,
                index: (i as f64).into(),
                value: [1., 2., 0., 1.],
                count: 1,
            })
            .collect();
        let mut cache = ConflationCache::default();
        cache.ensure(&rows, 8);
        assert_eq!(cache.rows(8).len(), 1024);
        assert!(cache.rows(8).iter().all(|r| r.count == 8));
        cache.ensure(&rows, 512);
        assert!(cache.levels.values().map(Vec::len).sum::<usize>() < rows.len());
        for spacing in [0.00001, 0.125, 0.5, 1., 4.] {
            for dpr in [1., 1.25, 1.5, 2.] {
                for threshold in [0.25, 1., 4.] {
                    println!(
                        "CONFLATION_FACTOR {{\"spacing\":{spacing},\"dpr\":{dpr},\"threshold\":{threshold},\"actual\":{}}}",
                        factor(spacing, dpr, threshold)
                    );
                }
            }
        }
    }
    #[test]
    fn recursive_remainders_counts_identity_and_extremes() {
        let rows: Vec<_> = (0..7)
            .map(|i| Bar {
                first: i,
                end: i + 1,
                index: (i as f64 * 2.).into(),
                value: [i as f64, i as f64 + 5., i as f64 - 3., i as f64 + 1.],
                count: 1,
            })
            .collect();
        let mut cache = ConflationCache::default();
        cache.ensure(&rows, 2);
        let bars = cache.rows(2);
        assert_eq!(bars.iter().map(|r| r.count).collect::<Vec<_>>(), [2, 2, 3]);
        assert_eq!(bars[2].first, 4);
        assert_eq!(bars[2].value, [4., 11., 1., 7.]);
        cache.ensure(&rows, 4);
        assert_eq!(cache.rows(4).len(), 1);
        assert_eq!(cache.rows(4)[0].count, 7);
        assert_eq!(cache.rows(4)[0].index.value(), 0.);
        assert_eq!(factor(0.5, 1., 1.), 2);
        assert_eq!(factor(0.5, 2., 1.), 1);
        assert_eq!(factor(0.00001, 1., 1.), 512);
    }
}
