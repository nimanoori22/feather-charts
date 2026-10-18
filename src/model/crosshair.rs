use crate::{
    model::time_data::TimePointIndex,
    renderers::{
        draw_line::{LineStyle, LineWidth},
        line_renderer::Point,
    },
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CrosshairMode {
    Normal,
    #[default]
    Magnet,
    Hidden,
    MagnetOhlc,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CrosshairLineOptions {
    pub color: String,
    pub width: LineWidth,
    pub style: LineStyle,
    pub visible: bool,
    pub label_visible: bool,
    pub label_background_color: String,
}
impl Default for CrosshairLineOptions {
    fn default() -> Self {
        Self {
            color: "#758696".into(),
            width: LineWidth::One,
            style: LineStyle::LargeDashed,
            visible: true,
            label_visible: true,
            label_background_color: "#4c525e".into(),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CrosshairOptions {
    pub mode: CrosshairMode,
    pub vertical: CrosshairLineOptions,
    pub horizontal: CrosshairLineOptions,
    pub do_not_snap_to_hidden_series_indices: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrosshairOrigin {
    pub pane: usize,
    pub point: Point,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrosshairPosition {
    pub pane: usize,
    pub index: TimePointIndex,
    pub x: f64,
    pub y: Option<f64>,
}
#[derive(Clone, Debug)]
pub struct CrosshairMarker {
    pub point: Point,
    pub radius: f64,
    pub border_width: f64,
    pub color: String,
    pub border_color: String,
}
#[derive(Default)]
pub struct Crosshair {
    pub(crate) synthetic: bool,
    pub(crate) options: CrosshairOptions,
    pub(crate) origin: Option<CrosshairOrigin>,
    pub(crate) position: Option<CrosshairPosition>,
}
impl Crosshair {
    pub fn options(&self) -> &CrosshairOptions {
        &self.options
    }
    pub fn origin(&self) -> Option<CrosshairOrigin> {
        self.origin
    }
    pub fn position(&self) -> Option<CrosshairPosition> {
        self.position
    }
}
