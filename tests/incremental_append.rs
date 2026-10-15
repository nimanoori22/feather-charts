use feather_charts::model::{
    data_consumer::{
        BuiltInSeriesDataItem, HistogramData, HistogramDataItem, LineData, LineDataItem,
    },
    data_layer::{DataLayer, DataLayerError, SeriesId},
    horz_scale_behavior_time::{
        horz_scale_behavior_time::HorzScaleBehaviorTime,
        types::{Time, UtcTimestamp},
    },
    plot_data::PlotRowLike,
    series_options::SeriesType,
};
use feather_charts::{
    api::Chart,
    model::{
        data_consumer::{
            AreaData, AreaDataItem, BarData, BarDataItem, BaselineData, BaselineDataItem,
            CandlestickData, CandlestickDataItem, TimedData, WhitespaceData,
        },
        horz_scale_behavior_time::types::TimePoint,
        icustom_series::*,
        pane::PriceScalePosition,
        series_options::{CustomSeriesOptions, SeriesOptionsMap},
        time_scale_host::TimeScaleHost,
    },
};
use iced::widget::canvas::Frame;
use std::{cell::Cell, rc::Rc};

fn builtin(
    kind: SeriesType,
    time: usize,
    value: f64,
    whitespace: bool,
) -> BuiltInSeriesDataItem<Time> {
    let time = Time::from(UtcTimestamp::new(time as f64));
    let gap = WhitespaceData {
        time: time.clone(),
        custom_values: None,
    };
    match kind {
        SeriesType::Line => BuiltInSeriesDataItem::Line(if whitespace {
            LineDataItem::Whitespace(gap)
        } else {
            LineDataItem::Data(LineData {
                time,
                value,
                color: None,
                custom_values: None,
            })
        }),
        SeriesType::Histogram => BuiltInSeriesDataItem::Histogram(if whitespace {
            HistogramDataItem::Whitespace(gap)
        } else {
            HistogramDataItem::Data(HistogramData {
                time,
                value,
                color: None,
                custom_values: None,
            })
        }),
        SeriesType::Area => BuiltInSeriesDataItem::Area(if whitespace {
            AreaDataItem::Whitespace(gap)
        } else {
            AreaDataItem::Data(AreaData {
                time,
                value,
                line_color: None,
                top_color: None,
                bottom_color: None,
                custom_values: None,
            })
        }),
        SeriesType::Baseline => BuiltInSeriesDataItem::Baseline(if whitespace {
            BaselineDataItem::Whitespace(gap)
        } else {
            BaselineDataItem::Data(BaselineData {
                time,
                value,
                top_fill_color1: None,
                top_fill_color2: None,
                top_line_color: None,
                bottom_fill_color1: None,
                bottom_fill_color2: None,
                bottom_line_color: None,
                custom_values: None,
            })
        }),
        SeriesType::Bar => BuiltInSeriesDataItem::Bar(if whitespace {
            BarDataItem::Whitespace(gap)
        } else {
            BarDataItem::Data(BarData {
                time,
                open: value - 1.,
                high: value + 2.,
                low: value - 2.,
                close: value,
                color: None,
                custom_values: None,
            })
        }),
        SeriesType::Candlestick => BuiltInSeriesDataItem::Candlestick(if whitespace {
            CandlestickDataItem::Whitespace(gap)
        } else {
            CandlestickDataItem::Data(CandlestickData {
                time,
                open: value - 1.,
                high: value + 2.,
                low: value - 2.,
                close: value,
                color: None,
                border_color: None,
                wick_color: None,
                custom_values: None,
            })
        }),
        SeriesType::Custom => unreachable!(),
    }
}

fn add_builtin(chart: &mut Chart, kind: SeriesType) -> SeriesId {
    let options = match kind {
        SeriesType::Line => SeriesOptionsMap::Line(Default::default()),
        SeriesType::Histogram => SeriesOptionsMap::Histogram(Default::default()),
        SeriesType::Area => SeriesOptionsMap::Area(Default::default()),
        SeriesType::Baseline => SeriesOptionsMap::Baseline(Default::default()),
        SeriesType::Bar => SeriesOptionsMap::Bar(Default::default()),
        SeriesType::Candlestick => SeriesOptionsMap::Candlestick(Default::default()),
        SeriesType::Custom => unreachable!(),
    };
    chart
        .add_series(options, 0, PriceScalePosition::Right)
        .unwrap()
}

