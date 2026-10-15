use feather_charts::model::{
    data_consumer::TimedData,
    ihorz_scale_behavior::{HorzScaleBehavior, TimeMark},
    localization_options::LocalizationOptions,
    tick_marks::TickMark,
    time_data::{LogicalRange, TickMarkWeightValue, TimePointIndex, TimeScalePoint},
    time_scale::{TimeScale, TimeScaleUpdate},
    time_scale_host::TimeScaleLayoutContext,
    time_scale_options::HorzScaleOptions,
};
use std::{cell::Cell, convert::Infallible, rc::Rc};

struct CountedTime {
    value: f64,
    clones: Rc<Cell<usize>>,
}
impl Clone for CountedTime {
    fn clone(&self) -> Self {
        self.clones.set(self.clones.get() + 1);
        Self {
            value: self.value,
            clones: self.clones.clone(),
        }
    }
}
#[derive(Default)]
struct Behavior {
    keys: Rc<Cell<usize>>,
    incremental: bool,
}
impl HorzScaleBehavior for Behavior {
    type Item = CountedTime;
    type InternalItem = CountedTime;
    type Key = f64;
    type CacheKey = u64;
    type Options = ();
    type Converter = fn(&CountedTime) -> CountedTime;
    type Error = Infallible;
    fn options(&self) -> &() {
        &()
    }
    fn set_options(&mut self, _: ()) {}
    fn preprocess_data<D: TimedData<Item = CountedTime>>(
        &mut self,
        _: &mut [D],
    ) -> Result<(), Infallible> {
        Ok(())
    }
    fn to_internal(&self, item: &CountedTime) -> Result<CountedTime, Infallible> {
        Ok(item.clone())
    }
    fn create_converter_to_internal<D: TimedData<Item = CountedTime>>(
        &mut self,
        _: &[D],
    ) -> Result<Self::Converter, Infallible> {
        Ok(Clone::clone)
    }
    fn key(&self, item: &CountedTime) -> f64 {
        self.keys.set(self.keys.get() + 1);
        item.value
    }
    fn cache_key(&self, item: &CountedTime) -> u64 {
        item.value.to_bits()
    }
    fn update_formatter(&mut self, _: &LocalizationOptions<CountedTime>) {}
    fn format_item(&self, item: &CountedTime) -> String {
        item.value.to_string()
    }
    fn format_tickmark(
        &self,
        item: &TickMark<CountedTime, CountedTime>,
        _: &LocalizationOptions<CountedTime>,
    ) -> String {
        item.time.value.to_string()
    }
    fn max_tick_mark_weight(&self, marks: &[TimeMark]) -> TickMarkWeightValue {
        marks.iter().map(|m| m.weight).max().unwrap_or_default()
    }
    fn append_weight(
        &mut self,
        _: Option<&CountedTime>,
        _: &CountedTime,
    ) -> Option<TickMarkWeightValue> {
        self.incremental.then_some(TickMarkWeightValue::new(1))
    }
    fn fill_weights_for_points(
        &mut self,
        points: &mut [TimeScalePoint<CountedTime, CountedTime>],
        _: usize,
    ) {
        for p in points {
            p.time_weight = TickMarkWeightValue::new(1);
        }
    }
}

#[test]
fn moving_viewport_clones_only_visible_cached_time_marks() {
    for count in [1_200, 100_000] {
        let clones = Rc::new(Cell::new(0));
        let mut scale = TimeScale::new(
            Behavior::default(),
            HorzScaleOptions::default(),
            LocalizationOptions::new("en-US", ""),
        );
        scale.update(TimeScaleUpdate {
            points: (0..count)
                .map(|index| {
                    let item = || CountedTime {
                        value: index as f64,
                        clones: clones.clone(),
                    };
                    TimeScalePoint {
                        time_weight: TickMarkWeightValue::new(1),
                        time: item(),
                        original_time: item(),
                    }
                })
                .collect(),
            first_changed_point_index: 0,
        });
        scale.set_width(640.);
        scale.set_base_index(Some(TimePointIndex::new((count - 1) as f64)));
        scale.set_logical_range(LogicalRange {
            from: 20.0.into(),
            to: 100.0.into(),
        });
        let layout = TimeScaleLayoutContext { font_size: 12. };
        assert!(!scale.marks(layout).unwrap().is_empty());
        for shift in 1..=3 {
            scale.set_right_offset(100. + shift as f64 - (count - 1) as f64);
            assert_eq!(
                scale.visible_strict_range().unwrap().right().value(),
                100. + shift as f64
            );
            clones.set(0);
            let labels = scale.marks(layout).unwrap();
            assert!(!labels.is_empty());
            assert!(
                clones.get() <= 40,
                "{count} rows: viewport preparation copied {} time payloads",
                clones.get()
            );
        }
    }
}

