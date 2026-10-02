//! Sorted-slice search helpers used by chart-model indexes.
pub fn lower_bound<T, V>(values: &[T], value: &V, before: impl Fn(&T, &V) -> bool) -> usize {
    bound(values, value, before, true)
}
pub fn upper_bound<T, V>(values: &[T], value: &V, before: impl Fn(&T, &V) -> bool) -> usize {
    bound(values, value, before, false)
}
fn bound<T, V>(values: &[T], value: &V, before: impl Fn(&T, &V) -> bool, lower: bool) -> usize {
    let (mut start, mut count) = (0, values.len());
    while count > 0 {
        let half = count / 2;
        let mid = start + half;
        if before(&values[mid], value) == lower {
            start = mid + 1;
            count -= half + 1
        } else {
            count = half
        }
    }
    start
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_equal_run_boundaries() {
        let values = [1, 2, 2, 4];
        assert_eq!(lower_bound(&values, &2, |a, b| a < b), 1);
        assert_eq!(upper_bound(&values, &2, |a, b| a > b), 3);
    }
}
