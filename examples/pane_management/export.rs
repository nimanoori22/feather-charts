//! Example-only preview/file handling through the public screenshot API.
use super::*;
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Capture,
    Top,
    Crosshair,
    Ratio,
    Canvas,
    Save,
    Hide,
}
pub struct Panel {
    exporter: ChartExporter,
    layers: ScreenshotLayers,
    options: ScreenshotRasterOptions,
    preview: Option<iced::widget::image::Handle>,
    image: Option<ScreenshotImage>,
    pub status: String,
}
impl Panel {
    pub fn new() -> Self {
        let backend = if std::env::var("ICED_BACKEND").as_deref() == Ok("tiny-skia") {
            ScreenshotBackend::Software
        } else {
            ScreenshotBackend::Wgpu
        };
        Self::with_exporter(
            iced::futures::executor::block_on(ChartExporter::new(backend))
                .expect("explicit export backend"),
        )
    }
    fn with_exporter(exporter: ChartExporter) -> Self {
        let backend = exporter.backend();
        Self {
            exporter,
            layers: Default::default(),
            options: Default::default(),
            preview: None,
            image: None,
            status: format!("Export backend: {backend:?}"),
        }
    }
}
impl<C: Host> App<C> {
    pub fn export_action(&mut self, action: Action) -> Result<(), ChartError> {
        let Some(panel) = &mut self.export else {
            return Ok(());
        };
        match action {
            Action::Top => panel.layers.include_top_layer = !panel.layers.include_top_layer,
            Action::Crosshair => panel.layers.include_crosshair = !panel.layers.include_crosshair,
            Action::Ratio => {
                panel.options.pixel_ratio = if panel.options.pixel_ratio == 1. {
                    2.
                } else {
                    1.
                }
            }
            Action::Canvas => {
                panel.options.output_size = if panel.options.output_size.is_none() {
                    Some(Size::new(640, 480))
                } else {
                    None
                }
            }
            Action::Hide => {
                panel.preview = None;
                return Ok(());
            }
            Action::Save => {
                if let Some(image) = &panel.image {
                    let result = save(std::path::Path::new("target/screenshot-export"), image);
                    panel.status = match result {
                        Ok(()) => "Saved target/screenshot-export.{rgba,ppm,json}".into(),
                        Err(e) => e.to_string(),
                    };
                } else {
                    panel.status = "Capture first".into();
                }
                return Ok(());
            }
            Action::Capture => {}
        }
        let screenshot = self.chart.fresh_screenshot(panel.layers, Instant::now())?;
        let image = panel
            .exporter
            .render(&screenshot, panel.options)
            .map_err(ChartError::Screenshot)?;
        panel.status = format!(
            "{:?} | top={} crosshair={} ratio={} canvas={:?} | {}×{} RGBA",
            panel.exporter.backend(),
            panel.layers.include_top_layer,
            panel.layers.include_crosshair,
            panel.options.pixel_ratio,
            panel.options.output_size,
            image.size().width,
            image.size().height
        );
        panel.preview = Some(image.clone().into_handle());
        panel.image = Some(image);
        Ok(())
    }
    pub fn export_controls(&self) -> Element<'_, Message> {
        let Some(panel) = &self.export else {
            return row![].into();
        };
        let mut controls = row![];
        for (label, action) in [
            ("Capture (F8)", Action::Capture),
            ("Top layer", Action::Top),
            ("Crosshair", Action::Crosshair),
            ("Export ratio", Action::Ratio),
            ("640×480 / original", Action::Canvas),
            ("Save RGBA + PPM", Action::Save),
            ("Hide preview", Action::Hide),
        ] {
            controls = controls.push(button(label).on_press(Message::Export(action)));
        }
        let mut content = iced::widget::column![
            iced::widget::scrollable(controls.spacing(6)).direction(
                iced::widget::scrollable::Direction::Horizontal(Default::default())
            ),
            text(&panel.status).size(12).wrapping(text::Wrapping::None)
        ];
        if let Some(handle) = &panel.preview {
            content = content.push(iced::widget::image(handle.clone()).height(100));
        }
        content.into()
    }
}
fn with_suffix(path: &std::path::Path, suffix: &str) -> std::path::PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    name.into()
}
pub fn save(path: &std::path::Path, image: &ScreenshotImage) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(with_suffix(path, ".rgba"), image.rgba())?;
    std::fs::write(
        with_suffix(path, ".json"),
        format!(
            "{{\"width\":{},\"height\":{},\"format\":\"straight-alpha sRGB RGBA8\"}}\n",
            image.size().width,
            image.size().height
        ),
    )?;
    // PPM has no alpha. Preview transparent pixels over white for this host format.
    let mut ppm = format!("P6\n{} {}\n255\n", image.size().width, image.size().height).into_bytes();
    for pixel in image.rgba().as_chunks::<4>().0 {
        let a = u32::from(pixel[3]);
        for c in &pixel[..3] {
            ppm.push(((u32::from(*c) * a + 255 * (255 - a) + 127) / 255) as u8);
        }
    }
    std::fs::write(with_suffix(path, ".ppm"), ppm)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn check<C: Host>(name: &str) {
        let mut app = App::<C>::with_public_events(false, false, false, false, true);
        app.size = Size::new(300., 200.);
        app.chart.prepare(app.size, 1., Instant::now()).unwrap();
        app.export = Some(Panel::with_exporter(
            iced::futures::executor::block_on(ChartExporter::new(ScreenshotBackend::Software))
                .unwrap(),
        ));
        app.export_action(Action::Capture).unwrap();
        assert!(app.export.as_ref().unwrap().preview.is_some());
        app.export_action(Action::Ratio).unwrap();
        assert_eq!(
            app.export.as_ref().unwrap().image.as_ref().unwrap().size(),
            Size::new(600, 400)
        );
        app.export_action(Action::Canvas).unwrap();
        let panel = app.export.as_ref().unwrap();
        assert_eq!(panel.image.as_ref().unwrap().size(), Size::new(640, 480));
        let path = std::path::Path::new("target/screenshot-export-host-tests")
            .join(format!("{name}-1.25-toptrue"));
        save(&path, panel.image.as_ref().unwrap()).unwrap();
        assert_eq!(
            std::fs::read(with_suffix(&path, ".rgba")).unwrap(),
            panel.image.as_ref().unwrap().rgba()
        );
        assert!(
            std::fs::read(with_suffix(&path, ".ppm"))
                .unwrap()
                .starts_with(b"P6\n640 480\n255\n")
        );
        app.export_action(Action::Hide).unwrap();
        assert!(app.export.as_ref().unwrap().preview.is_none());
        app.action(Message::Reset).unwrap();
        assert!(app.export.is_some());
    }
    #[test]
    fn preview_and_file_export_use_public_api_in_all_modes() {
        check::<Chart>("calendar");
        check::<OptionsChart>("strike");
        check::<YieldCurveChart>("maturity");
    }
}
