//! Tick-mark generation parameterized by price-scale coordinate callbacks.
use crate::{
    helpers::mathex::min,
    model::{
        coordinate::Coordinate, price_scale::PriceMark,
        price_tick_span_calculator::PriceTickSpanCalculator,
    },
};
pub struct PriceTickMarkBuilder {
    base: f64,
    font_size: f64,
    density: f64,
}
impl PriceTickMarkBuilder {
    pub fn new(base: f64, font_size: f64, density: f64) -> Self {
        Self {
            base,
            font_size,
            density,
        }
    }
    pub fn tick_span(&self, high: f64, low: f64, height: f64) -> f64 {
        assert!(high >= low, "high < low");
        let max = (high - low) * (self.font_size * self.density).ceil() / height.max(1.);
        min(&[
            PriceTickSpanCalculator::new(self.base, vec![2., 2.5, 2.]).tick_span(high, low, max),
            PriceTickSpanCalculator::new(self.base, vec![2., 2., 2.5]).tick_span(high, low, max),
            PriceTickSpanCalculator::new(self.base, vec![2.5, 2., 2.]).tick_span(high, low, max),
        ])
    }
    pub fn build(
        &self,
        first: f64,
        height: f64,
        low: f64,
        high: f64,
        mut coordinate: impl FnMut(f64, f64) -> Coordinate,
        mut label: impl FnMut(f64) -> String,
    ) -> Vec<PriceMark> {
        if high == low {
            return vec![];
        }
        let span = self.tick_span(high, low, height);
        let mut logical = high - high.rem_euclid(span);
        let mut previous = None;
        let mut marks = vec![];
        while logical > low {
            let coord = coordinate(logical, first);
            if previous
                .is_none_or(|old: f64| (coord.value() - old).abs() >= self.font_size * self.density)
            {
                marks.push(PriceMark {
                    coord,
                    label: label(logical),
                    logical,
                });
                previous = Some(coord.value())
            }
            logical -= span;
        }
        marks
    }
}
