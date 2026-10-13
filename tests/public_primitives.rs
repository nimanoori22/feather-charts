use feather_charts::api::{Chart, DrawingPrimitive, PreparedPrimitive, PrimitiveContext};
use feather_charts::model::{pane::PriceScalePosition, series_options::SeriesOptionsMap};

#[derive(Debug)]
struct Empty;
impl DrawingPrimitive for Empty {
    fn prepare(&self, _: &PrimitiveContext<'_>) -> PreparedPrimitive {
        PreparedPrimitive::default()
    }
}
#[test]
fn attachment_ids_are_owner_checked_and_not_reused() {
    let mut chart = Chart::default();
    let series = chart
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let first = chart.add_series_primitive(series, Empty).unwrap();
    let mut other = Chart::default();
    assert!(other.remove_primitive(first).is_err());
    chart.remove_primitive(first).unwrap();
    assert!(chart.remove_primitive(first).is_err());
    let second = chart.add_series_primitive(series, Empty).unwrap();
    assert_ne!(first, second);
}

use feather_charts::api::{PrimitiveLayer, PrimitiveOwner, PrimitiveRenderer, PrimitiveView};
use iced::{Point, Size, widget::canvas::Frame};
use std::{cell::RefCell, rc::Rc, time::Instant};
#[derive(Debug)]
struct Drawing {
    label: &'static str,
    layer: PrimitiveLayer,
    calls: Rc<RefCell<Vec<String>>>,
}
#[derive(Debug)]
struct Frozen {
    label: &'static str,
    width: f32,
    calls: Rc<RefCell<Vec<String>>>,
}
impl PrimitiveRenderer for Frozen {
    fn draw(&self, frame: &mut Frame) {
        self.calls
            .borrow_mut()
            .push(format!("fg:{}:{}", self.label, self.width));
        frame.fill_rectangle(
            Point::new(10., 10.),
            Size::new(self.width, 20.),
            iced::Color::from_rgba(1., 0., 0., 0.5),
        );
    }
    fn draw_background(&self, _: &mut Frame) {
        self.calls
            .borrow_mut()
            .push(format!("bg:{}:{}", self.label, self.width));
    }
}
impl DrawingPrimitive for Drawing {
    fn attached(&self, _: PrimitiveOwner) {
        self.calls
            .borrow_mut()
            .push(format!("attach:{}", self.label));
    }
    fn detached(&self) {
        self.calls
            .borrow_mut()
            .push(format!("detach:{}", self.label));
    }
    fn prepare(&self, c: &PrimitiveContext<'_>) -> PreparedPrimitive {
        PreparedPrimitive {
            views: vec![PrimitiveView::new(
                self.layer,
                Frozen {
                    label: self.label,
                    width: c.size.width / 2.,
                    calls: self.calls.clone(),
                },
            )],
            ..Default::default()
        }
    }
}
fn renderer() -> iced::Renderer {
    use iced::advanced::renderer::Headless;
    iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some("tiny-skia"),
    ))
    .unwrap()
}
fn series(chart: &mut Chart, pane: usize) -> feather_charts::model::data_layer::SeriesId {
    chart
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            pane,
            PriceScalePosition::Right,
        )
        .unwrap()
}
fn drawing(
    label: &'static str,
    layer: PrimitiveLayer,
    calls: &Rc<RefCell<Vec<String>>>,
) -> Drawing {
    Drawing {
        label,
        layer,
        calls: calls.clone(),
    }
}
#[test]
fn retained_frames_keep_old_geometry_without_keeping_attachment_active() {
    let calls = Rc::new(RefCell::new(vec![]));
    let mut chart = Chart::default();
    let s = series(&mut chart, 0);
    let id = chart
        .add_series_primitive(s, drawing("old", PrimitiveLayer::Normal, &calls))
        .unwrap();
    chart
        .prepare(Size::new(640., 480.), 1.25, Instant::now())
        .unwrap();
    let old = chart.frame().clone();
    chart
        .replace_primitive(id, drawing("new", PrimitiveLayer::Normal, &calls))
        .unwrap();
    assert_eq!(*calls.borrow(), ["attach:old", "detach:old", "attach:new"]);
    chart
        .prepare(Size::new(400., 300.), 2., Instant::now())
        .unwrap();
    let new = chart.frame().clone();
    chart.remove_primitive(id).unwrap();
    calls.borrow_mut().clear();
    let renderer = renderer();
    old.draw(&mut Frame::new(&renderer, Size::new(640., 480.)));
    new.draw(&mut Frame::new(&renderer, Size::new(400., 300.)));
    let calls = calls.borrow();
    assert!(
        calls
            .iter()
            .any(|c| c == &format!("fg:old:{}", old.plot.size.width / 2.))
    );
    assert!(
        calls
            .iter()
            .any(|c| c == &format!("fg:new:{}", new.plot.size.width / 2.))
    );
    assert_eq!(calls.len(), 4);
}
#[test]
fn pane_keys_survive_index_shifts_and_cleanup_occurs_once() {
    let calls = Rc::new(RefCell::new(vec![]));
    let mut chart = Chart::default();
    let first = series(&mut chart, 0);
    let second_pane = chart.add_pane();
    let second = series(&mut chart, second_pane);
    let p = chart
        .add_pane_primitive(second_pane, drawing("pane", PrimitiveLayer::Normal, &calls))
        .unwrap();
    let s = chart
        .add_series_primitive(second, drawing("series", PrimitiveLayer::Normal, &calls))
        .unwrap();
    let owner = chart.primitive_owner(p).unwrap();
    chart.remove_series(first).unwrap();
    assert_eq!(chart.primitive_owner(p).unwrap(), owner);
    chart
        .prepare(Size::new(640., 480.), 1., Instant::now())
        .unwrap();
    assert_eq!(chart.frame().plot.primitives.len(), 2);
    chart.remove_series(second).unwrap();
    assert!(chart.primitive_owner(s).is_err());
    // The final pane is retained; its pane attachment remains until Chart drops.
    assert_eq!(chart.primitive_owner(p).unwrap(), owner);
    drop(chart);
    assert_eq!(
        calls
            .borrow()
            .iter()
            .filter(|c| *c == "detach:pane")
            .count(),
        1
    );
    assert_eq!(
        calls
            .borrow()
            .iter()
            .filter(|c| *c == "detach:series")
            .count(),
        1
    );
}
#[test]
fn all_backgrounds_precede_foregrounds_in_each_layer() {
    let calls = Rc::new(RefCell::new(vec![]));
    let mut chart = Chart::default();
    let s = series(&mut chart, 0);
    for layer in [
        PrimitiveLayer::Bottom,
        PrimitiveLayer::Normal,
        PrimitiveLayer::Top,
    ] {
        chart
            .add_series_primitive(s, drawing("series", layer, &calls))
            .unwrap();
        chart
            .add_pane_primitive(0, drawing("pane", layer, &calls))
            .unwrap();
    }
    chart
        .prepare(Size::new(640., 480.), 1., Instant::now())
        .unwrap();
    calls.borrow_mut().clear();
    let renderer = renderer();
    chart
        .frame()
        .draw(&mut Frame::new(&renderer, Size::new(640., 480.)));
    let calls = calls.borrow();
    let kinds = calls
        .iter()
        .map(|s| s.split(':').take(2).collect::<Vec<_>>().join(":"))
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        ["bg:pane", "bg:series", "fg:pane", "fg:series"].repeat(3)
    );
}

