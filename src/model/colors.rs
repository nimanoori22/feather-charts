//! Color parsing and transformations used by chart labels and gradients.

use std::{collections::HashMap, error::Error, fmt};

/// An sRGB color with 8-bit channels and a floating-point alpha component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: f32,
}

impl Rgba {
    pub const fn new(red: u8, green: u8, blue: u8, alpha: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContrastColors {
    pub foreground: String,
    pub background: String,
}

/// A parser for color formats not represented by native CSS sRGB syntax.
pub type CustomColorParser = Box<dyn Fn(&str) -> Option<Rgba>>;

/// An error returned when neither CSS parsing nor a custom parser recognizes a color.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColorParseError {
    color: String,
}

impl ColorParseError {
    fn new(color: &str) -> Self {
        Self {
            color: color.to_owned(),
        }
    }
}

impl fmt::Display for ColorParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "failed to parse color: {}", self.color)
    }
}

impl Error for ColorParseError {}

/// Parses, caches, and transforms chart colors.
///
/// Browser-based Lightweight Charts delegates sRGB CSS parsing to the browser.
/// Here, `csscolorparser` supplies that behavior natively; caller-provided
/// parsers retain support for non-sRGB formats such as `display-p3`.
pub struct ColorParser {
    rgba_cache: HashMap<String, Rgba>,
    custom_parsers: Vec<CustomColorParser>,
}

impl Default for ColorParser {
    fn default() -> Self {
        Self::new(Vec::new(), None)
    }
}

impl ColorParser {
    pub fn new(
        custom_parsers: Vec<CustomColorParser>,
        initial_cache: Option<HashMap<String, Rgba>>,
    ) -> Self {
        Self {
            rgba_cache: initial_cache.unwrap_or_default(),
            custom_parsers,
        }
    }

    /// Applies alpha over the color's existing alpha, preserving `transparent`.
    pub fn apply_alpha(&mut self, color: &str, alpha: f32) -> Result<String, ColorParseError> {
        if color == "transparent" {
            return Ok(color.to_owned());
        }

        let origin = self.parse_color(color)?;
        Ok(format!(
            "rgba({}, {}, {}, {})",
            origin.red,
            origin.green,
            origin.blue,
            alpha * origin.alpha
        ))
    }

    /// Returns an opaque background and readable black-or-white foreground.
    pub fn generate_contrast_colors(
        &mut self,
        background: &str,
    ) -> Result<ContrastColors, ColorParseError> {
        let rgba = self.parse_color(background)?;
        Ok(ContrastColors {
            background: format!("rgb({}, {}, {})", rgba.red, rgba.green, rgba.blue),
            foreground: if rgba_to_grayscale(rgba) > 160.0 {
                "black".to_owned()
            } else {
                "white".to_owned()
            },
        })
    }

    pub fn color_string_to_grayscale(&mut self, color: &str) -> Result<f32, ColorParseError> {
        Ok(rgba_to_grayscale(self.parse_color(color)?))
    }

    /// Linearly interpolates two colors without clamping the supplied percentage.
    pub fn gradient_color_at_percent(
        &mut self,
        top_color: &str,
        bottom_color: &str,
        percent: f32,
    ) -> Result<String, ColorParseError> {
        let top = self.parse_color(top_color)?;
        let bottom = self.parse_color(bottom_color)?;
        let result = Rgba::new(
            normalize_rgb_component(
                top.red as f32 + percent * (bottom.red as f32 - top.red as f32),
            ),
            normalize_rgb_component(
                top.green as f32 + percent * (bottom.green as f32 - top.green as f32),
            ),
            normalize_rgb_component(
                top.blue as f32 + percent * (bottom.blue as f32 - top.blue as f32),
            ),
            normalize_alpha_component(top.alpha + percent * (bottom.alpha - top.alpha)),
        );

        Ok(format!(
            "rgba({}, {}, {}, {})",
            result.red, result.green, result.blue, result.alpha
        ))
    }

    pub fn parse_color(&mut self, color: &str) -> Result<Rgba, ColorParseError> {
        if let Some(rgba) = self.rgba_cache.get(color) {
            return Ok(*rgba);
        }

        // The TypeScript implementation intentionally sends display-p3 through
        // custom parsers after browser serialization leaves it non-sRGB.
        let parsed = if is_display_p3(color) {
            None
        } else {
            csscolorparser::parse(color).ok().map(|color| {
                Rgba::new(
                    normalize_rgb_component(color.r * 255.0),
                    normalize_rgb_component(color.g * 255.0),
                    normalize_rgb_component(color.b * 255.0),
                    color.a,
                )
            })
        };

        let rgba = parsed.or_else(|| self.custom_parsers.iter().find_map(|parser| parser(color)));

        let Some(rgba) = rgba else {
            return Err(ColorParseError::new(color));
        };

        self.rgba_cache.insert(color.to_owned(), rgba);
        Ok(rgba)
    }
}