fn plotted(chart: &Chart, id: SeriesId) -> Vec<(f64, f64)> {
    chart
        .model()
        .series(id)
        .unwrap()
        .bars()
        .rows()
        .iter()
        .map(|row| (row.index().value(), row.values()[3]))
        .collect()
}

#[test]
fn public_append_and_whitespace_membership_work_for_every_builtin_kind() {
    for kind in [
        SeriesType::Line,
        SeriesType::Histogram,
        SeriesType::Area,
        SeriesType::Baseline,
        SeriesType::Bar,
        SeriesType::Candlestick,
    ] {
        let mut chart = Chart::default();
        let owner = add_builtin(&mut chart, kind);
        let peer = add_builtin(&mut chart, SeriesType::Line);
        chart
            .set_data(owner, vec![builtin(kind, 10, 10., false)])
            .unwrap();
        chart.set_data(peer, vec![line(10, 100., None)]).unwrap();
        chart
            .update_data(owner, builtin(kind, 20, 20., false), false)
            .unwrap();
        chart
            .update_data(owner, builtin(kind, 30, 30., true), false)
            .unwrap();
        assert_eq!(
            plotted(&chart, owner),
            vec![(0., 10.), (1., 20.)],
            "{kind:?}"
        );
        assert_eq!(
            chart.model().time_scale().base_index(),
            Some(1.0.into()),
            "{kind:?}: whitespace is not the value-bearing base"
        );
        assert_eq!(
            chart
                .model()
                .fulfilled_time_indices()
                .into_iter()
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        chart
            .update_data(peer, line(30, 300., None), false)
            .unwrap();
        assert_eq!(chart.model().time_scale().base_index(), Some(2.0.into()));
        chart
            .update_data(owner, builtin(kind, 30, 31., false), false)
            .unwrap();
        chart
            .update_data(owner, builtin(kind, 40, 40., false), false)
            .unwrap();
        chart
            .update_data(owner, builtin(kind, 20, 21., false), true)
            .unwrap();
        assert_eq!(
            plotted(&chart, owner),
            vec![(0., 10.), (1., 21.), (2., 31.), (3., 40.)]
        );
        assert_eq!(plotted(&chart, peer), vec![(0., 100.), (2., 300.)]);
        assert_eq!(
            chart
                .model()
                .fulfilled_time_indices()
                .into_iter()
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
        chart.fit_content();
        chart
            .prepare_snapshot(iced::Size::new(640., 480.), 1.25, std::time::Instant::now())
            .unwrap();
    }
}

#[derive(Clone)]
struct CustomDatum {
    time: Time,
    value: f64,
}
impl TimedData for CustomDatum {
    type Item = Time;
    fn time(&self) -> &Time {
        &self.time
    }
    fn time_mut(&mut self) -> &mut Time {
        &mut self.time
    }
}
impl CustomData for CustomDatum {}
#[derive(Clone)]
struct EmptyRenderer;
impl CustomSeriesPaneRenderer<Frame> for EmptyRenderer {
    type HitTestData = ();
    fn draw(&self, _: &mut Frame, _: &PriceToCoordinateConverter<'_>, _: bool, _: Option<&()>) {}
}
struct CustomView {
    renderer: EmptyRenderer,
    destroyed: Rc<Cell<usize>>,
    invalid_builder: Rc<Cell<bool>>,
}
impl CustomSeriesPaneView<Frame> for CustomView {
    type Item = Time;
    type InternalTime = TimePoint;
    type Data = CustomDatum;
    type Metadata = ();
    type Options = ();
    type Renderer = EmptyRenderer;
    fn renderer(&self) -> &EmptyRenderer {
        &self.renderer
    }
    fn update(&mut self, _: PaneRendererCustomData<CustomDatum>, _: &()) {}
    fn price_value_builder(&self, data: &CustomDatum) -> CustomSeriesPricePlotValues {
        if self.invalid_builder.get() {
            vec![]
        } else {
            vec![data.value.into()]
        }
    }
    fn is_whitespace(&self, item: &CustomSeriesDataItem<CustomDatum, Time>) -> bool {
        matches!(item, CustomSeriesDataItem::Whitespace(_))
    }
    fn default_options(&self) {}
    fn destroy(&mut self) {
        self.destroyed.set(self.destroyed.get() + 1);
    }
}
fn custom(time: usize, value: f64) -> CustomSeriesDataItem<CustomDatum, Time> {
    CustomSeriesDataItem::Data(CustomDatum {
        time: UtcTimestamp::new(time as f64).into(),
        value,
    })
}
fn custom_gap(time: usize) -> CustomSeriesDataItem<CustomDatum, Time> {
    CustomSeriesDataItem::Whitespace(CustomSeriesWhitespaceData {
        time: UtcTimestamp::new(time as f64).into(),
        custom_values: None,
    })
}
fn add_custom(
    chart: &mut Chart,
    destroyed: Rc<Cell<usize>>,
    invalid_builder: Rc<Cell<bool>>,
) -> feather_charts::api::CustomSeriesHandle<CustomView> {
    chart
        .add_custom_series(
            CustomView {
                renderer: EmptyRenderer,
                destroyed,
                invalid_builder,
            },
            CustomSeriesOptions::default(),
            0,
            PriceScalePosition::Right,
        )
        .unwrap()
}

#[test]
fn mixed_custom_and_builtin_tails_keep_typed_rows_and_fulfilled_membership() {
    let mut chart = Chart::default();
    let a = add_custom(&mut chart, Rc::new(Cell::new(0)), Rc::new(Cell::new(false)));
    let b = add_custom(&mut chart, Rc::new(Cell::new(0)), Rc::new(Cell::new(false)));
    let builtin = add_builtin(&mut chart, SeriesType::Line);
    chart.set_custom_data(&a, vec![custom(10, 10.)]).unwrap();
    chart.set_custom_data(&b, vec![custom(10, 100.)]).unwrap();
    chart
        .set_data(builtin, vec![line(10, 1000., None)])
        .unwrap();
    chart
        .update_custom_data(&a, custom(20, 20.), false)
        .unwrap();
    chart.update_custom_data(&b, custom_gap(30), false).unwrap();
    assert_eq!(chart.model().time_scale().base_index(), Some(1.0.into()));
    assert_eq!(
        chart
            .model()
            .fulfilled_time_indices()
            .into_iter()
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert!(b.data_at(2.0.into()).is_none());
    chart
        .update_data(builtin, line(30, 3000., None), false)
        .unwrap();
    assert_eq!(chart.model().time_scale().base_index(), Some(2.0.into()));
    chart.update_custom_data(&a, custom_gap(40), false).unwrap();
    assert_eq!(chart.model().time_scale().base_index(), Some(2.0.into()));
    chart
        .update_custom_data(&a, custom(40, 40.), false)
        .unwrap();
    chart
        .update_custom_data(&b, custom(50, 500.), false)
        .unwrap();
    chart.update_custom_data(&a, custom(20, 21.), true).unwrap();
    assert_eq!(a.data_at(1.0.into()).unwrap().value, 21.);
    assert_eq!(a.data_at(3.0.into()).unwrap().value, 40.);
    assert_eq!(b.data_at(4.0.into()).unwrap().value, 500.);
    assert_eq!(plotted(&chart, builtin), vec![(0., 1000.), (2., 3000.)]);
    assert_eq!(
        chart
            .model()
            .fulfilled_time_indices()
            .into_iter()
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4]
    );
    // Inserting before the global maximum must now reindex all three owners.
    chart
        .update_data(builtin, line(35, 3500., None), false)
        .unwrap();
    assert_eq!(a.data_at(4.0.into()).unwrap().value, 40.);
    assert_eq!(b.data_at(5.0.into()).unwrap().value, 500.);
    assert_eq!(
        plotted(&chart, builtin),
        vec![(0., 1000.), (2., 3000.), (3., 3500.)]
    );
    chart
        .update_custom_data(&a, custom(60, 60.), false)
        .unwrap();
    assert_eq!(a.data_at(6.0.into()).unwrap().value, 60.);
    chart
        .prepare_snapshot(iced::Size::new(640., 480.), 2., std::time::Instant::now())
        .unwrap();
}

#[test]
fn failed_custom_tail_builder_is_atomic_and_owner_destroys_once() {
    let mut chart = Chart::default();
    let destroyed = Rc::new(Cell::new(0));
    let invalid = Rc::new(Cell::new(false));
    let owner = add_custom(&mut chart, destroyed.clone(), invalid.clone());
    let builtin = add_builtin(&mut chart, SeriesType::Line);
    chart
        .set_custom_data(&owner, vec![custom(10, 10.)])
        .unwrap();
    chart.set_data(builtin, vec![line(10, 100., None)]).unwrap();
    invalid.set(true);
    assert!(
        chart
            .update_custom_data(&owner, custom(100, 100.), false)
            .is_err()
    );
    invalid.set(false);
    chart
        .update_custom_data(&owner, custom(20, 20.), false)
        .unwrap();
    chart
        .update_data(builtin, line(15, 150., None), false)
        .unwrap();
    assert!(
        chart
            .model()
            .time_scale()
            .index_to_time(3.0.into())
            .is_none(),
        "failed builder must not leak a shared point"
    );
    assert_eq!(owner.data_at(2.0.into()).unwrap().value, 20.);
    chart
        .update_custom_data(&owner, custom(30, 30.), false)
        .unwrap();
    chart.remove_series(owner.id()).unwrap();
    assert_eq!(destroyed.get(), 1);
    assert!(
        chart
            .update_custom_data(&owner, custom(40, 40.), false)
            .is_err()
    );
    assert_eq!(owner.data_at(3.0.into()).unwrap().value, 30.);
    chart
        .update_data(builtin, line(50, 500., None), false)
        .unwrap();
    assert_eq!(
        plotted(&chart, builtin),
        vec![(0., 100.), (1., 150.), (2., 500.)]
    );
    drop(chart);
    assert_eq!(destroyed.get(), 1);
}

#[test]
fn pop_remove_and_full_replacement_restore_later_append_indexes() {
    let mut layer: DataLayer<HorzScaleBehaviorTime> =
        DataLayer::new(HorzScaleBehaviorTime::default());
    let owner = SeriesId::new(1);
    let peer = SeriesId::new(2);
    layer
        .set_series_data(
            owner,
            SeriesType::Line,
            vec![line(10, 10., None), line(20, 20., None)],
        )
        .unwrap();
    layer
        .set_series_data(peer, SeriesType::Line, vec![line(10, 100., None)])
        .unwrap();
    layer
        .update_series_data(owner, line(30, 30., None), false)
        .unwrap();
    let (removed, _) = layer.pop_series_data(owner, 1).unwrap();
    assert_eq!(removed[0].values()[3], 30.);
    let after_pop = layer
        .update_series_data(owner, line(25, 25., None), false)
        .unwrap();
    assert_eq!(after_pop.series[&owner].changed_index, Some(2.0.into()));
    layer.remove_series(owner).unwrap();
    let after_remove = layer
        .update_series_data(peer, line(40, 400., None), false)
        .unwrap();
    assert_eq!(after_remove.series[&peer].changed_index, Some(1.0.into()));
    layer
        .set_series_data(
            peer,
            SeriesType::Line,
            vec![line(5, 50., None), line(15, 150., None)],
        )
        .unwrap();
    let after_replace = layer
        .update_series_data(peer, line(20, 200., None), false)
        .unwrap();
    assert_eq!(after_replace.series[&peer].changed_index, Some(2.0.into()));
    assert_eq!(after_replace.time_scale.base_index, Some(2.0.into()));
    layer
        .set_series_data(peer, SeriesType::Line, vec![])
        .unwrap();
    let after_clear = layer
        .update_series_data(peer, line(1, 1., None), false)
        .unwrap();
    assert_eq!(after_clear.series[&peer].changed_index, Some(0.0.into()));
    assert_eq!(after_clear.time_scale.base_index, Some(0.0.into()));
}

#[test]
fn shared_fulfilled_membership_survives_until_the_final_owner_becomes_whitespace() {
    let mut chart = Chart::default();
    let a = add_builtin(&mut chart, SeriesType::Line);
    let b = add_builtin(&mut chart, SeriesType::Line);
    for id in [a, b] {
        chart.set_data(id, vec![line(10, 10., None)]).unwrap();
        chart.update_data(id, line(20, 20., None), false).unwrap();
    }
    chart
        .update_data(a, builtin(SeriesType::Line, 20, 0., true), false)
        .unwrap();
    assert_eq!(
        chart
            .model()
            .fulfilled_time_indices()
            .into_iter()
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(chart.model().time_scale().base_index(), Some(1.0.into()));
    chart
        .update_data(b, builtin(SeriesType::Line, 20, 0., true), false)
        .unwrap();
    assert_eq!(
        chart
            .model()
            .fulfilled_time_indices()
            .into_iter()
            .collect::<Vec<_>>(),
        vec![0]
    );
    assert_eq!(chart.model().time_scale().base_index(), Some(0.0.into()));
    chart
        .update_data(a, builtin(SeriesType::Line, 30, 0., true), false)
        .unwrap();
    assert_eq!(chart.model().time_scale().base_index(), Some(0.0.into()));
    chart.update_data(a, line(30, 30., None), false).unwrap();
    chart.update_data(b, line(20, 21., None), false).unwrap();
    assert_eq!(
        chart
            .model()
            .fulfilled_time_indices()
            .into_iter()
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(chart.model().time_scale().base_index(), Some(2.0.into()));
    assert_eq!(plotted(&chart, a), vec![(0., 10.), (2., 30.)]);
    assert_eq!(plotted(&chart, b), vec![(0., 10.), (1., 21.)]);
}

struct CustomBuilder;
impl feather_charts::model::get_series_plot_row_creator::CustomPlotRowBuilder<CustomDatum, Time, ()>
    for CustomBuilder
{
    fn values(&self, data: &CustomDatum) -> Vec<f64> {
        vec![data.value]
    }
    fn is_whitespace(&self, item: &CustomSeriesDataItem<CustomDatum, Time>) -> bool {
        matches!(item, CustomSeriesDataItem::Whitespace(_))
    }
}

#[test]
fn custom_pop_and_replacement_then_append_omit_unaffected_builtin_history() {
    let mut layer: DataLayer<HorzScaleBehaviorTime> =
        DataLayer::new(HorzScaleBehaviorTime::default());
    let owner = SeriesId::new(1);
    let peer = SeriesId::new(2);
    layer
        .set_series_data(peer, SeriesType::Line, vec![line(10, 100., None)])
        .unwrap();
    layer
        .set_custom_series_data(
            owner,
            vec![custom(10, 10.), custom(20, 20.)],
            &CustomBuilder,
        )
        .unwrap();
    let appended = layer
        .update_custom_series_data(owner, custom(30, 30.), &CustomBuilder, false)
        .unwrap();
    assert!(
        appended.series.is_empty(),
        "custom append must not resend unchanged built-in history"
    );
    assert_eq!(appended.custom_rows.len(), 1);
    assert_eq!(appended.custom_rows[0].base.index, 2.0.into());
    assert!(!appended.is_full_replacement);
    let (removed, _) = layer.pop_custom_series_data(owner, 1).unwrap();
    assert_eq!(removed, vec![2.0.into()]);
    let appended = layer
        .update_custom_series_data(owner, custom(25, 25.), &CustomBuilder, false)
        .unwrap();
    assert_eq!(appended.custom_rows[0].base.index, 2.0.into());
    assert!(appended.series.is_empty());
    layer
        .set_custom_series_data(owner, vec![custom(5, 5.)], &CustomBuilder)
        .unwrap();
    let appended = layer
        .update_custom_series_data(owner, custom(15, 15.), &CustomBuilder, false)
        .unwrap();
    assert_eq!(appended.custom_rows[0].base.index, 2.0.into());
    assert!(appended.series.is_empty());
    layer
        .set_custom_series_data::<CustomDatum, _>(owner, vec![], &CustomBuilder)
        .unwrap();
    let appended = layer
        .update_custom_series_data(owner, custom(20, 20.), &CustomBuilder, false)
        .unwrap();
    assert_eq!(appended.custom_rows[0].base.index, 1.0.into());
    assert_eq!(appended.time_scale.base_index, Some(1.0.into()));
    assert!(appended.series.is_empty());
}

fn line<M>(time: usize, value: f64, metadata: Option<M>) -> BuiltInSeriesDataItem<Time, M> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: Time::from(UtcTimestamp::new(time as f64)),
        value,
        color: None,
        custom_values: metadata,
    }))
}

#[derive(Debug)]
struct CountedMetadata(Rc<Cell<usize>>);

impl Clone for CountedMetadata {
    fn clone(&self) -> Self {
        self.0.set(self.0.get() + 1);
        Self(self.0.clone())
    }
}

#[test]
fn tail_append_clones_only_changed_row_metadata() {
    let clones = Rc::new(Cell::new(0));
    let mut layer = DataLayer::new(HorzScaleBehaviorTime::default());
    let owner = SeriesId::new(1);
    let peer = SeriesId::new(2);
    for id in [owner, peer] {
        layer
            .set_series_data(
                id,
                SeriesType::Line,
                (0..5000)
                    .map(|time| line(time, time as f64, Some(CountedMetadata(clones.clone()))))
                    .collect(),
            )
            .unwrap();
    }
    clones.set(0);

    layer
        .update_series_data(
            owner,
            line(5000, 6000., Some(CountedMetadata(clones.clone()))),
            false,
        )
        .unwrap();

    assert!(
        clones.get() <= 8,
        "one new tail point cloned metadata {} times for two unchanged histories",
        clones.get()
    );
}

#[test]
fn tail_append_does_not_fan_out_unaffected_peer_histories() {
    let mut layer: DataLayer<HorzScaleBehaviorTime> =
        DataLayer::new(HorzScaleBehaviorTime::default());
    let owner = SeriesId::new(1);
    let peer = SeriesId::new(2);
    for id in [owner, peer] {
        layer
            .set_series_data(
                id,
                SeriesType::Line,
                (0..100).map(|time| line(time, time as f64, None)).collect(),
            )
            .unwrap();
    }

    let response = layer
        .update_series_data(owner, line(100, 200., None), false)
        .unwrap();

    assert!(
        !response.series.contains_key(&peer),
        "a tail append preserves every peer logical index and must not resend its history"
    );
    assert_eq!(response.series.len(), 1);
    assert_eq!(response.series[&owner].rows.len(), 1);
    assert_eq!(response.series[&owner].changed_index, Some(100.0.into()));
    assert_eq!(response.time_scale.base_index, Some(100.0.into()));
}

#[test]
fn rejected_new_time_wrong_kind_does_not_leave_an_orphan_time_point() {
    let mut layer: DataLayer<HorzScaleBehaviorTime> =
        DataLayer::new(HorzScaleBehaviorTime::default());
    let owner = SeriesId::new(1);
    let peer = SeriesId::new(2);
    for id in [owner, peer] {
        layer
            .set_series_data(id, SeriesType::Line, vec![line(10, 10., None)])
            .unwrap();
    }
    let wrong_kind = BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
        time: Time::from(UtcTimestamp::new(100.)),
        value: 100.,
        color: None,
        custom_values: None,
    }));
    assert!(matches!(
        layer.update_series_data(owner, wrong_kind, false),
        Err(DataLayerError::PlotRow(_))
    ));

    // This remains legal only if rejection did not advance the owner's last time.
    layer
        .update_series_data(owner, line(20, 20., None), false)
        .unwrap();
    // A real middle insertion exposes the full shared mapping regardless of
    // whether the preceding tail update used an incremental handoff.
    let response = layer
        .update_series_data(peer, line(15, 15., None), false)
        .unwrap();
    let points = response.time_scale.points.unwrap();

    assert_eq!(
        points.len(),
        3,
        "a rejected row must not reserve a shared time point"
    );
    assert_eq!(
        points
            .into_iter()
            .map(|point| point.original_time)
            .collect::<Vec<_>>(),
        [10, 15, 20].map(|time| Time::from(UtcTimestamp::new(time as f64)))
    );
    assert_eq!(response.time_scale.base_index, Some(2.0.into()));
}

