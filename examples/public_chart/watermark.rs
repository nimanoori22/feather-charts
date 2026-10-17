//! Host-owned options, existing primitive IDs and native capture verification.
use super::*;
use feather_charts::api::{PrimitiveId, TextWatermark, TextWatermarkLine, TextWatermarkOptions};
use iced::{
    advanced::{
        graphics::geometry::Renderer as _,
        renderer::{Headless, Renderer as _},
    },
    alignment::{Horizontal, Vertical},
};
fn options() -> TextWatermarkOptions {
    TextWatermarkOptions {
        lines: vec![
            TextWatermarkLine {
                text: "BTCUSD".into(),
                color: "rgba(171,71,188,0.65)".into(),
                ..Default::default()
            },
            TextWatermarkLine {
                text: "1h · Feather Charts".into(),
                font: iced::Font::MONOSPACE,
                font_size: 20.,
                color: "rgba(50,80,160,0.65)".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}
pub(super) struct Scene {
    id: PrimitiveId,
    options: TextWatermarkOptions,
    alignment: usize,
}
impl Scene {
    pub(super) fn new(chart: &mut Chart) -> Self {
        let options = options();
        let id = chart
            .add_pane_primitive(0, TextWatermark::new(options.clone()).unwrap())
            .unwrap();
        Self {
            id,
            options,
            alignment: 0,
        }
    }
    pub(super) fn change(&mut self, chart: &mut Chart, message: &Message) {
        match message {
            Message::ToggleWatermark => self.options.visible = !self.options.visible,
            Message::WatermarkAlignment => {
                self.alignment = (self.alignment + 1) % 9;
                self.options.horizontal_alignment =
                    [Horizontal::Center, Horizontal::Left, Horizontal::Right][self.alignment % 3];
                self.options.vertical_alignment =
                    [Vertical::Center, Vertical::Top, Vertical::Bottom][self.alignment / 3];
            }
            Message::WatermarkText => {
                self.options.lines[0].text = if self.options.lines[0].text == "BTCUSD" {
                    "ETHUSD"
                } else {
                    "BTCUSD"
                }
                .into();
            }
            Message::WatermarkStyle => {
                let l = &mut self.options.lines[0];
                l.font = iced::Font {
                    weight: if l.font.weight == iced::font::Weight::Bold {
                        iced::font::Weight::Normal
                    } else {
                        iced::font::Weight::Bold
                    },
                    style: iced::font::Style::Italic,
                    ..l.font
                };
            }
            _ => unreachable!(),
        }
        chart
            .replace_primitive(self.id, TextWatermark::new(self.options.clone()).unwrap())
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
            for scene in ["default", "multiline", "edges", "long", "mixed", "empty"] {
                let mut app = App {
                    size,
                    ratio,
                    ..App::new(SeriesType::Line)
                };
                if scene == "mixed" {
                    app.primitive_scene =
                        Some(primitive_demo::Scene::new(&mut app.chart, app.price));
                    hover_demo::add_overlays(&mut app.chart);
                    app.chart
                        .add_series_markers(
                            app.price,
                            sample_markers(false),
                            SeriesMarkersOptions::default(),
                        )
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
                    if scene == "edges" {
                        options.horizontal_alignment = Horizontal::Right;
                        options.vertical_alignment = Vertical::Bottom;
                        options.lines[0].line_height = Some(39.);
                    }
                    if scene == "long" {
                        options.lines[0].text = "LONG WATERMARK TO FIT INSIDE A NARROW PANE".into();
                        options.lines[0].font_size = 96.;
                    }
                    app.chart
                        .add_pane_primitive(0, TextWatermark::new(options).unwrap())
                        .unwrap();
                    app.chart.prepare(size, ratio, Instant::now()).unwrap();
                }
                let rgba = draw(&app.chart);
                assert!(rgba == draw(&app.chart), "retained frame changed pixels");
                if scene != "default" && app.chart.frame().plot.size.height > 1. {
                    assert!(
                        rgba != baseline,
                        "watermark drew nothing: {scene} {width} {height} {ratio}"
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
    fn demo_controls_preserve_pane_ownership_and_narrow_bounds() {
        let mut chart = Chart::default();
        let mut scene = Scene::new(&mut chart);
        let owner = chart.primitive_owner(scene.id).unwrap();
        for message in [
            Message::WatermarkText,
            Message::WatermarkStyle,
            Message::WatermarkAlignment,
            Message::ToggleWatermark,
            Message::ToggleWatermark,
        ] {
            scene.change(&mut chart, &message);
            chart
                .prepare(Size::new(300., 90.), 1.5, Instant::now())
                .unwrap();
            assert_eq!(chart.primitive_owner(scene.id).unwrap(), owner);
        }
    }
}
