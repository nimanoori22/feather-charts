//! The synchronous contract between chart data and a horizontal scale.

use crate::model::{
    data_consumer::TimedData,
    localization_options::LocalizationOptions,
    tick_marks::TickMark,
    time_data::{TickMarkWeightValue, TimeScalePoint},
};

/// Converts a public horizontal item into the behavior's internal form.
pub trait HorzScaleItemConverter<Item, InternalItem> {
    fn convert(&mut self, item: &Item) -> InternalItem;
}

impl<Item, InternalItem, F> HorzScaleItemConverter<Item, InternalItem> for F
where
    F: FnMut(&Item) -> InternalItem,
{
    fn convert(&mut self, item: &Item) -> InternalItem {
        self(item)
    }
}

/// A formatted horizontal-axis mark prepared for display.
#[derive(Clone, Debug, PartialEq)]
pub struct TimeMark {
    pub need_align_coordinate: bool,
    pub coordinate: f64,
    pub label: String,
    pub weight: TickMarkWeightValue,
}

/// Behavior for converting, ordering, weighting, and formatting a horizontal
/// scale. It is deliberately synchronous and backend-independent.
pub trait HorzScaleBehavior {
    type Item: Clone;
    type InternalItem: Clone;
    type Key: Clone;
    type CacheKey: Clone;
    type DataItem: TimedData<Item = Self::Item>;
    type Options;
    type Converter: HorzScaleItemConverter<Self::Item, Self::InternalItem>;

    fn options(&self) -> &Self::Options;
    fn set_options(&mut self, options: Self::Options);
    fn preprocess_data(&mut self, data: &mut [Self::DataItem]);
    fn to_internal(&self, item: &Self::Item) -> Self::InternalItem;
    fn create_converter_to_internal(&mut self, data: &[Self::DataItem]) -> Self::Converter;
    fn key(&self, item: &Self::InternalItem) -> Self::Key;
    fn cache_key(&self, item: &Self::InternalItem) -> Self::CacheKey;
    fn update_formatter(&mut self, options: &LocalizationOptions<Self::Item>);
    fn format_item(&self, item: &Self::InternalItem) -> String;
    fn format_tickmark(
        &self,
        item: &TickMark<Self::InternalItem, Self::Item>,
        localization_options: &LocalizationOptions<Self::Item>,
    ) -> String;
    fn max_tick_mark_weight(&self, marks: &[TimeMark]) -> TickMarkWeightValue;
    fn fill_weights_for_points(
        &mut self,
        sorted_time_points: &mut [TimeScalePoint<Self::InternalItem, Self::Item>],
        start_index: usize,
    );

    /// Returns whether cached tick labels must be recalculated for this set.
    fn should_reset_tickmark_labels(
        &self,
        _tick_marks: &[TickMark<Self::InternalItem, Self::Item>],
    ) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{
        data_consumer::WhitespaceData,
        localization_options::LocalizationOptions,
        tick_marks::TickMark,
        time_data::{TickMarkWeightValue, TimePointIndex, TimeScalePoint},
    };

    use super::{HorzScaleBehavior, HorzScaleItemConverter, TimeMark};

    struct NumberBehavior {
        options: (),
    }

    impl HorzScaleBehavior for NumberBehavior {
        type Item = f64;
        type InternalItem = f64;
        type Key = f64;
        type CacheKey = f64;
        type DataItem = WhitespaceData<f64>;
        type Options = ();
        type Converter = fn(&f64) -> f64;

        fn options(&self) -> &Self::Options {
            &self.options
        }
        fn set_options(&mut self, options: Self::Options) {
            self.options = options;
        }
        fn preprocess_data(&mut self, _data: &mut [Self::DataItem]) {}
        fn to_internal(&self, item: &Self::Item) -> Self::InternalItem {
            *item
        }
        fn create_converter_to_internal(&mut self, _data: &[Self::DataItem]) -> Self::Converter {
            |item| *item
        }
        fn key(&self, item: &Self::InternalItem) -> Self::Key {
            *item
        }
        fn cache_key(&self, item: &Self::InternalItem) -> Self::CacheKey {
            *item
        }
        fn update_formatter(&mut self, _options: &LocalizationOptions<Self::Item>) {}
        fn format_item(&self, item: &Self::InternalItem) -> String {
            item.to_string()
        }
        fn format_tickmark(
            &self,
            item: &TickMark<Self::InternalItem, Self::Item>,
            _options: &LocalizationOptions<Self::Item>,
        ) -> String {
            item.time.to_string()
        }
        fn max_tick_mark_weight(&self, marks: &[TimeMark]) -> TickMarkWeightValue {
            marks
                .iter()
                .map(|mark| mark.weight)
                .max()
                .unwrap_or_default()
        }
        fn fill_weights_for_points(
            &mut self,
            points: &mut [TimeScalePoint<Self::InternalItem, Self::Item>],
            _start_index: usize,
        ) {
            for point in points {
                point.time_weight = TickMarkWeightValue::new(1);
            }
        }
    }

    #[test]
    fn behavior_contract_supports_conversion_formatting_and_weighting() {
        let mut behavior = NumberBehavior { options: () };
        let mut converter = behavior.create_converter_to_internal(&[]);
        assert_eq!(converter.convert(&2.5), 2.5);
        assert_eq!(behavior.key(&behavior.to_internal(&2.5)), 2.5);

        let options = LocalizationOptions::new("en-US", "dd MMM 'yy");
        let mark = TickMark {
            index: TimePointIndex::new(0.0),
            time: 2.5,
            weight: TickMarkWeightValue::new(3),
            original_time: 2.5,
        };
        assert_eq!(behavior.format_tickmark(&mark, &options), "2.5");
        assert!(!behavior.should_reset_tickmark_labels(&[mark]));

        let mut points = [TimeScalePoint {
            time_weight: TickMarkWeightValue::default(),
            time: 2.5,
            original_time: 2.5,
        }];
        behavior.fill_weights_for_points(&mut points, 0);
        assert_eq!(points[0].time_weight, TickMarkWeightValue::new(1));
    }
}
