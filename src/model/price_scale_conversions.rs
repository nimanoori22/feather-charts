use crate::model::price_range_impl::PriceRangeImpl;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogFormula {
    pub logical_offset: f64,
    pub coord_offset: f64,
}
pub const DEFAULT_LOG_FORMULA: LogFormula = LogFormula {
    logical_offset: 4.0,
    coord_offset: 0.0001,
};
pub fn from_percent(v: f64, b: f64) -> f64 {
    (if b < 0.0 { -v } else { v }) / 100.0 * b + b
}
pub fn to_percent(v: f64, b: f64) -> f64 {
    let r = 100.0 * (v - b) / b;
    if b < 0.0 { -r } else { r }
}
pub fn from_indexed_to_100(v: f64, b: f64) -> f64 {
    from_percent(v - 100.0, b)
}
pub fn to_indexed_to_100(v: f64, b: f64) -> f64 {
    to_percent(v, b) + 100.0
}
pub fn to_percent_range(r: PriceRangeImpl, b: f64) -> PriceRangeImpl {
    PriceRangeImpl::new(to_percent(r.min_value(), b), to_percent(r.max_value(), b))
}
pub fn to_indexed_to_100_range(r: PriceRangeImpl, b: f64) -> PriceRangeImpl {
    PriceRangeImpl::new(
        to_indexed_to_100(r.min_value(), b),
        to_indexed_to_100(r.max_value(), b),
    )
}
pub fn to_log(v: f64, f: LogFormula) -> f64 {
    if v.abs() < 1e-15 {
        0.0
    } else {
        let r = (v.abs() + f.coord_offset).log10() + f.logical_offset;
        if v < 0.0 { -r } else { r }
    }
}
pub fn from_log(v: f64, f: LogFormula) -> f64 {
    if v.abs() < 1e-15 {
        0.0
    } else {
        let r = 10_f64.powf(v.abs() - f.logical_offset) - f.coord_offset;
        if v < 0.0 { -r } else { r }
    }
}
pub fn convert_price_range_to_log(
    r: Option<PriceRangeImpl>,
    f: LogFormula,
) -> Option<PriceRangeImpl> {
    r.map(|r| PriceRangeImpl::new(to_log(r.min_value(), f), to_log(r.max_value(), f)))
}
pub fn convert_price_range_from_log(
    r: Option<PriceRangeImpl>,
    f: LogFormula,
) -> Option<PriceRangeImpl> {
    r.map(|r| PriceRangeImpl::new(from_log(r.min_value(), f), from_log(r.max_value(), f)))
}
pub fn can_convert_price_range_from_log(r: Option<PriceRangeImpl>, f: LogFormula) -> bool {
    convert_price_range_from_log(r, f)
        .is_some_and(|r| r.min_value().is_finite() && r.max_value().is_finite())
}
pub fn log_formula_for_price_range(r: Option<PriceRangeImpl>) -> LogFormula {
    let Some(r) = r else {
        return DEFAULT_LOG_FORMULA;
    };
    let diff = (r.max_value() - r.min_value()).abs();
    if !(1e-15..1.0).contains(&diff) {
        return DEFAULT_LOG_FORMULA;
    }
    let offset = 4.0 + diff.log10().abs().ceil();
    LogFormula {
        logical_offset: offset,
        coord_offset: 10_f64.powf(-offset),
    }
}
pub fn log_formulas_are_same(a: LogFormula, b: LogFormula) -> bool {
    a == b
}
