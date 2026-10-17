//! Logical-coordinate layout; native shaping is supplied separately.
use crate::api::TextWatermarkOptions;
use iced::{
    Size,
    alignment::{Horizontal, Vertical},
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LinePlacement {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}
pub(crate) fn layout(
    options: &TextWatermarkOptions,
    size: Size,
    widths: &[f64],
) -> Vec<LinePlacement> {
    assert_eq!(options.lines.len(), widths.len());
    let zooms = options
        .lines
        .iter()
        .zip(widths)
        .map(|(line, width)| {
            if line.text.is_empty() {
                0.
            } else if *width > f64::from(size.width) {
                f64::from(size.width) / width
            } else {
                1.
            }
        })
        .collect::<Vec<_>>();
    let height = options
        .lines
        .iter()
        .zip(&zooms)
        .map(|(l, z)| l.height() * z)
        .sum::<f64>();
    let mut y = match options.vertical_alignment {
        Vertical::Top => 0.,
        Vertical::Center => ((f64::from(size.height) - height) / 2.).max(0.),
        Vertical::Bottom => (f64::from(size.height) - height).max(0.),
    };
    options
        .lines
        .iter()
        .zip(zooms)
        .map(|(line, zoom)| {
            let x = match options.horizontal_alignment {
                Horizontal::Left => line.height() / 2.,
                Horizontal::Center => f64::from(size.width) / 2.,
                Horizontal::Right => f64::from(size.width) - 1. - line.height() / 2.,
            };
            let result = LinePlacement { x, y, zoom };
            y += line.height() * zoom;
            result
        })
        .collect()
}
#[cfg(test)]
mod tests;
