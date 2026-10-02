//! Derives price-axis label rendering options from current chart styling.

use crate::{
    helpers::make_font::make_font,
    renderers::iprice_axis_view_renderer::PriceAxisViewRendererOptions,
};

const BORDER_SIZE: f32 = 1.0;
const TICK_LENGTH: f32 = 5.0;

/// The chart state required to derive price-axis renderer options.
///
/// This deliberately contains only the data that the TypeScript provider reads
/// from `IChartModelBase`, keeping the provider independent from `ChartModel`.
#[derive(Clone, Copy, Debug)]
pub struct PriceAxisRendererStyle<'a> {
    pub font_size: f32,
    pub font_family: &'a str,
    pub text_color: &'a str,
    pub pane_background_color: &'a str,
}

/// Caches typography-derived options while refreshing chart colors each call.
///
/// A future price-axis widget or Iced adapter owns this provider and passes it a
/// borrowed style snapshot from the chart model. Consequently, no chart-model /
/// renderer-provider reference cycle can form.
#[derive(Debug)]
pub struct PriceAxisRendererOptionsProvider {
    cached: PriceAxisViewRendererOptions,
}

impl Default for PriceAxisRendererOptionsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl PriceAxisRendererOptionsProvider {
    pub fn new() -> Self {
        Self {
            cached: PriceAxisViewRendererOptions {
                baseline_offset: 0.0,
                border_size: BORDER_SIZE,
                font: String::new(),
                font_family: String::new(),
                color: String::new(),
                pane_background_color: String::new(),
                font_size: f32::NAN,
                padding_bottom: 0.0,
                padding_inner: 0.0,
                padding_outer: 0.0,
                padding_top: 0.0,
                tick_length: TICK_LENGTH,
            },
        }
    }

    /// Updates cached typography when needed and always refreshes colors.
    pub fn options(&mut self, style: PriceAxisRendererStyle<'_>) -> &PriceAxisViewRendererOptions {
        if self.cached.font_size != style.font_size || self.cached.font_family != style.font_family
        {
            self.cached.font_size = style.font_size;
            self.cached.font_family = style.font_family.to_owned();
            self.cached.font = make_font(style.font_size, Some(style.font_family), None);
            self.cached.padding_top = 2.5 / 12.0 * style.font_size;
            self.cached.padding_bottom = self.cached.padding_top;
            self.cached.padding_inner = style.font_size / 12.0 * self.cached.tick_length;
            self.cached.padding_outer = self.cached.padding_inner;
            self.cached.baseline_offset = 0.0;
        }

        self.cached.color = style.text_color.to_owned();
        self.cached.pane_background_color = style.pane_background_color.to_owned();
        &self.cached
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style<'a>(
        font_size: f32,
        font_family: &'a str,
        text_color: &'a str,
        pane_background_color: &'a str,
    ) -> PriceAxisRendererStyle<'a> {
        PriceAxisRendererStyle {
            font_size,
            font_family,
            text_color,
            pane_background_color,
        }
    }

    #[test]
    fn derives_lightweight_charts_defaults_and_typography() {
        let mut provider = PriceAxisRendererOptionsProvider::new();
        let options = provider.options(style(12.0, "Inter", "#111", "#fff"));

        assert_eq!(options.border_size, 1.0);
        assert_eq!(options.tick_length, 5.0);
        assert_eq!(options.font, "12px Inter");
        assert_eq!(options.padding_top, 2.5);
        assert_eq!(options.padding_bottom, 2.5);
        assert_eq!(options.padding_inner, 5.0);
        assert_eq!(options.padding_outer, 5.0);
        assert_eq!(options.baseline_offset, 0.0);
        assert_eq!(options.color, "#111");
        assert_eq!(options.pane_background_color, "#fff");
    }

    #[test]
    fn refreshes_colors_without_changing_cached_typography() {
        let mut provider = PriceAxisRendererOptionsProvider::new();
        provider.options(style(12.0, "Inter", "#111", "#fff"));

        let options = provider.options(style(12.0, "Inter", "#222", "#eee"));
        assert_eq!(options.font, "12px Inter");
        assert_eq!(options.padding_top, 2.5);
        assert_eq!(options.color, "#222");
        assert_eq!(options.pane_background_color, "#eee");
    }

    #[test]
    fn recomputes_typography_when_font_changes() {
        let mut provider = PriceAxisRendererOptionsProvider::new();
        provider.options(style(12.0, "Inter", "#111", "#fff"));

        let options = provider.options(style(18.0, "Roboto", "#111", "#fff"));
        assert_eq!(options.font, "18px Roboto");
        assert_eq!(options.padding_top, 3.75);
        assert_eq!(options.padding_inner, 7.5);
        assert_eq!(options.font_family, "Roboto");
    }
}
