/// The browser-oriented fallback family used by Lightweight Charts when a font
/// family is not supplied explicitly.
pub const DEFAULT_FONT_FAMILY: &str =
    "-apple-system, BlinkMacSystemFont, 'Trebuchet MS', Roboto, Ubuntu, sans-serif";

/// Produces the CSS-compatible font descriptor retained by chart renderer data.
///
/// Iced adapters can use the size and family independently; retaining this
/// string preserves the source renderer contract for backends that need it.
pub fn make_font(size: f32, family: Option<&str>, style: Option<&str>) -> String {
    let style = style.map_or(String::new(), |style| format!("{style} "));
    let family = family.unwrap_or(DEFAULT_FONT_FAMILY);
    format!("{style}{size}px {family}")
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_FONT_FAMILY, make_font};

    #[test]
    fn formats_optional_style_and_uses_the_default_family() {
        assert_eq!(
            make_font(12.0, None, None),
            format!("12px {DEFAULT_FONT_FAMILY}")
        );
        assert_eq!(
            make_font(13.5, Some("Inter"), Some("bold")),
            "bold 13.5px Inter"
        );
    }
}