#[test]
fn global_middle_insertion_reindexes_peers_even_when_it_is_the_owners_latest_time() {
    let mut layer: DataLayer<HorzScaleBehaviorTime> =
        DataLayer::new(HorzScaleBehaviorTime::default());
    let owner = SeriesId::new(1);
    let peer = SeriesId::new(2);
    layer
        .set_series_data(owner, SeriesType::Line, vec![line(10, 10., None)])
        .unwrap();
    layer
        .set_series_data(peer, SeriesType::Line, vec![line(30, 30., None)])
        .unwrap();

    let inserted = layer
        .update_series_data(owner, line(20, 20., None), false)
        .unwrap();
    assert_eq!(inserted.time_scale.first_changed_point_index, Some(1));
    assert_eq!(inserted.time_scale.points.as_ref().unwrap().len(), 3);
    assert_eq!(inserted.series[&peer].changed_index, None);
    assert_eq!(inserted.series[&peer].rows[0].index(), 2.0.into());
    assert_eq!(inserted.series[&owner].rows.len(), 2);
    assert_eq!(inserted.series[&owner].rows[1].index(), 1.0.into());

    let shared = layer
        .update_series_data(owner, line(30, 31., None), false)
        .unwrap();
    assert!(shared.time_scale.points.is_none());
    assert!(!shared.series.contains_key(&peer));
    assert_eq!(shared.series[&owner].changed_index, Some(2.0.into()));
    assert_eq!(shared.series[&owner].rows[0].values()[3], 31.);

    let appended = layer
        .update_series_data(owner, line(40, 40., None), false)
        .unwrap();
    assert_eq!(appended.series[&owner].changed_index, Some(3.0.into()));
    assert_eq!(appended.series[&owner].rows.len(), 1);
    assert!(!appended.series.contains_key(&peer));

    let historical = layer
        .update_series_data(owner, line(20, 21., None), true)
        .unwrap();
    assert!(historical.time_scale.points.is_none());
    assert_eq!(historical.series[&owner].changed_index, Some(1.0.into()));
    assert_eq!(historical.series[&owner].rows[0].values()[3], 21.);

    // A later insertion into the shared middle must still retain all the
    // owner rows created by both kinds of incremental update above.
    let middle = layer
        .update_series_data(peer, line(35, 35., None), false)
        .unwrap();
    assert_eq!(middle.time_scale.first_changed_point_index, Some(3));
    assert_eq!(middle.series[&owner].changed_index, None);
    assert_eq!(
        middle.series[&owner]
            .rows
            .iter()
            .map(|row| (row.index().value(), row.values()[3]))
            .collect::<Vec<_>>(),
        vec![(0., 10.), (1., 21.), (2., 31.), (4., 40.)]
    );
    assert_eq!(
        middle.series[&peer]
            .rows
            .iter()
            .map(|row| (row.index().value(), row.values()[3]))
            .collect::<Vec<_>>(),
        vec![(2., 30.), (3., 35.)]
    );

    let appended_again = layer
        .update_series_data(owner, line(50, 50., None), false)
        .unwrap();
    assert_eq!(
        appended_again.series[&owner].changed_index,
        Some(5.0.into())
    );
    assert_eq!(appended_again.series[&owner].rows.len(), 1);
    assert_eq!(appended_again.time_scale.base_index, Some(5.0.into()));
}

