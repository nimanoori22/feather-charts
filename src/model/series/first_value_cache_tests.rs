use super::*;
use crate::model::{data_layer::TimeScaleChanges, plot_data::PlotRow, series_data::LinePlotRow};
use std::collections::BTreeMap;

fn base(index: f64, value: f64) -> PlotRow<(), ()> {
    PlotRow {
        index: index.into(),
        time: (),
        original_time: (),
        value: [value; 4],
        custom_values: None,
        original_data_count: None,
    }
}
fn row(index: f64, value: f64) -> SeriesPlotRow<(), ()> {
    SeriesPlotRow::Line(LinePlotRow {
        base: base(index, value),
        color: None,
    })
}
fn custom_row(index: f64, value: f64) -> CustomPlotRow<(), (), ()> {
    CustomPlotRow {
        base: base(index, value),
        data: (),
        color: None,
    }
}
fn builtin() -> Series<(), ()> {
    let mut result = Series::new(
        SeriesId::new(1),
        SeriesType::Line,
        SeriesOptionsMap::Line(Default::default()),
    )
    .unwrap();
    result.set_data(
        [10., 30., 90., 120.]
            .map(|index| row(index, index * 10.))
            .to_vec(),
        None,
    );
    result
}
fn custom() -> CustomSeries<(), (), ()> {
    let mut result = CustomSeries::new(SeriesId::new(1), CustomSeriesOptions::default());
    result.set_data(
        [10., 30., 90., 120.]
            .map(|index| custom_row(index, index * 10.))
            .to_vec(),
        None,
    );
    result
}
fn visible(left: f64, right: f64) -> RangeImpl<TimePointIndex> {
    RangeImpl::new(left.into(), right.into())
}
fn check(query: impl FnOnce() -> Option<FirstValue>, expected: Option<(f64, f64)>) -> usize {
    FIRST_VALUE_SEARCH_READS.with(|reads| reads.set(0));
    assert_eq!(
        query().map(|first| (first.time_point.value(), first.value)),
        expected
    );
    FIRST_VALUE_SEARCH_READS.with(Cell::get)
}
fn custom_update(source: &mut CustomSeries<(), (), ()>, index: f64, value: Option<f64>) {
    source.apply_update(CustomDataUpdateResponse {
        series: BTreeMap::new(),
        custom_rows: value
            .map(|value| custom_row(index, value))
            .into_iter()
            .collect(),
        custom: BTreeMap::from([(source.id(), CustomSeriesChanges::unchanged(None, true))]),
        time_scale: TimeScaleChanges {
            append: None,
            membership: None,
            points: None,
            first_changed_point_index: None,
            base_index: None,
        },
        is_full_replacement: false,
        changed_index: Some(index.into()),
    });
}

#[test]
fn builtin_repeated_first_value_search_keys_only_the_visible_left_edge() {
    let mut source = builtin();
    let narrow = visible(20., 21.);
    assert!(
        check(|| source.first_value(Some(&narrow)), Some((30., 300.))) > 0,
        "nearest-right may be beyond visible.right"
    );
    assert_eq!(
        check(|| source.first_value(Some(&narrow)), Some((30., 300.))),
        0
    );
    let wider = visible(20., 100.);
    assert_eq!(
        check(|| source.first_value(Some(&wider)), Some((30., 300.))),
        0,
        "changing only visible.right does not require a search"
    );
    source.apply_rows(vec![row(200., 2000.)], None, Some(200.0.into()));
    assert_eq!(
        check(|| source.first_value(Some(&wider)), Some((30., 300.))),
        0
    );
    assert_eq!(check(|| source.first_value(None), None), 0);
    assert_eq!(
        check(|| source.first_value(Some(&wider)), Some((30., 300.))),
        0
    );
    let later = visible(40., 41.);
    assert!(check(|| source.first_value(Some(&later)), Some((90., 900.))) > 0);
    source.set_data(vec![row(50., 5.)], None);
    assert!(check(|| source.first_value(Some(&later)), Some((50., 5.))) > 0);
    source.set_data(vec![], None);
    assert_eq!(check(|| source.first_value(Some(&later)), None), 0);
    source.set_data(vec![row(60., 6.)], None);
    assert!(check(|| source.first_value(Some(&later)), Some((60., 6.))) > 0);
}

