use super::candlesticks_renderer::CandleRectangle;
use crate::{model::time_data::SeriesItemsIndexesRange, renderers::grid_renderer::PixelRatio};
#[derive(Clone, Debug, PartialEq)]
pub struct BarPoint {
    pub x: f64,
    pub open_y: f64,
    pub high_y: f64,
    pub low_y: f64,
    pub close_y: f64,
    pub color: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BarsRendererData {
    pub items: Vec<BarPoint>,
    pub visible_range: SeriesItemsIndexesRange,
    pub bar_spacing: f64,
    pub open_visible: bool,
    pub thin_bars: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarRenderError {
    InvalidPixelRatio,
    InvalidData,
}
impl BarsRendererData {
    pub fn draw_commands(&self, ratio: PixelRatio) -> Result<Vec<CandleRectangle>, BarRenderError> {
        let h = f64::from(ratio.horizontal);
        let v = f64::from(ratio.vertical);
        if !h.is_finite() || !v.is_finite() || h <= 0. || v <= 0. {
            return Err(BarRenderError::InvalidPixelRatio);
        }
        if !self.bar_spacing.is_finite()
            || self.bar_spacing <= 0.
            || self.visible_range.from > self.visible_range.to
            || self.visible_range.to > self.items.len()
        {
            return Err(BarRenderError::InvalidData);
        }
        let bars = &self.items[self.visible_range.from..self.visible_range.to];
        if bars.iter().any(|b| {
            [
                b.x * h,
                b.open_y * v,
                b.high_y * v,
                b.low_y * v,
                b.close_y * v,
            ]
            .iter()
            .any(|x| !x.is_finite())
        }) {
            return Err(BarRenderError::InvalidData);
        }
        let mut width = h.floor().max((self.bar_spacing * 0.3 * h).floor());
        if !width.is_finite() {
            return Err(BarRenderError::InvalidData);
        }
        if width >= 2. && width % 2. != h.floor().max(1.) % 2. {
            width -= 1.;
        }
        let stem = if self.thin_bars {
            width.min(h.floor())
        } else {
            width
        };
        let ticks = stem <= width && self.bar_spacing >= (1.5 * h).floor();
        let half = (stem * 0.5).floor();
        let side = (width * 1.5).ceil();
        let round = |x: f64| (x + 0.5).floor();
        let mut commands = Vec::with_capacity(bars.len() * 3);
        for b in bars {
            let center = round(b.x * h);
            let left = center - half;
            let right = left + stem - 1.;
            let top = round(b.high_y.min(b.low_y) * v) - half;
            let height = (round(b.high_y.max(b.low_y) * v) + half - top).max(stem);
            commands.push(CandleRectangle {
                x: left,
                y: top,
                width: stem,
                height,
                color: b.color.clone(),
            });
            if ticks {
                let tick_top = |value: f64| {
                    let start = top.max(round(value * v) - half);
                    if start + stem - 1. > top + height - 1. {
                        top + height - stem
                    } else {
                        start
                    }
                };
                if self.open_visible {
                    commands.push(CandleRectangle {
                        x: center - side,
                        y: tick_top(b.open_y),
                        width: left - (center - side),
                        height: stem,
                        color: b.color.clone(),
                    });
                }
                commands.push(CandleRectangle {
                    x: right + 1.,
                    y: tick_top(b.close_y),
                    width: center + side - right,
                    height: stem,
                    color: b.color.clone(),
                });
            }
        }
        if commands
            .iter()
            .any(|r| [r.x, r.y, r.width, r.height].iter().any(|x| !x.is_finite()))
        {
            return Err(BarRenderError::InvalidData);
        }
        Ok(commands)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn data() -> BarsRendererData {
        BarsRendererData {
            items: vec![BarPoint {
                x: 10.,
                open_y: 20.,
                high_y: 10.,
                low_y: 40.,
                close_y: 30.,
                color: "red".into(),
            }],
            visible_range: SeriesItemsIndexesRange { from: 0, to: 1 },
            bar_spacing: 8.,
            open_visible: true,
            thin_bars: true,
        }
    }
    #[test]
    fn ticks_stems_compression_and_inversion_follow_source() {
        let mut d = data();
        let ratio = PixelRatio {
            horizontal: 1.,
            vertical: 1.,
        };
        let r = d.draw_commands(ratio).unwrap();
        assert_eq!(r.len(), 3);
        assert_eq!(
            (r[0].x, r[0].y, r[0].width, r[0].height),
            (10., 10., 1., 30.)
        );
        assert_eq!((r[1].x, r[1].y, r[1].width, r[1].height), (8., 20., 2., 1.));
        d.open_visible = false;
        assert_eq!(d.draw_commands(ratio).unwrap().len(), 2);
        d.bar_spacing = 0.5;
        assert_eq!(d.draw_commands(ratio).unwrap().len(), 1);
        d.items[0].high_y = 40.;
        d.items[0].low_y = 10.;
        assert_eq!(d.draw_commands(ratio).unwrap()[0], r[0]);
        d.items[0].low_y = 40.;
        assert_eq!(d.draw_commands(ratio).unwrap()[0].height, 1.);
    }
    #[test]
    fn invalid_inputs_and_empty_ranges_are_safe() {
        let mut d = data();
        let ratio = PixelRatio {
            horizontal: 1.5,
            vertical: 2.,
        };
        d.visible_range.to = 0;
        assert!(d.draw_commands(ratio).unwrap().is_empty());
        d.visible_range.to = 2;
        assert_eq!(d.draw_commands(ratio), Err(BarRenderError::InvalidData));
        d.visible_range.to = 1;
        d.items[0].x = f64::NAN;
        assert!(d.draw_commands(ratio).is_err());
        assert_eq!(
            d.draw_commands(PixelRatio {
                horizontal: 0.,
                vertical: 1.
            }),
            Err(BarRenderError::InvalidPixelRatio)
        );
    }
}
