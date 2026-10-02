pub fn is_number(value: f64) -> bool {
    value.is_finite()
}
pub fn is_integer(value: f64) -> bool {
    value.fract() == 0.0
}
/// Rust models option patches as typed patch structs; this marker documents the
/// source's recursive optional-field concept without dynamic object merging.
pub trait DeepPartial {}

/// Rust's `Clone` trait is the safe equivalent of the source's object clone.
pub fn clone_value<T: Clone>(value: &T) -> T {
    value.clone()
}
pub const fn is_string(_value: &str) -> bool {
    true
}
pub const fn is_boolean(_value: bool) -> bool {
    true
}
pub fn not_null<T>(value: Option<T>) -> bool {
    value.is_some()
}
pub fn undefined_if_null<T>(value: Option<T>) -> Option<T> {
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uses_typed_clone_and_option_helpers() {
        let original = vec![1, 2];
        assert_eq!(clone_value(&original), original);
        assert!(not_null(Some(1)));
        assert_eq!(undefined_if_null::<u8>(None), None);
    }
}
