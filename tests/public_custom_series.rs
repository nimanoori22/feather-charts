use feather_charts::{
    api::Chart,
    model::{
        bar::BarPrice,
        data_consumer::TimedData,
        horz_scale_behavior_time::types::{Time, TimePoint, UtcTimestamp},
        icustom_series::*,
        pane::PriceScalePosition,
        series_options::CustomSeriesOptions,
        time_data::TimePointIndex,
    },
};
use iced::{Size, widget::canvas::Frame};
use std::{cell::RefCell, rc::Rc, time::Instant};

struct Datum {
    time: Time,
    low: f64,
    high: f64,
    median: f64,
}
thread_local! { static COPIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
impl Clone for Datum {
    fn clone(&self) -> Self {
        COPIES.with(|copies| copies.set(copies.get() + 1));
        Self {
            time: self.time.clone(),
            low: self.low,
            high: self.high,
            median: self.median,
        }
    }
}
impl TimedData for Datum {
    type Item = Time;
    fn time(&self) -> &Time {
        &self.time
    }
    fn time_mut(&mut self) -> &mut Time {
        &mut self.time
    }
}
impl CustomData for Datum {}
#[derive(Clone)]
struct Renderer {
    data: Option<PaneRendererCustomData<Datum>>,
    draws: Rc<RefCell<Vec<f64>>>,
}
impl CustomSeriesPaneRenderer<Frame> for Renderer {
    type HitTestData = ();
    fn draw(
        &self,
        _frame: &mut Frame,
        convert: &PriceToCoordinateConverter<'_>,
        _: bool,
        _: Option<&()>,
    ) {
        if let Some(data) = &self.data {
            for bar in &data.bars {
                assert!(convert(BarPrice::new(bar.original_data.median)).is_some());
                self.draws.borrow_mut().push(bar.original_data.median);
            }
        }
    }
}
struct View {
    renderer: Renderer,
    destroyed: Rc<RefCell<usize>>,
}
impl CustomSeriesPaneView<Frame> for View {
    type Item = Time;
    type InternalTime = TimePoint;
    type Data = Datum;
    type Metadata = ();
    type Options = ();
    type Renderer = Renderer;
    fn renderer(&self) -> &Renderer {
        &self.renderer
    }
    fn update(&mut self, data: PaneRendererCustomData<Datum>, _: &()) {
        self.renderer.data = Some(data);
    }
    fn price_value_builder(&self, data: &Datum) -> CustomSeriesPricePlotValues {
        vec![data.high.into(), data.low.into(), data.median.into()]
    }
    fn is_whitespace(&self, data: &CustomSeriesDataItem<Datum, Time>) -> bool {
        matches!(data, CustomSeriesDataItem::Whitespace(_))
    }
    fn default_options(&self) {}
    fn destroy(&mut self) {
        *self.destroyed.borrow_mut() += 1;
    }
}
fn datum(time: f64, median: f64) -> CustomSeriesDataItem<Datum, Time> {
    CustomSeriesDataItem::Data(Datum {
        time: UtcTimestamp::new(time).into(),
        low: median - 2.,
        high: median + 3.,
        median,
    })
}
#[test]
fn custom_public_lifecycle_keeps_typed_data_and_frozen_frames() {
    let draws = Rc::new(RefCell::new(vec![]));
    let destroyed = Rc::new(RefCell::new(0));
    let mut chart = Chart::default();
    let handle = chart
        .add_custom_series(
            View {
                renderer: Renderer {
                    data: None,
                    draws: draws.clone(),
                },
                destroyed: destroyed.clone(),
            },
            CustomSeriesOptions::default(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_custom_data(&handle, vec![datum(1., 10.), datum(2., 20.)])
        .unwrap();
    chart.fit_content();
    let old = chart
        .prepare_snapshot(Size::new(640., 480.), 1.25, Instant::now())
        .unwrap();
    assert_eq!(handle.data_at(TimePointIndex::new(1.)).unwrap().median, 20.);
    assert!(handle.data_at(TimePointIndex::new(10.)).is_none());
    chart
        .update_custom_data(&handle, datum(2., 30.), false)
        .unwrap();
    chart
        .update_custom_data(&handle, datum(3., 40.), false)
        .unwrap();
    let new = chart
        .prepare_snapshot(Size::new(640., 480.), 1.25, Instant::now())
        .unwrap();
    chart.remove_series(handle.id()).unwrap();
    assert_eq!(*destroyed.borrow(), 1);
    assert!(
        chart
            .update_custom_data(&handle, datum(3., 40.), false)
            .is_err()
    );
    assert_eq!(handle.data_at(1.0.into()).unwrap().median, 30.);
    use iced::advanced::renderer::Headless;
    let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some("tiny-skia"),
    ))
    .unwrap();
    old.panes[0]
        .plot
        .draw(&mut Frame::new(&renderer, Size::new(640., 480.)));
    new.panes[0]
        .plot
        .draw(&mut Frame::new(&renderer, Size::new(640., 480.)));
    assert_eq!(*draws.borrow(), vec![10., 20., 10., 30., 40.]);
    drop(chart);
    assert_eq!(*destroyed.borrow(), 1);
}

#[test]
fn custom_price_lines_and_markers_share_the_actual_owner() {
    use feather_charts::api::{
        MarkerPosition, MarkerShape, PriceLineOptions, SeriesMarker, SeriesMarkersOptions,
    };
    let mut chart = Chart::default();
    let handle = chart
        .add_custom_series(
            View {
                renderer: Renderer {
                    data: None,
                    draws: Rc::new(RefCell::new(vec![])),
                },
                destroyed: Rc::new(RefCell::new(0)),
            },
            CustomSeriesOptions::default(),
            0,
            PriceScalePosition::Left,
        )
        .unwrap();
    chart
        .set_custom_data(&handle, vec![datum(1., 10.), datum(2., 20.)])
        .unwrap();
    chart.fit_content();
    let line = PriceLineOptions::new(12.);
    let price_line = chart.add_price_line(handle.id(), line).unwrap();
    let markers = chart
        .add_series_markers(
            handle.id(),
            vec![SeriesMarker::new(
                UtcTimestamp::new(2.).into(),
                MarkerPosition::AboveBar,
                MarkerShape::Circle,
                "red",
            )],
            SeriesMarkersOptions::default(),
        )
        .unwrap();
    #[derive(Debug)]
    struct Probe(Rc<std::cell::Cell<usize>>);
    impl feather_charts::api::DrawingPrimitive for Probe {
        fn prepare(
            &self,
            context: &feather_charts::api::PrimitiveContext<'_>,
        ) -> feather_charts::api::PreparedPrimitive {
            let Some(y) = context.price_to_coordinate(12.) else {
                return feather_charts::api::PreparedPrimitive::default();
            };
            self.0.set(self.0.get() + 1);
            assert!((context.coordinate_to_price(y).unwrap() - 12.).abs() < 1e-8);
            let frozen = context.coordinate_transform().unwrap();
            assert!(
                (frozen
                    .coordinate_to_price(frozen.price_to_coordinate(12.).unwrap())
                    .unwrap()
                    - 12.)
                    .abs()
                    < 1e-8
            );
            feather_charts::api::PreparedPrimitive::default()
        }
    }
    let converted = Rc::new(std::cell::Cell::new(0));
    chart
        .add_series_primitive(handle.id(), Probe(converted.clone()))
        .unwrap();
    let frame = chart
        .prepare_snapshot(Size::new(640., 480.), 1.5, Instant::now())
        .unwrap();
    assert!(
        frame.panes[0]
            .plot
            .hit_price_lines
            .iter()
            .any(|line| line.kind == feather_charts::api::PriceLineKind::Custom(price_line))
    );
    assert_eq!(frame.panes[0].plot.annotations[0].id, markers);
    assert_eq!(frame.panes[0].plot.annotations[0].series, handle.id());
    assert!(
        converted.get() > 0,
        "settled custom primitive must receive owner conversions"
    );
    assert!(
        !frame.panes[0]
            .plot
            .axes
            .as_ref()
            .unwrap()
            .left
            .labels
            .is_empty()
    );
}

#[test]
fn custom_errors_and_whitespace_keep_the_typed_owner_intact() {
    let mut chart = Chart::default();
    let handle = chart
        .add_custom_series(
            View {
                renderer: Renderer {
                    data: None,
                    draws: Rc::new(RefCell::new(vec![])),
                },
                destroyed: Rc::new(RefCell::new(0)),
            },
            CustomSeriesOptions::default(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_custom_data(&handle, vec![datum(1., 10.), datum(2., 20.)])
        .unwrap();
    assert!(
        chart
            .set_custom_data(&handle, vec![datum(3., 30.), datum(3., 31.)])
            .is_err()
    );
    assert!(
        chart
            .update_custom_data(&handle, datum(2., f64::NAN), false)
            .is_err()
    );
    assert_eq!(handle.data_at(1.0.into()).unwrap().median, 20.);
    let mut foreign = Chart::default();
    assert!(
        foreign
            .set_custom_data(&handle, vec![datum(4., 40.)])
            .is_err()
    );
    chart
        .update_custom_data(
            &handle,
            CustomSeriesDataItem::Whitespace(CustomSeriesWhitespaceData {
                time: UtcTimestamp::new(2.).into(),
                custom_values: None,
            }),
            false,
        )
        .unwrap();
    assert!(handle.data_at(1.0.into()).is_none());
    assert_eq!(handle.data_at(0.0.into()).unwrap().median, 10.);
    chart
        .update_custom_data(&handle, datum(2., 25.), false)
        .unwrap();
    assert_eq!(handle.data_at(1.0.into()).unwrap().median, 25.);
    chart
        .update_custom_data(&handle, datum(1., 15.), true)
        .unwrap();
    assert_eq!(handle.data_at(0.0.into()).unwrap().median, 15.);
    chart.set_custom_data(&handle, vec![]).unwrap();
    assert!(handle.data_at(0.0.into()).is_none());
    chart
        .set_custom_data(&handle, vec![datum(5., 50.)])
        .unwrap();
    assert_eq!(handle.data_at(0.0.into()).unwrap().median, 50.);
}

#[derive(Clone)]
struct HitRenderer {
    base: Renderer,
    calls: Rc<RefCell<usize>>,
    drawn_hit: Rc<RefCell<Vec<usize>>>,
}
impl CustomSeriesPaneRenderer<Frame> for HitRenderer {
    type HitTestData = usize;
    fn draw(
        &self,
        frame: &mut Frame,
        converter: &PriceToCoordinateConverter<'_>,
        hovered: bool,
        payload: Option<&usize>,
    ) {
        if hovered {
            self.drawn_hit
                .borrow_mut()
                .push(*payload.expect("selected custom payload must survive arbitration"));
        }
        self.base.draw(frame, converter, hovered, None);
    }
    fn hit_test(
        &self,
        _: feather_charts::model::coordinate::Coordinate,
        _: feather_charts::model::coordinate::Coordinate,
        _: &PriceToCoordinateConverter<'_>,
    ) -> Option<CustomSeriesHitTestResult<usize>> {
        *self.calls.borrow_mut() += 1;
        Some(CustomSeriesHitTestResult {
            distance: 0.,
            object_id: Some("median".into()),
            hit_type: CustomSeriesHitTestType::Point,
            cursor_style: Some("pointer".into()),
            hit_test_data: Some(42),
        })
    }
}
struct HitView {
    renderer: HitRenderer,
}
impl CustomSeriesPaneView<Frame> for HitView {
    type Item = Time;
    type InternalTime = TimePoint;
    type Data = Datum;
    type Metadata = ();
    type Options = ();
    type Renderer = HitRenderer;
    fn renderer(&self) -> &HitRenderer {
        &self.renderer
    }
    fn update(&mut self, data: PaneRendererCustomData<Datum>, _: &()) {
        self.renderer.base.data = Some(data);
    }
    fn price_value_builder(&self, data: &Datum) -> CustomSeriesPricePlotValues {
        vec![data.high.into(), data.low.into(), data.median.into()]
    }
    fn is_whitespace(&self, data: &CustomSeriesDataItem<Datum, Time>) -> bool {
        matches!(data, CustomSeriesDataItem::Whitespace(_))
    }
    fn default_options(&self) {}
}
#[test]
fn custom_pointer_payload_and_selected_hit_are_owned_and_typed() {
    use feather_charts::api::{ChartEvent, ChartMessage};
    let calls = Rc::new(RefCell::new(0));
    let drawn_hit = Rc::new(RefCell::new(vec![]));
    let mut chart = Chart::default();
    let handle = chart
        .add_custom_series(
            HitView {
                renderer: HitRenderer {
                    base: Renderer {
                        data: None,
                        draws: Rc::new(RefCell::new(vec![])),
                    },
                    calls: calls.clone(),
                    drawn_hit: drawn_hit.clone(),
                },
            },
            CustomSeriesOptions::default(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_custom_data(&handle, vec![datum(1., 10.), datum(2., 20.)])
        .unwrap();
    chart
        .prepare(Size::new(640., 480.), 1., Instant::now())
        .unwrap();
    chart.fit_content();
    let snapshot = chart
        .prepare_snapshot(Size::new(640., 480.), 1., Instant::now())
        .unwrap();
    let x = chart
        .model()
        .time_scale()
        .index_to_coordinate(1.0.into())
        .value() as f32;
    let events = chart
        .update(ChartMessage::Click {
            pane: 0,
            position: iced::Point::new(x, snapshot.panes[0].plot.size.height / 2.),
        })
        .unwrap();
    let pointer = events
        .into_iter()
        .find_map(|e| {
            if let ChartEvent::Click(p) = e {
                Some(p)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(pointer.custom_data(&handle).unwrap().median, 20.);
    assert_eq!(pointer.custom_hit_data(&handle), Some(&42));
    *calls.borrow_mut() = 0;
    chart
        .prepare(Size::new(640., 480.), 1., Instant::now())
        .unwrap();
    assert_eq!(
        *calls.borrow(),
        1,
        "prepare must arbitrate custom hover only once"
    );
    let frame = chart.frame().clone();
    chart.remove_series(handle.id()).unwrap();
    let count = *calls.borrow();
    use iced::advanced::renderer::Headless;
    let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some("tiny-skia"),
    ))
    .unwrap();
    frame.draw(&mut Frame::new(&renderer, Size::new(640., 480.)));
    assert_eq!(
        *calls.borrow(),
        count,
        "drawing must not rerun hit callbacks"
    );
    assert_eq!(*drawn_hit.borrow(), vec![42]);
    assert_eq!(pointer.custom_data(&handle).unwrap().median, 20.);
}

#[path = "../examples/custom_series/box_whisker.rs"]
mod box_whisker;

#[test]
fn different_custom_types_share_reindexed_time_without_sharing_payloads() {
    let mut chart = Chart::default();
    let a = chart
        .add_custom_series(
            View {
                renderer: Renderer {
                    data: None,
                    draws: Rc::new(RefCell::new(vec![])),
                },
                destroyed: Rc::new(RefCell::new(0)),
            },
            CustomSeriesOptions::default(),
            0,
            PriceScalePosition::Left,
        )
        .unwrap();
    let pane = chart.add_pane();
    let b = chart
        .add_custom_series(
            box_whisker::View::default(),
            CustomSeriesOptions::default(),
            pane,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_custom_data(&a, vec![datum(1., 10.), datum(3., 30.)])
        .unwrap();
    let CustomSeriesDataItem::Data(mut box_data) = box_whisker::sample(0) else {
        unreachable!()
    };
    box_data.time = UtcTimestamp::new(2.).into();
    chart
        .set_custom_data(&b, vec![CustomSeriesDataItem::Data(box_data)])
        .unwrap();
    assert!(a.data_at(1.0.into()).is_none());
    assert_eq!(a.data_at(2.0.into()).unwrap().median, 30.);
    assert_eq!(b.data_at(1.0.into()).unwrap().quartiles[2], 100.);
    chart
        .prepare(Size::new(640., 480.), 1.25, Instant::now())
        .unwrap();
    chart.fit_content();
    let frame = chart
        .prepare_snapshot(Size::new(640., 480.), 1.25, Instant::now())
        .unwrap();
    assert_eq!(frame.panes.len(), 2);
    assert!(frame.panes.iter().all(|pane| pane.plot.custom.is_some()));
    chart.update_custom_data(&a, datum(4., 40.), false).unwrap();
    let CustomSeriesDataItem::Data(mut tail) = box_whisker::sample(1) else {
        unreachable!()
    };
    tail.time = UtcTimestamp::new(5.).into();
    chart
        .update_custom_data(&b, CustomSeriesDataItem::Data(tail), false)
        .unwrap();
    assert_eq!(a.data_at(3.0.into()).unwrap().median, 40.);
    assert_eq!(
        b.data_at(4.0.into()).unwrap().time,
        UtcTimestamp::new(5.).into()
    );
    chart.remove_series(b.id()).unwrap();
    assert_eq!(a.data_at(1.0.into()).unwrap().median, 30.);
    assert_eq!(a.data_at(2.0.into()).unwrap().median, 40.);
    chart.update_custom_data(&a, datum(6., 60.), false).unwrap();
    assert_eq!(a.data_at(3.0.into()).unwrap().median, 60.);
}

#[test]
fn custom_preparation_and_existing_time_updates_copy_only_visible_payloads() {
    use feather_charts::model::time_data::LogicalRange;
    let mut chart = Chart::default();
    let handle = chart
        .add_custom_series(
            View {
                renderer: Renderer {
                    data: None,
                    draws: Rc::new(RefCell::new(vec![])),
                },
                destroyed: Rc::new(RefCell::new(0)),
            },
            CustomSeriesOptions::default(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_custom_data(
            &handle,
            (0..100_000)
                .map(|index| datum(index as f64 + 1., 100.))
                .collect(),
        )
        .unwrap();
    chart
        .prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    chart.set_logical_range(LogicalRange {
        from: 99_910.0.into(),
        to: 99_990.0.into(),
    });
    COPIES.with(|copies| copies.set(0));
    chart
        .prepare_snapshot(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let prepared = COPIES.with(|copies| copies.get());
    // This test renderer clones its Vec once when frozen: at most two bounded copies.
    assert!(
        prepared <= 170,
        "preparation cloned {prepared} custom payloads for 81 visible slots"
    );
    COPIES.with(|copies| copies.set(0));
    chart
        .update_custom_data(&handle, datum(100_000., 101.), false)
        .unwrap();
    let updated = COPIES.with(|copies| copies.get());
    assert!(
        updated <= 4,
        "existing-time update copied {updated} payloads"
    );
    COPIES.with(|copies| copies.set(0));
    chart
        .update_custom_data(&handle, datum(100_001., 102.), false)
        .unwrap();
    let appended = COPIES.with(|copies| copies.get());
    assert!(
        appended <= 4,
        "tail append copied {appended} custom payloads"
    );
}

type ReductionRecords = Rc<RefCell<Vec<(f64, Vec<f64>)>>>;
type InspectCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;
struct ReducingView {
    inner: View,
    seen: ReductionRecords,
    reduce: bool,
    inspect: Option<Box<dyn Fn()>>,
}
impl CustomSeriesPaneView<Frame> for ReducingView {
    type Item = Time;
    type InternalTime = TimePoint;
    type Data = Datum;
    type Metadata = ();
    type Options = ();
    type Renderer = Renderer;
    fn renderer(&self) -> &Renderer {
        self.inner.renderer()
    }
    fn update(&mut self, data: PaneRendererCustomData<Datum>, _: &()) {
        self.seen.borrow_mut().push((
            data.conflation_factor,
            data.bars.iter().map(|b| b.original_data.median).collect(),
        ));
        self.inner.update(data, &());
    }
    fn price_value_builder(&self, data: &Datum) -> CustomSeriesPricePlotValues {
        self.inner.price_value_builder(data)
    }
    fn is_whitespace(&self, data: &CustomSeriesDataItem<Datum, Time>) -> bool {
        self.inner.is_whitespace(data)
    }
    fn default_options(&self) {}
    fn conflate(
        &self,
        a: &CustomConflationContext<Datum, Time, TimePoint>,
        b: &CustomConflationContext<Datum, Time, TimePoint>,
    ) -> Option<Datum> {
        if let Some(inspect) = &self.inspect {
            inspect();
        }
        self.reduce.then(|| Datum {
            time: a.original_time.clone(),
            low: a.data.low.min(b.data.low),
            high: a.data.high.max(b.data.high),
            median: a.data.median * 10. + b.data.median,
        })
    }
}
#[test]
fn custom_conflation_preserves_reduction_order_and_no_reducer_passthrough() {
    use feather_charts::model::time_scale_options::HorzScaleOptionsPatch;
    for reduce in [false, true] {
        let seen = Rc::new(RefCell::new(vec![]));
        let mut chart = Chart::default();
        let handle = chart
            .add_custom_series(
                ReducingView {
                    inner: View {
                        renderer: Renderer {
                            data: None,
                            draws: Rc::new(RefCell::new(vec![])),
                        },
                        destroyed: Rc::new(RefCell::new(0)),
                    },
                    seen: seen.clone(),
                    reduce,
                    inspect: None,
                },
                CustomSeriesOptions::default(),
                0,
                PriceScalePosition::Right,
            )
            .unwrap();
        chart
            .set_custom_data(
                &handle,
                (0..7)
                    .map(|i| datum(1000. + i as f64 * 60., i as f64 + 1.))
                    .collect(),
            )
            .unwrap();
        chart.apply_time_scale_options(HorzScaleOptionsPatch {
            enable_conflation: Some(true),
            min_bar_spacing: Some(0.001),
            bar_spacing: Some(0.125),
            ..Default::default()
        });
        chart
            .prepare(Size::new(800., 500.), 1., Instant::now())
            .unwrap();
        let records = seen.borrow();
        let (level, values) = records.last().unwrap();
        assert_eq!(*level, 8.);
        assert_eq!(
            *values,
            if reduce {
                vec![2107.]
            } else {
                (1..=7).map(f64::from).collect()
            }
        );
        assert_eq!(handle.data_at(0.0.into()).unwrap().median, 1.);
        drop(records);
        chart
            .update_custom_data(&handle, datum(1360., 9.), false)
            .unwrap();
        chart
            .prepare(Size::new(800., 500.), 1., Instant::now())
            .unwrap();
        assert_eq!(handle.data_at(6.0.into()).unwrap().median, 9.);
        let values = seen.borrow().last().unwrap().1.clone();
        assert_eq!(
            values,
            if reduce {
                vec![2109.]
            } else {
                vec![1., 2., 3., 4., 5., 6., 9.]
            }
        );
        if reduce {
            println!(
                "CUSTOM_CONFLATION {{\"initial\":2107,\"latest\":{}}}",
                values[0]
            );
        }
    }
}

#[test]
fn custom_warmup_is_bounded_releases_model_borrows_and_drops_with_owner() {
    use feather_charts::model::time_scale_options::HorzScaleOptionsPatch;
    use std::cell::Cell;
    let calls = Rc::new(Cell::new(0));
    let inspect: InspectCallback = Rc::new(RefCell::new(None));
    let mut chart = Chart::default();
    let handle = chart
        .add_custom_series(
            ReducingView {
                inner: View {
                    renderer: Renderer {
                        data: None,
                        draws: Rc::new(RefCell::new(vec![])),
                    },
                    destroyed: Rc::new(RefCell::new(0)),
                },
                seen: Rc::new(RefCell::new(vec![])),
                reduce: true,
                inspect: Some(Box::new({
                    let calls = calls.clone();
                    let inspect = inspect.clone();
                    move || {
                        calls.set(calls.get() + 1);
                        if let Some(read) = &*inspect.borrow() {
                            read();
                        }
                    }
                })),
            },
            CustomSeriesOptions::default(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    *inspect.borrow_mut() = Some(Box::new({
        let handle = handle.clone();
        move || {
            assert!(handle.data_at(0.0.into()).is_some());
        }
    }));
    chart
        .set_custom_data(
            &handle,
            (0..2048)
                .map(|i| datum(1000. + i as f64 * 60., 1.))
                .collect(),
        )
        .unwrap();
    chart.apply_time_scale_options(HorzScaleOptionsPatch {
        enable_conflation: Some(true),
        precompute_conflation_on_init: Some(true),
        min_bar_spacing: Some(0.001),
        ..Default::default()
    });
    chart
        .prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    assert_eq!(calls.get(), 128);
    assert!(chart.is_animating());
    chart
        .update_custom_data(&handle, datum(1000. + 2047. * 60., 2.), false)
        .unwrap();
    chart
        .prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    assert_eq!(calls.get(), 256);
    chart.remove_series(handle.id()).unwrap();
    let before = calls.get();
    chart
        .prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    assert_eq!(calls.get(), before);
}

#[test]
fn box_example_reducer_keeps_last_box_and_combined_whiskers() {
    let view = box_whisker::View::default();
    let context = |i: usize| {
        let CustomSeriesDataItem::Data(data) = box_whisker::sample(i) else {
            unreachable!()
        };
        CustomConflationContext {
            original_time: data.time.clone(),
            time: TimePoint {
                timestamp: UtcTimestamp::new(1_700_000_000. + i as f64 * 60.),
                business_day: None,
            },
            index: (i as f64).into(),
            price_values: view.price_value_builder(&data),
            data,
        }
    };
    let a = context(0);
    let b = context(10);
    let merged = view.conflate(&a, &b).unwrap();
    assert_eq!(
        merged.quartiles[0],
        a.data.quartiles[0].min(b.data.quartiles[0])
    );
    assert_eq!(
        merged.quartiles[4],
        a.data.quartiles[4].max(b.data.quartiles[4])
    );
    assert_eq!(&merged.quartiles[1..4], &b.data.quartiles[1..4]);
    assert_eq!(merged.time, a.original_time);
}
