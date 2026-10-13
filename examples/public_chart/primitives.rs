//! Custom geometry through the public API, including axis views and owned hits.
use super::*;
use feather_charts::api::{
    DrawingPrimitive, PreparedPrimitive, PrimitiveAxisLabel, PrimitiveContext,
    PrimitiveHitCandidate, PrimitiveId, PrimitiveLayer, PrimitiveRenderer, PrimitiveView,
};
use feather_charts::model::{
    autoscale_info_impl::{AutoScaleMargins, AutoscaleInfo},
    internal_hit_test::HitTestPriority,
    price_range_impl::PriceRange,
    time_data::TimePointIndex,
};
use iced::{
    Color, Point, Rectangle,
    widget::canvas::{Frame, Path, Stroke},
};

pub struct Scene {
    pub region: Option<PrimitiveId>,
    series: feather_charts::model::data_layer::SeriesId,
    step: usize,
    layer: PrimitiveLayer,
}
impl Scene {
    pub fn new(chart: &mut Chart, series: feather_charts::model::data_layer::SeriesId) -> Self {
        chart.add_pane_primitive(1, PaneBand).unwrap();
        let mut scene = Self {
            region: None,
            series,
            step: 0,
            layer: PrimitiveLayer::Normal,
        };
        scene.toggle(chart);
        scene
    }
    fn value(&self) -> Region {
        Region {
            from: 20. + (self.step % 3) as f64 * 10.,
            to: 65. + (self.step % 3) as f64 * 10.,
            low: 88. + (self.step % 3) as f64,
            high: 115. + (self.step % 3) as f64,
            layer: self.layer,
        }
    }
    pub fn toggle(&mut self, chart: &mut Chart) {
        if let Some(id) = self.region.take() {
            chart.remove_primitive(id).unwrap();
        } else {
            self.region = Some(
                chart
                    .add_series_primitive(self.series, self.value())
                    .unwrap(),
            );
        }
    }
    pub fn shift(&mut self, chart: &mut Chart) {
        self.step += 1;
        if let Some(id) = self.region {
            chart.replace_primitive(id, self.value()).unwrap();
        }
    }
    pub fn layer(&mut self, chart: &mut Chart) {
        self.layer = match self.layer {
            PrimitiveLayer::Bottom => PrimitiveLayer::Normal,
            PrimitiveLayer::Normal => PrimitiveLayer::Top,
            PrimitiveLayer::Top => PrimitiveLayer::Bottom,
        };
        if let Some(id) = self.region {
            chart.replace_primitive(id, self.value()).unwrap();
        }
    }
}
#[derive(Clone, Debug)]
struct Region {
    from: f64,
    to: f64,
    low: f64,
    high: f64,
    layer: PrimitiveLayer,
}
impl DrawingPrimitive for Region {
    fn prepare(&self, c: &PrimitiveContext<'_>) -> PreparedPrimitive {
        let Some((x1, x2, y1, y2)) = c
            .logical_to_coordinate(self.from)
            .zip(c.logical_to_coordinate(self.to))
            .zip(
                c.price_to_coordinate(self.low)
                    .zip(c.price_to_coordinate(self.high)),
            )
            .map(|((x1, x2), (y1, y2))| (x1 as f32, x2 as f32, y1 as f32, y2 as f32))
        else {
            return Default::default();
        };
        let rect = Rectangle::new(
            Point::new(x1.min(x2), y1.min(y2)),
            Size::new((x2 - x1).abs(), (y2 - y1).abs()),
        );
        let region = RegionDrawing {
            rect,
            layer: self.layer,
        };
        PreparedPrimitive {
            views: vec![PrimitiveView::new(self.layer, region)],
            price_labels: vec![
                PrimitiveAxisLabel::new(
                    f64::from(y1),
                    format!("Region low {:.0}", self.low),
                    "#9c27b0",
                    "white",
                ),
                PrimitiveAxisLabel::new(
                    f64::from(y2),
                    format!("Region high {:.0}", self.high),
                    "#9c27b0",
                    "white",
                ),
            ],
            time_labels: vec![
                PrimitiveAxisLabel::new(f64::from(x1), "Region start", "#9c27b0", "white"),
                PrimitiveAxisLabel::new(f64::from(x2), "Region end", "#9c27b0", "white"),
            ],
            price_views: vec![PrimitiveView::new(
                PrimitiveLayer::Bottom,
                AxisBand {
                    from: rect.y,
                    length: rect.height,
                    vertical: true,
                },
            )],
            time_views: vec![PrimitiveView::new(
                PrimitiveLayer::Bottom,
                AxisBand {
                    from: rect.x,
                    length: rect.width,
                    vertical: false,
                },
            )],
            ..Default::default()
        }
    }
    fn autoscale_info(&self, start: TimePointIndex, end: TimePointIndex) -> Option<AutoscaleInfo> {
        (start.value() <= self.to && end.value() >= self.from).then_some(AutoscaleInfo {
            price_range: Some(PriceRange {
                min_value: self.low,
                max_value: self.high,
            }),
            margins: Some(AutoScaleMargins {
                above: 3.,
                below: 3.,
            }),
        })
    }
}
#[derive(Debug)]
struct RegionDrawing {
    rect: Rectangle,
    layer: PrimitiveLayer,
}
impl PrimitiveRenderer for RegionDrawing {
    fn draw_background(&self, f: &mut Frame) {
        f.fill_rectangle(
            self.rect.position(),
            self.rect.size(),
            Color::from_rgba8(156, 39, 176, 0.15),
        );
    }
    fn draw(&self, f: &mut Frame) {
        f.stroke(
            &Path::rectangle(self.rect.position(), self.rect.size()),
            Stroke::default()
                .with_width(2.)
                .with_color(Color::from_rgb8(156, 39, 176)),
        );
    }
    fn hit_test(&self, p: Point) -> Option<PrimitiveHitCandidate> {
        let r = self.rect;
        let distance = [
            (
                (p.x - r.x).abs(),
                p.y >= r.y - 7. && p.y <= r.y + r.height + 7.,
            ),
            (
                (p.x - r.x - r.width).abs(),
                p.y >= r.y - 7. && p.y <= r.y + r.height + 7.,
            ),
            (
                (p.y - r.y).abs(),
                p.x >= r.x - 7. && p.x <= r.x + r.width + 7.,
            ),
            (
                (p.y - r.y - r.height).abs(),
                p.x >= r.x - 7. && p.x <= r.x + r.width + 7.,
            ),
        ]
        .into_iter()
        .filter(|(_, eligible)| *eligible)
        .map(|(d, _)| d)
        .min_by(f32::total_cmp);
        let boundary = distance.is_some_and(|d| d <= 7.);
        if !boundary && !r.contains(p) {
            return None;
        }
        let mut h = PrimitiveHitCandidate::new(if boundary {
            "region boundary"
        } else {
            "shaded region"
        });
        h.layer = self.layer;
        h.is_background = !boundary;
        h.priority = if boundary {
            HitTestPriority::Line
        } else {
            HitTestPriority::Range
        };
        h.distance = if boundary {
            f64::from(distance.unwrap())
        } else {
            0.
        };
        h.cursor = Some(iced::mouse::Interaction::Pointer);
        Some(h)
    }
}
#[derive(Debug)]
struct AxisBand {
    from: f32,
    length: f32,
    vertical: bool,
}
impl PrimitiveRenderer for AxisBand {
    fn draw(&self, f: &mut Frame) {
        let (p, size) = if self.vertical {
            (Point::new(0., self.from), Size::new(8., self.length))
        } else {
            (Point::new(self.from, 0.), Size::new(self.length, 6.))
        };
        f.fill_rectangle(p, size, Color::from_rgba8(156, 39, 176, 0.35));
    }
}
#[derive(Debug)]
struct PaneBand;
impl DrawingPrimitive for PaneBand {
    fn prepare(&self, c: &PrimitiveContext<'_>) -> PreparedPrimitive {
        PreparedPrimitive {
            views: vec![PrimitiveView::new(
                PrimitiveLayer::Bottom,
                AxisBand {
                    from: c.size.width * 0.15,
                    length: c.size.width * 0.25,
                    vertical: false,
                },
            )],
            ..Default::default()
        }
    }
}
pub fn setup_capture(app: &mut App, scenario: &str) {
    if app.primitive_scene.is_none() {
        app.primitive_scene = Some(Scene::new(&mut app.chart, app.price));
    }
    app.prepare();
    match scenario {
        "primitive-top" => {
            app.primitive_scene.as_mut().unwrap().layer(&mut app.chart);
        }
        "primitive-bottom" => {
            let scene = app.primitive_scene.as_mut().unwrap();
            scene.layer(&mut app.chart);
            scene.layer(&mut app.chart);
        }
        "primitive-updated" => {
            app.primitive_scene.as_mut().unwrap().shift(&mut app.chart);
        }
        "primitive-removed" => {
            app.primitive_scene.as_mut().unwrap().toggle(&mut app.chart);
        }
        "primitive-narrow" => {
            app.size = Size::new(400., 300.);
        }
        "primitive-hover" => {
            // Put the boundary above the mixed overlay sources for this capture.
            app.primitive_scene.as_mut().unwrap().layer(&mut app.chart);
            app.prepare();
            let x = app
                .chart
                .model()
                .time_scale()
                .index_to_coordinate(TimePointIndex::new(20.))
                .value() as f32;
            app.chart
                .update(ChartMessage::Interaction(
                    feather_charts::ui::chart_input::ChartInteraction {
                        position: Some(Point::new(x, app.chart.frame().plot.size.height / 2.)),
                        navigation: None,
                        capture: true,
                    },
                ))
                .unwrap();
            assert!(
                app.chart
                    .primitive_hit(0, Point::new(x, app.chart.frame().plot.size.height / 2.))
                    .is_some()
            );
        }
        _ => {}
    }
}

