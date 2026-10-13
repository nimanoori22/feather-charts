//! Custom geometry through the public API, including axis views and owned hits.
use super::*;
use feather_charts::api::{
    DrawingPrimitive, PreparedPrimitive, PrimitiveAxisLabel, PrimitiveContext,
    PrimitiveCoordinateTransform, PrimitiveEditEvent, PrimitiveEditPhase, PrimitiveHitCandidate,
    PrimitiveId, PrimitiveLayer, PrimitiveRenderer, PrimitiveView,
};
use std::{cell::RefCell, rc::Rc};
#[path = "primitive_editing.rs"]
mod editing;
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
    value: Rc<RefCell<Region>>,
    edit: Option<editing::BoxEdit>,
}
impl Scene {
    pub fn new(chart: &mut Chart, series: feather_charts::model::data_layer::SeriesId) -> Self {
        chart.add_pane_primitive(1, PaneBand).unwrap();
        let mut scene = Self {
            region: None,
            series,
            value: Rc::new(RefCell::new(Region {
                from: 20.,
                to: 65.,
                low: 88.,
                high: 115.,
                layer: PrimitiveLayer::Normal,
                selected: false,
            })),
            edit: None,
        };
        scene.toggle(chart);
        scene
    }
    fn value(&self) -> Region {
        self.value.borrow().clone()
    }
    pub fn toggle(&mut self, chart: &mut Chart) {
        self.edit = None;
        self.value.borrow_mut().selected = false;
        if let Some(id) = self.region.take() {
            chart.remove_primitive(id).unwrap();
        } else {
            self.region = Some(
                chart
                    .add_series_primitive(self.series, RegionSource(self.value.clone()))
                    .unwrap(),
            );
        }
    }
    pub fn shift(&mut self, chart: &mut Chart) {
        self.edit = None;
        let mut value = self.value.borrow_mut();
        value.from += 10.;
        value.to += 10.;
        value.low += 1.;
        value.high += 1.;
        drop(value);
        if let Some(id) = self.region {
            chart.request_primitive_update(id).unwrap();
        }
    }
    pub fn layer(&mut self, chart: &mut Chart) {
        self.edit = None;
        let mut value = self.value.borrow_mut();
        value.layer = match value.layer {
            PrimitiveLayer::Bottom => PrimitiveLayer::Normal,
            PrimitiveLayer::Normal => PrimitiveLayer::Top,
            PrimitiveLayer::Top => PrimitiveLayer::Bottom,
        };
        drop(value);
        if let Some(id) = self.region {
            chart.request_primitive_update(id).unwrap();
        }
    }
    pub fn deselect(&mut self, chart: &mut Chart) {
        if self.value.borrow().selected {
            self.value.borrow_mut().selected = false;
            if let Some(id) = self.region {
                chart.request_primitive_update(id).unwrap();
            }
        }
    }
    pub fn edit(
        &mut self,
        chart: &mut Chart,
        event: &PrimitiveEditEvent,
    ) -> Result<(), feather_charts::api::ChartError> {
        if self.region != Some(event.hit.primitive) {
            return Ok(());
        }
        match event.phase {
            PrimitiveEditPhase::Begin => {
                let Some(transform) = chart
                    .primitive_context(event.hit.primitive)?
                    .coordinate_transform()
                else {
                    return Ok(());
                };
                let mut value = self.value();
                value.selected = true;
                self.edit = editing::BoxEdit::new(event, value.clone(), transform);
                *self.value.borrow_mut() = value;
                chart.preview_primitive_update(event.id)?;
            }
            PrimitiveEditPhase::Update | PrimitiveEditPhase::Commit => {
                if let Some(edit) = &self.edit
                    && edit.id == event.id
                    && let Some(value) = edit.preview(event.position)
                {
                    *self.value.borrow_mut() = value;
                    if event.phase == PrimitiveEditPhase::Update {
                        chart.preview_primitive_update(event.id)?;
                    } else {
                        self.edit = None;
                        chart.request_primitive_update(event.hit.primitive)?;
                    }
                }
                if event.phase == PrimitiveEditPhase::Commit {
                    self.edit = None;
                }
            }
            PrimitiveEditPhase::Cancel(reason) => {
                if self.edit.as_ref().is_some_and(|e| e.id == event.id)
                    && let Some(edit) = self.edit.take()
                    && edit.id == event.id
                    && reason.restores_bounds()
                    && chart.primitive_owner(event.hit.primitive).is_ok()
                {
                    *self.value.borrow_mut() = edit.original;
                    chart.request_primitive_update(event.hit.primitive)?;
                }
                if chart.primitive_owner(event.hit.primitive).is_err() {
                    self.region = None;
                    self.value.borrow_mut().selected = false;
                }
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
struct Region {
    from: f64,
    to: f64,
    low: f64,
    high: f64,
    layer: PrimitiveLayer,
    selected: bool,
}
#[derive(Debug)]
struct RegionSource(Rc<RefCell<Region>>);
impl DrawingPrimitive for RegionSource {
    fn prepare(&self, c: &PrimitiveContext<'_>) -> PreparedPrimitive {
        self.0.borrow().prepare(c)
    }
    fn autoscale_info(&self, start: TimePointIndex, end: TimePointIndex) -> Option<AutoscaleInfo> {
        self.0.borrow().autoscale_info(start, end)
    }
}
impl DrawingPrimitive for Region {
    fn prepare(&self, c: &PrimitiveContext<'_>) -> PreparedPrimitive {
        let Some((x1, x2, y1, y2)) = c
            .continuous_logical_to_coordinate(self.from)
            .zip(c.continuous_logical_to_coordinate(self.to))
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
        let mut views = vec![PrimitiveView::new(self.layer, region)];
        if self.selected {
            views.push(PrimitiveView::new(
                PrimitiveLayer::Top,
                editing::SelectionDrawing { rect },
            ));
        }
        PreparedPrimitive {
            hit_tester: self.selected.then(|| {
                Rc::new(editing::SelectionDrawing { rect })
                    as Rc<dyn feather_charts::api::PrimitiveHitTester>
            }),
            views,
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
        h.editable = true;
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
        "primitive-selected"
        | "primitive-selected-narrow"
        | "primitive-moving"
        | "primitive-resized"
        | "primitive-cancelled" => {
            editing::setup_capture(app, scenario);
        }
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
    if matches!(
        scenario,
        "primitive-selected"
            | "primitive-selected-narrow"
            | "primitive-moving"
            | "primitive-resized"
            | "primitive-cancelled"
    ) {
        editing::verify_pixels(app, rgba, width);
        return;
    }
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
pub fn profile() {
    editing::profile();
}

#[cfg(test)]
mod editing_tests {
    use super::*;
    use iced::{
        mouse,
        widget::canvas::{Event, Program},
    };
    fn dispatch(
        app: &mut App,
        state: &mut feather_charts::api::ChartState,
        event: &Event,
        p: Point,
    ) {
        let plot = app.chart.frame().layout.plot;
        let message = app
            .chart
            .program(std::convert::identity)
            .update(
                state,
                event,
                Rectangle::new(Point::ORIGIN, app.size),
                mouse::Cursor::Available(Point::new(
                    p.x + plot.origin.x as f32,
                    p.y + plot.origin.y as f32,
                )),
            )
            .and_then(|a| a.into_inner().0);
        if let Some(message) = message {
            let _ = app.update(Message::Chart(message));
        }
    }
    fn setup() -> App {
        let mut app = App::new(SeriesType::Line);
        app.primitive_scene = Some(Scene::new(&mut app.chart, app.price));
        app.size = Size::new(800., 500.);
        app.prepare();
        app
    }
    #[test]
    fn box_selection_exposes_handles_and_native_drag_moves_fractional_bounds() {
        let mut app = setup();
        let id = app.primitive_scene.as_ref().unwrap().region.unwrap();
        let original = app.primitive_scene.as_ref().unwrap().value();
        let c = app.chart.primitive_context(id).unwrap();
        let x = c.continuous_logical_to_coordinate(30.).unwrap() as f32;
        let y = c.price_to_coordinate(114.).unwrap() as f32;
        let mut state = Default::default();
        dispatch(
            &mut app,
            &mut state,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Point::new(x, y),
        );
        assert!(app.primitive_scene.as_ref().unwrap().value().selected);
        dispatch(
            &mut app,
            &mut state,
            &Event::Mouse(mouse::Event::CursorMoved {
                position: Point::ORIGIN,
            }),
            Point::new(x + 17., y + 13.),
        );
        let preview = app.primitive_scene.as_ref().unwrap().value();
        assert!(preview.from > original.from);
        assert!(((preview.to - preview.from) - (original.to - original.from)).abs() < 1e-9);
        assert!(preview.from.fract().abs() > 0.001);
        dispatch(
            &mut app,
            &mut state,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            Point::new(x + 17., y + 13.),
        );
        assert!(!app.chart.is_panning());
        assert!(
            app.chart.model().panes()[0]
                .right_price_scale()
                .is_auto_scale()
        );
    }
    #[test]
    fn focus_loss_restores_original_box_and_empty_click_deselects() {
        let mut app = setup();
        let original = app.primitive_scene.as_ref().unwrap().value();
        let id = app.primitive_scene.as_ref().unwrap().region.unwrap();
        let c = app.chart.primitive_context(id).unwrap();
        let p = Point::new(
            c.continuous_logical_to_coordinate(30.).unwrap() as f32,
            c.price_to_coordinate(114.).unwrap() as f32,
        );
        let mut state = Default::default();
        dispatch(
            &mut app,
            &mut state,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            p,
        );
        dispatch(
            &mut app,
            &mut state,
            &Event::Mouse(mouse::Event::CursorMoved {
                position: Point::ORIGIN,
            }),
            Point::new(p.x + 30., p.y + 20.),
        );
        assert_ne!(
            app.primitive_scene.as_ref().unwrap().value().from,
            original.from
        );
        dispatch(
            &mut app,
            &mut state,
            &Event::Window(iced::window::Event::Unfocused),
            p,
        );
        let restored = app.primitive_scene.as_ref().unwrap().value();
        assert_eq!(
            [restored.from, restored.to, restored.low, restored.high],
            [original.from, original.to, original.low, original.high]
        );
        assert!(restored.selected);
        assert!(app.primitive_scene.as_ref().unwrap().edit.is_none());
        dispatch(
            &mut app,
            &mut state,
            &Event::Window(iced::window::Event::Focused),
            p,
        );
        let outside = Point::new(5., 5.);
        dispatch(
            &mut app,
            &mut state,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            outside,
        );
        dispatch(
            &mut app,
            &mut state,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            outside,
        );
        assert!(!app.primitive_scene.as_ref().unwrap().value().selected);
        assert!(!app.chart.is_panning());
    }
}
