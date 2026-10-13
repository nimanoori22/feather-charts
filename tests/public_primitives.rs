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
    let mut chart = Chart::default();
    let s = series(&mut chart, 0);
    data(&mut chart, s);
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
