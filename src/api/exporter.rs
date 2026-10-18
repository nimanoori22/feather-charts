//! Reusable offscreen renderer. Encoding and file ownership stay with the host.
use super::{ChartScreenshot, ScreenshotError};
use iced::{
    Color, Point, Rectangle, Size, Transformation,
    advanced::{
        graphics::geometry::Renderer as _,
        renderer::{Headless, Renderer as _},
    },
    widget::canvas::Frame,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenshotBackend {
    Software,
    Wgpu,
}
impl ScreenshotBackend {
    fn name(self) -> &'static str {
        match self {
            Self::Software => "tiny-skia",
            Self::Wgpu => "wgpu",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ScreenshotRasterOptions {
    pub pixel_ratio: f32,
    /// Physical output canvas. Uniform fit, centered padding, no chart reflow.
    pub output_size: Option<Size<u32>>,
    pub background: Color,
}
impl Default for ScreenshotRasterOptions {
    fn default() -> Self {
        Self {
            pixel_ratio: 1.,
            output_size: None,
            background: Color::TRANSPARENT,
        }
    }
}
impl ScreenshotRasterOptions {
    /// Bounds allocation/readback to 64 MiB and Iced's 8192px texture dimension.
    fn dimensions(self, logical: Size) -> Result<Size<u32>, ScreenshotError> {
        let invalid = || ScreenshotError::InvalidRasterOptions;
        if !self.pixel_ratio.is_finite()
            || self.pixel_ratio <= 0.
            || !logical.width.is_finite()
            || !logical.height.is_finite()
            || logical.width <= 0.
            || logical.height <= 0.
            || [
                self.background.r,
                self.background.g,
                self.background.b,
                self.background.a,
            ]
            .iter()
            .any(|c| !c.is_finite() || !(0. ..=1.).contains(c))
        {
            return Err(invalid());
        }
        let size = if let Some(size) = self.output_size {
            size
        } else {
            let w = (f64::from(logical.width) * f64::from(self.pixel_ratio)).ceil();
            let h = (f64::from(logical.height) * f64::from(self.pixel_ratio)).ceil();
            if w > 8192. || h > 8192. {
                return Err(invalid());
            }
            Size::new(w as u32, h as u32)
        };
        let bytes = u64::from(size.width)
            .checked_mul(u64::from(size.height))
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(invalid)?;
        if size.width == 0
            || size.height == 0
            || size.width > 8192
            || size.height > 8192
            || bytes > 64 * 1024 * 1024
        {
            return Err(invalid());
        }
        Ok(size)
    }
}
/// Top-to-bottom, tightly packed straight-alpha sRGB RGBA8; no row padding.
#[derive(Clone, Debug)]
pub struct ScreenshotImage {
    size: Size<u32>,
    rgba: Vec<u8>,
}
impl ScreenshotImage {
    pub fn size(&self) -> Size<u32> {
        self.size
    }
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }
    pub fn into_rgba(self) -> Vec<u8> {
        self.rgba
    }
    pub fn into_handle(self) -> iced::widget::image::Handle {
        iced::widget::image::Handle::from_rgba(self.size.width, self.size.height, self.rgba)
    }
}

pub struct ChartExporter {
    renderer: iced::Renderer,
    backend: ScreenshotBackend,
}
impl ChartExporter {
    /// Initialize once outside rendering/input hot paths. Explicit backend; no fallback.
    /// Fonts must be loaded by the host before preparing chart glyphs.
    pub async fn new(backend: ScreenshotBackend) -> Result<Self, ScreenshotError> {
        let renderer = <iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            iced::Pixels(12.),
            Some(backend.name()),
        )
        .await
        .ok_or(ScreenshotError::BackendUnavailable)?;
        if renderer.name() != backend.name() {
            return Err(ScreenshotError::BackendUnavailable);
        }
        Ok(Self { renderer, backend })
    }
    pub fn backend(&self) -> ScreenshotBackend {
        self.backend
    }
    /// Synchronous readback. Iced device-loss/map/allocation failures can panic;
    /// its Headless API offers no recoverable readback error channel.
    pub fn render(
        &mut self,
        screenshot: &ChartScreenshot,
        options: ScreenshotRasterOptions,
    ) -> Result<ScreenshotImage, ScreenshotError> {
        let logical = screenshot.logical_size();
        let size = options.dimensions(logical)?;
        let canvas = Size::new(size.width as f32, size.height as f32);
        let (scale, offset) = if options.output_size.is_some() {
            let scale = (canvas.width / logical.width).min(canvas.height / logical.height);
            (
                scale,
                (canvas - Size::new(logical.width * scale, logical.height * scale)) * 0.5,
            )
        } else {
            // Rounding the physical buffer must not recenter the logical layout.
            (options.pixel_ratio, Size::ZERO)
        };
        self.renderer.reset(Rectangle::with_size(canvas));
        let mut backdrop = Frame::new(&self.renderer, canvas);
        backdrop.fill_rectangle(Point::ORIGIN, canvas, options.background);
        self.renderer.draw_geometry(backdrop.into_geometry());
        let mut frame = Frame::new(&self.renderer, logical);
        screenshot.draw(&mut frame);
        self.renderer.with_transformation(
            Transformation::translate(offset.width, offset.height) * Transformation::scale(scale),
            |renderer| renderer.draw_geometry(frame.into_geometry()),
        );
        let mut rgba = self.renderer.screenshot(size, 1., Color::TRANSPARENT);
        if rgba.len() != size.width as usize * size.height as usize * 4 {
            return Err(ScreenshotError::InvalidReadback);
        }
        for pixel in rgba.as_chunks_mut::<4>().0 {
            let alpha = f32::from(pixel[3]) / 255.;
            if alpha == 0. {
                pixel.fill(0);
            } else if alpha < 1. {
                for channel in &mut pixel[..3] {
                    let mut value = f32::from(*channel) / 255.;
                    let linear = self.backend == ScreenshotBackend::Wgpu
                        && iced::advanced::graphics::color::GAMMA_CORRECTION;
                    if linear {
                        value = if value <= 0.04045 {
                            value / 12.92
                        } else {
                            ((value + 0.055) / 1.055).powf(2.4)
                        };
                    }
                    value = (value / alpha).min(1.);
                    if linear {
                        value = if value <= 0.0031308 {
                            value * 12.92
                        } else {
                            1.055 * value.powf(1. / 2.4) - 0.055
                        };
                    }
                    *channel = (value * 255.).round() as u8;
                }
            }
        }
        Ok(ScreenshotImage { size, rgba })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raster_limits_and_explicit_canvas_precedence() {
        let logical = Size::new(300., 90.);
        assert_eq!(
            ScreenshotRasterOptions {
                pixel_ratio: 1.25,
                ..Default::default()
            }
            .dimensions(logical)
            .unwrap(),
            Size::new(375, 113)
        );
        for ratio in [0., -1., f32::NAN, f32::INFINITY, f32::MAX] {
            assert!(
                ScreenshotRasterOptions {
                    pixel_ratio: ratio,
                    ..Default::default()
                }
                .dimensions(logical)
                .is_err()
            );
        }
        for size in [
            Size::new(0, 20),
            Size::new(8193, 1),
            Size::new(8192, 8192),
            Size::new(u32::MAX, u32::MAX),
        ] {
            assert!(
                ScreenshotRasterOptions {
                    output_size: Some(size),
                    ..Default::default()
                }
                .dimensions(logical)
                .is_err()
            );
        }
        assert_eq!(
            ScreenshotRasterOptions {
                pixel_ratio: 2.,
                output_size: Some(Size::new(100, 100)),
                ..Default::default()
            }
            .dimensions(logical)
            .unwrap(),
            Size::new(100, 100)
        );
    }
}
