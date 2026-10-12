//! Shared chart dimensions with source-compatible weighted pane allocation.
use super::chart_layout::{
    AxisSizeRequests, ChartLayout, ChartLayoutError, LayoutRect, allocate_chart_layout,
};
use crate::renderers::{grid_renderer::PixelRatio, price_axis_renderer::AxisBounds};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MultiPaneLayout {
    pub chart: ChartLayout,
    pub panes: Vec<ChartLayout>,
    pub separators: Vec<LayoutRect>,
}

pub fn allocate_multi_pane_layout(
    bounds: AxisBounds,
    requests: AxisSizeRequests,
    ratio: PixelRatio,
    stretches: &[f64],
) -> Result<MultiPaneLayout, ChartLayoutError> {
    if stretches.is_empty()
        || stretches.iter().any(|n| !n.is_finite() || *n <= 0.)
        || !stretches.iter().sum::<f64>().is_finite()
    {
        return Err(ChartLayoutError::InvalidRequests);
    }
    let mut chart = allocate_chart_layout(bounds, requests, ratio)?;
    if chart.chart_size.width == 0. || chart.chart_size.height == 0. {
        return Ok(MultiPaneLayout {
            panes: vec![chart; stretches.len()],
            chart,
            separators: vec![],
        });
    }
    let total =
        (chart.chart_size.height - requests.time_height - (stretches.len() - 1) as f64).max(0.);
    let per_stretch = total / stretches.iter().sum::<f64>();
    let pixel = f64::from(ratio.vertical);
    let width = f64::from(chart.plot.size.width as f32);
    chart.plot.size.width = width;
    chart.time_axis.size.width = width;
    chart.right_axis.origin.x = chart.plot.origin.x + width;
    chart.right_corner.origin.x = chart.right_axis.origin.x;
    let mut used = 0.;
    let mut y = 0.;
    let mut panes = vec![];
    let mut separators = vec![];
    for (i, stretch) in stretches.iter().enumerate() {
        let height = if i + 1 == stretches.len() {
            ((total - used) * pixel).ceil() / pixel
        } else {
            (stretch * per_stretch * pixel + 0.5).floor() / pixel
        };
        used += height.max(2.);
        let height = f64::from(height.max(2.) as f32);
        let mut pane = chart;
        for rect in [&mut pane.plot, &mut pane.left_axis, &mut pane.right_axis] {
            rect.origin.y = y;
            rect.size.height = height;
        }
        y += height;
        panes.push(pane);
        if i + 1 < stretches.len() {
            separators.push(LayoutRect {
                origin: crate::renderers::line_renderer::Point { x: 0., y },
                size: AxisBounds {
                    width: chart.chart_size.width,
                    height: 1.,
                },
            });
            y += 1.;
        }
    }
    for rect in [
        &mut chart.time_axis,
        &mut chart.left_corner,
        &mut chart.right_corner,
    ] {
        rect.origin.y = y;
    }
    for pane in &mut panes {
        pane.time_axis = chart.time_axis;
        pane.left_corner = chart.left_corner;
        pane.right_corner = chart.right_corner;
    }
    Ok(MultiPaneLayout {
        chart,
        panes,
        separators,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_width_weighted_heights_and_empty_bounds() {
        for r in [1., 1.25, 1.5, 2.] {
            let ratio = PixelRatio {
                horizontal: r,
                vertical: r,
            };
            let layout = allocate_multi_pane_layout(
                AxisBounds {
                    width: 800.,
                    height: 500.,
                },
                AxisSizeRequests {
                    left_width: 62.,
                    right_width: 80.,
                    time_height: 28.,
                },
                ratio,
                &[2., 1.],
            )
            .unwrap();
            assert_eq!(layout.panes[0].plot.size.width, 658.);
            assert_eq!(layout.panes[1].plot.size.width, 658.);
            assert_eq!(
                layout.panes[1].plot.origin.y,
                layout.panes[0].plot.size.height + 1.
            );
            assert_eq!(
                layout.chart.time_axis.origin.y,
                layout.panes[1].plot.origin.y + layout.panes[1].plot.size.height
            );
            assert!(layout.panes[0].plot.size.height > layout.panes[1].plot.size.height);
            let empty = allocate_multi_pane_layout(
                AxisBounds::default(),
                AxisSizeRequests::default(),
                ratio,
                &[2., 1.],
            )
            .unwrap();
            assert!(empty.panes.iter().all(|p| p.plot.size.height == 0.));
            assert!(
                allocate_multi_pane_layout(
                    AxisBounds::default(),
                    AxisSizeRequests::default(),
                    ratio,
                    &[0.]
                )
                .is_err()
            );
        }
    }
}
