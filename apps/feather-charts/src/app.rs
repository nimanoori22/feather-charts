use crate::{chart::DocumentChart, load};
use feather_chart_file::ChartDocument;
use feather_charts::api::ChartMessage;
use iced::{
    Element, Size, Subscription, Task,
    widget::{column, container, stack, text, text_input},
};
use std::{path::PathBuf, time::Instant};
pub struct App {
    current: Option<DocumentChart>,
    path: String,
    loaded_path: Option<PathBuf>,
    request: u64,
    status: Option<String>,
    show_open: bool,
    size: Size,
    ratio: f32,
}
#[derive(Clone, Debug)]
pub enum Message {
    Focus(bool),
    FocusPath,
    Dismiss,
    Path(String),
    Open,
    Reload,
    Fit,
    Loaded(u64, PathBuf, Result<ChartDocument, String>),
    Chart(ChartMessage),
    Window(iced::window::Id, iced::window::Event),
    Ratio(f32),
}
impl App {
    pub fn new(path: Option<PathBuf>) -> (Self, Task<Message>) {
        let mut app = Self {
            current: None,
            path: String::new(),
            loaded_path: None,
            request: 0,
            status: None,
            show_open: false,
            size: Size::new(900., 500.),
            ratio: 1.,
        };
        let task = path.map_or_else(
            || iced::widget::operation::focus("document-path"),
            |p| app.open(p),
        );
        (app, task)
    }
    fn open(&mut self, path: PathBuf) -> Task<Message> {
        self.request += 1;
        let request = self.request;
        self.path = path.to_string_lossy().into_owned();
        self.status = Some(format!("Loading {}…", path.display()));
        self.show_open = false;
        // Iced's native executor keeps file I/O and parsing off the UI thread.
        Task::perform(
            async move {
                let result = load::read(&path);
                (request, path, result)
            },
            |(id, path, result)| Message::Loaded(id, path, result),
        )
    }
    fn loaded(&mut self, id: u64, path: PathBuf, result: Result<ChartDocument, String>) {
        if id != self.request {
            return;
        }
        let candidate = result
            .and_then(DocumentChart::new)
            .and_then(|mut candidate| {
                candidate
                    .chart
                    .prepare(self.size, self.ratio, Instant::now())
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                Ok(candidate)
            });
        match candidate {
            Ok(candidate) => {
                self.loaded_path = Some(path);
                self.current = Some(candidate);
                self.status = None;
                self.show_open = false;
            }
            Err(e) => {
                self.status = Some(e);
                self.show_open = true;
            }
        }
    }
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FocusPath => {
                self.show_open = true;
                return iced::widget::operation::focus("document-path");
            }
            Message::Dismiss => {
                self.show_open = false;
                self.status = None;
            }
            Message::Focus(back) => {
                return if back {
                    iced::widget::operation::focus_previous()
                } else {
                    iced::widget::operation::focus_next()
                };
            }
            Message::Path(path) => self.path = path,
            Message::Open => return self.open(PathBuf::from(&self.path)),
            Message::Reload => {
                if let Some(path) = self.loaded_path.clone() {
                    return self.open(path);
                }
            }
            Message::Fit => {
                if let Some(c) = &mut self.current {
                    c.chart.fit_content();
                }
            }
            Message::Loaded(id, path, result) => self.loaded(id, path, result),
            Message::Chart(message) => {
                if let ChartMessage::Bounds(size) = message {
                    self.size = size;
                }
                if let Some(c) = &mut self.current
                    && let Err(e) = c.chart.update(message)
                {
                    self.status = Some(e.to_string());
                }
            }
            Message::Window(id, iced::window::Event::Opened { .. }) => {
                return iced::window::scale_factor(id).map(Message::Ratio);
            }
            Message::Window(_, iced::window::Event::FileDropped(path)) => return self.open(path),
            Message::Window(_, iced::window::Event::Rescaled(r)) | Message::Ratio(r) => {
                self.ratio = r
            }
            Message::Window(_, _) => {}
        }
        if let Some(c) = &mut self.current
            && let Err(e) = c.chart.prepare(self.size, self.ratio, Instant::now())
        {
            self.status = Some(e.to_string());
        }
        Task::none()
    }
    pub fn view(&self) -> Element<'_, Message> {
        let base: Element<'_, Message> = match &self.current {
            Some(c) => c.chart.element_with_readout().map(Message::Chart),
            None => container(text("Feather Charts"))
                .center_x(iced::Fill)
                .center_y(iced::Fill)
                .into(),
        };
        let mut layers = stack![base];
        if self.current.as_ref().is_some_and(DocumentChart::empty) {
            layers = layers.push(
                container(text("This document contains no points."))
                    .center_x(iced::Fill)
                    .center_y(iced::Fill),
            );
        }
        if self.show_open || self.current.is_none() {
            let mut panel = column![
                text("Open .fchart"),
                text_input("/path/to/results.fchart", &self.path)
                    .id("document-path")
                    .on_input(Message::Path)
                    .on_submit(Message::Open),
                text("Enter: open • Esc: close").size(12)
            ]
            .spacing(8);
            if let Some(status) = &self.status {
                panel = panel.push(text(status).size(13));
            }
            layers = layers.push(
                container(
                    container(panel)
                        .padding(16)
                        .max_width(480)
                        .style(container::rounded_box),
                )
                .padding(16)
                .center_x(iced::Fill)
                .center_y(iced::Fill),
            );
        } else if let Some(status) = &self.status {
            layers = layers.push(
                container(
                    container(text(status).size(13))
                        .padding(8)
                        .style(container::rounded_box),
                )
                .padding(8)
                .align_bottom(iced::Fill),
            );
        }
        layers.into()
    }
    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::window::events().map(|(id, e)| Message::Window(id, e)),
            iced::event::listen_with(|event, _, _| {
                if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    ..
                }) = event
                {
                    shortcut(key, modifiers)
                } else {
                    None
                }
            }),
        ])
    }
}
fn shortcut(key: iced::keyboard::Key, modifiers: iced::keyboard::Modifiers) -> Option<Message> {
    use iced::keyboard::{Key, key::Named};
    match key {
        Key::Named(Named::Escape) => Some(Message::Dismiss),
        Key::Named(Named::Tab) => Some(Message::Focus(modifiers.shift())),
        Key::Character(key) if modifiers.command() => match key.to_lowercase().as_str() {
            "o" => Some(Message::FocusPath),
            "r" => Some(Message::Reload),
            "0" => Some(Message::Fit),
            _ => None,
        },
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn doc() -> ChartDocument {
        feather_chart_file::read_document(include_bytes!("../fixtures/minimal.fchart").as_slice())
            .unwrap()
    }
    #[test]
    fn ordinary_view_pixels_are_exactly_the_chart_with_its_readout() {
        use iced::{
            Color, Rectangle, Theme,
            advanced::{
                Layout, layout,
                renderer::{Headless, Renderer as _},
                widget::Tree,
            },
        };
        let (mut app, _) = App::new(None);
        app.size = Size::new(800., 500.);
        app.loaded(0, "valid.fchart".into(), Ok(doc()));
        let size = app.size;
        let bounds = Rectangle::with_size(size);
        let backend =
            std::env::var("FEATHER_READOUT_BACKEND").unwrap_or_else(|_| "tiny-skia".into());
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            iced::Pixels(12.),
            Some(&backend),
        ))
        .unwrap();
        assert_eq!(renderer.name(), backend, "renderer fallback");
        let mut render = |mut element: Element<'_, Message>| {
            renderer.reset(bounds);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(size, size),
            );
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &Theme::Light,
                &Default::default(),
                Layout::new(&node),
                iced::mouse::Cursor::Unavailable,
                &bounds,
            );
            renderer.screenshot(Size::new(800, 500), 1., Color::WHITE)
        };
        let viewer = render(app.view());
        let chart = render(
            app.current
                .as_ref()
                .unwrap()
                .chart
                .element_with_readout()
                .map(Message::Chart),
        );
        assert_eq!(
            viewer, chart,
            "viewer added content or space around the chart"
        );
        std::fs::write("/tmp/feather-chart-only-view.rgba", viewer).unwrap();
    }
    #[test]
    fn loaded_view_is_chart_only_and_open_controls_are_temporary() {
        let (mut app, _) = App::new(None);
        app.request = 1;
        app.loaded(1, "valid.fchart".into(), Ok(doc()));
        assert!(app.current.is_some());
        assert!(!app.show_open);
        assert!(app.status.is_none());
        let _ = app.update(Message::FocusPath);
        assert!(app.show_open);
        let _ = app.update(Message::Dismiss);
        assert!(!app.show_open);
        assert!(app.current.is_some());
        app.request = 2;
        app.loaded(2, "bad.fchart".into(), Err("bad JSON".into()));
        assert!(app.show_open);
        let _ = app.update(Message::Dismiss);
        assert!(!app.show_open);
        assert!(app.status.is_none());
        assert_eq!(app.loaded_path, Some(PathBuf::from("valid.fchart")));
    }
    #[test]
    fn keyboard_controls_are_explicit_and_do_not_require_button_focus() {
        use iced::keyboard::{Key, Modifiers};
        assert!(matches!(
            shortcut(Key::Character("r".into()), Modifiers::CTRL),
            Some(Message::Reload)
        ));
        assert!(matches!(
            shortcut(Key::Character("0".into()), Modifiers::CTRL),
            Some(Message::Fit)
        ));
        assert!(matches!(
            shortcut(Key::Character("o".into()), Modifiers::CTRL),
            Some(Message::FocusPath)
        ));
        assert!(shortcut(Key::Character("r".into()), Modifiers::empty()).is_none());
    }
    #[test]
    fn newest_success_wins_and_errors_preserve_previous_chart() {
        let (mut app, _) = App::new(None);
        app.request = 2;
        app.loaded(1, "old.fchart".into(), Ok(doc()));
        assert!(app.current.is_none());
        app.loaded(2, "valid file.fchart".into(), Ok(doc()));
        assert!(app.current.is_some());
        app.request = 3;
        app.loaded(3, "broken.fchart".into(), Err("bad JSON".into()));
        assert_eq!(app.loaded_path, Some(PathBuf::from("valid file.fchart")));
        assert_eq!(app.status.as_deref(), Some("bad JSON"));
        app.loaded(2, "stale.fchart".into(), Ok(doc()));
        assert_eq!(app.status.as_deref(), Some("bad JSON"));
    }
}
