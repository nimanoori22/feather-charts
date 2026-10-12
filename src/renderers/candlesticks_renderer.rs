use crate::{model::time_data::SeriesItemsIndexesRange, renderers::grid_renderer::PixelRatio};

#[derive(Clone, Debug, PartialEq)]
pub struct CandlePoint {
    pub x: f64,
    pub open_y: f64,
    pub high_y: f64,
    pub low_y: f64,
    pub close_y: f64,
    pub color: String,
    pub border_color: String,
    pub wick_color: String,
}

/// Bitmap rectangle, retaining Canvas2D signed extents and paint order.
#[derive(Clone, Debug, PartialEq)]
pub struct CandleRectangle {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub color: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandlestickRendererData {
    pub items: Vec<CandlePoint>,
    pub visible_range: SeriesItemsIndexesRange,
    pub bar_spacing: f64,
    pub wick_visible: bool,
    pub border_visible: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandlestickRenderError {
    InvalidPixelRatio,
    InvalidData,
}

fn optimal_width(spacing: f64, ratio: f64) -> f64 {
    if (2.5..=4.).contains(&spacing) {
        return (3. * ratio).floor();
    }
    let coefficient = 1. - 0.2 * (spacing.max(4.) - 4.).atan() / std::f64::consts::FRAC_PI_2;
    (spacing * coefficient * ratio)
        .floor()
        .min((spacing * ratio).floor())
        .max(ratio.floor())
}
fn round(value: f64) -> f64 {
    (value + 0.5).floor()
}

impl CandlestickRendererData {
    pub fn draw_commands(
        &self,
        ratio: PixelRatio,
    ) -> Result<Vec<CandleRectangle>, CandlestickRenderError> {
        let h = f64::from(ratio.horizontal);
        let v = f64::from(ratio.vertical);
        if !h.is_finite() || !v.is_finite() || h <= 0. || v <= 0. {
            return Err(CandlestickRenderError::InvalidPixelRatio);
        }
        if !self.bar_spacing.is_finite()
            || self.bar_spacing <= 0.
            || self.visible_range.from > self.visible_range.to
            || self.visible_range.to > self.items.len()
        {
            return Err(CandlestickRenderError::InvalidData);
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
            return Err(CandlestickRenderError::InvalidData);
        }
        let mut width = optimal_width(self.bar_spacing, h);
        if !width.is_finite() {
            return Err(CandlestickRenderError::InvalidData);
        }
        if width >= 2. && width % 2. != h.floor() % 2. {
            width -= 1.;
        }
        let border = h.floor();
        let mut commands = vec![];
        let mut rect = |x, y, width, height, color: &str| {
            commands.push(CandleRectangle {
                x,
                y,
                width,
                height,
                color: color.into(),
            });
        };
        if self.wick_visible {
            let wick = h
                .floor()
                .max(h.floor().min((self.bar_spacing * h).floor()).min(width));
            let offset = (wick * 0.5).floor();
            let mut previous: Option<f64> = None;
            for b in bars {
                let top = round(b.open_y.min(b.close_y) * v);
                let bottom = round(b.open_y.max(b.close_y) * v);
                let mut left = round(b.x * h) - offset;
                let right = left + wick - 1.;
                if let Some(edge) = previous {
                    left = left.max(edge + 1.).min(right);
                }
                rect(
                    left,
                    round(b.high_y * v),
                    right - left + 1.,
                    top - round(b.high_y * v),
                    &b.wick_color,
                );
                rect(
                    left,
                    bottom + 1.,
                    right - left + 1.,
                    round(b.low_y * v) - bottom,
                    &b.wick_color,
                );
                previous = Some(right);
            }
        }
        if self.border_visible {
            let mut previous: Option<f64> = None;
            for b in bars {
                let mut left = round(b.x * h) - (width * 0.5).floor();
                let right = left + width - 1.;
                let top = round(b.open_y.min(b.close_y) * v);
                let bottom = round(b.open_y.max(b.close_y) * v);
                if let Some(edge) = previous {
                    left = left.max(edge + 1.).min(right);
                }
                let w = right - left + 1.;
                let height = bottom - top + 1.;
                if self.bar_spacing * h > 2. * border {
                    rect(left + border, top, w - 2. * border, border, &b.border_color);
                    rect(
                        left + border,
                        top + height - border,
                        w - 2. * border,
                        border,
                        &b.border_color,
                    );
                    rect(left, top, border, height, &b.border_color);
                    rect(left + w - border, top, border, height, &b.border_color);
                } else {
                    rect(left, top, w, height, &b.border_color);
                }
                previous = Some(right);
            }
        }
        if !self.border_visible || width > border * 2. {
            for b in bars {
                let mut top = round(b.open_y.min(b.close_y) * v);
                let mut bottom = round(b.open_y.max(b.close_y) * v);
                let mut left = round(b.x * h) - (width * 0.5).floor();
                let mut right = left + width - 1.;
                if self.border_visible {
                    left += border;
                    top += border;
                    right -= border;
                    bottom -= border;
                }
                if top <= bottom {
                    rect(left, top, right - left + 1., bottom - top + 1., &b.color);
                }
            }
        }
        if commands
            .iter()
            .any(|r| [r.x, r.y, r.width, r.height].iter().any(|x| !x.is_finite()))
        {
            return Err(CandlestickRenderError::InvalidData);
        }
        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn data() -> CandlestickRendererData {
        CandlestickRendererData {
            items: vec![CandlePoint {
                x: 10.,
                open_y: 20.,
                high_y: 10.,
                low_y: 40.,
                close_y: 30.,
                color: "body".into(),
                border_color: "border".into(),
                wick_color: "wick".into(),
            }],
            visible_range: SeriesItemsIndexesRange { from: 0, to: 1 },
            bar_spacing: 8.,
            wick_visible: true,
            border_visible: true,
        }
    }
    #[test]
    fn layers_doji_and_narrow_bodies_follow_source() {
        let mut data = data();
        let ratio = PixelRatio {
            horizontal: 1.,
            vertical: 1.,
        };
        let commands = data.draw_commands(ratio).unwrap();
        assert_eq!(
            commands
                .iter()
                .map(|r| r.color.as_str())
                .collect::<Vec<_>>(),
            vec![
                "wick", "wick", "border", "border", "border", "border", "body"
            ]
        );
        assert_eq!((commands[0].y, commands[0].height), (10., 10.));
        data.items[0].close_y = 20.;
        assert_eq!(data.draw_commands(ratio).unwrap().len(), 6);
        data.border_visible = false;
        assert_eq!(
            data.draw_commands(ratio).unwrap().last().unwrap().height,
            1.
        );
        data.border_visible = true;
        data.bar_spacing = 1.;
        assert!(
            data.draw_commands(ratio)
                .unwrap()
                .iter()
                .all(|r| r.color != "body")
        );
    }
    #[test]
    fn invalid_inputs_and_empty_ranges_are_safe() {
        let mut data = data();
        let ratio = PixelRatio {
            horizontal: 1.5,
            vertical: 2.,
        };
        data.visible_range.to = 0;
        assert!(data.draw_commands(ratio).unwrap().is_empty());
        data.visible_range.to = 2;
        assert_eq!(
            data.draw_commands(ratio),
            Err(CandlestickRenderError::InvalidData)
        );
        data.visible_range.to = 1;
        data.items[0].x = f64::NAN;
        assert!(data.draw_commands(ratio).is_err());
        assert_eq!(
            data.draw_commands(PixelRatio {
                horizontal: 0.,
                vertical: 1.
            }),
            Err(CandlestickRenderError::InvalidPixelRatio)
        );
    }
}
