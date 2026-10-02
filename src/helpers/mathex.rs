pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}
pub fn is_base_decimal(value: f64) -> bool {
    if value < 0.0 {
        return false;
    }
    if value > 1e18 {
        return true;
    }
    let mut v = value;
    while v > 1.0 {
        if v % 10.0 != 0.0 {
            return false;
        }
        v /= 10.0;
    }
    true
}
pub fn greater_or_equal(a: f64, b: f64, epsilon: f64) -> bool {
    b - a <= epsilon
}
pub fn equal(a: f64, b: f64, epsilon: f64) -> bool {
    (a - b).abs() < epsilon
}
pub fn min(values: &[f64]) -> f64 {
    values
        .iter()
        .copied()
        .reduce(f64::min)
        .expect("array is empty")
}
pub fn ceiled_even(value: f64) -> f64 {
    let ceil = value.ceil();
    if ceil % 2.0 != 0.0 { ceil - 1.0 } else { ceil }
}
pub fn ceiled_odd(value: f64) -> f64 {
    let ceil = value.ceil();
    if ceil % 2.0 == 0.0 { ceil - 1.0 } else { ceil }
}
