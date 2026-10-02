use crate::model::colors::CustomColorParser;
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ColorType {
    Solid,
    VerticalGradient,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Background {
    Solid {
        color: String,
    },
    VerticalGradient {
        top_color: String,
        bottom_color: String,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutPanesOptions {
    pub enable_resize: bool,
    pub separator_color: String,
    pub separator_hover_color: String,
}
pub struct LayoutOptions {
    pub background: Background,
    pub text_color: String,
    pub font_size: f64,
    pub font_family: String,
    pub panes: LayoutPanesOptions,
    pub attribution_logo: bool,
    pub color_space: ColorSpace,
    pub color_parsers: Vec<CustomColorParser>,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ColorSpace {
    DisplayP3,
    #[default]
    Srgb,
}
