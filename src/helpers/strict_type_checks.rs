pub fn is_number(value: f64) -> bool {
    value.is_finite()
}
pub fn is_integer(value: f64) -> bool {
    value.fract() == 0.0
}
/// Rust models option patches as typed patch structs; this marker documents the
/// source's recursive optional-field concept without dynamic object merging.
pub trait DeepPartial {}
