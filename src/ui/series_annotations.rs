//! Converts annotations once; immutable drawing and hit testing share settled bounds.
use super::{
    chart_frame::ChartFrameError,
    line_chart::{PlotSnapshot, color},
    price_axis::AxisFontResolver,
};
use crate::{
    model::series_markers::MarkerZOrder, renderers::series_markers_renderer::MarkerShapeCommand,
};
use iced::{
    Color, Point, Size,
    widget::canvas::{Frame, Path},
};

#[derive(Clone, Debug)]
pub struct CanvasAnnotations {
    pub series_layer: usize,
    pub z_order: MarkerZOrder,
    pub paths: Vec<(Path, Color)>,
}
impl PlotSnapshot {
    pub(crate) fn prepare_annotations(
        &mut self,
        fonts: &AxisFontResolver,
        ratio: f64,
    ) -> Result<(), ChartFrameError> {
        self.annotation_geometry.clear();
        for collection in &mut self.annotations {
            let font = fonts
                .resolve(&collection.font_family)
                .map_err(ChartFrameError::PriceUi)?;
            let mut paths = vec![];
            for item in &mut collection.items {
                let color = color(&item.color).map_err(ChartFrameError::Plot)?;
                if let Some(t) = &mut item.text {
                    t.width = fonts
                        .text_width(&t.content, font, collection.font_size as f32)
                        .map_err(ChartFrameError::PriceUi)?;
                    t.x = item.center.x - t.width / 2.;
                    paths.extend(fonts.glyph_paths(
                        &t.content,
                        font,
                        collection.font_size as f32,
                        iced::advanced::text::Alignment::Left,
                        color,
                        Point::new(t.x as f32, t.y as f32),
                    ));
                }
                let logical = |p: crate::renderers::line_renderer::Point| {
                    Point::new((p.x / ratio) as f32, (p.y / ratio) as f32)
                };
                if let Some(cmd) = item.shape_command(ratio) {
                    let path = match cmd {
                        MarkerShapeCommand::Circle { center, radius } => {
                            Path::circle(logical(center), (radius / ratio) as f32)
                        }
                        MarkerShapeCommand::Square { origin, size } => Path::rectangle(
                            logical(origin),
                            Size::new((size / ratio) as f32, (size / ratio) as f32),
                        ),
                        MarkerShapeCommand::Polygon(points) => Path::new(|b| {
                            b.move_to(logical(points[0]));
                            for p in &points[1..] {
                                b.line_to(logical(*p));
                            }
                            b.close();
                        }),
                    };
                    paths.push((path, color));
                }
            }
            self.annotation_geometry.push(CanvasAnnotations {
                series_layer: collection.series_layer,
                z_order: collection.z_order,
                paths,
            });
        }
        Ok(())
    }
    pub(crate) fn draw_annotations(
        &self,
        frame: &mut Frame,
        z_order: MarkerZOrder,
        layer: Option<usize>,
    ) {
        for collection in &self.annotation_geometry {
            if collection.z_order == z_order && layer.is_none_or(|l| l == collection.series_layer) {
                for (path, color) in &collection.paths {
                    frame.fill(path, *color);
                }
            }
        }
    }
    pub fn marker_hit(&self, point: Point) -> Option<crate::model::series_markers::MarkerHit> {
        if point.x < 0. || point.y < 0. || point.x >= self.size.width || point.y >= self.size.height
        {
            return None;
        }
        // Reverse source order, then attachment order, then the first visible marker.
        for layer in (0..=self.series_layers.len()).rev() {
            for collection in self.annotations.iter().filter(|c| c.series_layer == layer) {
                if let Some(item) = collection
                    .items
                    .iter()
                    .find(|m| m.hit_test(f64::from(point.x), f64::from(point.y)))
                {
                    return Some(item.hit.clone());
                }
            }
        }
        None
    }
}
