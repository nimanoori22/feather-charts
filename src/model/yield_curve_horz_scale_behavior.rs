//! Numeric maturities use months and never pass through calendar conversion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Maturity(f64);
#[derive(Clone, Debug, PartialEq)]
pub enum YieldCurveError {
    InvalidMaturity,
    InvalidOptions,
    ScaffoldTooLarge,
}
impl Maturity {
    pub fn from_months(months: f64) -> Result<Self, YieldCurveError> {
        if !months.is_finite() {
            return Err(YieldCurveError::InvalidMaturity);
        }
        Ok(Self(if months == 0. { 0. } else { months }))
    }
    pub fn months(self) -> f64 {
        self.0
    }
    pub fn label(self) -> String {
        if self.0 < 12. {
            return format!("{}M", self.0);
        }
        let years = (self.0 / 12.).floor();
        let remainder = self.0 % 12.;
        if remainder == 0. {
            format!("{years}Y")
        } else {
            format!("{years}Y{remainder}M")
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_maturity_uses_years_and_months_instead_of_calendar_dates() {
        assert_eq!(Maturity::from_months(18.).unwrap().label(), "1Y6M");
        assert_eq!(Maturity::from_months(0.).unwrap().label(), "0M");
        assert!(Maturity::from_months(f64::NAN).is_err());
    }
}

impl Eq for Maturity {}
impl PartialOrd for Maturity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Maturity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}
impl std::hash::Hash for Maturity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}
impl std::fmt::Display for YieldCurveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for YieldCurveError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct YieldCurveOptions {
    pub base_resolution: f64,
    pub minimum_time_range: f64,
    pub start_time_range: f64,
}
impl Default for YieldCurveOptions {
    fn default() -> Self {
        Self {
            base_resolution: 1.,
            minimum_time_range: 120.,
            start_time_range: 0.,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScaffoldState {
    pub start: f64,
    pub end: f64,
    pub resolution: f64,
    pub count: usize,
}
impl YieldCurveOptions {
    pub(crate) fn scaffold(self, last: f64) -> Result<ScaffoldState, YieldCurveError> {
        if ![
            self.base_resolution,
            self.minimum_time_range,
            self.start_time_range,
            last,
        ]
        .iter()
        .all(|v| v.is_finite())
        {
            return Err(YieldCurveError::InvalidOptions);
        }
        let start = self.start_time_range.max(0.);
        let end = self.minimum_time_range.max(0.).max(last);
        let resolution = self.base_resolution.max(1.);
        let count = (((end - start) / resolution).floor() + 1.).max(0.);
        if !count.is_finite() || count > 100_000. {
            return Err(YieldCurveError::ScaffoldTooLarge);
        }
        Ok(ScaffoldState {
            start,
            end,
            resolution,
            count: count as usize,
        })
    }
}
impl ScaffoldState {
    pub fn items(self) -> impl Iterator<Item = Maturity> {
        (0..self.count).map(move |i| Maturity(self.start + i as f64 * self.resolution))
    }
}
use crate::model::{
    data_consumer::TimedData,
    ihorz_scale_behavior::{HorzScaleBehavior, HorzScaleItemConverter, TimeMark},
    localization_options::{LocalizationOptions, TimeFormatterFn},
    tick_marks::TickMark,
    time_data::{TickMarkWeightValue, TimeScalePoint},
};
#[derive(Default)]
pub struct YieldCurveHorzScaleBehavior {
    options: YieldCurveOptions,
    formatter: Option<TimeFormatterFn<Maturity>>,
}
pub struct MaturityConverter;
impl HorzScaleItemConverter<Maturity, Maturity> for MaturityConverter {
    type Error = YieldCurveError;
    fn convert(&mut self, item: &Maturity) -> Result<Maturity, YieldCurveError> {
        Ok(*item)
    }
}
fn weight(item: Maturity) -> TickMarkWeightValue {
    for (divisor, value) in [
        (120., 10),
        (60., 9),
        (36., 8),
        (12., 7),
        (6., 6),
        (3., 5),
        (1., 4),
    ] {
        if item.0 % divisor == 0. {
            return TickMarkWeightValue::new(value);
        }
    }
    TickMarkWeightValue::new(0)
}
impl HorzScaleBehavior for YieldCurveHorzScaleBehavior {
    type Item = Maturity;
    type InternalItem = Maturity;
    type Key = Maturity;
    type CacheKey = Maturity;
    type Options = YieldCurveOptions;
    type Converter = MaturityConverter;
    type Error = YieldCurveError;
    fn options(&self) -> &Self::Options {
        &self.options
    }
    fn set_options(&mut self, options: Self::Options) {
        self.options = options;
    }
    fn preprocess_data<D: TimedData<Item = Maturity>>(
        &mut self,
        _data: &mut [D],
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    fn to_internal(&self, item: &Maturity) -> Result<Maturity, Self::Error> {
        Ok(*item)
    }
    fn create_converter_to_internal<D: TimedData<Item = Maturity>>(
        &mut self,
        _data: &[D],
    ) -> Result<Self::Converter, Self::Error> {
        Ok(MaturityConverter)
    }
    fn key(&self, item: &Maturity) -> Maturity {
        *item
    }
    fn cache_key(&self, item: &Maturity) -> Maturity {
        *item
    }
    fn update_formatter(&mut self, options: &LocalizationOptions<Maturity>) {
        self.formatter = options.time_formatter.clone();
    }
    fn format_item(&self, item: &Maturity) -> String {
        self.formatter
            .as_ref()
            .map_or_else(|| item.label(), |f| f(item))
    }
    fn format_tickmark(
        &self,
        item: &TickMark<Maturity, Maturity>,
        _options: &LocalizationOptions<Maturity>,
    ) -> String {
        self.format_item(&item.time)
    }
    fn max_tick_mark_weight(&self, marks: &[TimeMark]) -> TickMarkWeightValue {
        marks.iter().map(|m| m.weight).max().unwrap_or_default()
    }
    fn fill_weights_for_points(
        &mut self,
        points: &mut [TimeScalePoint<Maturity, Maturity>],
        start: usize,
    ) {
        for p in points.iter_mut().skip(start) {
            p.time_weight = weight(p.time);
        }
    }
    fn append_weight(
        &mut self,
        _previous: Option<&Maturity>,
        current: &Maturity,
    ) -> Option<TickMarkWeightValue> {
        Some(weight(*current))
    }
}
