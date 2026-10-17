//! Decoded fixture and host-owned image options; no file or network loading.
use super::*;
use feather_charts::api::{ImageWatermark, ImageWatermarkOptions, PrimitiveId};
use iced::advanced::{
    graphics::geometry::Renderer as _,
    renderer::{Headless, Renderer as _},
};
fn options() -> ImageWatermarkOptions {
    ImageWatermarkOptions {
        alpha: 0.35,
        padding: 20.,
        max_width: Some(400.),
        max_height: Some(200.),
    }
}
fn fixture(alternate: bool, options: ImageWatermarkOptions) -> ImageWatermark {
    let (width, height) = if alternate { (16, 32) } else { (32, 16) };
    let mut pixels = vec![];
    for y in 0..height {
        for x in 0..width {
            let inside = x > 1 && y > 1 && x < width - 2 && y < height - 2;
            let stripe = (x + y) % 8 < 4;
            pixels.extend(if alternate {
                [160, 50, 140, if inside { 180 } else { 0 }]
            } else {
                [
                    30,
                    if stripe { 110 } else { 180 },
                    220,
                    if inside { 220 } else { 0 },
                ]
            });
        }
    }
    ImageWatermark::from_rgba(width, height, pixels, options).unwrap()
}
pub(super) struct Scene {
    id: PrimitiveId,
    image: ImageWatermark,
    options: ImageWatermarkOptions,
    alternate: bool,
}
impl Scene {
    pub(super) fn new(chart: &mut Chart) -> Self {
        let options = options();
        let image = fixture(false, options.clone());
        let id = chart.add_pane_primitive(0, image.clone()).unwrap();
        Self {
            id,
            image,
            options,
            alternate: false,
        }
    }
    pub(super) fn change(&mut self, chart: &mut Chart, message: &Message) {
        match message {
            Message::ImageOpacity => {
                self.options.alpha = if self.options.alpha == 0. {
                    0.35
                } else if self.options.alpha < 0.5 {
                    0.8
                } else {
                    0.
                }
            }
            Message::ImagePadding => {
                self.options.padding = if self.options.padding == 20. {
                    60.
                } else {
                    20.
                }
            }
            Message::ImageLimits => {
                let limited = self.options.max_width.is_some();
                self.options.max_width = (!limited).then_some(400.);
                self.options.max_height = (!limited).then_some(200.);
            }
            Message::ImageReplace => {
                self.alternate = !self.alternate;
                self.image = fixture(self.alternate, self.options.clone());
            }
            _ => unreachable!(),
        }
        self.image = self.image.with_options(self.options.clone()).unwrap();
        chart
            .replace_primitive(self.id, self.image.clone())
            .unwrap();
    }
}
pub(super) fn capture(directory: &std::path::Path, wgpu: bool) {
    let backend = if wgpu { "wgpu" } else { "tiny-skia" };
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some(backend),
    ))
    .unwrap();
    assert!(
        matches!(
            (&renderer, wgpu),
            (iced::Renderer::Primary(_), true) | (iced::Renderer::Secondary(_), false)
        ),
        "backend fallback"
    );
    std::fs::create_dir_all(directory).unwrap();
    for (width, height) in [(800., 500.), (300., 300.), (300., 90.)] {
        for ratio in [1., 1.25, 1.5, 2.] {
            let size = Size::new(width, height);
            for scene in ["default", "logo", "limits", "mixed", "empty", "multiple"] {
                let mut app = App {
                    size,
                    ratio,
                    ..App::new(SeriesType::Line)
                };
                if scene == "mixed" {
                    app.primitive_scene =
                        Some(primitive_demo::Scene::new(&mut app.chart, app.price));
                    hover_demo::add_overlays(&mut app.chart);
                    app.watermark = Some(watermark_demo::Scene::new(&mut app.chart));
                }
                if scene == "limits" {
                    let mut hidden =
                        feather_charts::model::series_options::LineSeriesOptions::default();
                    hidden.common.visible = false;
                    app.chart
                        .replace_series_options(app.price, SeriesOptionsMap::Line(hidden))
                        .unwrap();
                }
                if scene == "empty" {
                    app.chart.set_preserve_empty_pane(0, true).unwrap();
                    app.chart.remove_series(app.price).unwrap();
                }
                app.chart.prepare(size, ratio, Instant::now()).unwrap();
                let mut draw = |chart: &Chart| {
                    renderer.reset(iced::Rectangle::with_size(size));
                    let mut canvas = iced::widget::canvas::Frame::new(&renderer, size);
                    chart.multi_frame().unwrap().draw(&mut canvas);
                    renderer.draw_geometry(canvas.into_geometry());
                    renderer.screenshot(
                        Size::new((width * ratio) as u32, (height * ratio) as u32),
                        ratio,
                        iced::Color::BLACK,
                    )
                };
                let baseline = draw(&app.chart);
                if scene != "default" {
                    let mut options = options();
                    if scene == "limits" {
                        options.max_width = Some(70.);
                        options.max_height = Some(90.);
                        options.padding = 3.;
                    }
                    app.chart
                        .add_pane_primitive(0, fixture(scene == "limits", options))
                        .unwrap();
                    if scene == "multiple" {
                        app.chart
                            .add_pane_primitive(
                                0,
                                fixture(
                                    true,
                                    ImageWatermarkOptions {
                                        alpha: 0.5,
                                        max_height: Some(100.),
                                        ..Default::default()
                                    },
                                ),
                            )
                            .unwrap();
                    }
                    app.chart.prepare(size, ratio, Instant::now()).unwrap();
                }
                let rgba = draw(&app.chart);
                assert!(
                    rgba == draw(&app.chart),
                    "retained image frame changed pixels"
                );
                // A multi-pane layout can leave no padded drawable area at short heights.
                let pane_height = app.chart.multi_frame().unwrap().panes[0].plot.size.height;
                let padding = if scene == "limits" { 3. } else { 20. };
                if scene != "default" && pane_height > padding * 2. + 4. {
                    assert!(
                        rgba != baseline,
                        "image drew nothing: {backend} {scene} {width} {height} {ratio}"
                    );
                }
                std::fs::write(
                    directory.join(format!("{scene}-{width}x{height}-{ratio}.rgba")),
                    rgba,
                )
                .unwrap();
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn controls_preserve_owner_through_short_height_recovery() {
        let mut chart = Chart::default();
        let mut scene = Scene::new(&mut chart);
        let owner = chart.primitive_owner(scene.id).unwrap();
        for message in [
            Message::ImageOpacity,
            Message::ImagePadding,
            Message::ImageLimits,
            Message::ImageReplace,
            Message::ImageOpacity,
            Message::ImageOpacity,
        ] {
            scene.change(&mut chart, &message);
            for size in [Size::new(300., 90.), Size::new(800., 500.)] {
                chart.prepare(size, 1.25, Instant::now()).unwrap();
                assert_eq!(chart.primitive_owner(scene.id).unwrap(), owner);
            }
        }
    }
}
