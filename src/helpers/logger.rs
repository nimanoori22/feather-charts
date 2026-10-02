//! Development diagnostics for the native chart host.
pub fn warn(message: impl AsRef<str>) {
    if cfg!(debug_assertions) {
        eprintln!("feather-charts warning: {}", message.as_ref());
    }
}
