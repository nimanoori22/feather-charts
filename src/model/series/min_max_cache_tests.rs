use super::*;
use crate::model::{
    autoscale_info_impl::{AutoScaleMargins, AutoscaleInfo, PriceRange},
    plot_data::PlotRow,
    plot_list::MIN_MAX_ROWS_READ,
    series_data::{
        AreaPlotRow, BarPlotRow, BaselinePlotRow, CandlestickPlotRow, HistogramPlotRow, LinePlotRow,
    },
    series_markers::{
        MarkerCollection, MarkerCollectionId, MarkerPosition, MarkerShape, SeriesMarker,
        SeriesMarkersOptions,
    },
};

fn options(kind: SeriesType) -> SeriesOptionsMap {
    match kind {
        SeriesType::Line => SeriesOptionsMap::Line(Default::default()),
        SeriesType::Area => SeriesOptionsMap::Area(Default::default()),
        SeriesType::Baseline => SeriesOptionsMap::Baseline(Default::default()),
        SeriesType::Histogram => {
            let mut options = crate::model::series_options::HistogramSeriesOptions::default();
            options.style.base = 50.;
            SeriesOptionsMap::Histogram(options)
        }
        SeriesType::Bar => SeriesOptionsMap::Bar(Default::default()),
        SeriesType::Candlestick => SeriesOptionsMap::Candlestick(Default::default()),
        SeriesType::Custom => unreachable!(),
    }
}
fn row(kind: SeriesType, index: f64, close: f64) -> SeriesPlotRow<(), ()> {
    let base = PlotRow {
        index: index.into(),
        time: (),
        original_time: (),
        value: [close - 1., close + 3., close - 2., close],
        custom_values: None,
        original_data_count: None,
    };
    match kind {
        SeriesType::Line => SeriesPlotRow::Line(LinePlotRow { base, color: None }),
        SeriesType::Area => SeriesPlotRow::Area(AreaPlotRow {
            base,
            line_color: None,
            top_color: None,
            bottom_color: None,
        }),
        SeriesType::Baseline => SeriesPlotRow::Baseline(BaselinePlotRow {
            base,
            top_fill_color1: None,
            top_fill_color2: None,
            top_line_color: None,
            bottom_fill_color1: None,
            bottom_fill_color2: None,
            bottom_line_color: None,
        }),
        SeriesType::Histogram => SeriesPlotRow::Histogram(HistogramPlotRow { base, color: None }),
        SeriesType::Bar => SeriesPlotRow::Bar(BarPlotRow { base, color: None }),
        SeriesType::Candlestick => SeriesPlotRow::Candlestick(CandlestickPlotRow {
            base,
            color: None,
            border_color: None,
            wick_color: None,
        }),
        SeriesType::Custom => unreachable!(),
    }
}
fn series(kind: SeriesType) -> Series<(), ()> {
    let mut result = Series::new(SeriesId::new(1), kind, options(kind)).unwrap();
    result.set_data(
        (0..120)
            .map(|index| row(kind, index as f64, index as f64))
            .collect(),
        None,
    );
    result
}
fn query(series: &Series<(), ()>, start: f64, end: f64) -> (Option<AutoscaleInfoImpl>, usize) {
    MIN_MAX_ROWS_READ.with(|reads| reads.set(0));
    let info = series.built_in_autoscale_info(start.into(), end.into());
    (info, MIN_MAX_ROWS_READ.with(Cell::get))
}
fn range(series: &Series<(), ()>, start: f64, end: f64, expected: Option<(f64, f64)>) -> usize {
    let (info, reads) = query(series, start, end);
    assert_eq!(
        info.unwrap().price_range(),
        expected.map(|(min, max)| PriceRangeImpl::new(min, max))
    );
    reads
}