#[test]
fn native_mouse_pan_survives_builtin_and_custom_tail_updates() {
    use feather_charts::api::ChartState;
    use iced::{
        Point, Rectangle, Size, mouse,
        widget::canvas::{Event, Program},
    };
    use std::time::Instant;
    for (custom_owner, axis) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut chart = Chart::default();
        let builtin_id = chart
            .add_series(
                SeriesOptionsMap::Line(Default::default()),
                0,
                PriceScalePosition::Right,
            )
            .unwrap();
        chart
            .set_data(
                builtin_id,
                (0..120)
                    .map(|index| {
                        builtin(SeriesType::Line, index, 100. + (index as f64).sin(), false)
                    })
                    .collect(),
            )
            .unwrap();
        let handle = chart
            .add_custom_series(
                CustomView {
                    renderer: EmptyRenderer,
                    destroyed: Rc::default(),
                    invalid_builder: Rc::default(),
                },
                CustomSeriesOptions::default(),
                0,
                PriceScalePosition::Right,
            )
            .unwrap();
        chart
            .set_custom_data(
                &handle,
                (0..120)
                    .map(|index| custom(index, 100. + (index as f64).sin()))
                    .collect(),
            )
            .unwrap();
        chart
            .prepare(Size::new(640., 480.), 1.25, Instant::now())
            .unwrap();
        let origin = chart.frame().layout.plot.origin;
        let start = Point::new(
            if axis {
                chart.frame().layout.right_axis.origin.x as f32 + 5.
            } else {
                origin.x as f32 + 200.
            },
            origin.y as f32 + 150.,
        );
        let mut state = ChartState::default();
        let native = |chart: &mut Chart, state: &mut ChartState, event, point| {
            let message = chart
                .program(std::convert::identity)
                .update(
                    state,
                    &Event::Mouse(event),
                    Rectangle::with_size(Size::new(640., 480.)),
                    mouse::Cursor::Available(point),
                )
                .and_then(|action| action.into_inner().0);
            if let Some(message) = message {
                chart.update(message).unwrap();
            }
        };
        native(
            &mut chart,
            &mut state,
            mouse::Event::ButtonPressed(mouse::Button::Left),
            start,
        );
        native(
            &mut chart,
            &mut state,
            mouse::Event::CursorMoved {
                position: Point::new(start.x + 20., start.y + 20.),
            },
            Point::new(start.x + 20., start.y + 20.),
        );
        assert!(if axis {
            chart.active_price_axis().is_some()
        } else {
            chart.is_panning()
        });
        if custom_owner {
            chart
                .update_custom_data(&handle, custom(120, 101.), false)
                .unwrap();
        } else {
            chart
                .update_data(
                    builtin_id,
                    builtin(SeriesType::Line, 120, 101., false),
                    false,
                )
                .unwrap();
        }
        chart
            .prepare(Size::new(640., 480.), 1.25, Instant::now())
            .unwrap();
        let next = Point::new(start.x + 40., start.y + 40.);
        native(
            &mut chart,
            &mut state,
            mouse::Event::CursorMoved { position: next },
            next,
        );
        assert!(
            if axis {
                chart.active_price_axis().is_some()
            } else {
                chart.is_panning()
            },
            "live update cancelled mouse drag: custom={custom_owner}, axis={axis}"
        );
        native(
            &mut chart,
            &mut state,
            mouse::Event::ButtonReleased(mouse::Button::Left),
            next,
        );
        assert!(!chart.is_panning() && chart.active_price_axis().is_none());
    }
}

