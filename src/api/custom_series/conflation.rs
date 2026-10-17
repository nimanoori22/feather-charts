use super::*;
use std::collections::BTreeMap;
type Row<V> = crate::model::series_data::CustomPlotRow<
    <V as CustomSeriesPaneView<Frame>>::InternalTime,
    <V as CustomSeriesPaneView<Frame>>::Item,
    <V as CustomSeriesPaneView<Frame>>::Data,
>;
struct Job<V: CustomSeriesPaneView<Frame>> {
    level: usize,
    position: usize,
    rows: Vec<Row<V>>,
}
pub(super) struct Cache<V: CustomSeriesPaneView<Frame>> {
    generation: Option<u64>,
    levels: BTreeMap<usize, Vec<Row<V>>>,
    unavailable: bool,
    job: Option<Job<V>>,
}
impl<V: CustomSeriesPaneView<Frame>> Default for Cache<V> {
    fn default() -> Self {
        Self {
            generation: None,
            levels: BTreeMap::new(),
            unavailable: false,
            job: None,
        }
    }
}
impl<V: CustomSeriesPaneView<Frame, Metadata = ()>> Cache<V> {
    pub fn contains(&self, generation: u64, level: usize) -> bool {
        self.generation == Some(generation)
            && (self.unavailable || self.levels.contains_key(&level))
    }
    pub fn rows(&self, level: usize) -> Option<&[Row<V>]> {
        self.levels.get(&level).map(Vec::as_slice)
    }
    pub fn ensure(
        &mut self,
        source: (u64, usize),
        target: usize,
        mut row: impl FnMut(usize) -> Row<V>,
        view: &V,
    ) {
        while !self.contains(source.0, target) {
            self.step(source, target, usize::MAX, &mut row, view);
        }
    }
    pub fn step(
        &mut self,
        source: (u64, usize),
        target: usize,
        mut budget: usize,
        mut row: impl FnMut(usize) -> Row<V>,
        view: &V,
    ) -> usize {
        let initial = budget;
        if self.generation != Some(source.0) {
            self.levels.clear();
            self.job = None;
            self.unavailable = false;
            self.generation = Some(source.0);
        }
        while budget > 0 && !self.contains(source.0, target) {
            let level = (1..=9)
                .map(|i| 1 << i)
                .find(|v| !self.levels.contains_key(v))
                .unwrap();
            let mut job = self.job.take().filter(|j| j.level == level).unwrap_or(Job {
                level,
                position: 0,
                rows: vec![],
            });
            let length = if level == 2 {
                source.1
            } else {
                self.levels[&(level / 2)].len()
            };
            while job.position < length && budget > 0 {
                let mut input = |i: usize| {
                    if level == 2 {
                        row(i)
                    } else {
                        self.levels[&(level / 2)][i].clone()
                    }
                };
                let a = input(job.position);
                let merged = if job.position + 1 < length {
                    merge(&a, &input(job.position + 1), view)
                } else if let Some(last) = job.rows.pop() {
                    merge(&last, &a, view)
                } else {
                    let mut a = a;
                    a.base.original_data_count = Some(a.base.original_data_count.unwrap_or(1));
                    Some(a)
                };
                budget -= 1;
                let Some(merged) = merged else {
                    self.levels.clear();
                    self.job = None;
                    self.unavailable = true;
                    return initial - budget;
                };
                job.rows.push(merged);
                job.position += 2;
            }
            if job.position >= length {
                self.levels.insert(level, job.rows);
            } else {
                self.job = Some(job);
            }
        }
        initial - budget
    }
}
fn merge<V>(a: &Row<V>, b: &Row<V>, view: &V) -> Option<Row<V>>
where
    V: CustomSeriesPaneView<Frame, Metadata = ()>,
{
    let context = |r: &Row<V>| CustomConflationContext {
        data: r.data.clone(),
        index: r.base.index,
        original_time: r.base.original_time.clone(),
        time: r.base.time.clone(),
        price_values: view.price_value_builder(&r.data),
    };
    let data = view.conflate(&context(a), &context(b))?;
    let prices = view.price_value_builder(&data);
    let close = prices.last().map_or(0., |p| p.value());
    let mut row = a.clone();
    row.base.value = [
        close,
        a.base.value[1].max(close),
        a.base.value[2].min(close),
        close,
    ];
    row.base.original_data_count =
        Some(a.base.original_data_count.unwrap_or(1) + b.base.original_data_count.unwrap_or(1));
    row.data = data;
    Some(row)
}
