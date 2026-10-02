use crate::renderers::ipane_renderer::TimeAxisRenderer;
pub trait TimeAxisView<Target> {
    fn renderer(&self) -> Box<dyn TimeAxisRenderer<Target>>;
}