#[test]
fn append_preserves_business_day_original_time_and_metadata_across_later_reindex() {
    use feather_charts::model::plot_data::MutablePlotRow;
    let mut layer: DataLayer<HorzScaleBehaviorTime, String> = DataLayer::new(Default::default());
    let owner = SeriesId::new(1);
    let peer = SeriesId::new(2);
    let row = |date: &str, value| {
        BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time: Time::from(date),
            value,
            color: None,
            custom_values: Some(date.to_owned()),
        }))
    };
    layer
        .set_series_data(owner, SeriesType::Line, vec![row("2024-01-01", 10.)])
        .unwrap();
    layer
        .set_series_data(peer, SeriesType::Line, vec![])
        .unwrap();
    let append = layer
        .update_series_data(owner, row("2024-01-02", 20.), false)
        .unwrap();
    assert_eq!(
        append.time_scale.append.unwrap().point.original_time,
        Time::from("2024-01-02")
    );
    let response = layer
        .update_series_data(
            peer,
            line(1_704_110_400, 15., Some("middle".to_owned())),
            false,
        )
        .unwrap();
    let points = response.time_scale.points.unwrap();
    assert_eq!(points[0].original_time, Time::from("2024-01-01"));
    assert_eq!(points[2].original_time, Time::from("2024-01-02"));
    let rows = &response.series[&owner].rows;
    assert_eq!(
        rows.iter().map(PlotRowLike::index).collect::<Vec<_>>(),
        vec![0.0.into(), 2.0.into()]
    );
    assert_eq!(rows[0].original_time(), &Time::from("2024-01-01"));
    assert_eq!(rows[1].original_time(), &Time::from("2024-01-02"));
    for (row, expected) in rows.iter().zip(["2024-01-01", "2024-01-02"]) {
        let feather_charts::model::series_data::SeriesPlotRow::Line(row) = row else {
            unreachable!()
        };
        assert_eq!(row.base.custom_values.as_deref(), Some(expected));
    }
}