#[test]
fn repeated_visible_queries_and_outside_updates_do_not_rescan_old_rows() {
    for kind in [
        SeriesType::Line,
        SeriesType::Area,
        SeriesType::Baseline,
        SeriesType::Histogram,
        SeriesType::Bar,
        SeriesType::Candlestick,
    ] {
        let mut source = series(kind);
        let expected = if matches!(kind, SeriesType::Bar | SeriesType::Candlestick) {
            Some((18., 103.))
        } else {
            Some((20., 100.))
        };
        assert_eq!(
            range(&source, 20., 100., expected),
            81,
            "{kind:?}: initial scan"
        );
        assert_eq!(
            range(&source, 20., 100., expected),
            0,
            "{kind:?}: repeated query"
        );
        source.apply_rows(vec![row(kind, 120., 10000.)], None, Some(120.0.into()));
        assert_eq!(
            range(&source, 20., 100., expected),
            0,
            "{kind:?}: append beyond visible range"
        );
        source.apply_rows(vec![row(kind, 10., -10000.)], None, Some(10.0.into()));
        assert_eq!(
            range(&source, 20., 100., expected),
            0,
            "{kind:?}: replacement before visible range"
        );
        source.apply_rows(vec![], None, Some(10.0.into()));
        assert_eq!(
            range(&source, 20., 100., expected),
            0,
            "{kind:?}: removal before visible range"
        );
    }
}

#[test]
fn visible_replacement_removal_insertion_and_range_changes_recompute_correct_extrema() {
    let mut source = series(SeriesType::Line);
    assert_eq!(range(&source, 20., 100., Some((20., 100.))), 81);
    source.apply_rows(
        vec![row(SeriesType::Line, 40., -500.)],
        None,
        Some(40.0.into()),
    );
    assert_eq!(range(&source, 20., 100., Some((-500., 100.))), 81);
    source.apply_rows(vec![], None, Some(40.0.into()));
    assert_eq!(range(&source, 20., 100., Some((20., 100.))), 80);
    source.apply_rows(
        vec![row(SeriesType::Line, 40., 999.)],
        None,
        Some(40.0.into()),
    );
    assert_eq!(range(&source, 20., 100., Some((20., 999.))), 81);
    assert_eq!(range(&source, 50., 60., Some((50., 60.))), 11);
    assert_eq!(range(&source, 50., 60., Some((50., 60.))), 0);
    source.set_data(
        vec![
            row(SeriesType::Line, 50., 7.),
            row(SeriesType::Line, 60., 9.),
        ],
        None,
    );
    assert_eq!(range(&source, 50., 60., Some((7., 9.))), 2);
    source.set_data(vec![], None);
    assert_eq!(query(&source, 50., 60.), (None, 0));
    source.set_data(vec![row(SeriesType::Line, 55., -8.)], None);
    assert_eq!(range(&source, 50., 60., Some((-8., -8.))), 1);
}

#[test]
fn inclusive_visible_boundaries_invalidate_the_raw_range() {
    let mut source = series(SeriesType::Line);
    range(&source, 20., 100., Some((20., 100.)));
    source.apply_rows(
        vec![row(SeriesType::Line, 20., -123.)],
        None,
        Some(20.0.into()),
    );
    assert_eq!(range(&source, 20., 100., Some((-123., 100.))), 81);
    source.apply_rows(
        vec![row(SeriesType::Line, 100., 1234.)],
        None,
        Some(100.0.into()),
    );
    assert_eq!(range(&source, 20., 100., Some((-123., 1234.))), 81);
    source.apply_rows(vec![], None, Some(100.0.into()));
    assert_eq!(range(&source, 20., 100., Some((-123., 99.))), 80);
}

#[test]
fn cached_empty_interval_is_invalidated_when_it_receives_its_first_row() {
    let mut source = series(SeriesType::Line);
    assert_eq!(range(&source, 150., 160., None), 0);
    source.apply_rows(
        vec![row(SeriesType::Line, 200., 200.)],
        None,
        Some(200.0.into()),
    );
    assert_eq!(range(&source, 150., 160., None), 0);
    source.apply_rows(
        vec![row(SeriesType::Line, 155., 5.)],
        None,
        Some(155.0.into()),
    );
    assert_eq!(range(&source, 150., 160., Some((5., 5.))), 1);
    assert_eq!(range(&source, 150., 160., Some((5., 5.))), 0);
}

