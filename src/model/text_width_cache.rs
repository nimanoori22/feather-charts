//! Cached text measurements used by axis and marker renderers.
//!
//! The chart engine does not own a drawing backend, so a backend supplies text
//! metrics through [`TextMeasurer`]. The cache deliberately keeps FIFO eviction
//! and digit normalization from Lightweight Charts.

use std::collections::{HashMap, VecDeque};

/// Metrics required by chart label layout.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextMetrics {
    pub width: f32,
    pub actual_bounding_box_ascent: Option<f32>,
    pub actual_bounding_box_descent: Option<f32>,
}

/// A drawing backend capable of measuring text in its current font settings.
pub trait TextMeasurer {
    fn measure_text(&mut self, text: &str) -> TextMetrics;
}

/// A bounded FIFO cache for text metrics.
///
/// Digits `2` through `9` are normalized to `0` by the default methods, as in
/// Lightweight Charts. This shares measurements for numeric labels that are
/// expected to have equal-width digits in the chart font.
#[derive(Debug)]
pub struct TextWidthCache {
    max_size: usize,
    cache: HashMap<String, TextMetrics>,
    insertion_order: VecDeque<String>,
}

impl Default for TextWidthCache {
    fn default() -> Self {
        Self::new(50)
    }
}

impl TextWidthCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            cache: HashMap::new(),
            insertion_order: VecDeque::new(),
        }
    }

    pub fn reset(&mut self) {
        self.cache.clear();
        self.insertion_order.clear();
    }

    pub fn measure_text<M: TextMeasurer>(&mut self, measurer: &mut M, text: &str) -> f32 {
        self.metrics(measurer, text).width
    }

    pub fn y_mid_correction<M: TextMeasurer>(&mut self, measurer: &mut M, text: &str) -> f32 {
        let metrics = self.metrics(measurer, text);
        (metrics.actual_bounding_box_ascent.unwrap_or_default()
            - metrics.actual_bounding_box_descent.unwrap_or_default())
            / 2.0
    }

    /// Measures with a caller-supplied cache-key normalizer.
    pub fn measure_text_with<M: TextMeasurer>(
        &mut self,
        measurer: &mut M,
        text: &str,
        normalize: impl FnOnce(&str) -> String,
    ) -> f32 {
        self.metrics_with(measurer, text, normalize).width
    }

    /// Computes vertical centering correction with a caller-supplied cache-key normalizer.
    pub fn y_mid_correction_with<M: TextMeasurer>(
        &mut self,
        measurer: &mut M,
        text: &str,
        normalize: impl FnOnce(&str) -> String,
    ) -> f32 {
        let metrics = self.metrics_with(measurer, text, normalize);
        (metrics.actual_bounding_box_ascent.unwrap_or_default()
            - metrics.actual_bounding_box_descent.unwrap_or_default())
            / 2.0
    }

    fn metrics<M: TextMeasurer>(&mut self, measurer: &mut M, text: &str) -> TextMetrics {
        self.metrics_with(measurer, text, normalize_numeric_label)
    }

    fn metrics_with<M: TextMeasurer>(
        &mut self,
        measurer: &mut M,
        text: &str,
        normalize: impl FnOnce(&str) -> String,
    ) -> TextMetrics {
        let key = normalize(text);

        if let Some(metrics) = self.cache.get(&key) {
            return *metrics;
        }

        let metrics = measurer.measure_text(&key);

        // Preserve the browser implementation's workaround for zero-sized
        // canvases: nonempty text with width zero must be measured again later.
        if self.max_size == 0 || (metrics.width == 0.0 && !text.is_empty()) {
            return metrics;
        }

        if self.cache.len() == self.max_size {
            let oldest = self
                .insertion_order
                .pop_front()
                .expect("cache order matches cached entries");
            self.cache.remove(&oldest);
        }

        self.insertion_order.push_back(key.clone());
        self.cache.insert(key, metrics);
        metrics
    }
}

fn normalize_numeric_label(text: &str) -> String {
    text.chars()
        .map(|character| match character {
            '2'..='9' => '0',
            _ => character,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeMeasurer {
        calls: Vec<String>,
        zero_width: bool,
    }

    impl TextMeasurer for FakeMeasurer {
        fn measure_text(&mut self, text: &str) -> TextMetrics {
            self.calls.push(text.to_owned());
            TextMetrics {
                width: if self.zero_width {
                    0.0
                } else {
                    text.len() as f32
                },
                actual_bounding_box_ascent: Some(6.0),
                actual_bounding_box_descent: Some(2.0),
            }
        }
    }

    #[test]
    fn caches_fifo_and_does_not_refresh_entries_on_hits() {
        let mut cache = TextWidthCache::new(3);
        let mut measurer = FakeMeasurer::default();

        for text in ["foo", "bar", "baz", "baz", "bar", "foo", "quux", "foo"] {
            cache.measure_text(&mut measurer, text);
        }

        assert_eq!(measurer.calls, ["foo", "bar", "baz", "quux", "foo"]);
    }

    #[test]
    fn nonempty_zero_width_text_is_not_cached_but_empty_text_is() {
        let mut cache = TextWidthCache::new(3);
        let mut measurer = FakeMeasurer {
            zero_width: true,
            ..FakeMeasurer::default()
        };

        cache.measure_text(&mut measurer, "");
        cache.measure_text(&mut measurer, "not empty");
        cache.measure_text(&mut measurer, "");
        cache.measure_text(&mut measurer, "not empty");

        assert_eq!(measurer.calls, ["", "not empty", "not empty"]);
    }

    #[test]
    fn normalizes_numeric_labels_and_allows_custom_normalization() {
        let mut cache = TextWidthCache::default();
        let mut measurer = FakeMeasurer::default();

        assert_eq!(cache.measure_text(&mut measurer, "test2345"), 8.0);
        assert_eq!(cache.measure_text(&mut measurer, "test6789"), 8.0);
        assert_eq!(measurer.calls, ["test0000"]);

        cache.measure_text_with(&mut measurer, "test01234", |value| {
            value
                .chars()
                .map(|character| {
                    if character.is_ascii_digit() {
                        '0'
                    } else {
                        character
                    }
                })
                .collect()
        });
        cache.measure_text_with(&mut measurer, "test56789", |value| {
            value
                .chars()
                .map(|character| {
                    if character.is_ascii_digit() {
                        '0'
                    } else {
                        character
                    }
                })
                .collect()
        });

        assert_eq!(measurer.calls, ["test0000", "test00000"]);
    }

    #[test]
    fn returns_vertical_centering_correction_and_reset_forgets_measurements() {
        let mut cache = TextWidthCache::default();
        let mut measurer = FakeMeasurer::default();

        assert_eq!(cache.y_mid_correction(&mut measurer, "label"), 2.0);
        cache.measure_text(&mut measurer, "label");
        assert_eq!(measurer.calls, ["label"]);

        cache.reset();
        cache.measure_text(&mut measurer, "label");
        assert_eq!(measurer.calls, ["label", "label"]);
    }
}
