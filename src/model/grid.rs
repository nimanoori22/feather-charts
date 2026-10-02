//! Grid options and ownership of the Pane-local grid view.
use crate::{renderers::draw_line::LineStyle, views::pane::grid_pane_view::GridPaneView};
#[derive(Clone, Debug, PartialEq)]
pub struct GridLineOptions {
    pub color: String,
    pub style: LineStyle,
    pub visible: bool,
}
impl Default for GridLineOptions {
    fn default() -> Self {
        Self {
            color: "#D6DCDE".into(),
            style: LineStyle::Solid,
            visible: true,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GridOptions {
    pub vert_lines: GridLineOptions,
    pub horz_lines: GridLineOptions,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UpdateType {
    Data,
    #[default]
    Other,
    Options,
}
#[derive(Clone, Debug)]
pub struct Grid {
    pane_view: GridPaneView,
}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}
impl Grid {
    pub fn new() -> Self {
        Self {
            pane_view: GridPaneView::new(),
        }
    }
    pub fn pane_view(&self) -> &GridPaneView {
        &self.pane_view
    }
    pub fn pane_view_mut(&mut self) -> &mut GridPaneView {
        &mut self.pane_view
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owns_one_invalidatable_view() {
        let mut grid = Grid::new();
        assert!(grid.pane_view().is_invalidated());
        grid.pane_view_mut().update(UpdateType::Options);
        assert!(grid.pane_view().is_invalidated());
        assert_eq!(GridOptions::default().vert_lines.color, "#D6DCDE");
    }
}