#[test]
fn warmed_tick_append_copies_only_a_bounded_selection_suffix() {
    use feather_charts::model::tick_marks::TickMarks;
    for count in [1200, 100_000] {
        let clones = Rc::new(Cell::new(0));
        let point = |index| TimeScalePoint {
            time: CountedTime {
                value: index as f64,
                clones: clones.clone(),
            },
            original_time: CountedTime {
                value: index as f64,
                clones: clones.clone(),
            },
            time_weight: TickMarkWeightValue::new(1),
        };
        let mut marks = TickMarks::default();
        let points = (0..count).map(point).collect::<Vec<_>>();
        marks.set_time_scale_points(&points, 0);
        marks.build(6., 60., false, &Default::default(), 0);
        clones.set(0);
        marks.append_point(count, &point(count));
        marks.build(6., 60., false, &Default::default(), 0);
        assert!(
            clones.get() <= 12,
            "{count}: {} tick payload clones",
            clones.get()
        );
    }
}

#[test]
fn arbitrary_horizontal_behavior_keeps_the_full_context_append_fallback() {
    use feather_charts::model::{
        data_consumer::{BuiltInSeriesDataItem, LineDataItem, WhitespaceData},
        data_layer::{DataLayer, SeriesId},
        series_options::SeriesType,
    };
    let clones = Rc::new(Cell::new(0));
    let row = |value| {
        BuiltInSeriesDataItem::Line(LineDataItem::<CountedTime, ()>::Whitespace(
            WhitespaceData {
                time: CountedTime {
                    value,
                    clones: clones.clone(),
                },
                custom_values: None,
            },
        ))
    };
    let mut layer = DataLayer::new(Behavior::default());
    let id = SeriesId::new(1);
    layer
        .set_series_data(id, SeriesType::Line, vec![row(1.)])
        .unwrap();
    let update = layer.update_series_data(id, row(2.), false).unwrap();
    assert!(update.time_scale.append.is_none());
    let points = update.time_scale.points.unwrap();
    assert_eq!(points.len(), 2);
    assert!(
        points
            .iter()
            .all(|point| point.time_weight == TickMarkWeightValue::new(1))
    );
}

#[test]
fn native_style_tail_lookup_uses_constant_key_work() {
    use feather_charts::model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        data_layer::{DataLayer, SeriesId},
        series_options::SeriesType,
    };
    for count in [1200, 100_000] {
        let keys = Rc::new(Cell::new(0));
        let clones = Rc::new(Cell::new(0));
        let mut layer = DataLayer::new(Behavior {
            keys: keys.clone(),
            incremental: true,
        });
        let row = |index| {
            BuiltInSeriesDataItem::Line(LineDataItem::<_, ()>::Data(LineData {
                time: CountedTime {
                    value: index as f64,
                    clones: clones.clone(),
                },
                value: 100.,
                color: None,
                custom_values: None,
            }))
        };
        let id = SeriesId::new(1);
        layer
            .set_series_data(id, SeriesType::Line, (0..count).map(row).collect())
            .unwrap();
        keys.set(0);
        clones.set(0);
        let update = layer.update_series_data(id, row(count), false).unwrap();
        assert!(update.time_scale.append.is_some());
        assert!(
            keys.get() <= 12,
            "{count}: tail lookup evaluated {} keys",
            keys.get()
        );
        assert!(
            clones.get() <= 10,
            "{count}: tail lookup cloned {} times",
            clones.get()
        );
    }
}
