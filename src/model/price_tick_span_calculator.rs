use crate::helpers::mathex::{equal, greater_or_equal, is_base_decimal};
pub struct PriceTickSpanCalculator {
    base: f64,
    integral: Vec<f64>,
    fractional: Vec<f64>,
}
impl PriceTickSpanCalculator {
    pub fn new(base: f64, integral: Vec<f64>) -> Self {
        let fractional = if is_base_decimal(base) {
            vec![2.0, 2.5, 2.0]
        } else {
            let mut v = base;
            let mut r = vec![];
            while v != 1.0 {
                if v % 2.0 == 0.0 {
                    r.push(2.0);
                    v /= 2.0
                } else if v % 5.0 == 0.0 {
                    r.extend([2.0, 2.5]);
                    v /= 5.0
                } else {
                    panic!("unexpected base")
                }
            }
            r
        };
        Self {
            base,
            integral,
            fractional,
        }
    }
    pub fn tick_span(&self, high: f64, low: f64, max: f64) -> f64 {
        let min = if self.base == 0.0 {
            0.0
        } else {
            1.0 / self.base
        };
        let mut result = 10_f64.powf((high - low).log10().ceil().max(0.0));
        let mut i = 0;
        let mut c = self.integral[0];
        while greater_or_equal(result, min, 1e-14)
            && result > min + 1e-14
            && greater_or_equal(result, max * c, 1e-14)
            && result >= 1.0
        {
            result /= c;
            i = (i + 1) % self.integral.len();
            c = self.integral[i]
        }
        if result <= min + 1e-14 {
            result = min
        }
        result = result.max(1.0);
        if !self.fractional.is_empty() && equal(result, 1.0, 1e-14) {
            i = 0;
            c = self.fractional[0];
            while greater_or_equal(result, max * c, 1e-14) && result > min + 1e-14 {
                result /= c;
                i = (i + 1) % self.fractional.len();
                c = self.fractional[i]
            }
        }
        result
    }
}