fn normalize_rgb_component(component: f32) -> u8 {
    if !component.is_finite() || component <= 0.0 {
        return 0;
    }
    if component >= 255.0 {
        return 255;
    }
    component.round() as u8
}

fn normalize_alpha_component(component: f32) -> f32 {
    if !component.is_finite() {
        return 0.0;
    }
    if !(0.0..=1.0).contains(&component) {
        return component.clamp(0.0, 1.0);
    }
    (component * 10_000.0).round() / 10_000.0
}

fn rgba_to_grayscale(color: Rgba) -> f32 {
    0.199 * color.red as f32 + 0.687 * color.green as f32 + 0.114 * color.blue as f32
}

fn is_display_p3(color: &str) -> bool {
    let color = color.trim_start().to_ascii_lowercase();
    color
        .strip_prefix("color(")
        .is_some_and(|arguments| arguments.trim_start().starts_with("display-p3"))
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, collections::HashMap, rc::Rc};

    use super::*;

    fn parser() -> ColorParser {
        ColorParser::default()
    }

    #[test]
    fn generates_contrast_colors_ignoring_background_alpha() {
        let mut parser = parser();

        assert_eq!(
            parser.generate_contrast_colors("rgba(255, 255, 255, 0)"),
            Ok(ContrastColors {
                foreground: "black".into(),
                background: "rgb(255, 255, 255)".into(),
            })
        );
        assert_eq!(
            parser.generate_contrast_colors("rgb(150, 150, 150)"),
            Ok(ContrastColors {
                foreground: "white".into(),
                background: "rgb(150, 150, 150)".into(),
            })
        );
        assert_eq!(
            parser.generate_contrast_colors("rgb(170, 170, 170)"),
            Ok(ContrastColors {
                foreground: "black".into(),
                background: "rgb(170, 170, 170)".into(),
            })
        );
    }

    #[test]
    fn interpolates_source_gradient_examples() {
        let mut parser = parser();

        assert_eq!(
            parser.gradient_color_at_percent("#fff", "#000", 0.0),
            Ok("rgba(255, 255, 255, 1)".into())
        );
        assert_eq!(
            parser.gradient_color_at_percent("#ffffff", "#000000", 0.5),
            Ok("rgba(128, 128, 128, 1)".into())
        );
        assert_eq!(
            parser.gradient_color_at_percent("rgba(255, 255, 255, 1)", "rgba(0, 0, 0, 0)", 0.5),
            Ok("rgba(128, 128, 128, 0.5)".into())
        );
    }

    #[test]
    fn supports_css_colors_and_preserves_transparent_alpha_behavior() {
        let mut parser = parser();

        assert_eq!(
            parser.parse_color("rebeccapurple"),
            Ok(Rgba::new(102, 51, 153, 1.0))
        );
        assert_eq!(
            parser.apply_alpha("rgba(10, 20, 30, 0.5)", 0.4),
            Ok("rgba(10, 20, 30, 0.2)".into())
        );
        assert_eq!(
            parser.apply_alpha("transparent", 0.4),
            Ok("transparent".into())
        );
    }

    #[test]
    fn uses_initial_cache_and_custom_display_p3_parser_then_caches_the_result() {
        let calls = Rc::new(Cell::new(0));
        let calls_by_parser = Rc::clone(&calls);
        let custom: CustomColorParser = Box::new(move |color| {
            calls_by_parser.set(calls_by_parser.get() + 1);
            (color == "color(display-p3 1 0 0)").then_some(Rgba::new(255, 0, 0, 1.0))
        });
        let mut initial = HashMap::new();
        initial.insert("cached".into(), Rgba::new(1, 2, 3, 0.4));
        let mut parser = ColorParser::new(vec![custom], Some(initial));

        assert_eq!(parser.parse_color("cached"), Ok(Rgba::new(1, 2, 3, 0.4)));
        assert_eq!(calls.get(), 0);
        assert_eq!(
            parser.parse_color("color(display-p3 1 0 0)"),
            Ok(Rgba::new(255, 0, 0, 1.0))
        );
        assert_eq!(
            parser.parse_color("color(display-p3 1 0 0)"),
            Ok(Rgba::new(255, 0, 0, 1.0))
        );
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn reports_unparseable_colors() {
        let mut parser = parser();
        assert_eq!(
            parser.parse_color("not-a-color").unwrap_err().to_string(),
            "failed to parse color: not-a-color"
        );
    }
}
