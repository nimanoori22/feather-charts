//! Ordinary price-axis commands in local logical pixels. Only tick/border
//! rectangles are bitmap-snapped, then converted back once.
use crate::{
    model::{
        axis_snapshots::{PriceAxisSide, PriceAxisSnapshot},
        layout_options::Background,
        text_width_cache::{TextMeasurer, TextMetrics},
    },
    renderers::{grid_renderer::PixelRatio, line_renderer::Point},
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AxisBounds {
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisRect {
    pub origin: Point,
    pub size: AxisBounds,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AxisTextAlignment {
    Left,
    Right,
}
#[derive(Clone, Debug, PartialEq)]
pub enum PriceAxisCommand {
    Background(Background),
    Rectangle {
        rect: AxisRect,
        color: String,
    },
    Text {
        label: String,
        position: Point,
        alignment: AxisTextAlignment,
        font_size: f32,
        font_family: String,
        color: String,
    },
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PreparedPriceAxis {
    pub required_width: f64,
    pub bounds: AxisBounds,
    pub commands: Vec<PriceAxisCommand>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PriceAxisRenderError {
    InvalidBounds,
    InvalidPixelRatio,
    InvalidSnapshot,
    InvalidMetrics,
}

/// Owns the exact snapshot measured, preventing font/mark mismatches on replay.
#[derive(Clone, Debug)]
pub struct PriceAxisMeasurement {
    snapshot: PriceAxisSnapshot,
    metrics: Vec<TextMetrics>,
    required_width: f64,
}
impl PriceAxisMeasurement {
    pub fn required_width(&self) -> f64 {
        self.required_width
    }
}

fn even_width(width: f64) -> f64 {
    width + width % 2.
}
fn source_round(value: f64) -> f64 {
    // Math.round ties toward +infinity, including negative coordinates.
    (value + 0.5).floor()
}
pub fn measure_price_axis<M: TextMeasurer>(
    snapshot: &PriceAxisSnapshot,
    measurer: &mut M,
) -> Result<PriceAxisMeasurement, PriceAxisRenderError> {
    let options = &snapshot.renderer_options;
    if !snapshot.minimum_width.is_finite()
        || !options.font_size.is_finite()
        || options.font_size <= 0.
        || [
            options.border_size,
            options.tick_length,
            options.padding_inner,
            options.padding_outer,
        ]
        .iter()
        .any(|value| !value.is_finite() || *value < 0.)
        || snapshot
            .marks
            .iter()
            .any(|mark| !mark.coord.value().is_finite())
    {
        return Err(PriceAxisRenderError::InvalidSnapshot);
    }
    let metrics = if snapshot.visible {
        snapshot
            .marks
            .iter()
            .map(|mark| measurer.measure_text(&mark.label))
            .collect::<Vec<_>>()
    } else {
        vec![]
    };
    if metrics.iter().any(|metric| {
        !metric.width.is_finite()
            || metric.width < 0.
            || metric
                .actual_bounding_box_ascent
                .is_some_and(|v| !v.is_finite())
            || metric
                .actual_bounding_box_descent
                .is_some_and(|v| !v.is_finite())
    }) {
        return Err(PriceAxisRenderError::InvalidMetrics);
    }
    // Other labels still need metrics for y correction; only endpoints affect
    // intrinsic width, matching the source ordinary-tick sizing policy.
    let label_width = metrics
        .first()
        .map_or(0., |metric| metric.width)
        .max(metrics.last().map_or(0., |metric| metric.width));
    let intrinsic = even_width(
        (f64::from(
            options.border_size
                + options.tick_length
                + options.padding_inner
                + options.padding_outer,
        ) + 5.
            + if label_width > 0. {
                f64::from(label_width)
            } else {
                34.
            })
        .ceil(),
    );
    let required_width = if snapshot.visible {
        // ChartWidget reapplies the source size hint after minimumWidth.
        even_width(intrinsic.max(snapshot.minimum_width))
    } else {
        0.
    };
    Ok(PriceAxisMeasurement {
        snapshot: snapshot.clone(),
        metrics,
        required_width,
    })
}

/// Explicit bounds are an allocation, not a request to resize the model.
pub fn prepare_price_axis(
    measurement: &PriceAxisMeasurement,
    bounds: AxisBounds,
    ratio: PixelRatio,
) -> Result<PreparedPriceAxis, PriceAxisRenderError> {
    if !bounds.width.is_finite()
        || !bounds.height.is_finite()
        || bounds.width < 0.
        || bounds.height < 0.
        || bounds.width > f64::from(f32::MAX)
        || bounds.height > f64::from(f32::MAX)
    {
        return Err(PriceAxisRenderError::InvalidBounds);
    }
    if !ratio.horizontal.is_finite()
        || !ratio.vertical.is_finite()
        || ratio.horizontal <= 0.
        || ratio.vertical <= 0.
        || !(bounds.width * f64::from(ratio.horizontal)).is_finite()
        || !(bounds.height * f64::from(ratio.vertical)).is_finite()
    {
        return Err(PriceAxisRenderError::InvalidPixelRatio);
    }
    let mut result = PreparedPriceAxis {
        required_width: measurement.required_width,
        bounds,
        commands: vec![],
    };
    let snapshot = &measurement.snapshot;
    if !snapshot.visible || bounds.width == 0. || bounds.height == 0. {
        return Ok(result);
    }
    let options = &snapshot.renderer_options;
    let horizontal = f64::from(ratio.horizontal);
    let vertical = f64::from(ratio.vertical);
    let left = snapshot.side == PriceAxisSide::Left;
    result
        .commands
        .push(PriceAxisCommand::Background(snapshot.background.clone()));
    if snapshot.border_visible {
        let border_width = (f64::from(options.border_size) * horizontal)
            .floor()
            .max(1.)
            / horizontal;
        result.commands.push(PriceAxisCommand::Rectangle {
            rect: AxisRect {
                origin: Point {
                    x: if left {
                        bounds.width - border_width
                    } else {
                        0.
                    },
                    y: 0.,
                },
                size: AxisBounds {
                    width: border_width,
                    height: bounds.height,
                },
            },
            color: snapshot.border_color.clone(),
        });
    }
    let tick_x = if left {
        bounds.width - f64::from(options.tick_length)
    } else {
        0.
    };
    if snapshot.border_visible && snapshot.ticks_visible {
        let tick_height = vertical.floor().max(1.);
        let tick_offset = (vertical * 0.5).floor();
        for mark in &snapshot.marks {
            result.commands.push(PriceAxisCommand::Rectangle {
                rect: AxisRect {
                    origin: Point {
                        x: (tick_x * horizontal).floor() / horizontal,
                        y: (source_round(mark.coord.value() * vertical) - tick_offset) / vertical,
                    },
                    size: AxisBounds {
                        width: source_round(f64::from(options.tick_length) * horizontal)
                            / horizontal,
                        height: tick_height / vertical,
                    },
                },
                color: snapshot.border_color.clone(),
            });
        }
    }
    let text_x = source_round(if left {
        tick_x - f64::from(options.padding_inner)
    } else {
        tick_x + f64::from(options.tick_length + options.padding_inner)
    });
    for (mark, metric) in snapshot.marks.iter().zip(&measurement.metrics).rev() {
        let correction = f64::from(
            metric.actual_bounding_box_ascent.unwrap_or_default()
                - metric.actual_bounding_box_descent.unwrap_or_default(),
        ) / 2.;
        result.commands.push(PriceAxisCommand::Text {
            label: mark.label.clone(),
            position: Point {
                x: text_x,
                y: mark.coord.value() + correction,
            },
            alignment: if left {
                AxisTextAlignment::Right
            } else {
                AxisTextAlignment::Left
            },
            font_size: options.font_size,
            font_family: options.font_family.clone(),
            color: options.color.clone(),
        });
    }
    Ok(result)
}

#[cfg(test)]
pub(crate) mod tests;