use feather_charts::model::{
    autoscale_info_impl::{AutoScaleMargins, AutoscaleInfo},
    data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
    horz_scale_behavior_time::types::{Time, UtcTimestamp},
    price_range_impl::PriceRange,
    time_data::TimePointIndex,
};
#[derive(Debug)]
struct ScaleRegion {
    min: f64,
    max: f64,
    margin: f64,
}
impl DrawingPrimitive for ScaleRegion {
    fn prepare(&self, _: &PrimitiveContext<'_>) -> PreparedPrimitive {
        Default::default()
    }
    fn autoscale_info(&self, _: TimePointIndex, _: TimePointIndex) -> Option<AutoscaleInfo> {
        Some(AutoscaleInfo {
            price_range: Some(PriceRange {
                min_value: self.min,
                max_value: self.max,
            }),
            margins: Some(AutoScaleMargins {
                above: self.margin,
                below: self.margin,
            }),
        })
    }
}
fn data(chart: &mut Chart, id: feather_charts::model::data_layer::SeriesId) {
    chart
        .set_data(
            id,
            (0..10)
                .map(|i| {
                    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                        time: Time::from(UtcTimestamp::new(1000. + i as f64 * 10.)),
                        value: 100. + i as f64,
                        color: None,
                        custom_values: None,
                    }))
                })
                .collect(),
        )
        .unwrap();
    chart.fit_content();
}
#[test]
fn autoscale_merges_ranges_last_margins_win_and_removal_restores_builtin() {
    use feather_charts::model::{
        axis_snapshots::PriceAxisSide, price_scale::PriceScaleOptionsPatch,
    };
    let mut chart = Chart::default();
    let s = series(&mut chart, 0);
    data(&mut chart, s);
    chart
        .prepare(Size::new(640., 480.), 1., Instant::now())
        .unwrap();
    let manual_range = chart.model().panes()[0]
        .right_price_scale()
        .price_range()
        .unwrap()
        .to_raw();
    chart
        .apply_price_axis_options(
            0,
            PriceAxisSide::Right,
            PriceScaleOptionsPatch {
                auto_scale: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
    let a = chart
        .add_series_primitive(
            s,
            ScaleRegion {
                min: 20.,
                max: 130.,
                margin: 7.,
            },
        )
        .unwrap();
    let b = chart
        .add_series_primitive(
            s,
            ScaleRegion {
                min: 90.,
                max: 150.,
                margin: 3.,
            },
        )
        .unwrap();
    chart
        .prepare(Size::new(640., 480.), 1., Instant::now())
        .unwrap();
    assert_eq!(
        chart.model().panes()[0]
            .right_price_scale()
            .price_range()
            .unwrap()
            .to_raw(),
        manual_range
    );
    let raw = || {
        chart
            .model()
            .series(s)
            .unwrap()
            .autoscale_info(TimePointIndex::new(0.), TimePointIndex::new(9.))
            .unwrap()
            .to_raw()
    };
    assert_eq!(
        raw().price_range.unwrap(),
        PriceRange {
            min_value: 20.,
            max_value: 150.
        }
    );
    assert_eq!(raw().margins.unwrap().above, 3.);
    chart.remove_primitive(b).unwrap();
    chart.remove_primitive(a).unwrap();
    let info = chart
        .model()
        .series(s)
        .unwrap()
        .autoscale_info(TimePointIndex::new(0.), TimePointIndex::new(9.))
        .unwrap();
    assert_eq!(
        info.price_range().unwrap().to_raw(),
        PriceRange {
            min_value: 100.,
            max_value: 109.
        }
    );
    assert_eq!(info.margins(), None);
}

#[derive(Debug)]
struct Labels {
    calls: Rc<RefCell<Vec<String>>>,
}
impl DrawingPrimitive for Labels {
    fn prepare(&self, context: &PrimitiveContext<'_>) -> PreparedPrimitive {
        use feather_charts::api::PrimitiveAxisLabel;
        let mut fixed = PrimitiveAxisLabel::new(100., "Fixed", "blue", "white");
        fixed.fixed_coordinate = Some(35.);
        fixed.tick_visible = false;
        let mut time = PrimitiveAxisLabel::new(-10., "Outside time", "blue", "white");
        time.fixed_coordinate = Some(800.);
        time.tick_visible = false;
        PreparedPrimitive {
            price_labels: vec![
                PrimitiveAxisLabel::new(
                    context.price_to_coordinate(105.).unwrap_or(100.),
                    "A very wide primitive price label",
                    "blue",
                    "white",
                ),
                fixed,
            ],
            time_labels: vec![time],
            price_views: vec![PrimitiveView::new(
                PrimitiveLayer::Bottom,
                Frozen {
                    label: "price",
                    width: 10.,
                    calls: self.calls.clone(),
                },
            )],
            time_views: vec![PrimitiveView::new(
                PrimitiveLayer::Top,
                Frozen {
                    label: "time",
                    width: 10.,
                    calls: self.calls.clone(),
                },
            )],
            ..Default::default()
        }
    }
}
#[test]
fn primitive_labels_settle_axis_width_and_axis_views_are_clipped_and_drawn() {
    use feather_charts::renderers::{
        price_axis_renderer::PriceAxisCommand, time_axis_renderer::TimeAxisCommand,
    };
    let calls = Rc::new(RefCell::new(vec![]));
    let mut chart = Chart::default();
    let s = series(&mut chart, 0);
    data(&mut chart, s);
    chart
        .prepare(Size::new(640., 480.), 1.25, Instant::now())
        .unwrap();
    let base_width = chart.frame().layout.right_axis.size.width;
    chart
        .add_series_primitive(
            s,
            Labels {
                calls: calls.clone(),
            },
        )
        .unwrap();
    let snapshot = chart
        .prepare_snapshot(Size::new(640., 480.), 1.25, Instant::now())
        .unwrap();
    let pane = &snapshot.panes[0];
    assert!(pane.layout.right_axis.size.width > base_width);
    assert!(pane.passes <= 8);
    assert!(
        pane.right
            .commands
            .iter()
            .any(|cmd| matches!(cmd, PriceAxisCommand::Text{label,..} if label=="Fixed"))
    );
    let labels = &pane.plot.axes.as_ref().unwrap().right.labels;
    assert_eq!(
        labels
            .iter()
            .find(|l| l.axis.text == "Fixed")
            .unwrap()
            .fixed_coordinate,
        Some(35.)
    );
    // The source ignores fixedCoordinate for time labels; it clamps the background only.
    assert!(pane.time.commands.iter().any(|cmd| matches!(cmd, TimeAxisCommand::RoundedRectangle{rect,..} if rect.origin.x>=0. && rect.origin.x<2.)));
    chart
        .prepare(Size::new(640., 480.), 1.25, Instant::now())
        .unwrap();
    let width = chart.frame().layout.right_axis.size.width;
    chart
        .prepare(Size::new(640., 480.), 1.25, Instant::now())
        .unwrap();
    assert_eq!(width, chart.frame().layout.right_axis.size.width);
    let renderer = renderer();
    calls.borrow_mut().clear();
    chart
        .frame()
        .draw(&mut Frame::new(&renderer, Size::new(640., 480.)));
    assert_eq!(
        calls
            .borrow()
            .iter()
            .map(|s| s.split(':').take(2).collect::<Vec<_>>().join(":"))
            .collect::<Vec<_>>(),
        ["bg:price", "fg:price", "bg:time", "fg:time"]
    );
}

#[derive(Debug)]
struct Selectable {
    layer: PrimitiveLayer,
    background: bool,
}
impl DrawingPrimitive for Selectable {
    fn prepare(&self, _: &PrimitiveContext<'_>) -> PreparedPrimitive {
        PreparedPrimitive {
            views: vec![PrimitiveView::new(
                self.layer,
                Selectable {
                    layer: self.layer,
                    background: self.background,
                },
            )],
            ..Default::default()
        }
    }
}
impl PrimitiveRenderer for Selectable {
    fn draw(&self, _: &mut Frame) {}
    fn hit_test(&self, point: Point) -> Option<feather_charts::api::PrimitiveHitCandidate> {
        (point.x >= 20. && point.x <= 1000. && point.y >= 20. && point.y <= 1000.).then(|| {
            let mut h = feather_charts::api::PrimitiveHitCandidate::new("same external ID");
            h.layer = self.layer;
            h.is_background = self.background;
            h.cursor = Some(iced::mouse::Interaction::Pointer);
            h
        })
    }
}
#[test]
fn pane_hit_has_owned_identity_and_removal_reconciles_once() {
    use feather_charts::api::{ChartEvent, ChartMessage};
    let mut chart = Chart::default();
    let s = series(&mut chart, 0);
    data(&mut chart, s);
    let id = chart
        .add_pane_primitive(
            0,
            Selectable {
                layer: PrimitiveLayer::Top,
                background: false,
            },
        )
        .unwrap();
    chart
        .prepare(Size::new(640., 480.), 1.25, Instant::now())
        .unwrap();
    let point = Point::new(50., 50.);
    let h = chart.primitive_hit(0, point).unwrap();
    assert_eq!(h.primitive, id);
    assert!(matches!(h.owner, PrimitiveOwner::Pane(_)));
    assert_eq!(chart.hit_test(0, point), None);
    let events = chart
        .update(ChartMessage::Click {
            pane: 0,
            position: point,
        })
        .unwrap();
    assert!(events.iter().any(
        |e| matches!(e,ChartEvent::Click(e) if e.primitive.as_ref()==Some(&h) && e.hit.is_none())
    ));
    chart.remove_primitive(id).unwrap();
    chart
        .prepare(Size::new(640., 480.), 1.25, Instant::now())
        .unwrap();
    assert!(chart.is_animating());
    let events = chart.update(ChartMessage::Tick(Instant::now())).unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e,ChartEvent::CrosshairMoved(e) if e.primitive.is_none()))
    );
    assert!(!chart.is_animating());
}

