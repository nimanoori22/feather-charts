use crate::{
    model::{
        data_source::PriceScaleHandle,
        ipane_primitive::{PrimitiveHitTestSource, PrimitivePaneViewZOrder},
        pane::Pane,
        text_width_cache::TextMeasurer,
    },
    views::{
        pane::{
            hovered_source_pane_views::HoveredSourcePaneViews, iaxis_view::AxisView,
            ipane_view::PaneView,
        },
        price_axis::iprice_axis_view::PriceAxisView,
        time_axis::itime_axis_view::TimeAxisView,
    },
};
use std::rc::Rc;
pub trait ZOrdered {
    fn z_order(&self) -> i32;
    fn set_z_order(&mut self, value: i32);
}
pub trait IDataSource<Target: TextMeasurer>: PrimitiveHitTestSource + ZOrdered {
    fn price_scale(&self) -> Option<PriceScaleHandle>;
    fn set_price_scale(&mut self, scale: Option<PriceScaleHandle>);
    fn update_all_views(&mut self);
    fn price_axis_views(
        &self,
        pane: Option<&Pane>,
        scale: Option<&PriceScaleHandle>,
    ) -> Vec<Rc<dyn PriceAxisView<Target>>>;
    fn pane_views(&self, pane: &Pane) -> Vec<Rc<dyn PaneView<Target>>>;
    fn label_pane_views(&self, pane: Option<&Pane>) -> Vec<Rc<dyn PaneView<Target>>>;
    fn top_pane_views(&self, _pane: &Pane) -> Vec<Rc<dyn PaneView<Target>>> {
        Vec::new()
    }
    fn bottom_pane_views(&self, _pane: &Pane) -> Vec<Rc<dyn PaneView<Target>>> {
        Vec::new()
    }
    fn price_pane_views(&self, _z: PrimitivePaneViewZOrder) -> Vec<Rc<dyn AxisView<Target>>> {
        Vec::new()
    }
    fn time_pane_views(&self, _z: PrimitivePaneViewZOrder) -> Vec<Rc<dyn AxisView<Target>>> {
        Vec::new()
    }
    fn pane_views_for_hovered_source_on_top(
        &self,
        _pane: &Pane,
    ) -> Option<HoveredSourcePaneViews<Target>> {
        None
    }
    fn time_axis_views(&self) -> Vec<Rc<dyn TimeAxisView<Target>>>;
    fn visible(&self) -> bool;
}
