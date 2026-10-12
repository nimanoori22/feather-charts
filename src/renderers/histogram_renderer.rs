use crate::{
    model::time_data::SeriesItemsIndexesRange,
    renderers::{candlesticks_renderer::CandleRectangle, grid_renderer::PixelRatio},
};

#[derive(Clone, Debug, PartialEq)]
pub struct HistogramPoint {
    pub index: f64,
    pub x: f64,
    pub y: f64,
    pub color: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HistogramRendererData {
    pub items: Vec<HistogramPoint>,
    pub visible_range: SeriesItemsIndexesRange,
    pub bar_spacing: f64,
    pub baseline: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistogramRenderError {
    InvalidPixelRatio,
    InvalidData,
}

fn round(value: f64) -> f64 {
    (value + 0.5).floor()
}

impl HistogramRendererData {
    pub fn draw_commands(
        &self,
        ratio: PixelRatio,
    ) -> Result<Vec<CandleRectangle>, HistogramRenderError> {
        let h = f64::from(ratio.horizontal);
        let v = f64::from(ratio.vertical);
        if !h.is_finite() || !v.is_finite() || h <= 0. || v <= 0. {
            return Err(HistogramRenderError::InvalidPixelRatio);
        }
        if !self.bar_spacing.is_finite()
            || self.bar_spacing <= 0.
            || !(self.bar_spacing * h).is_finite()
            || !(self.baseline * v).is_finite()
            || self.visible_range.from > self.visible_range.to
            || self.visible_range.to > self.items.len()
        {
            return Err(HistogramRenderError::InvalidData);
        }
        let bars = &self.items[self.visible_range.from..self.visible_range.to];
        if bars
            .iter()
            .any(|b| [b.index, b.x * h, b.y * v].iter().any(|n| !n.is_finite()))
        {
            return Err(HistogramRenderError::InvalidData);
        }
        let spacing = if (self.bar_spacing * h).ceil() <= 1. {
            0.
        } else {
            h.floor().max(1.)
        };
        let width = round(self.bar_spacing * h) - spacing;
        let mut edges = bars
            .iter()
            .map(|b| {
                let x = round(b.x * h);
                if width % 2. != 0. {
                    (x - (width - 1.) / 2., x + (width - 1.) / 2.)
                } else {
                    (x - width / 2., x + width / 2. - 1.)
                }
            })
            .collect::<Vec<_>>();
        for i in 1..bars.len() {
            if bars[i].index == bars[i - 1].index + 1.
                && edges[i].0 - edges[i - 1].1 != spacing + 1.
            {
                if round(bars[i - 1].x * h) > bars[i - 1].x * h {
                    edges[i - 1].1 = edges[i].0 - spacing - 1.;
                } else {
                    edges[i].0 = edges[i - 1].1 + spacing + 1.;
                }
            }
        }
        let mut min_width = (self.bar_spacing * h).ceil();
        for (left, right) in &mut edges {
            *right = right.max(*left);
            min_width = min_width.min(*right - *left + 1.);
        }
        if spacing > 0. && min_width < 4. {
            for ((left, right), bar) in edges.iter_mut().zip(bars) {
                if *right - *left + 1. > min_width {
                    if round(bar.x * h) > bar.x * h {
                        *right -= 1.;
                    } else {
                        *left += 1.;
                    }
                }
            }
        }
        let tick = v.floor().max(1.);
        let base_top = round(self.baseline * v) - (tick / 2.).floor();
        let base_bottom = base_top + tick;
        let commands = edges
            .into_iter()
            .zip(bars)
            .map(|((left, right), bar)| {
                let y = round(bar.y * v);
                let (top, bottom) = if y <= base_top {
                    (y, base_bottom)
                } else {
                    (base_top, y - (tick / 2.).floor() + tick)
                };
                CandleRectangle {
                    x: left,
                    y: top,
                    width: right - left + 1.,
                    height: bottom - top,
                    color: bar.color.clone(),
                }
            })
            .collect::<Vec<_>>();
        if commands
            .iter()
            .any(|r| [r.x, r.y, r.width, r.height].iter().any(|n| !n.is_finite()))
        {
            return Err(HistogramRenderError::InvalidData);
        }
        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn baseline_and_both_sides_remain_visible() {
        let data = HistogramRendererData {
            items: [5., 10., 15.]
                .into_iter()
                .enumerate()
                .map(|(i, y)| HistogramPoint {
                    index: i as f64,
                    x: i as f64 * 8.,
                    y,
                    color: format!("row{i}"),
                })
                .collect(),
            visible_range: SeriesItemsIndexesRange { from: 0, to: 3 },
            bar_spacing: 8.,
            baseline: 10.,
        };
        let result = data
            .draw_commands(PixelRatio {
                horizontal: 1.,
                vertical: 2.,
            })
            .unwrap();
        assert_eq!(
            result.iter().map(|r| (r.y, r.height)).collect::<Vec<_>>(),
            vec![(10., 11.), (19., 2.), (19., 12.)]
        );
        assert_eq!(result[1].color, "row1");
    }
    #[test]
    fn invalid_and_empty_data() {
        let mut data = HistogramRendererData {
            items: vec![],
            visible_range: SeriesItemsIndexesRange { from: 0, to: 0 },
            bar_spacing: 0.5,
            baseline: 0.,
        };
        let ratio = PixelRatio {
            horizontal: 1.25,
            vertical: 1.5,
        };
        assert!(data.draw_commands(ratio).unwrap().is_empty());
        data.visible_range.to = 1;
        assert_eq!(
            data.draw_commands(ratio),
            Err(HistogramRenderError::InvalidData)
        );
        assert_eq!(
            data.draw_commands(PixelRatio {
                horizontal: 0.,
                vertical: 1.
            }),
            Err(HistogramRenderError::InvalidPixelRatio)
        );
    }
}