pub fn verify_pixels(app: &App, scenario: &str, rgba: &[u8], width: usize) {
    if !scenario.starts_with("primitive-") || scenario == "primitive-narrow" {
        return;
    }
    if scenario == "primitive-removed" {
        assert!(app.primitive_scene.as_ref().unwrap().region.is_none());
        assert!(
            !app.chart
                .frame()
                .plot
                .axes
                .as_ref()
                .unwrap()
                .right
                .labels
                .iter()
                .any(|l| l.axis.text.starts_with("Region"))
        );
        return;
    }
    let region = app.primitive_scene.as_ref().unwrap().value();
    let plot = app.chart.frame().layout.plot;
    let x = app
        .chart
        .model()
        .time_scale()
        .index_to_coordinate(TimePointIndex::new(
            ((region.from + region.to) / 2.).floor(),
        ))
        .value();
    let scale = app.chart.model().panes()[0].right_price_scale();
    let y = scale.price_to_coordinate(region.high - 1., 100.).value();
    let sample = |x: f64, y: f64| {
        let x = (x * f64::from(app.ratio)).round() as usize;
        let y = (y * f64::from(app.ratio)).round() as usize;
        &rgba[(y * width + x) * 4..][..3]
    };
    let pixel = sample(plot.origin.x + x, plot.origin.y + y);
    for (actual, expected) in pixel.iter().zip([240u8, 223, 243]) {
        assert!(
            actual.abs_diff(expected) <= 2,
            "{scenario}: primitive background must blend once: {pixel:?}"
        );
    }
    let axis = app.chart.frame().layout.time_axis;
    let pixel = sample(axis.origin.x + x, axis.origin.y + 2.);
    for (actual, expected) in pixel.iter().zip([220u8, 179, 227]) {
        assert!(
            actual.abs_diff(expected) <= 2,
            "{scenario}: prepared time-axis band must be visible: {pixel:?}"
        );
    }
}