#[test]
fn builtin_first_value_invalidates_only_mutations_that_can_change_the_answer() {
    let mut source = builtin();
    let range = visible(20., 100.);
    check(|| source.first_value(Some(&range)), Some((30., 300.)));
    source.apply_rows(vec![row(10., -1.)], None, Some(10.0.into()));
    assert_eq!(
        check(|| source.first_value(Some(&range)), Some((30., 300.))),
        0
    );
    source.apply_rows(vec![], None, Some(10.0.into()));
    assert_eq!(
        check(|| source.first_value(Some(&range)), Some((30., 300.))),
        0
    );
    source.apply_rows(vec![row(90., -9.)], None, Some(90.0.into()));
    assert_eq!(
        check(|| source.first_value(Some(&range)), Some((30., 300.))),
        0
    );
    source.apply_rows(vec![row(19., -19.)], None, Some(19.0.into()));
    assert_eq!(
        check(|| source.first_value(Some(&range)), Some((30., 300.))),
        0
    );
    source.apply_rows(vec![row(30., 333.)], None, Some(30.0.into()));
    assert!(check(|| source.first_value(Some(&range)), Some((30., 333.))) > 0);
    source.apply_rows(vec![row(25., 250.)], None, Some(25.0.into()));
    assert!(check(|| source.first_value(Some(&range)), Some((25., 250.))) > 0);
    source.apply_rows(vec![row(20., 200.)], None, Some(20.0.into()));
    assert!(check(|| source.first_value(Some(&range)), Some((20., 200.))) > 0);
    source.apply_rows(vec![], None, Some(20.0.into()));
    assert!(check(|| source.first_value(Some(&range)), Some((25., 250.))) > 0);
    source.apply_rows(vec![], None, Some(25.0.into()));
    assert!(check(|| source.first_value(Some(&range)), Some((30., 333.))) > 0);
}

#[test]
fn builtin_cached_missing_first_value_becomes_available_after_a_later_append() {
    let mut source = builtin();
    let range = visible(500., 501.);
    assert!(check(|| source.first_value(Some(&range)), None) > 0);
    assert_eq!(check(|| source.first_value(Some(&range)), None), 0);
    source.apply_rows(vec![row(200., 2000.)], None, Some(200.0.into()));
    assert_eq!(check(|| source.first_value(Some(&range)), None), 0);
    source.apply_rows(vec![row(600., 6000.)], None, Some(600.0.into()));
    assert!(check(|| source.first_value(Some(&range)), Some((600., 6000.))) > 0);
    source.apply_rows(vec![row(700., 7000.)], None, Some(700.0.into()));
    assert_eq!(
        check(|| source.first_value(Some(&range)), Some((600., 6000.))),
        0
    );
}

#[test]
fn custom_repeated_first_value_search_keys_only_the_visible_left_edge() {
    let mut source = custom();
    let narrow = visible(20., 21.);
    assert!(check(|| source.first_value(Some(&narrow)), Some((30., 300.))) > 0);
    assert_eq!(
        check(|| source.first_value(Some(&narrow)), Some((30., 300.))),
        0
    );
    let wider = visible(20., 100.);
    assert_eq!(
        check(|| source.first_value(Some(&wider)), Some((30., 300.))),
        0
    );
    custom_update(&mut source, 200., Some(2000.));
    assert_eq!(
        check(|| source.first_value(Some(&wider)), Some((30., 300.))),
        0
    );
    assert_eq!(check(|| source.first_value(None), None), 0);
    assert_eq!(
        check(|| source.first_value(Some(&wider)), Some((30., 300.))),
        0
    );
    let later = visible(40., 41.);
    assert!(check(|| source.first_value(Some(&later)), Some((90., 900.))) > 0);
}

#[test]
fn custom_first_value_invalidates_only_mutations_that_can_change_the_answer() {
    let mut source = custom();
    let range = visible(20., 100.);
    check(|| source.first_value(Some(&range)), Some((30., 300.)));
    for (index, value) in [
        (10., Some(-1.)),
        (10., None),
        (90., Some(-9.)),
        (19., Some(-19.)),
    ] {
        custom_update(&mut source, index, value);
        assert_eq!(
            check(|| source.first_value(Some(&range)), Some((30., 300.))),
            0
        );
    }
    custom_update(&mut source, 30., Some(333.));
    assert!(check(|| source.first_value(Some(&range)), Some((30., 333.))) > 0);
    custom_update(&mut source, 25., Some(250.));
    assert!(check(|| source.first_value(Some(&range)), Some((25., 250.))) > 0);
    custom_update(&mut source, 20., Some(200.));
    assert!(check(|| source.first_value(Some(&range)), Some((20., 200.))) > 0);
    custom_update(&mut source, 20., None);
    assert!(check(|| source.first_value(Some(&range)), Some((25., 250.))) > 0);
    custom_update(&mut source, 25., None);
    assert!(check(|| source.first_value(Some(&range)), Some((30., 333.))) > 0);
    source.pop_indices(&[90.0.into()]);
    assert_eq!(
        check(|| source.first_value(Some(&range)), Some((30., 333.))),
        0
    );
}

#[test]
fn custom_cached_missing_first_value_becomes_available_after_a_later_append() {
    let mut source = custom();
    let range = visible(500., 501.);
    assert!(check(|| source.first_value(Some(&range)), None) > 0);
    assert_eq!(check(|| source.first_value(Some(&range)), None), 0);
    custom_update(&mut source, 200., Some(2000.));
    assert_eq!(check(|| source.first_value(Some(&range)), None), 0);
    custom_update(&mut source, 600., Some(6000.));
    assert!(check(|| source.first_value(Some(&range)), Some((600., 6000.))) > 0);
    custom_update(&mut source, 700., Some(7000.));
    assert_eq!(
        check(|| source.first_value(Some(&range)), Some((600., 6000.))),
        0
    );
}

