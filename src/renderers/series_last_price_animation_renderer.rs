use crate::model::{
    data_layer::SeriesId, last_price_animation::LastPriceAnimationState,
    series_options::LastPriceAnimationMode,
};
use iced::{
    Color, Point,
    widget::canvas::{Frame, Path, Stroke},
};
use std::time::Instant;

/// An owned, sampled top-pane marker. Drawing never consults a live series or clock.
#[derive(Clone, Debug)]
pub struct LastPriceAnimation {
    pub series: SeriesId,
    pub center: Point,
    pub dot_radius: f32,
    pub radius: f32,
    pub stroke_width: f32,
    pub color: Color,
    pub fill_color: Color,
    pub stroke_color: Color,
    pub(crate) state: LastPriceAnimationState,
    pub(crate) mode: LastPriceAnimationMode,
    pub(crate) active: bool,
}
impl LastPriceAnimation {
    pub(crate) fn sample(&mut self, now: Instant) {
        let sample = self.state.sample(self.mode, now);
        self.radius = sample.radius as f32;
        self.fill_color = Color {
            a: self.color.a * sample.fill_alpha as f32,
            ..self.color
        };
        self.stroke_color = Color {
            a: self.color.a * sample.stroke_alpha as f32,
            ..self.color
        };
        self.active = self.state.is_active(self.mode, now);
    }
    pub(crate) fn draw(&self, frame: &mut Frame) {
        frame.fill(&Path::circle(self.center, self.dot_radius), self.color);
        frame.fill(&Path::circle(self.center, self.radius), self.fill_color);
        frame.stroke(
            &Path::circle(self.center, self.radius + self.stroke_width / 2.),
            Stroke::default()
                .with_color(self.stroke_color)
                .with_width(self.stroke_width),
        );
    }
}
