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
    PrimitiveLayer(crate::model::ipane_primitive::PrimitivePaneViewZOrder),
    RoundedRectangle {
        rect: AxisRect,
        radii: [f32; 4],
        color: String,
    },
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
    /// Command ranges on the separate overlay canvas, used by owned exports.
    pub screenshot_crosshair: std::ops::Range<usize>,
    pub screenshot_top_labels: std::ops::Range<usize>,
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
    floating_width: Option<f64>,
    primitive_widths: Vec<f64>,
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
            options.padding_horizontal,
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
    let floating_width = if snapshot.visible {
        snapshot
            .crosshair_label
            .as_ref()
            .map(|label| f64::from(normal.measure_text(&label.text).width))
    } else {
        None
    };
    let primitive_widths = if snapshot.visible {
        snapshot
            .primitive_labels
            .iter()
            .map(|l| f64::from(normal.measure_text(&l.axis.text).width))
            .collect::<Vec<_>>()
    } else {
        vec![]
    };
    if primitive_widths.iter().any(|w| !w.is_finite() || *w < 0.) {
        return Err(TimeAxisRenderError::InvalidMetrics);
    }
    if snapshot
        .primitive_labels
        .iter()
        .any(|l| !l.axis.coordinate.is_finite())
    {
        return Err(TimeAxisRenderError::InvalidSnapshot);
    }
    if floating_width.is_some_and(|width| !width.is_finite() || width < 0.) {
        return Err(TimeAxisRenderError::InvalidMetrics);
    }
    if snapshot
        .crosshair_label
        .as_ref()
        .is_some_and(|label| !label.coordinate.is_finite())
    {
        return Err(TimeAxisRenderError::InvalidSnapshot);
    }
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
        floating_width,
        primitive_widths,
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
        screenshot_crosshair: 0..0,
        screenshot_top_labels: 0..0,
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
    if snapshot.primitive_graphics {
        result.commands.push(TimeAxisCommand::PrimitiveLayer(
            crate::api::PrimitiveLayer::Bottom,
        ));
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
    if snapshot.primitive_graphics {
        result.commands.push(TimeAxisCommand::PrimitiveLayer(
            crate::api::PrimitiveLayer::Normal,
        ));
    }
    let labels_start = result.commands.len();
    let primitive_labels = snapshot
        .primitive_labels
        .iter()
        .zip(&measurement.primitive_widths)
        .map(|(l, w)| (&l.axis, *w, l.tick_visible));
    let crosshair = snapshot
        .crosshair_label
        .as_ref()
        .zip(measurement.floating_width)
        .map(|(l, w)| (l, w, snapshot.ticks_visible));
    for (label, text_width, tick_visible, is_crosshair) in primitive_labels
        .map(|(l, w, t)| (l, w, t, false))
        .chain(crosshair.map(|(l, w, t)| (l, w, t, true)))
    {
        let label_start = result.commands.len();
        let width = source_round(text_width) + 2. * f64::from(options.padding_horizontal);
        let mut x = (label.coordinate - width / 2.).floor() + 0.5;
        if x < 0. {
            x = 0.5;
        } else if x + width > bounds.width {
            x = (bounds.width - width).floor() + 0.5;
        }
        let snap = |value: f64, ratio: f64| source_round(value * ratio) / ratio;
        let right = snap(x + width, horizontal);
        let top = snap(x, horizontal);
        let height = snap(
            f64::from(
                options.border_size
                    + options.tick_length
                    + options.padding_top
                    + options.font_size
                    + options.padding_bottom,
            )
            .ceil(),
            vertical,
        );
        result.commands.push(TimeAxisCommand::RoundedRectangle {
            rect: AxisRect {
                origin: Point { x: top, y: 0. },
                size: AxisBounds {
                    width: right - top,
                    height,
                },
            },
            radii: [
                0.,
                0.,
                (source_round(2. * horizontal) / horizontal) as f32,
                (source_round(2. * horizontal) / horizontal) as f32,
            ],
            color: label.background.clone(),
        });
        if tick_visible {
            result.commands.push(TimeAxisCommand::Rectangle {
                rect: AxisRect {
                    origin: Point {
                        x: (source_round(label.coordinate * horizontal)
                            - (horizontal * 0.5).floor())
                            / horizontal,
                        y: 0.,
                    },
                    size: AxisBounds {
                        width: horizontal.floor().max(1.) / horizontal,
                        height: snap(f64::from(options.tick_length), vertical),
                    },
                },
                color: label.color.clone(),
            });
        }
        result.commands.push(TimeAxisCommand::Text {
            label: label.text.clone(),
            position: Point {
                x: x + f64::from(options.padding_horizontal) + source_round(text_width) / 2.,
                y,
            },
            emphasis: TimeLabelEmphasis::Normal,
            font_size: options.font_size,
            font_family: options.font_family.clone(),
            color: label.color.clone(),
        });
        if is_crosshair {
            result.screenshot_crosshair = label_start..result.commands.len();
        }
    }
    result.screenshot_top_labels = labels_start..result.commands.len();
    if snapshot.primitive_graphics {
        result.commands.push(TimeAxisCommand::PrimitiveLayer(
            crate::api::PrimitiveLayer::Top,
        ));
    }
    Ok(result)
}

#[cfg(test)]
pub(crate) mod tests;
