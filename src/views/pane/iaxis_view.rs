use crate::{model::pane::Pane, renderers::ipane_renderer::AxisRenderer};
pub trait AxisView<Target> {
    fn renderer(&self, pane: &Pane) -> Option<Box<dyn AxisRenderer<Target>>>;
}
