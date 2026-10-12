use crate::{
    model::crosshair::{CrosshairOptions, CrosshairPosition},
    renderers::{
        draw_line::dash_pattern,
        grid_renderer::PixelRatio,
        line_renderer::{PathOperation, Point, StyledStroke},
        price_axis_renderer::AxisBounds,
    },
};

pub fn draw_commands(
    position: CrosshairPosition,
    options: &CrosshairOptions,
    bounds: AxisBounds,
    ratio: PixelRatio,
) -> Vec<StyledStroke> {
    if !bounds.width.is_finite()
        || !bounds.height.is_finite()
        || bounds.width <= 0.
        || bounds.height <= 0.
        || !ratio.horizontal.is_finite()
        || !ratio.vertical.is_finite()
        || ratio.horizontal <= 0.
        || ratio.vertical <= 0.
    {
        return vec![];
    }
    let mut commands = vec![];
    for (vertical, coordinate, line) in [
        (true, Some(position.x), &options.vertical),
        (false, position.y, &options.horizontal),
    ] {
        let Some(coordinate) = coordinate.filter(|v| v.is_finite() && *v >= 0.) else {
            continue;
        };
        if !line.visible {
            continue;
        }
        let pixel_ratio = f64::from(if vertical {
            ratio.horizontal
        } else {
            ratio.vertical
        });
        let width = (f64::from(line.width.pixels()) * pixel_ratio).floor();
        let coordinate =
            (coordinate * pixel_ratio + 0.5).floor() + if width as u64 % 2 == 1 { 0.5 } else { 0. };
        let (a, b) = if vertical {
            (
                Point {
                    x: coordinate,
                    y: 0.,
                },
                Point {
                    x: coordinate,
                    y: bounds.height * f64::from(ratio.vertical),
                },
            )
        } else {
            (
                Point {
                    x: 0.,
                    y: coordinate,
                },
                Point {
                    x: bounds.width * f64::from(ratio.horizontal),
                    y: coordinate,
                },
            )
        };
        commands.push(StyledStroke {
            path: vec![PathOperation::MoveTo(a), PathOperation::LineTo(b)],
            color: line.color.clone(),
            width,
            dash_pattern: dash_pattern(line.style, width as f32)
                .into_iter()
                .map(f64::from)
                .collect(),
            dash_offset: 0.,
        });
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_bitmap_alignment_and_independent_visibility() {
        let p = CrosshairPosition {
            pane: 0,
            index: 0.0.into(),
            x: 20.2,
            y: Some(30.8),
        };
        let mut options = CrosshairOptions::default();
        let bounds = AxisBounds {
            width: 100.,
            height: 60.,
        };
        let result = draw_commands(
            p,
            &options,
            bounds,
            PixelRatio {
                horizontal: 1.5,
                vertical: 1.5,
            },
        );
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].width, 1.);
        assert!(matches!(
            result[0].path[0],
            PathOperation::MoveTo(Point { x: 30.5, y: 0. })
        ));
        options.horizontal.visible = false;
        assert!(
            draw_commands(
                p,
                &options,
                bounds,
                PixelRatio {
                    horizontal: f32::NAN,
                    vertical: 1.
                }
            )
            .is_empty()
        );
        assert_eq!(
            draw_commands(
                p,
                &options,
                bounds,
                PixelRatio {
                    horizontal: 2.,
                    vertical: 2.
                }
            )
            .len(),
            1
        );
    }
}
