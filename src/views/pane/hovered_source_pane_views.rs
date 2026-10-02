use crate::views::pane::ipane_view::PaneView;
use std::rc::Rc;
pub struct HoveredSourcePaneViews<Target> {
    pub normal_pane_views: Vec<Rc<dyn PaneView<Target>>>,
    pub top_pane_views: Vec<Rc<dyn PaneView<Target>>>,
}
