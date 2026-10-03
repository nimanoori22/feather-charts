//! Pure one-pane layout in logical pixels. Requests already include the scale's
//! minimum sizes and source size hints; they must not be rounded a second time.
use crate::renderers::{
    grid_renderer::PixelRatio,
    line_renderer::Point,
    price_axis_renderer::{AxisBounds, AxisRect},
};

pub type LayoutRect = AxisRect;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AxisSizeRequests {
    pub left_width: f64,
    pub right_width: f64,
    pub time_height: f64,
}
impl AxisSizeRequests {
    pub fn grow_to(self, measured: Self) -> Self {
        // A zero request represents a hidden axis and must release its slot.
        let grow = |old: f64, new: f64| if new == 0. { 0. } else { old.max(new) };
        Self {
            left_width: grow(self.left_width, measured.left_width),
            right_width: grow(self.right_width, measured.right_width),
            time_height: grow(self.time_height, measured.time_height),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartLayout {
    pub available: AxisBounds,
    pub chart_size: AxisBounds,
    pub plot: LayoutRect,
    pub left_axis: LayoutRect,
    pub right_axis: LayoutRect,
    pub time_axis: LayoutRect,
    pub left_corner: LayoutRect,
    pub right_corner: LayoutRect,
}
impl Default for ChartLayout {
    fn default() -> Self {
        let rect = rect(0., 0., 0., 0.);
        Self {
            available: AxisBounds::default(),
            chart_size: AxisBounds::default(),
            plot: rect,
            left_axis: rect,
            right_axis: rect,
            time_axis: rect,
            left_corner: rect,
            right_corner: rect,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartLayoutError {
    InvalidBounds,
    InvalidRequests,
    InvalidPixelRatio,
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> LayoutRect {
    LayoutRect {
        origin: Point { x, y },
        size: AxisBounds { width, height },
    }
}

pub fn allocate_chart_layout(
    available: AxisBounds,
    requests: AxisSizeRequests,
    ratio: PixelRatio,
) -> Result<ChartLayout, ChartLayoutError> {
    if [available.width, available.height]
        .iter()
        .any(|v| !v.is_finite() || *v < 0. || *v > f64::from(f32::MAX))
    {
        return Err(ChartLayoutError::InvalidBounds);
    }
    if [
        requests.left_width,
        requests.right_width,
        requests.time_height,
    ]
    .iter()
    .any(|v| !v.is_finite() || *v < 0. || *v > f64::from(f32::MAX))
    {
        return Err(ChartLayoutError::InvalidRequests);
    }
    if [ratio.horizontal, ratio.vertical]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
    {
        return Err(ChartLayoutError::InvalidPixelRatio);
    }
    let even_floor = |v: f64| {
        let v = v.floor();
        v - v % 2.
    };
    let chart_size = AxisBounds {
        width: even_floor(available.width),
        height: even_floor(available.height),
    };
    // Explicit empty layouts suppress all regions, unlike the DOM's 2px pane.
    if chart_size.width == 0. || chart_size.height == 0. {
        return Ok(ChartLayout {
            available,
            chart_size,
            ..Default::default()
        });
    }
    let left = requests.left_width;
    let right = requests.right_width;
    let time = requests.time_height;
    let width = (chart_size.width - left - right).max(0.);
    let vertical = f64::from(ratio.vertical);
    // Source last-pane rounding and minimum. Oversized axes/minimum panes are
    // clipped by the chart region, not silently shrunk to different model sizes.
    let height = (((chart_size.height - time).max(0.) * vertical).ceil() / vertical).max(2.);
    Ok(ChartLayout {
        available,
        chart_size,
        plot: rect(left, 0., width, height),
        left_axis: rect(0., 0., left, height),
        right_axis: rect(left + width, 0., right, height),
        time_axis: rect(left, height, width, time),
        left_corner: rect(0., height, left, time),
        right_corner: rect(left + width, height, right, time),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ratio(r: f32) -> PixelRatio {
        PixelRatio {
            horizontal: r,
            vertical: r,
        }
    }
    #[test]
    fn regions_share_edges_for_every_visibility_and_fractional_ratio() {
        for r in [1., 1.25, 1.5, 2.] {
            for left in [0., 62.] {
                for right in [0., 80.] {
                    for time in [0., 28.] {
                        let l = allocate_chart_layout(
                            AxisBounds {
                                width: 801.75,
                                height: 501.5,
                            },
                            AxisSizeRequests {
                                left_width: left,
                                right_width: right,
                                time_height: time,
                            },
                            ratio(r),
                        )
                        .unwrap();
                        assert_eq!(
                            l.chart_size,
                            AxisBounds {
                                width: 800.,
                                height: 500.
                            }
                        );
                        assert_eq!(
                            l.left_axis.origin.x + l.left_axis.size.width,
                            l.plot.origin.x
                        );
                        assert_eq!(l.plot.origin.x + l.plot.size.width, l.right_axis.origin.x);
                        assert_eq!(l.time_axis.origin.x, l.plot.origin.x);
                        assert_eq!(l.time_axis.size.width, l.plot.size.width);
                        assert_eq!(l.left_axis.size.height, l.plot.size.height);
                        assert_eq!(l.right_axis.size.height, l.plot.size.height);
                        assert_eq!(l.time_axis.origin.y, l.plot.size.height);
                        assert_eq!(l.left_corner.origin.y, l.time_axis.origin.y);
                        assert_eq!(l.right_corner.origin.x, l.right_axis.origin.x);
                        assert_eq!(l.time_axis.size.height, time);
                    }
                }
            }
        }
    }
    #[test]
    fn source_tiny_pane_overflow_and_explicit_empty_layouts() {
        let q = AxisSizeRequests {
            left_width: 62.,
            right_width: 80.,
            time_height: 28.,
        };
        let small = allocate_chart_layout(
            AxisBounds {
                width: 100.,
                height: 10.,
            },
            q,
            ratio(1.5),
        )
        .unwrap();
        assert_eq!(
            small.plot.size,
            AxisBounds {
                width: 0.,
                height: 2.
            }
        );
        assert_eq!(small.left_axis.size.width, 62.);
        assert_eq!(small.right_axis.size.width, 80.);
        assert!(small.right_axis.origin.x + small.right_axis.size.width > small.chart_size.width);
        for bounds in [
            AxisBounds {
                width: 0.,
                height: 100.,
            },
            AxisBounds {
                width: 100.,
                height: 0.,
            },
            AxisBounds {
                width: 1.,
                height: 100.,
            },
        ] {
            let empty = allocate_chart_layout(bounds, q, ratio(1.)).unwrap();
            assert_eq!(empty.plot.size, AxisBounds::default());
            assert_eq!(empty.time_axis.size, AxisBounds::default());
        }
    }
    #[test]
    fn normalized_minima_are_not_hinted_twice_and_growth_can_release_hidden_axes() {
        let q = AxisSizeRequests {
            left_width: 83.,
            right_width: 0.,
            time_height: 33.,
        };
        let layout = allocate_chart_layout(
            AxisBounds {
                width: 400.,
                height: 200.,
            },
            q,
            ratio(1.),
        )
        .unwrap();
        assert_eq!(layout.left_axis.size.width, 83.);
        assert_eq!(layout.time_axis.size.height, 33.);
        assert_eq!(
            q.grow_to(AxisSizeRequests {
                left_width: 80.,
                right_width: 60.,
                time_height: 0.
            }),
            AxisSizeRequests {
                left_width: 83.,
                right_width: 60.,
                time_height: 0.
            }
        );
    }
    #[test]
    fn invalid_layout_inputs_return_errors() {
        assert_eq!(
            allocate_chart_layout(
                AxisBounds {
                    width: f64::NAN,
                    height: 20.
                },
                AxisSizeRequests::default(),
                ratio(1.)
            ),
            Err(ChartLayoutError::InvalidBounds)
        );
        assert_eq!(
            allocate_chart_layout(
                AxisBounds::default(),
                AxisSizeRequests {
                    right_width: -1.,
                    ..Default::default()
                },
                ratio(1.)
            ),
            Err(ChartLayoutError::InvalidRequests)
        );
        assert_eq!(
            allocate_chart_layout(
                AxisBounds::default(),
                AxisSizeRequests::default(),
                ratio(0.)
            ),
            Err(ChartLayoutError::InvalidPixelRatio)
        );
    }
}