#[test]
fn histogram_base_is_applied_after_the_cached_raw_prices() {
    let mut source = series(SeriesType::Histogram);
    range(&source, 20., 100., Some((20., 100.)));
    // Options are separate from the unchanged data-derived extrema.
    let SeriesOptionsMap::Histogram(options) = &mut source.options else {
        unreachable!()
    };
    options.style.base = -50.;
    assert_eq!(range(&source, 20., 100., Some((-50., 100.))), 0);
    let SeriesOptionsMap::Histogram(options) = &mut source.options else {
        unreachable!()
    };
    options.style.base = 500.;
    assert_eq!(range(&source, 20., 100., Some((20., 500.))), 0);
}

#[test]
fn marker_margins_recalculate_without_rescanning_prices() {
    let mut source = series(SeriesType::Line);
    source.marker_collections.push(MarkerCollection::new(
        MarkerCollectionId(1),
        vec![SeriesMarker::new(
            (),
            MarkerPosition::AboveBar,
            MarkerShape::Circle,
            "red",
        )],
        SeriesMarkersOptions::default(),
    ));
    source.marker_spacing.set(6.);
    let (first, reads) = query(&source, 20., 100.);
    assert_eq!(reads, 81);
    assert_eq!(
        first.unwrap().margins(),
        Some(AutoScaleMargins {
            above: 21.,
            below: 0.
        })
    );
    source.marker_spacing.set(30.);
    let (second, reads) = query(&source, 20., 100.);
    assert_eq!(reads, 0);
    assert_eq!(
        second.unwrap().margins(),
        Some(AutoScaleMargins {
            above: 48.,
            below: 0.
        })
    );
    source.marker_collections[0].options.auto_scale = false;
    let (disabled, reads) = query(&source, 20., 100.);
    assert_eq!(reads, 0);
    assert_eq!(disabled.unwrap().margins(), None);
}

#[test]
fn primitive_provider_executes_for_each_query_and_is_not_part_of_the_raw_cache() {
    let mut source = series(SeriesType::Line);
    let extent = Rc::new(Cell::new(200.));
    let calls = Rc::new(Cell::new(0));
    let provider_extent = extent.clone();
    let provider_calls = calls.clone();
    source.primitive_autoscale.push((
        1,
        Rc::new(move |start, end| {
            assert_eq!((start, end), (20.0.into(), 100.0.into()));
            provider_calls.set(provider_calls.get() + 1);
            Some(AutoscaleInfo {
                price_range: Some(PriceRange {
                    min_value: -provider_extent.get(),
                    max_value: provider_extent.get(),
                }),
                margins: Some(AutoScaleMargins {
                    above: provider_extent.get() / 100.,
                    below: 1.,
                }),
            })
        }),
    ));
    let (first, reads) = query(&source, 20., 100.);
    assert_eq!(reads, 81);
    assert_eq!(
        first.unwrap().price_range(),
        Some(PriceRangeImpl::new(-200., 200.))
    );
    extent.set(300.);
    let (second, reads) = query(&source, 20., 100.);
    assert_eq!(reads, 0);
    let second = second.unwrap();
    assert_eq!(second.price_range(), Some(PriceRangeImpl::new(-300., 300.)));
    assert_eq!(
        second.margins(),
        Some(AutoScaleMargins {
            above: 3.,
            below: 1.
        })
    );
    assert_eq!(calls.get(), 2);
    source.primitive_autoscale.clear();
    assert_eq!(range(&source, 20., 100., Some((20., 100.))), 0);
}

#[test]
fn public_autoscale_provider_can_call_default_repeatedly_and_change_its_result() {
    let adjustment = Rc::new(Cell::new(1.));
    let calls = Rc::new(Cell::new(0));
    let provider_adjustment = adjustment.clone();
    let provider_calls = calls.clone();
    let mut source = series(SeriesType::Line);
    let SeriesOptionsMap::Line(options) = &mut source.options else {
        unreachable!()
    };
    options.common.autoscale_info_provider = Some(Box::new(move |default| {
        provider_calls.set(provider_calls.get() + 1);
        let mut info = default()?;
        assert_eq!(Some(info.clone()), default());
        let range = info.price_range.as_mut()?;
        range.min_value -= provider_adjustment.get();
        range.max_value += provider_adjustment.get();
        Some(info)
    }));
    MIN_MAX_ROWS_READ.with(|reads| reads.set(0));
    assert_eq!(
        source
            .autoscale_info(20.0.into(), 100.0.into())
            .unwrap()
            .price_range(),
        Some(PriceRangeImpl::new(19., 101.))
    );
    assert_eq!(MIN_MAX_ROWS_READ.with(Cell::get), 81);
    adjustment.set(5.);
    MIN_MAX_ROWS_READ.with(|reads| reads.set(0));
    assert_eq!(
        source
            .autoscale_info(20.0.into(), 100.0.into())
            .unwrap()
            .price_range(),
        Some(PriceRangeImpl::new(15., 105.))
    );
    assert_eq!(MIN_MAX_ROWS_READ.with(Cell::get), 0);
    assert_eq!(calls.get(), 2);
}

