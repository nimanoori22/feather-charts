/// Stable minimum by screen distance: the first source wins equal distances.
pub fn nearest_coordinate(origin: f64, candidates: impl IntoIterator<Item = f64>) -> f64 {
    let mut nearest = origin;
    let mut distance = f64::INFINITY;
    for candidate in candidates {
        let next = (candidate - origin).abs();
        if next < distance {
            nearest = candidate;
            distance = next;
        }
    }
    nearest
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ties_empty_and_invalid_candidates() {
        assert_eq!(nearest_coordinate(10., [8., 12.]), 8.);
        assert_eq!(nearest_coordinate(10., [12., 8.]), 12.);
        assert_eq!(nearest_coordinate(10., []), 10.);
        assert_eq!(nearest_coordinate(10., [f64::NAN, f64::INFINITY, 11.]), 11.);
    }
}
