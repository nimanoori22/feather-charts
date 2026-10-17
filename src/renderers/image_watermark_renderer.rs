//! Source placement in logical coordinates; images remain native shared resources.
use crate::api::{ImageWatermarkOptions, PrimitiveRenderer};
use iced::{Rectangle, advanced::image::Image, widget::canvas::Frame};
#[derive(Debug)]
pub(crate) struct ImageWatermarkRenderer {
    pub image: Image,
    pub bounds: Rectangle,
}
impl PrimitiveRenderer for ImageWatermarkRenderer {
    fn draw(&self, frame: &mut Frame) {
        frame.draw_image(self.bounds, self.image.clone());
    }
}
pub(crate) fn placement(
    options: &ImageWatermarkOptions,
    pane: [f64; 2],
    image: [f64; 2],
) -> Option<[f64; 4]> {
    if pane
        .iter()
        .chain(&image)
        .any(|v| !v.is_finite() || *v <= 0.)
    {
        return None;
    }
    let mut available = [
        pane[0] - 2. * f64::from(options.padding),
        pane[1] - 2. * f64::from(options.padding),
    ];
    for (value, maximum) in available
        .iter_mut()
        .zip([options.max_width, options.max_height])
    {
        if let Some(limit) = maximum.filter(|v| *v != 0.) {
            *value = value.min(f64::from(limit));
        }
    }
    if available.iter().any(|v| *v <= 0.) {
        return None;
    }
    let scale = (available[0] / image[0]).min(available[1] / image[1]);
    let width = image[0] * scale;
    let height = image[1] * scale;
    Some([
        (pane[0] / 2.).round() - width / 2.,
        (pane[1] / 2.).round() - height / 2.,
        width,
        height,
    ])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_source_placement_fixtures() {
        let mut count = 0;
        for pane in [
            [800., 500.],
            [301., 301.],
            [300., 90.],
            [1., 1.],
            [941., 900.75],
        ] {
            for image in [[1., 1.], [292., 128.], [20., 100.], [2048., 512.]] {
                for (padding, max_width, max_height) in [
                    (0., None, None),
                    (20., None, None),
                    (0., Some(0.), Some(0.)),
                    (1., Some(120.), None),
                    (0., None, Some(50.)),
                    (0., Some(50.), Some(60.)),
                ] {
                    let options = ImageWatermarkOptions {
                        padding,
                        max_width,
                        max_height,
                        ..Default::default()
                    };
                    let actual = placement(&options, pane, image);
                    // Invalid/empty drawable areas are a documented native policy, not source negative-scale parity.
                    if let Some(a) = actual {
                        println!(
                            "IMAGE_LAYOUT {{\"pane\":{pane:?},\"image\":{image:?},\"padding\":{padding},\"max_width\":{},\"max_height\":{},\"actual\":{a:?}}}",
                            max_width.map_or("null".into(), |v| v.to_string()),
                            max_height.map_or("null".into(), |v| v.to_string())
                        );
                        count += 1;
                    }
                }
            }
        }
        assert_eq!(count, 112);
        assert_eq!(
            placement(&ImageWatermarkOptions::default(), [0., 20.], [1., 1.]),
            None
        );
        assert_eq!(
            placement(
                &ImageWatermarkOptions {
                    padding: 50.,
                    ..Default::default()
                },
                [10., 10.],
                [1., 1.]
            ),
            None
        );
    }
}