#[test]
fn foreground_and_background_primitive_hits_follow_source_order() {
    use feather_charts::renderers::line_renderer::PathOperation;
    let mut chart = Chart::default();
    let s = series(&mut chart, 0);
    data(&mut chart, s);
    chart
        .prepare(Size::new(640., 480.), 1., Instant::now())
        .unwrap();
    let p = chart
        .frame()
        .plot
        .lines
        .iter()
        .flat_map(|l| l.path.iter())
        .find_map(|p| match p {
            PathOperation::MoveTo(p) | PathOperation::LineTo(p) if p.x >= 20. && p.y >= 20. => {
                Some(Point::new(p.x as f32, p.y as f32))
            }
            _ => None,
        })
        .unwrap();
    let id = chart
        .add_series_primitive(
            s,
            Selectable {
                layer: PrimitiveLayer::Normal,
                background: true,
            },
        )
        .unwrap();
    chart
        .prepare(Size::new(640., 480.), 1., Instant::now())
        .unwrap();
    assert_eq!(chart.hit_test(0, p).unwrap().series, s);
    assert!(chart.primitive_hit(0, p).is_none());
    chart
        .replace_primitive(
            id,
            Selectable {
                layer: PrimitiveLayer::Normal,
                background: false,
            },
        )
        .unwrap();
    chart
        .prepare(Size::new(640., 480.), 1., Instant::now())
        .unwrap();
    assert_eq!(chart.primitive_hit(0, p).unwrap().primitive, id);
    chart
        .replace_primitive(
            id,
            Selectable {
                layer: PrimitiveLayer::Bottom,
                background: false,
            },
        )
        .unwrap();
    chart
        .prepare(Size::new(640., 480.), 1., Instant::now())
        .unwrap();
    assert_eq!(chart.hit_test(0, p).unwrap().series, s);
    // A top pane attachment overrides every built-in source.
    let pane = chart
        .add_pane_primitive(
            0,
            Selectable {
                layer: PrimitiveLayer::Top,
                background: true,
            },
        )
        .unwrap();
    chart
        .prepare(Size::new(640., 480.), 1., Instant::now())
        .unwrap();
    assert_eq!(chart.primitive_hit(0, p).unwrap().primitive, pane);
}
#[test]
fn primitive_cursor_is_available_before_draw_and_resets_outside_the_plot() {
    use iced::widget::canvas::Program;
    let mut chart = Chart::default();
    let s = series(&mut chart, 0);
    data(&mut chart, s);
    chart
        .add_series_primitive(
            s,
            Selectable {
                layer: PrimitiveLayer::Top,
                background: false,
            },
        )
        .unwrap();
    chart
        .prepare(Size::new(640., 480.), 1., Instant::now())
        .unwrap();
    let program = chart.program(std::convert::identity);
    let bounds = iced::Rectangle::with_size(Size::new(640., 480.));
    let origin = chart.frame().layout.plot.origin;
    let cursor =
        iced::mouse::Cursor::Available(Point::new(origin.x as f32 + 50., origin.y as f32 + 50.));
    assert_eq!(
        program.mouse_interaction(&Default::default(), bounds, cursor),
        iced::mouse::Interaction::Pointer
    );
    assert_eq!(
        program.mouse_interaction(
            &Default::default(),
            bounds,
            iced::mouse::Cursor::Unavailable
        ),
        iced::mouse::Interaction::default()
    );
}
#[test]
fn invalid_extension_output_cannot_poison_autoscale_or_create_hits() {
    let mut chart = Chart::default();
    let s = series(&mut chart, 0);
    data(&mut chart, s);
    chart
        .add_series_primitive(
            s,
            ScaleRegion {
                min: f64::NAN,
                max: 200.,
                margin: -1.,
            },
        )
        .unwrap();
    chart
        .prepare(Size::new(640., 480.), 1., Instant::now())
        .unwrap();
    let raw = chart
        .model()
        .series(s)
        .unwrap()
        .autoscale_info(TimePointIndex::new(0.), TimePointIndex::new(9.))
        .unwrap()
        .to_raw();
    assert_eq!(
        raw.price_range.unwrap(),
        PriceRange {
            min_value: 100.,
            max_value: 109.
        }
    );
    assert!(chart.primitive_hit(0, Point::new(f32::NAN, 0.)).is_none());
    assert!(chart.primitive_hit(0, Point::new(-1., 50.)).is_none());
}

