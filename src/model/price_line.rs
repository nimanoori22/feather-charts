//! Series-owned price levels; IDs never outlive their owning series membership.
use crate::renderers::draw_line::{LineStyle, LineWidth};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PriceLineId(pub(crate) u64);

#[derive(Clone, Debug, PartialEq)]
pub struct PriceLineOptions {
    pub price: f64,
    pub color: String,
    pub line_width: LineWidth,
    pub line_style: LineStyle,
    pub line_visible: bool,
    pub axis_label_visible: bool,
    pub title: String,
    pub axis_label_color: String,
    pub axis_label_text_color: String,
    pub external_id: Option<String>,
}
impl PriceLineOptions {
    pub fn new(price: f64) -> Self {
        Self {
            price,
            color: "#FF0000".into(),
            line_width: LineWidth::One,
            line_style: LineStyle::Dashed,
            line_visible: true,
            axis_label_visible: true,
            title: String::new(),
            axis_label_color: String::new(),
            axis_label_text_color: String::new(),
            external_id: None,
        }
    }
    pub(crate) fn valid(&self, parser: &mut crate::model::colors::ColorParser) -> bool {
        self.price.is_finite()
            && self.price.abs() <= 90_071_992_547_409.9
            && parser.parse_color(&self.color).is_ok()
            && [&self.axis_label_color, &self.axis_label_text_color]
                .into_iter()
                .all(|c| c.is_empty() || parser.parse_color(c).is_ok())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LastValueData {
    pub price: f64,
    pub index: crate::model::time_data::TimePointIndex,
    pub coordinate: f64,
    pub text: String,
    pub absolute: String,
    pub percentage: String,
    pub color: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PriceLine {
    pub coordinate: f64,
    pub color: String,
    pub width: LineWidth,
    pub style: LineStyle,
}
