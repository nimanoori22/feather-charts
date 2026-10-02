//! Bounded FIFO cache for formatted horizontal-axis labels.

use std::{collections::HashMap, collections::VecDeque, hash::Hash};

/// Caches formatted labels by a horizontal-scale behavior's cache key.
///
/// This intentionally uses insertion-order eviction rather than LRU: reading
/// an existing label must not keep it alive longer. That matches the label
/// stability and bounded-allocation behavior of Lightweight Charts.
#[derive(Clone, Debug)]
pub struct FormattedLabelsCache<Key> {
    max_size: usize,
    labels: HashMap<Key, String>,
    insertion_order: VecDeque<Key>,
}

impl<Key> FormattedLabelsCache<Key>
where
    Key: Clone + Eq + Hash,
{
    /// The source component's default bound.
    pub const DEFAULT_MAX_SIZE: usize = 50;

    /// Creates an empty cache with a fixed positive capacity.
    pub fn new(max_size: usize) -> Self {
        assert!(max_size > 0, "formatted-label cache size must be positive");

        Self {
            max_size,
            labels: HashMap::with_capacity(max_size),
            insertion_order: VecDeque::with_capacity(max_size),
        }
    }

    /// Returns the label for `key`, computing and inserting it only on a miss.
    ///
    /// A hit does not alter insertion order. When full, the oldest inserted
    /// key is evicted before the new label is formatted and stored.
    pub fn format(&mut self, key: Key, formatter: impl FnOnce() -> String) -> String {
        if let Some(label) = self.labels.get(&key) {
            return label.clone();
        }

        if self.labels.len() == self.max_size {
            let oldest = self
                .insertion_order
                .pop_front()
                .expect("non-empty cache must retain its oldest key");
            self.labels.remove(&oldest);
        }

        let label = formatter();
        self.insertion_order.push_back(key.clone());
        self.labels.insert(key, label.clone());
        label
    }

    /// Returns the number of labels currently retained.
    pub fn len(&self) -> usize {
        self.labels.len()
    }

    /// Returns whether the cache has no retained labels.
    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }
}

impl<Key> Default for FormattedLabelsCache<Key>
where
    Key: Clone + Eq + Hash,
{
    fn default() -> Self {
        Self::new(Self::DEFAULT_MAX_SIZE)
    }
}

#[cfg(test)]
mod tests {
    use super::FormattedLabelsCache;
    use std::cell::Cell;

    #[test]
    fn formats_a_key_only_once_while_it_is_cached() {
        let mut cache = FormattedLabelsCache::new(2);
        let calls = Cell::new(0);

        assert_eq!(
            cache.format("day", || {
                calls.set(calls.get() + 1);
                "01 Jan".to_owned()
            }),
            "01 Jan"
        );
        assert_eq!(
            cache.format("day", || {
                calls.set(calls.get() + 1);
                "incorrect replacement".to_owned()
            }),
            "01 Jan"
        );
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn evicts_by_insertion_order_not_by_recent_reads() {
        let mut cache = FormattedLabelsCache::new(2);
        cache.format("first", || "first label".to_owned());
        cache.format("second", || "second label".to_owned());
        cache.format("first", || "should remain cached".to_owned());
        cache.format("third", || "third label".to_owned());

        let recomputed_first = Cell::new(0);
        assert_eq!(
            cache.format("first", || {
                recomputed_first.set(recomputed_first.get() + 1);
                "new first label".to_owned()
            }),
            "new first label"
        );
        assert_eq!(recomputed_first.get(), 1);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    #[should_panic(expected = "formatted-label cache size must be positive")]
    fn rejects_zero_capacity() {
        let _ = FormattedLabelsCache::<String>::new(0);
    }
}
