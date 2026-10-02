pub fn assert(condition: bool, message: Option<&str>) {
    assert!(
        condition,
        "Assertion failed{}",
        message.map_or(String::new(), |m| format!(": {m}"))
    );
}
pub fn ensure_defined<T>(value: Option<T>) -> T {
    value.expect("Value is undefined")
}
pub fn ensure_not_null<T>(value: Option<T>) -> T {
    value.expect("Value is null")
}
pub fn ensure<T>(value: Option<T>) -> T {
    value.expect("Value is undefined or null")
}
