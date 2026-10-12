use crate::{
    model::price_line::PriceLine,
    renderers::{
        draw_line::dash_pattern,
        grid_renderer::PixelRatio,
        line_renderer::{PathOperation, Point, StyledStroke},
        price_axis_renderer::AxisBounds,
    },
};
pub fn draw_commands(
    line: &PriceLine,
    bounds: AxisBounds,
    ratio: PixelRatio,
) -> Option<StyledStroke> {
    if !line.coordinate.is_finite()
        || !bounds.width.is_finite()
        || !bounds.height.is_finite()
        || bounds.width <= 0.
        || bounds.height <= 0.
        || !ratio.horizontal.is_finite()
        || !ratio.vertical.is_finite()
        || ratio.horizontal <= 0.
        || ratio.vertical <= 0.
    {
        return None;
    }
    let horizontal = f64::from(ratio.horizontal);
    let vertical = f64::from(ratio.vertical);
    let y = (line.coordinate * vertical + 0.5).floor();
    if y < 0. || y > bounds.height * vertical {
        return None;
    }
    let width = (f64::from(line.width.pixels()) * horizontal).floor();
    let y = y + if width as u64 % 2 == 1 { 0.5 } else { 0. };
    Some(StyledStroke {
        path: vec![
            PathOperation::MoveTo(Point { x: 0., y }),
            PathOperation::LineTo(Point {
                x: bounds.width * horizontal,
                y,
            }),
        ],
        color: line.color.clone(),
        width,
        dash_offset: 0.,
        dash_pattern: dash_pattern(line.style, width as f32)
            .into_iter()
            .map(f64::from)
            .collect(),
    })
}
