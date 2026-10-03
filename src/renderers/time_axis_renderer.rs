//! Ordinary time-axis commands in local logical pixels. Tick selection belongs
//! to TimeScale; edge alignment moves text only, never ticks.
use crate::{
    model::{
        axis_snapshots::{TimeAxisSnapshot, TimeLabelEmphasis},
        text_width_cache::TextMeasurer,
    },
    renderers::{
        grid_renderer::PixelRatio,
        line_renderer::Point,
        price_axis_renderer::{AxisBounds, AxisRect},
    },
};

#[derive(Clone, Debug, PartialEq)]
pub enum TimeAxisCommand {
    Background(String),
    Rectangle {
        rect: AxisRect,
        color: String,
    },
    /// Centered horizontally and vertically at `position`.
    Text {
        label: String,
        position: Point,
        emphasis: TimeLabelEmphasis,
        font_size: f32,
        font_family: String,
        color: String,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PreparedTimeAxis {
    pub required_height: f64,
    pub bounds: AxisBounds,
    pub commands: Vec<TimeAxisCommand>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeAxisRenderError {
    InvalidSnapshot,
    InvalidMetrics,
    InvalidBounds,
    InvalidPixelRatio,
}

/// Owns the snapshot and the widths measured in each tick's actual font.
#[derive(Clone, Debug)]
pub struct TimeAxisMeasurement {
    snapshot: TimeAxisSnapshot,
    widths: Vec<f64>,
    required_height: f64,
}
impl TimeAxisMeasurement {
    pub fn required_height(&self) -> f64 {
        self.required_height
    }
}

/// Two backend contexts make font selection explicit without adding font state
/// to the existing TextMeasurer interface. No width cache or digit substitution.
pub fn measure_time_axis<N: TextMeasurer, E: TextMeasurer>(
    snapshot: &TimeAxisSnapshot,
    normal: &mut N,
    emphasized: &mut E,
) -> Result<TimeAxisMeasurement, TimeAxisRenderError> {
    let options = &snapshot.renderer_options;
    if !snapshot.minimum_height.is_finite()
        || snapshot.minimum_height < 0.
        || !options.font_size.is_finite()
        || options.font_size <= 0.
        || [
            options.border_size,
            options.tick_length,
            options.padding_top,
            options.padding_bottom,
            options.label_bottom_offset,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.)
        || snapshot
            .ticks
            .iter()
            .any(|tick| !tick.mark.coordinate.is_finite())
    {
        return Err(TimeAxisRenderError::InvalidSnapshot);
    }
    let mut widths = vec![];
    if snapshot.visible {
        for tick in &snapshot.ticks {
            let metrics = match tick.emphasis {
                TimeLabelEmphasis::Normal => normal.measure_text(&tick.mark.label),
                TimeLabelEmphasis::Emphasized => emphasized.measure_text(&tick.mark.label),
            };
            if !metrics.width.is_finite()
                || metrics.width < 0.
                || metrics
                    .actual_bounding_box_ascent
                    .is_some_and(|v| !v.is_finite())
                || metrics
                    .actual_bounding_box_descent
                    .is_some_and(|v| !v.is_finite())
            {
                return Err(TimeAxisRenderError::InvalidMetrics);
            }
            widths.push(f64::from(metrics.width));
        }
    }
    let intrinsic = (f64::from(options.border_size)
        + f64::from(options.tick_length)
        + f64::from(options.font_size)
        + f64::from(options.padding_top)
        + f64::from(options.padding_bottom)
        + f64::from(options.label_bottom_offset))
    .ceil();
    let height = intrinsic.max(snapshot.minimum_height);
    // ChartWidget applies this hint after optimalHeight and minimumHeight.
    let required_height = if snapshot.visible {
        height + height % 2.
    } else {
        0.
    };
    Ok(TimeAxisMeasurement {
        snapshot: snapshot.clone(),
        widths,
        required_height,
    })
}

fn source_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

pub fn prepare_time_axis(
    measurement: &TimeAxisMeasurement,
    bounds: AxisBounds,
    ratio: PixelRatio,
) -> Result<PreparedTimeAxis, TimeAxisRenderError> {
    if [bounds.width, bounds.height]
        .iter()
        .any(|v| !v.is_finite() || *v < 0. || *v > f64::from(f32::MAX))
    {
        return Err(TimeAxisRenderError::InvalidBounds);
    }
    if [ratio.horizontal, ratio.vertical]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
        || !(bounds.width * f64::from(ratio.horizontal)).is_finite()
        || !(bounds.height * f64::from(ratio.vertical)).is_finite()
    {
        return Err(TimeAxisRenderError::InvalidPixelRatio);
    }
    let mut result = PreparedTimeAxis {
        required_height: measurement.required_height,
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
    // The source uses the bottom color, not a gradient spanning this widget.
    result.commands.push(TimeAxisCommand::Background(
        snapshot.background_fill_color().into(),
    ));
    if snapshot.border_visible {
        result.commands.push(TimeAxisCommand::Rectangle {
            rect: AxisRect {
                origin: Point { x: 0., y: 0. },
                size: AxisBounds {
                    width: bounds.width,
                    height: (f64::from(options.border_size) * vertical).floor().max(1.) / vertical,
                },
            },
            color: snapshot.border_color.clone(),
        });
    }
    if snapshot.border_visible && snapshot.ticks_visible {
        for tick in snapshot.ticks.iter().rev() {
            result.commands.push(TimeAxisCommand::Rectangle {
                rect: AxisRect {
                    origin: Point {
                        x: (source_round(tick.mark.coordinate * horizontal)
                            - (horizontal * 0.5).floor())
                            / horizontal,
                        y: 0.,
                    },
                    size: AxisBounds {
                        width: horizontal.floor().max(1.) / horizontal,
                        height: source_round(f64::from(options.tick_length) * vertical) / vertical,
                    },
                },
                color: snapshot.border_color.clone(),
            });
        }
    }
    let y = f64::from(options.border_size)
        + f64::from(options.tick_length)
        + f64::from(options.padding_top)
        + f64::from(options.font_size) / 2.;
    // Priority order is independent of whether bold labels are enabled.
    for priority in [false, true] {
        for (tick, width) in snapshot.ticks.iter().zip(&measurement.widths) {
            if tick.priority != priority {
                continue;
            }
            let mut x = tick.mark.coordinate;
            if tick.mark.need_align_coordinate {
                let left = (x - width / 2.).floor() + 0.5;
                if left < 0. {
                    x -= left;
                } else if left + width > bounds.width {
                    x -= left + width - bounds.width;
                }
            }
            result.commands.push(TimeAxisCommand::Text {
                label: tick.mark.label.clone(),
                position: Point { x, y },
                emphasis: tick.emphasis,
                font_size: options.font_size,
                font_family: options.font_family.clone(),
                color: options.text_color.clone(),
            });
        }
    }
    Ok(result)
}

#[cfg(test)]
pub(crate) mod tests;
