//! Backend-neutral contract for price-axis label renderers.

use crate::{
    model::text_width_cache::{TextMeasurer, TextWidthCache},
    renderers::draw_line::LineWidth,
};

#[derive(Clone, Debug, PartialEq)]
pub struct PriceAxisViewRendererCommonData {
    pub active_background: Option<String>,
    pub background: String,
    pub coordinate: f32,
    pub fixed_coordinate: Option<f32>,
    pub render_coordinate: Option<f32>,
    pub additional_padding_top: f32,
    pub additional_padding_bottom: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PriceAxisViewRendererData {
    pub visible: bool,
    pub text: String,
    pub tick_visible: bool,
    pub move_text_to_invisible_tick: bool,
    pub border_color: String,
    pub color: String,
    pub line_width: Option<LineWidth>,
    pub border_visible: bool,
    pub separator_visible: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PriceAxisViewRendererOptions {
    pub baseline_offset: f32,
    pub border_size: f32,
    pub font: String,
    pub font_family: String,
    pub color: String,
    pub pane_background_color: String,
    pub font_size: f32,
    pub padding_bottom: f32,
    pub padding_inner: f32,
    pub padding_outer: f32,
    pub padding_top: f32,
    pub tick_length: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PriceAxisLabelAlignment {
    Left,
    Right,
}

/// Renders one price-axis label using a caller-owned drawing target.
///
/// `Target` represents a future backend adapter, rather than a browser canvas.
/// It supplies text measurement alongside the drawing operations needed by a
/// concrete renderer implementation.
pub trait PriceAxisViewRenderer<Target: TextMeasurer> {
    fn draw(
        &self,
        target: &mut Target,
        renderer_options: &PriceAxisViewRendererOptions,
        text_width_cache: &mut TextWidthCache,
        align: PriceAxisLabelAlignment,
    );

    fn height(&self, renderer_options: &PriceAxisViewRendererOptions, use_second_line: bool)
    -> f32;

    fn set_data(
        &mut self,
        data: PriceAxisViewRendererData,
        common_data: PriceAxisViewRendererCommonData,
    );
}

/// Creates a concrete renderer for a price-axis view.
///
/// This is the Rust equivalent of Lightweight Charts' injectable renderer
/// constructor. A closure with this signature implements the trait directly.
pub trait PriceAxisViewRendererFactory<Target: TextMeasurer> {
    fn create(
        &self,
        data: PriceAxisViewRendererData,
        common_data: PriceAxisViewRendererCommonData,
    ) -> Box<dyn PriceAxisViewRenderer<Target>>;
}

impl<Target, Factory> PriceAxisViewRendererFactory<Target> for Factory
where
    Target: TextMeasurer,
    Factory: Fn(
        PriceAxisViewRendererData,
        PriceAxisViewRendererCommonData,
    ) -> Box<dyn PriceAxisViewRenderer<Target>>,
{
    fn create(
        &self,
        data: PriceAxisViewRendererData,
        common_data: PriceAxisViewRendererCommonData,
    ) -> Box<dyn PriceAxisViewRenderer<Target>> {
        self(data, common_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TestTarget;

    impl TextMeasurer for TestTarget {
        fn measure_text(&mut self, text: &str) -> crate::model::text_width_cache::TextMetrics {
            crate::model::text_width_cache::TextMetrics {
                width: text.len() as f32,
                ..Default::default()
            }
        }
    }

    struct TestRenderer;

    impl PriceAxisViewRenderer<TestTarget> for TestRenderer {
        fn draw(
            &self,
            target: &mut TestTarget,
            _renderer_options: &PriceAxisViewRendererOptions,
            text_width_cache: &mut TextWidthCache,
            _align: PriceAxisLabelAlignment,
        ) {
            text_width_cache.measure_text(target, "price");
        }

        fn height(
            &self,
            renderer_options: &PriceAxisViewRendererOptions,
            _use_second_line: bool,
        ) -> f32 {
            renderer_options.font_size
        }

        fn set_data(
            &mut self,
            _data: PriceAxisViewRendererData,
            _common_data: PriceAxisViewRendererCommonData,
        ) {
        }
    }

    #[test]
    fn contract_supports_backend_owned_text_measurement() {
        let mut target = TestTarget;
        let mut cache = TextWidthCache::default();
        let options = PriceAxisViewRendererOptions {
            baseline_offset: 0.0,
            border_size: 1.0,
            font: "12px sans-serif".into(),
            font_family: "sans-serif".into(),
            color: "#000".into(),
            pane_background_color: "#fff".into(),
            font_size: 12.0,
            padding_bottom: 1.0,
            padding_inner: 1.0,
            padding_outer: 1.0,
            padding_top: 1.0,
            tick_length: 4.0,
        };
        let data = PriceAxisViewRendererData {
            visible: true,
            text: "price".into(),
            tick_visible: true,
            move_text_to_invisible_tick: false,
            border_color: "#000".into(),
            color: "#fff".into(),
            line_width: Some(LineWidth::One),
            border_visible: true,
            separator_visible: true,
        };
        let common_data = PriceAxisViewRendererCommonData {
            active_background: None,
            background: "#000".into(),
            coordinate: 4.0,
            fixed_coordinate: None,
            render_coordinate: None,
            additional_padding_top: 0.0,
            additional_padding_bottom: 0.0,
        };

        let factory =
            |_: PriceAxisViewRendererData,
             _: PriceAxisViewRendererCommonData|
             -> Box<dyn PriceAxisViewRenderer<TestTarget>> { Box::new(TestRenderer) };
        let renderer = factory.create(data, common_data);
        renderer.draw(
            &mut target,
            &options,
            &mut cache,
            PriceAxisLabelAlignment::Right,
        );
        assert_eq!(renderer.height(&options, false), 12.0);
    }
}
