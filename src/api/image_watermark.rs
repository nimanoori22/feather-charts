//! A pane-owned decoded image; IO and decoding belong to the host.
use super::{DrawingPrimitive, PreparedPrimitive, PrimitiveContext, PrimitiveLayer, PrimitiveView};
use crate::renderers::image_watermark_renderer::{ImageWatermarkRenderer, placement};
use iced::advanced::image::Handle;
use std::{cell::RefCell, rc::Rc};
#[derive(Clone, Debug, PartialEq)]
pub struct ImageWatermarkOptions {
    pub alpha: f32,
    pub padding: f32,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
}
impl Default for ImageWatermarkOptions {
    fn default() -> Self {
        Self {
            alpha: 1.,
            padding: 0.,
            max_width: None,
            max_height: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageWatermarkError {
    InvalidDimensions,
    InvalidBufferLength,
    InvalidAlpha,
    InvalidPadding,
    InvalidMaxWidth,
    InvalidMaxHeight,
}
impl std::fmt::Display for ImageWatermarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ImageWatermarkError {}
impl ImageWatermarkOptions {
    fn validate(&self) -> Result<(), ImageWatermarkError> {
        use ImageWatermarkError::*;
        if !self.alpha.is_finite() || !(0. ..=1.).contains(&self.alpha) {
            return Err(InvalidAlpha);
        }
        if !self.padding.is_finite() || self.padding < 0. {
            return Err(InvalidPadding);
        }
        for (value, error) in [
            (self.max_width, InvalidMaxWidth),
            (self.max_height, InvalidMaxHeight),
        ] {
            if value.is_some_and(|v| !v.is_finite() || v < 0.) {
                return Err(error);
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct ImageWatermark {
    image: Handle,
    width: u32,
    height: u32,
    options: ImageWatermarkOptions,
    prepared: RefCell<Option<(iced::Size, Rc<ImageWatermarkRenderer>)>>,
}
impl ImageWatermark {
    pub fn from_rgba(
        width: u32,
        height: u32,
        pixels: impl Into<Vec<u8>>,
        options: ImageWatermarkOptions,
    ) -> Result<Self, ImageWatermarkError> {
        if width == 0 || height == 0 {
            return Err(ImageWatermarkError::InvalidDimensions);
        }
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or(ImageWatermarkError::InvalidDimensions)?;
        let pixels = pixels.into();
        if pixels.len() != expected {
            return Err(ImageWatermarkError::InvalidBufferLength);
        }
        options.validate()?;
        Ok(Self {
            image: Handle::from_rgba(width, height, pixels),
            width,
            height,
            options,
            prepared: RefCell::new(None),
        })
    }
    pub fn with_options(
        &self,
        options: ImageWatermarkOptions,
    ) -> Result<Self, ImageWatermarkError> {
        options.validate()?;
        Ok(Self {
            options,
            prepared: RefCell::new(None),
            ..self.clone()
        })
    }
}
impl<B: super::ChartBehavior, M: Clone + 'static> DrawingPrimitive<B, M> for ImageWatermark {
    fn prepare(&self, context: &PrimitiveContext<'_, B, M>) -> PreparedPrimitive {
        if self.options.alpha == 0. {
            return PreparedPrimitive::default();
        }
        let size = context.size;
        let Some([x, y, width, height]) = placement(
            &self.options,
            [f64::from(size.width), f64::from(size.height)],
            [f64::from(self.width), f64::from(self.height)],
        ) else {
            return PreparedPrimitive::default();
        };
        let mut cached = self.prepared.borrow_mut();
        if cached
            .as_ref()
            .is_none_or(|(previous, _)| *previous != size)
        {
            *cached = Some((
                size,
                Rc::new(ImageWatermarkRenderer {
                    image: iced::advanced::image::Image::new(self.image.clone())
                        .opacity(self.options.alpha),
                    bounds: iced::Rectangle::new(
                        iced::Point::new(x as f32, y as f32),
                        iced::Size::new(width as f32, height as f32),
                    ),
                }),
            ));
        }
        PreparedPrimitive {
            views: vec![PrimitiveView {
                layer: PrimitiveLayer::Bottom,
                renderer: cached.as_ref().unwrap().1.clone(),
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
    fn options_share_pixels_and_unchanged_preparation_reuses_snapshot() {
        let image =
            ImageWatermark::from_rgba(1, 1, vec![255, 0, 0, 255], Default::default()).unwrap();
        let next = image
            .with_options(ImageWatermarkOptions {
                alpha: 0.5,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(image.image.id(), next.image.id());
        if let (Handle::Rgba { pixels: a, .. }, Handle::Rgba { pixels: b, .. }) =
            (&image.image, &next.image)
        {
            assert_eq!(a.as_ptr(), b.as_ptr());
        } else {
            panic!("RGBA expected");
        }
        let chart = Chart::default();
        let mut context = PrimitiveContext {
            owner: PrimitiveOwner::Pane(super::super::primitives::pane_id()),
            pane: 0,
            size: iced::Size::new(800., 500.),
            pixel_ratio: 1.,
            model: chart.model(),
            visible_range: None,
        };
        let first = image.prepare(&context);
        let weak = Rc::downgrade(&first.views[0].renderer);
        for _ in 0..100 {
            let next = image.prepare(&context);
            assert!(Rc::ptr_eq(
                &first.views[0].renderer,
                &next.views[0].renderer
            ));
        }
        context.pixel_ratio = 2.;
        assert!(Rc::ptr_eq(
            &first.views[0].renderer,
            &image.prepare(&context).views[0].renderer
        ));
        context.size = iced::Size::new(80., 40.);
        assert!(!Rc::ptr_eq(
            &first.views[0].renderer,
            &image.prepare(&context).views[0].renderer
        ));
        drop(image);
        assert!(weak.upgrade().is_some());
        drop(first);
        assert!(weak.upgrade().is_none());
    }
}
