//! Price/strike horizontal items, independent of calendar time and yield maturities.
use super::{
    data_consumer::TimedData,
    ihorz_scale_behavior::{HorzScaleBehavior, HorzScaleItemConverter, TimeMark},
    localization_options::LocalizationOptions,
    tick_marks::TickMark,
    time_data::{TickMarkWeightValue, TimeScalePoint},
};
use crate::formatters::fixed::fixed;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HorizontalPrice(f64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionsChartError {
    NonFinitePrice,
    InvalidPrecision,
}
impl std::fmt::Display for OptionsChartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFinitePrice => f.write_str("horizontal price must be finite"),
            Self::InvalidPrecision => f.write_str("horizontal precision must be at most 100"),
        }
    }
}
impl std::error::Error for OptionsChartError {}
impl HorizontalPrice {
    pub fn new(value: f64) -> Result<Self, OptionsChartError> {
        if !value.is_finite() {
            return Err(OptionsChartError::NonFinitePrice);
        }
        Ok(Self(if value == 0. { 0. } else { value }))
    }
    pub fn value(self) -> f64 {
        self.0
    }
}
impl Eq for HorizontalPrice {}
impl PartialOrd for HorizontalPrice {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for HorizontalPrice {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}
impl std::hash::Hash for HorizontalPrice {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.0.to_bits(), state);
    }
}
/// Horizontal decimal places. Zero matches omitted upstream localization.precision.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OptionsChartOptions {
    pub precision: u8,
}
impl OptionsChartOptions {
    pub fn validate(self) -> Result<(), OptionsChartError> {
        if self.precision > 100 {
            Err(OptionsChartError::InvalidPrecision)
        } else {
            Ok(())
        }
    }
}
#[derive(Default)]
pub struct HorzScaleBehaviorPrice {
    options: OptionsChartOptions,
}
pub struct PriceConverter;
impl HorzScaleItemConverter<HorizontalPrice, HorizontalPrice> for PriceConverter {
    type Error = OptionsChartError;
    fn convert(&mut self, value: &HorizontalPrice) -> Result<HorizontalPrice, Self::Error> {
        Ok(*value)
    }
}
impl HorzScaleBehavior for HorzScaleBehaviorPrice {
    type Item = HorizontalPrice;
    type InternalItem = HorizontalPrice;
    type Key = HorizontalPrice;
    type CacheKey = HorizontalPrice;
    type Options = OptionsChartOptions;
    type Converter = PriceConverter;
    type Error = OptionsChartError;
    fn options(&self) -> &Self::Options {
        &self.options
    }
    fn set_options(&mut self, options: Self::Options) {
        self.options = options;
    }
    fn preprocess_data<D: TimedData<Item = Self::Item>>(
        &mut self,
        _: &mut [D],
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    fn to_internal(&self, item: &Self::Item) -> Result<Self::InternalItem, Self::Error> {
        Ok(*item)
    }
    fn create_converter_to_internal<D: TimedData<Item = Self::Item>>(
        &mut self,
        _: &[D],
    ) -> Result<Self::Converter, Self::Error> {
        Ok(PriceConverter)
    }
    fn key(&self, item: &Self::InternalItem) -> Self::Key {
        *item
    }
    fn cache_key(&self, item: &Self::InternalItem) -> Self::CacheKey {
        *item
    }
    fn update_formatter(&mut self, _: &LocalizationOptions<Self::Item>) {}
    fn format_item(&self, item: &Self::InternalItem) -> String {
        fixed(item.0, self.options.precision)
    }
    fn format_tickmark(
        &self,
        item: &TickMark<Self::InternalItem, Self::Item>,
        _: &LocalizationOptions<Self::Item>,
    ) -> String {
        self.format_item(&item.time)
    }
    fn max_tick_mark_weight(&self, marks: &[TimeMark]) -> TickMarkWeightValue {
        marks.iter().map(|m| m.weight).max().unwrap_or_default()
    }
    fn fill_weights_for_points(
        &mut self,
        points: &mut [TimeScalePoint<Self::InternalItem, Self::Item>],
        start: usize,
    ) {
        for point in points.iter_mut().skip(start) {
            point.time_weight = weight(point.time.0);
        }
    }
    fn append_weight(
        &mut self,
        _: Option<&Self::InternalItem>,
        current: &Self::InternalItem,
    ) -> Option<TickMarkWeightValue> {
        Some(weight(current.0))
    }
}
fn weight(price: f64) -> TickMarkWeightValue {
    for (step, priority) in [(100., 8), (50., 7), (25., 6), (10., 5), (5., 4), (1., 3)] {
        if price == (price / step).ceil() * step {
            return TickMarkWeightValue::new(priority);
        }
    }
    TickMarkWeightValue::new(if price * 2. == (price * 2.).ceil() {
        1
    } else {
        0
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn options_chart_source_reference() {
        let mut behavior = HorzScaleBehaviorPrice::default();
        let localization = LocalizationOptions::new("en-US", "");
        for line in include_str!("../../docs/porting/options-chart-cases.tsv").lines() {
            let fields = line.split('\t').collect::<Vec<_>>();
            let value = f64::from_bits(u64::from_str_radix(fields[0], 16).unwrap());
            let price = HorizontalPrice::new(value).unwrap();
            behavior.set_options(OptionsChartOptions {
                precision: fields[1].parse().unwrap(),
            });
            assert_eq!(
                behavior.format_item(&price),
                fields[3],
                "{value} precision {}",
                fields[1]
            );
            let mark = TickMark {
                index: super::super::time_data::TimePointIndex::new(0.),
                time: price,
                original_time: price,
                weight: Default::default(),
            };
            assert_eq!(behavior.format_tickmark(&mark, &localization), fields[3]);
            assert_eq!(
                weight(value).value(),
                fields[2].parse::<i32>().unwrap(),
                "{value}"
            );
            assert_eq!(behavior.append_weight(None, &price), Some(weight(value)));
        }
    }
    #[test]
    fn price_validation_order_hash_and_partial_weights() {
        use std::{
            collections::hash_map::DefaultHasher,
            hash::{Hash, Hasher},
        };
        let zero = HorizontalPrice::new(0.).unwrap();
        let minus_zero = HorizontalPrice::new(-0.).unwrap();
        assert_eq!(zero, minus_zero);
        let hash = |p: HorizontalPrice| {
            let mut h = DefaultHasher::new();
            p.hash(&mut h);
            h.finish()
        };
        assert_eq!(hash(zero), hash(minus_zero));
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(HorizontalPrice::new(invalid).is_err());
        }
        assert!(HorizontalPrice::new(-5.).unwrap() < zero);
        assert!(OptionsChartOptions { precision: 101 }.validate().is_err());
        let mut points = [0., 25., 0.5].map(|v| TimeScalePoint {
            time: HorizontalPrice::new(v).unwrap(),
            original_time: HorizontalPrice::new(v).unwrap(),
            time_weight: TickMarkWeightValue::new(-1),
        });
        let mut b = HorzScaleBehaviorPrice::default();
        b.fill_weights_for_points(&mut points, 1);
        assert_eq!(points.map(|p| p.time_weight.value()), [-1, 6, 1]);
        assert_eq!(b.max_tick_mark_weight(&[]), Default::default());
    }
}
