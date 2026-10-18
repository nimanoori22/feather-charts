//! Source vector owned by a screen frame; never part of screenshot canvases.
#[path = "attribution_paths.rs"]
mod vector;
use crate::{
    api::PrimitivePaneId,
    model::colors::{ColorParseError, ColorParser},
    ui::chart_layout::LayoutRect,
};
use iced::{
    Color, Point, Rectangle, Size, Vector,
    widget::canvas::{Fill, Frame, Path, fill::Rule},
};
use std::rc::Rc;
#[derive(Clone, Debug)]
struct Style {
    paths: Rc<[Path; 2]>,
    text_color: String,
    dark: bool,
}
#[derive(Clone, Debug)]
pub struct AttributionLogo {
    bounds: Rectangle,
    clip: Rectangle,
    owner: PrimitivePaneId,
    style: Rc<Style>,
}
impl AttributionLogo {
    pub(crate) fn prepare(
        plot: LayoutRect,
        owner: PrimitivePaneId,
        text_color: &str,
        previous: Option<&Self>,
    ) -> Result<Option<Self>, ColorParseError> {
        let clip = Rectangle::new(
            Point::new(plot.origin.x as f32, plot.origin.y as f32),
            Size::new(plot.size.width as f32, plot.size.height as f32),
        );
        if clip.width <= 0. || clip.height <= 0. {
            return Ok(None);
        }
        let bounds = Rectangle::new(
            Point::new(clip.x + 10., clip.y + clip.height - 29.),
            Size::new(35., 19.),
        );
        if bounds.intersection(&clip).is_none() {
            return Ok(None);
        }
        let style = if let Some(old) = previous.filter(|p| p.style.text_color == text_color) {
            old.style.clone()
        } else {
            let color = ColorParser::default().parse_color(text_color)?;
            // Match JavaScript Number precision at the source's >160 boundary.
            let gray = 0.199 * f64::from(color.red)
                + 0.687 * f64::from(color.green)
                + 0.114 * f64::from(color.blue);
            Rc::new(Style {
                paths: previous
                    .map(|p| p.style.paths.clone())
                    .unwrap_or_else(|| Rc::new(vector::paths())),
                text_color: text_color.into(),
                dark: gray > 160.,
            })
        };
        Ok(Some(Self {
            bounds,
            clip,
            owner,
            style,
        }))
    }
    pub fn bounds(&self) -> Rectangle {
        self.bounds
    }
    pub fn clip(&self) -> Rectangle {
        self.clip
    }
    pub fn owner(&self) -> PrimitivePaneId {
        self.owner
    }
    pub fn hit_test(&self, point: Point) -> bool {
        contains(self.hit_bounds(), point)
    }
    pub fn dark(&self) -> bool {
        self.style.dark
    }
    pub fn hit_bounds(&self) -> Rectangle {
        self.bounds
            .intersection(&self.clip)
            .expect("visible prepared logo")
    }
    pub fn draw(&self, frame: &mut Frame) {
        let Some(clip) = self
            .hit_bounds()
            .intersection(&Rectangle::with_size(frame.size()))
        else {
            return;
        };
        frame.with_clip(clip, |frame| {
            frame.translate(Vector::new(self.bounds.x, self.bounds.y));
            let (fill, stroke) = if self.style.dark {
                (
                    Color::from_rgb8(209, 212, 220),
                    Color::from_rgb8(19, 23, 34),
                )
            } else {
                (Color::from_rgb8(19, 23, 34), Color::WHITE)
            };
            for (path, color) in self.style.paths.iter().zip([stroke, fill]) {
                frame.fill(
                    path,
                    Fill {
                        style: color.into(),
                        rule: Rule::EvenOdd,
                    },
                );
            }
        });
    }
}

pub(crate) fn contains(bounds: Rectangle, point: Point) -> bool {
    point.x >= bounds.x
        && point.x < bounds.x + bounds.width
        && point.y >= bounds.y
        && point.y < bounds.y + bounds.height
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Chart, ChartModelOptionsPatch, ChartOptionsPatch, LayoutOptionsPatch};
    #[test]
    fn frames_share_fixed_paths_and_keep_owned_style_after_owner_drop() {
        let mut c = Chart::default();
        c.add_pane();
        let patch = |color: &str| ChartOptionsPatch {
            model: ChartModelOptionsPatch {
                layout: Some(LayoutOptionsPatch {
                    attribution_logo: Some(true),
                    text_color: Some(color.into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        c.apply_chart_options(patch("#131722")).unwrap();
        let now = std::time::Instant::now();
        c.prepare(Size::new(800., 500.), 1., now).unwrap();
        let old = c.multi_frame().unwrap().attribution.clone().unwrap();
        c.prepare(Size::new(300., 300.), 1.5, now).unwrap();
        let moved = c.multi_frame().unwrap().attribution.clone().unwrap();
        assert!(Rc::ptr_eq(&old.style, &moved.style));
        assert_ne!(old.bounds, moved.bounds);
        c.apply_chart_options(patch("white")).unwrap();
        c.prepare(Size::new(300., 300.), 1.5, now).unwrap();
        let themed = c.multi_frame().unwrap().attribution.clone().unwrap();
        assert!(!Rc::ptr_eq(&old.style, &themed.style));
        assert!(Rc::ptr_eq(&old.style.paths, &themed.style.paths));
        drop(c);
        assert!(!old.dark());
        assert!(themed.dark());
    }
}