#[test]
fn native_primitive_click_drag_update_and_focus_loss_follow_navigation_contract() {
    use feather_charts::api::{ChartEvent, ChartState};
    use iced::{
        Rectangle, mouse,
        widget::canvas::{Event, Program},
    };
    let mut chart = Chart::default();
    let s = series(&mut chart, 0);
    data(&mut chart, s);
    let id = chart
        .add_series_primitive(
            s,
            Selectable {
                layer: PrimitiveLayer::Top,
                background: false,
            },
        )
        .unwrap();
    let size = Size::new(640., 480.);
    chart.prepare(size, 1.5, Instant::now()).unwrap();
    let origin = chart.frame().layout.plot.origin;
    let p = Point::new(origin.x as f32 + 50., origin.y as f32 + 50.);
    let mut state = ChartState::default();
    let mut events = vec![];
    for event in [
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
    ] {
        let message = chart
            .program(std::convert::identity)
            .update(
                &mut state,
                &Event::Mouse(event),
                Rectangle::with_size(size),
                mouse::Cursor::Available(p),
            )
            .and_then(|a| a.into_inner().0);
        if let Some(message) = message {
            events.extend(chart.update(message).unwrap());
        }
    }
    assert!(events.iter().any(
        |e| matches!(e,ChartEvent::Click(e) if e.primitive.as_ref().is_some_and(|h|h.primitive==id))
    ));
    events.clear();
    for (event, point) in [
        (
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            p,
        ),
        (
            Event::Mouse(mouse::Event::CursorMoved {
                position: Point::new(p.x + 30., p.y + 20.),
            }),
            Point::new(p.x + 30., p.y + 20.),
        ),
    ] {
        let message = chart
            .program(std::convert::identity)
            .update(
                &mut state,
                &event,
                Rectangle::with_size(size),
                mouse::Cursor::Available(point),
            )
            .and_then(|a| a.into_inner().0);
        if let Some(message) = message {
            events.extend(chart.update(message).unwrap());
        }
    }
    chart
        .replace_primitive(
            id,
            Selectable {
                layer: PrimitiveLayer::Top,
                background: false,
            },
        )
        .unwrap();
    chart.prepare(size, 1.5, Instant::now()).unwrap();
    assert_eq!(
        chart.program(std::convert::identity).mouse_interaction(
            &state,
            Rectangle::with_size(size),
            mouse::Cursor::Available(p)
        ),
        mouse::Interaction::Grabbing
    );
    let message = chart
        .program(std::convert::identity)
        .update(
            &mut state,
            &Event::Window(iced::window::Event::Unfocused),
            Rectangle::with_size(size),
            mouse::Cursor::Unavailable,
        )
        .and_then(|a| a.into_inner().0)
        .unwrap();
    events.extend(chart.update(message).unwrap());
    assert!(!events.iter().any(|e| matches!(e, ChartEvent::Click(_))));
    assert_eq!(
        chart.program(std::convert::identity).mouse_interaction(
            &state,
            Rectangle::with_size(size),
            mouse::Cursor::Unavailable
        ),
        mouse::Interaction::default()
    );
    assert_eq!(
        chart.program(std::convert::identity).mouse_interaction(
            &state,
            Rectangle::with_size(size),
            mouse::Cursor::Available(p)
        ),
        mouse::Interaction::default()
    );
    let _ = chart.program(std::convert::identity).update(
        &mut state,
        &Event::Window(iced::window::Event::Focused),
        Rectangle::with_size(size),
        mouse::Cursor::Available(p),
    );
    assert_eq!(
        chart.program(std::convert::identity).mouse_interaction(
            &state,
            Rectangle::with_size(size),
            mouse::Cursor::Available(p)
        ),
        mouse::Interaction::Pointer
    );
    assert!(!chart.is_animating());
}

