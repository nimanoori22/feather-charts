//! Grid options and the pane view owned by a chart pane.
use crate::renderers::draw_line::LineStyle;
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
pub struct GridPaneView {
    pane_index: usize,
    invalidated: bool,
}
impl GridPaneView {
    pub const fn new(pane_index: usize) -> Self {
        Self {
            pane_index,
            invalidated: true,
        }
    }
    pub fn update(&mut self, _kind: UpdateType) {
        self.invalidated = true
    }
    pub const fn pane_index(&self) -> usize {
        self.pane_index
    }
    pub fn take_invalidation(&mut self) -> bool {
        let value = self.invalidated;
        self.invalidated = false;
        value
    }
}
#[derive(Clone, Debug)]
pub struct Grid {
    pane_view: GridPaneView,
}
impl Grid {
    pub const fn new(pane_index: usize) -> Self {
        Self {
            pane_view: GridPaneView::new(pane_index),
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
        let mut grid = Grid::new(2);
        assert!(grid.pane_view_mut().take_invalidation());
        assert!(!grid.pane_view_mut().take_invalidation());
        grid.pane_view_mut().update(UpdateType::Options);
        assert!(grid.pane_view_mut().take_invalidation());
        assert_eq!(GridOptions::default().vert_lines.color, "#D6DCDE");
    }
}