#[test]
fn randomized_mutations_match_uncached_ranges_for_every_builtin() {
    for kind in [
        SeriesType::Line,
        SeriesType::Area,
        SeriesType::Baseline,
        SeriesType::Histogram,
        SeriesType::Bar,
        SeriesType::Candlestick,
    ] {
        let mut source = series(kind);
        let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            seed >> 32
        };
        let mut previous = (20., 100.);
        source.built_in_autoscale_info(previous.0.into(), previous.1.into());
        for step in 0..600 {
            if step % 37 == 0 {
                let rows = if step % 74 == 0 {
                    vec![]
                } else {
                    (0..120)
                        .filter_map(|index| {
                            if next() % 4 == 0 {
                                None
                            } else {
                                let value = if next() % 9 == 0 {
                                    f64::NAN
                                } else {
                                    (next() % 1000) as f64 - 500.
                                };
                                Some(row(kind, index as f64, value))
                            }
                        })
                        .collect()
                };
                source.set_data(rows, None);
            } else {
                let index = (next() % 150) as f64 - 10.;
                let rows = if step % 5 == 0 {
                    vec![]
                } else {
                    let value = if step % 11 == 0 {
                        f64::NAN
                    } else {
                        (next() % 1000) as f64 - 500.
                    };
                    let mut replacement = row(kind, index, value);
                    if step % 7 == 0 {
                        let values = match &mut replacement {
                            SeriesPlotRow::Line(row) => &mut row.base.value,
                            SeriesPlotRow::Area(row) => &mut row.base.value,
                            SeriesPlotRow::Baseline(row) => &mut row.base.value,
                            SeriesPlotRow::Histogram(row) => &mut row.base.value,
                            SeriesPlotRow::Bar(row) => &mut row.base.value,
                            SeriesPlotRow::Candlestick(row) => &mut row.base.value,
                            SeriesPlotRow::Custom(_) => unreachable!(),
                        };
                        values[(next() % 4) as usize] = f64::NAN;
                    }
                    vec![replacement]
                };
                source.apply_rows(rows, None, Some(index.into()));
            }
            if step % 53 == 0
                && let SeriesOptionsMap::Histogram(options) = &mut source.options
            {
                options.style.base = (next() % 1000) as f64 - 500.;
            }
            let from = (next() % 150) as f64 - 10.;
            let current = (from, from + (next() % 30) as f64);
            for (start, end) in [previous, previous, current, current] {
                let plots: &[PlotRowValueIndex] =
                    if matches!(kind, SeriesType::Bar | SeriesType::Candlestick) {
                        &[PlotRowValueIndex::Low, PlotRowValueIndex::High]
                    } else {
                        &[PlotRowValueIndex::Close]
                    };
                let mut expected = source
                    .bars()
                    .min_max_on_range_cached(start.into(), end.into(), plots)
                    .map(|values| PriceRangeImpl::new(values.min, values.max));
                if !source.bars().is_empty()
                    && let SeriesOptionsMap::Histogram(options) = &source.options
                {
                    let base = PriceRangeImpl::new(options.style.base, options.style.base);
                    expected = Some(expected.map_or(base, |range| range.merge(Some(&base))));
                }
                let actual = source
                    .built_in_autoscale_info(start.into(), end.into())
                    .and_then(|info| info.price_range());
                assert_eq!(
                    actual, expected,
                    "{kind:?} step={step} range={start}..{end}"
                );
            }
            previous = current;
        }
    }
}