#[test]
fn primitive_coordinates_follow_scale_modes_and_viewport_and_clear_on_empty_data() {
    use feather_charts::model::{
        axis_snapshots::PriceAxisSide,
        price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
    };
    #[derive(Debug)]
    struct Coordinates(Rc<RefCell<Vec<Projection>>>);
    type Projection = (Size, Option<f64>, Option<f64>);
    impl DrawingPrimitive for Coordinates {
        fn prepare(&self, c: &PrimitiveContext<'_>) -> PreparedPrimitive {
            assert!(c.logical_to_coordinate(f64::MAX).is_none());
            if !c.model.time_scale().is_empty() {
                assert_eq!(c.logical_to_coordinate(5.5), Some(0.));
            }
            if let PrimitiveOwner::Series(id) = c.owner
                && c.model.series(id).unwrap().bars().rows().len() == 10
            {
                assert_eq!(
                    c.time_to_coordinate(&Time::from(UtcTimestamp::new(1050.))),
                    c.logical_to_coordinate(5.)
                );
                assert!(
                    c.time_to_coordinate(&Time::from(UtcTimestamp::new(9999.)))
                        .is_none()
                );
            }
            self.0.borrow_mut().push((
                c.size,
                c.logical_to_coordinate(5.),
                c.price_to_coordinate(105.),
            ));
            Default::default()
        }
    }
    let mut chart = Chart::default();
    let s = series(&mut chart, 0);
    data(&mut chart, s);
    let coords = Rc::new(RefCell::new(vec![]));
    let id = chart
        .add_series_primitive(s, Coordinates(coords.clone()))
        .unwrap();
    for mode in [
        PriceScaleMode::Normal,
        PriceScaleMode::Logarithmic,
        PriceScaleMode::Percentage,
        PriceScaleMode::IndexedTo100,
    ] {
        for invert in [false, true] {
            chart
                .apply_price_axis_options(
                    0,
                    PriceAxisSide::Right,
                    PriceScaleOptionsPatch {
                        mode: Some(mode),
                        invert_scale: Some(invert),
                        ..Default::default()
                    },
                )
                .unwrap();
            for size in [Size::new(640., 480.), Size::new(320., 240.)] {
                chart.prepare(size, 1.25, Instant::now()).unwrap();
                let (_, x, y) = *coords.borrow().last().unwrap();
                assert!(x.unwrap().is_finite());
                assert!(y.unwrap().is_finite());
            }
        }
    }
    chart.set_data(s, vec![]).unwrap();
    chart
        .prepare(Size::new(640., 480.), 2., Instant::now())
        .unwrap();
    let (_, x, y) = *coords.borrow().last().unwrap();
    assert!(x.is_none() && y.is_none());
    chart
        .set_data(
            s,
            vec![BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(
                feather_charts::model::data_consumer::WhitespaceData {
                    time: Time::from(UtcTimestamp::new(1000.)),
                    custom_values: None,
                },
            ))],
        )
        .unwrap();
    chart
        .prepare(Size::new(640., 480.), 1., Instant::now())
        .unwrap();
    let (_, x, y) = *coords.borrow().last().unwrap();
    assert!(x.is_some() && y.is_none());
    data(&mut chart, s);
    chart
        .prepare(Size::new(640., 480.), 2., Instant::now())
        .unwrap();
    assert!(coords.borrow().last().unwrap().2.is_some());
    assert!(chart.primitive_owner(id).is_ok());
    for (scale_id, position) in [
        ("left", PriceScalePosition::Left),
        ("extra", PriceScalePosition::Overlay("extra".into())),
    ] {
        let owner = chart
            .add_series(SeriesOptionsMap::Line(Default::default()), 0, position)
            .unwrap();
        data(&mut chart, owner);
        let own_coords = Rc::new(RefCell::new(vec![]));
        chart
            .add_series_primitive(owner, Coordinates(own_coords.clone()))
            .unwrap();
        chart
            .prepare(Size::new(640., 480.), 1.5, Instant::now())
            .unwrap();
        let expected = chart.model().panes()[0]
            .price_scale_by_id(scale_id)
            .unwrap()
            .price_to_coordinate(105., 100.)
            .value();
        assert_eq!(own_coords.borrow().last().unwrap().2, Some(expected));
    }
}
