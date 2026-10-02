use crate::{
    model::price_scale::PriceScale,
    model::text_width_cache::TextMeasurer,
    renderers::iprice_axis_view_renderer::{PriceAxisViewRenderer, PriceAxisViewRendererOptions},
};
pub trait PriceAxisView<Target: TextMeasurer> {
    fn coordinate(&self) -> f64;
    fn fixed_coordinate(&self) -> Option<f64>;
    fn render_coordinate(&self) -> f64;
    fn height(&self, options: &PriceAxisViewRendererOptions, second: bool) -> f32;
    fn is_visible(&self) -> bool;
    fn is_axis_label_visible(&self) -> bool;
    fn renderer(&self, scale: &PriceScale) -> Box<dyn PriceAxisViewRenderer<Target>>;
    fn pane_renderer(&self) -> Box<dyn PriceAxisViewRenderer<Target>>;
    fn set_render_coordinate(&mut self, value: Option<f64>);
    fn text(&self) -> &str;
    fn update(&mut self);
}
