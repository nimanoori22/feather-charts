//! Pane-owned, viewport-relative text using the existing primitive lifecycle.
use super::{DrawingPrimitive, PreparedPrimitive, PrimitiveContext, PrimitiveLayer, PrimitiveView};
use crate::ui::{price_axis::AxisFontResolver, text_watermark::PreparedTextWatermark};
use iced::{
    Font,
    alignment::{Horizontal, Vertical},
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, PartialEq)]
pub struct TextWatermarkLine {
    pub text: String,
    pub color: String,
    pub font: Font,
    pub font_size: f32,
    /// None or zero uses 1.2 times the font size, matching the source.
    pub line_height: Option<f32>,
}
impl Default for TextWatermarkLine {
    fn default() -> Self {
        Self {
            text: String::new(),
            color: "rgba(0, 0, 0, 0.5)".into(),
            font: Font::DEFAULT,
            font_size: 48.,
            line_height: None,
        }
    }
}
impl TextWatermarkLine {
    pub(crate) fn height(&self) -> f64 {
        self.line_height
            .filter(|v| *v != 0.)
            .map_or(f64::from(self.font_size) * 1.2, f64::from)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct TextWatermarkOptions {
    pub visible: bool,
    pub horizontal_alignment: Horizontal,
    pub vertical_alignment: Vertical,
    pub lines: Vec<TextWatermarkLine>,
}
impl Default for TextWatermarkOptions {
    fn default() -> Self {
        Self {
            visible: true,
            horizontal_alignment: Horizontal::Center,
            vertical_alignment: Vertical::Center,
            lines: vec![],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextWatermarkError {
    InvalidFontSize { line: usize },
    InvalidLineHeight { line: usize },
    InvalidColor { line: usize },
}
impl std::fmt::Display for TextWatermarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TextWatermarkError {}

pub struct TextWatermark {
    options: TextWatermarkOptions,
    colors: Vec<iced::Color>,
    fonts: AxisFontResolver,
    prepared: RefCell<Option<(iced::Size, Rc<PreparedTextWatermark>)>>,
}
impl std::fmt::Debug for TextWatermark {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextWatermark")
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}
impl TextWatermark {
    pub fn new(mut options: TextWatermarkOptions) -> Result<Self, TextWatermarkError> {
        let mut colors = vec![];
        for (line, o) in options.lines.iter_mut().enumerate() {
            if !o.font_size.is_finite() || o.font_size <= 0. || !(o.font_size * 1.2).is_finite() {
                return Err(TextWatermarkError::InvalidFontSize { line });
            }
            if o.line_height.is_some_and(|h| !h.is_finite() || h < 0.) {
                return Err(TextWatermarkError::InvalidLineHeight { line });
            }
            colors.push(
                crate::ui::line_chart::color(&o.color)
                    .map_err(|_| TextWatermarkError::InvalidColor { line })?,
            );
            o.text = o
                .text
                .chars()
                .map(|c| {
                    if matches!(c, '\t' | '\n' | '\x0c' | '\r') {
                        ' '
                    } else {
                        c
                    }
                })
                .collect();
        }
        Ok(Self {
            options,
            colors,
            fonts: AxisFontResolver::default(),
            prepared: RefCell::new(None),
        })
    }
}
impl<B: super::ChartBehavior> DrawingPrimitive<B> for TextWatermark {
    fn prepare(&self, context: &PrimitiveContext<'_, B>) -> PreparedPrimitive {
        let size = context.size;
        if !self.options.visible
            || self.options.lines.is_empty()
            || !size.width.is_finite()
            || !size.height.is_finite()
            || size.width <= 0.
            || size.height <= 0.
        {
            return PreparedPrimitive::default();
        }
        let mut cached = self.prepared.borrow_mut();
        if cached
            .as_ref()
            .is_none_or(|(previous, _)| *previous != size)
        {
            *cached = Some((
                size,
                Rc::new(crate::ui::text_watermark::prepare(
                    &self.options,
                    &self.colors,
                    &self.fonts,
                    size,
                )),
            ));
        }
        PreparedPrimitive {
            views: vec![PrimitiveView {
                layer: PrimitiveLayer::Normal,
                renderer: cached.as_ref().expect("prepared watermark").1.clone(),
            }],
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Chart, PrimitiveOwner};
    #[test]
    fn unchanged_preparation_and_resize_do_not_reshape_text() {
        let watermark = TextWatermark::new(TextWatermarkOptions {
            lines: vec![TextWatermarkLine {
                text: "BTCUSD".into(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();
        let chart = Chart::default();
        let mut context = PrimitiveContext {
            owner: PrimitiveOwner::Pane(super::super::primitives::pane_id()),
            pane: 0,
            size: iced::Size::new(800., 500.),
            pixel_ratio: 1.,
            model: chart.model(),
            visible_range: None,
        };
        let first = watermark.prepare(&context);
        assert_eq!(watermark.fonts.work_counts(), (1, 1));
        for _ in 0..100 {
            let next = watermark.prepare(&context);
            assert!(Rc::ptr_eq(
                &first.views[0].renderer,
                &next.views[0].renderer
            ));
        }
        context.pixel_ratio = 2.;
        watermark.prepare(&context);
        assert_eq!(watermark.fonts.work_counts(), (1, 1));
        context.size = iced::Size::new(80., 40.);
        let resized = watermark.prepare(&context);
        assert!(!Rc::ptr_eq(
            &first.views[0].renderer,
            &resized.views[0].renderer
        ));
        assert_eq!(watermark.fonts.work_counts(), (1, 1));
    }
    #[test]
    fn canvas_ascii_whitespace_is_normalized_before_native_shaping() {
        let watermark = TextWatermark::new(TextWatermarkOptions {
            lines: vec![TextWatermarkLine {
                text: "A\tB\nC\rD\u{c}E".into(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();
        assert_eq!(watermark.options.lines[0].text, "A B C D E");
    }
}
