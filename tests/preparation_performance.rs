use feather_charts::{
    model::{
        chart_model::{ChartModel, ChartModelOptions},
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        data_layer::{DataLayer, SeriesId},
        horz_scale_behavior_time::{
            horz_scale_behavior_time::HorzScaleBehaviorTime,
            types::{Time, UtcTimestamp},
        },
        localization_options::LocalizationOptions,
        pane::PriceScalePosition,
        range_impl::RangeImpl,
        series_options::{LineSeriesOptions, SeriesOptionsMap, SeriesType},
        time_data::{LogicalRange, TimePointIndex, TimedValue, visible_timed_values},
        time_scale::TimeScale,
        time_scale_options::HorzScaleOptions,
    },
    renderers::{draw_line::LineType, grid_renderer::PixelRatio, line_renderer::LinePoint},
};
use std::{cell::Cell, rc::Rc};

#[derive(Debug)]
struct TrackedMetadata(Rc<Cell<usize>>);
impl Clone for TrackedMetadata {
    fn clone(&self) -> Self {
        self.0.set(self.0.get() + 1);
        Self(self.0.clone())
    }
}

#[test]
fn existing_timestamp_replacement_does_not_clone_unchanged_metadata() {
    let clones = Rc::new(Cell::new(0));
    let mut layer: DataLayer<HorzScaleBehaviorTime, TrackedMetadata> =
        DataLayer::new(HorzScaleBehaviorTime::default());
    let id = SeriesId::new(1);
    let row = |i, value| {
        BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time: Time::from(UtcTimestamp::new(i as f64)),
            value,
            color: None,
            custom_values: Some(TrackedMetadata(clones.clone())),
        }))
    };
    layer
        .set_series_data(
            id,
            SeriesType::Line,
            (0..5000).map(|i| row(i, i as f64)).collect(),
        )
        .unwrap();
    clones.set(0);
    let response = layer
        .update_series_data(id, row(4999, 6000.), false)
        .unwrap();
    assert!(
        clones.get() <= 8,
        "single existing-timestamp replacement cloned metadata {} times",
        clones.get()
    );
    assert_eq!(
        response.series[&id].rows.len(),
        1,
        "the response must carry only the changed plotted row"
    );
    assert!(
        response.time_scale.points.is_none(),
        "existing timestamp keeps shared time points unchanged"
    );
}

#[test]
fn line_preparation_retains_only_visible_rows_and_curve_neighbors() {
    for history in [1200, 20_000] {
        for line_type in [LineType::Simple, LineType::WithSteps, LineType::Curved] {
            let mut model: ChartModel<HorzScaleBehaviorTime> = ChartModel::new(
                TimeScale::new(
                    HorzScaleBehaviorTime::default(),
                    HorzScaleOptions::default(),
                    LocalizationOptions::new("en-US", "dd MMM 'yy"),
                ),
                ChartModelOptions::default(),
            );
            model.set_width(800.).unwrap();
            model.set_pane_height(0, 500.).unwrap();
            let id = SeriesId::new(1);
            let mut options = LineSeriesOptions::default();
            options.style.line_type = line_type;
            model
                .register_series(
                    id,
                    SeriesType::Line,
                    SeriesOptionsMap::Line(options),
                    0,
                    PriceScalePosition::Right,
                )
                .unwrap();
            let mut layer = DataLayer::new(HorzScaleBehaviorTime::default());
            let data = (0..history)
                .map(|i| {
                    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                        time: Time::from(UtcTimestamp::new(i as f64)),
                        value: 100. + (i as f64).sin(),
                        color: None,
                        custom_values: None,
                    }))
                })
                .collect();
            model
                .apply_data_update(layer.set_series_data(id, SeriesType::Line, data).unwrap())
                .unwrap();
            model.set_logical_range(LogicalRange {
                from: 20.0.into(),
                to: 100.0.into(),
            });
            if let Some(mask) = model.take_invalidation() {
                model.apply_invalidation(&mask);
            }
            let prepared = model.prepare_line(id).unwrap().unwrap();
            assert!(
                prepared.items.len() <= 85,
                "{history} rows, {line_type:?}: {} copied rows for 81 visible rows plus two neighbors each side",
                prepared.items.len()
            );
            assert!(
                prepared
                    .items
                    .iter()
                    .any(|row| row.index == TimePointIndex::new(20.))
            );
            assert!(
                prepared
                    .items
                    .iter()
                    .any(|row| row.index == TimePointIndex::new(100.))
            );
            assert!(prepared.visible_range.to <= prepared.items.len());
            assert_eq!(
                model.series(id).unwrap().bars().rows().len(),
                history,
                "preparation must not discard historical model data"
            );
            // Construct the original full-history renderer input independently,
            // using the settled model mapping, to prove edge controls are kept.
            let price = model.panes()[0].right_price_scale();
            let mut reference = prepared.clone();
            reference.items = (0..history)
                .map(|i| {
                    let index = TimePointIndex::new(i as f64);
                    LinePoint {
                        index,
                        x: model.time_scale().index_to_coordinate(index).value(),
                        y: price
                            .price_to_coordinate(100. + (i as f64).sin(), 100.)
                            .value(),
                        color: prepared.items[0].color.clone(),
                    }
                })
                .collect();
            let timed: Vec<_> = reference
                .items
                .iter()
                .map(|p| TimedValue {
                    time: p.index,
                    x: p.x.into(),
                })
                .collect();
            reference.visible_range =
                visible_timed_values(&timed, &RangeImpl::new(20.0.into(), 100.0.into()), true);
            for ratio in [1., 1.25, 2.] {
                let ratio = PixelRatio {
                    horizontal: ratio,
                    vertical: ratio,
                };
                assert_eq!(
                    prepared.draw_commands(ratio),
                    reference.draw_commands(ratio),
                    "bounded preparation must preserve {line_type:?} control points and edge joins"
                );
            }
        }
    }
}