#[test]
fn custom_reindex_pop_and_full_replacements_refresh_the_cached_first_value() {
    let mut source = custom();
    let range = visible(20., 100.);
    check(|| source.first_value(Some(&range)), Some((30., 300.)));
    source.apply_indices(&CustomSeriesChanges::unchanged(None, false));
    assert_eq!(
        check(|| source.first_value(Some(&range)), Some((30., 300.))),
        0
    );
    source.apply_indices(&CustomSeriesChanges::new(
        vec![5.0.into(), 15.0.into(), 25.0.into(), 35.0.into()],
        None,
    ));
    assert!(check(|| source.first_value(Some(&range)), Some((25., 900.))) > 0);
    let popped = source.pop_indices(&[25.0.into()]);
    assert_eq!(popped[0].base.value[3], 900.);
    assert!(check(|| source.first_value(Some(&range)), Some((35., 1200.))) > 0);
    source.set_data(vec![custom_row(40., 4.)], None);
    assert!(check(|| source.first_value(Some(&range)), Some((40., 4.))) > 0);
    source.apply_update(CustomDataUpdateResponse {
        series: BTreeMap::new(),
        custom_rows: vec![custom_row(22., 2.)],
        custom: BTreeMap::from([(
            source.id(),
            CustomSeriesChanges::new(vec![22.0.into()], None),
        )]),
        time_scale: TimeScaleChanges {
            append: None,
            membership: None,
            points: None,
            first_changed_point_index: None,
            base_index: None,
        },
        is_full_replacement: true,
        changed_index: None,
    });
    assert!(check(|| source.first_value(Some(&range)), Some((22., 2.))) > 0);
    source.set_data(vec![], None);
    assert_eq!(check(|| source.first_value(Some(&range)), None), 0);
    source.set_data(vec![custom_row(60., 6.)], None);
    assert!(check(|| source.first_value(Some(&range)), Some((60., 6.))) > 0);
}

#[test]
fn custom_raw_range_reuses_only_unchanged_prices_and_keeps_primitive_contributions_fresh() {
    use crate::model::{
        autoscale_info_impl::{AutoscaleInfo, PriceRange},
        plot_list::MIN_MAX_ROWS_READ,
    };
    let query = |source: &CustomSeries<(), (), ()>, expected: Option<(f64, f64)>| {
        MIN_MAX_ROWS_READ.with(|reads| reads.set(0));
        let actual = source
            .built_in_autoscale_info(20.0.into(), 100.0.into())
            .and_then(|info| info.price_range());
        assert_eq!(
            actual,
            expected.map(|(min, max)| PriceRangeImpl::new(min, max))
        );
        MIN_MAX_ROWS_READ.with(Cell::get)
    };
    let mut source = custom();
    assert_eq!(query(&source, Some((300., 900.))), 2);
    assert_eq!(query(&source, Some((300., 900.))), 0);
    custom_update(&mut source, 200., Some(2000.));
    assert_eq!(query(&source, Some((300., 900.))), 0);
    custom_update(&mut source, 10., Some(-1.));
    assert_eq!(query(&source, Some((300., 900.))), 0);
    custom_update(&mut source, 90., Some(999.));
    assert_eq!(query(&source, Some((300., 999.))), 2);
    source.apply_indices(&CustomSeriesChanges::new(
        vec![
            5.0.into(),
            15.0.into(),
            25.0.into(),
            35.0.into(),
            45.0.into(),
        ],
        None,
    ));
    assert_eq!(query(&source, Some((999., 2000.))), 3);
    source.pop_indices(&[45.0.into()]);
    assert_eq!(query(&source, Some((999., 1200.))), 2);
    source.set_data(vec![custom_row(30., 3.), custom_row(40., 4.)], None);
    assert_eq!(query(&source, Some((3., 4.))), 2);
    let extent = Rc::new(Cell::new(100.));
    let provider_extent = extent.clone();
    source.primitive_autoscale.push((
        1,
        Rc::new(move |_, _| {
            Some(AutoscaleInfo {
                price_range: Some(PriceRange {
                    min_value: -provider_extent.get(),
                    max_value: provider_extent.get(),
                }),
                margins: None,
            })
        }),
    ));
    assert_eq!(query(&source, Some((-100., 100.))), 0);
    extent.set(200.);
    assert_eq!(query(&source, Some((-200., 200.))), 0);
    source.primitive_autoscale.clear();
    assert_eq!(query(&source, Some((3., 4.))), 0);
    source.set_data(vec![], None);
    assert_eq!(query(&source, None), 0);
}
