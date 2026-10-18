use feather_charts::{
    api::Chart,
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        horz_scale_behavior_time::types::{Time, TimePoint, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{LineSeriesOptions, PriceFormat, PriceFormatCustom, SeriesOptionsMap},
    },
    renderers::price_axis_renderer::PriceAxisCommand,
};
use std::{rc::Rc, time::Instant};
#[path = "numeric_custom_series/support.rs"]
mod support;

fn time(t: f64) -> Time {
    UtcTimestamp::new(t).into()
}
fn row(t: f64, v: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: time(t),
        value: v,
        color: None,
        custom_values: None,
    }))
}
fn batch(prefix: &str, len: usize) -> Vec<String> {
    (0..len).map(|i| format!("{prefix}{i}/{len}")).collect()
}
fn custom_format() -> PriceFormat {
    PriceFormat::Custom(PriceFormatCustom {
        formatter: Rc::new(|_| "series".into()),
        tickmarks_formatter: Some(Rc::new(|v| batch("series-batch", v.len()))),
        min_move: 0.01,
        base: None,
    })
}
#[test]
fn custom_batch_formatter_reaches_public_formatter_and_real_axis() {
    for custom in [false, true] {
        let mut chart = Chart::default();
        if custom {
            let mut options: feather_charts::model::series_options::CustomSeriesOptions =
                Default::default();
            options.common.price_format = custom_format();
            let h = chart
                .add_custom_series(
                    support::View::<Time, TimePoint>::default(),
                    options,
                    0,
                    PriceScalePosition::Right,
                )
                .unwrap();
            chart
                .set_custom_data(
                    &h,
                    vec![
                        support::datum(time(1.), 10.),
                        support::datum(time(2.), 20.),
                        support::datum(time(3.), 30.),
                    ],
                )
                .unwrap();
        } else {
            let mut options = LineSeriesOptions::default();
            options.common.price_format = custom_format();
            let id = chart
                .add_series(
                    SeriesOptionsMap::Line(options),
                    0,
                    PriceScalePosition::Right,
                )
                .unwrap();
            chart
                .set_data(id, vec![row(1., 10.), row(2., 20.), row(3., 30.)])
                .unwrap();
            assert_eq!(
                chart
                    .model()
                    .series(id)
                    .unwrap()
                    .formatter()
                    .format_tickmarks(&[10., 20., 30.]),
                batch("series-batch", 3)
            );
        }
        chart.fit_content();
        let snapshot = chart
            .prepare_snapshot(iced::Size::new(800., 500.), 1., Instant::now())
            .unwrap();
        let scale = chart.model().panes()[0].right_price_scale();
        assert_eq!(
            scale.format_logical_tickmarks(&[10., 20., 30.]),
            batch("series-batch", 3)
        );
        let axis_labels: Vec<_> = snapshot.panes[0]
            .right
            .commands
            .iter()
            .filter_map(|c| match c {
                PriceAxisCommand::Text { label, .. } if label.starts_with("series-batch") => {
                    Some(label.clone())
                }
                _ => None,
            })
            .collect();
        assert!(
            !axis_labels.is_empty(),
            "batch labels must reach the actual axis, custom={custom}"
        );
    }
}

#[test]
fn percentage_and_indexed_scales_do_not_use_series_batch_labels() {
    use feather_charts::model::price_scale::{PriceScaleMode, PriceScaleOptionsPatch};
    let mut c = Chart::default();
    let mut options = LineSeriesOptions::default();
    options.common.price_format = custom_format();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(options),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(id, vec![row(1., 10.), row(2., 20.)]).unwrap();
    for (mode, expected) in [
        (PriceScaleMode::Percentage, vec!["10.00%", "20.00%"]),
        (PriceScaleMode::IndexedTo100, vec!["10.00", "20.00"]),
    ] {
        c.apply_price_scale_options(
            0,
            &PriceScalePosition::Right,
            PriceScaleOptionsPatch {
                mode: Some(mode),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            c.model().panes()[0]
                .right_price_scale()
                .format_logical_tickmarks(&[10., 20.]),
            expected
        );
    }
}

#[test]
#[should_panic(expected = "tickmarks formatter must return one label per input value")]
fn malformed_custom_batch_output_has_a_descriptive_callback_contract_failure() {
    let mut c = Chart::default();
    let mut options = LineSeriesOptions::default();
    options.common.price_format = PriceFormat::Custom(PriceFormatCustom {
        formatter: Rc::new(|_| "scalar".into()),
        tickmarks_formatter: Some(Rc::new(|_| vec![])),
        min_move: 0.01,
        base: None,
    });
    let id = c
        .add_series(
            SeriesOptionsMap::Line(options),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.model()
        .series(id)
        .unwrap()
        .formatter()
        .format_tickmarks(&[1.]);
}

#[test]
fn source_formatting_matrix() {
    use feather_charts::model::{
        horz_scale_behavior_time::horz_scale_behavior_time::HorzScaleBehaviorTime,
        localization_options::LocalizationOptions,
        price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
    };
    for custom in [false, true] {
        for (mode_number, mode) in [
            PriceScaleMode::Normal,
            PriceScaleMode::Logarithmic,
            PriceScaleMode::Percentage,
            PriceScaleMode::IndexedTo100,
        ]
        .into_iter()
        .enumerate()
        {
            for mask in [0, 1, 2, 4, 6, 8, 16, 24, 31] {
                let mut localization = LocalizationOptions::new("en-US", "dd MMM 'yy");
                if mask & 2 != 0 {
                    localization.base.price_formatter = Some(Rc::new(|_| "global-price".into()));
                }
                if mask & 4 != 0 {
                    localization.base.tickmarks_price_formatter =
                        Some(Rc::new(|v| batch("global-batch", v.len())));
                }
                if mask & 8 != 0 {
                    localization.base.percentage_formatter =
                        Some(Rc::new(|_| "global-percent".into()));
                }
                if mask & 16 != 0 {
                    localization.base.tickmarks_percentage_formatter =
                        Some(Rc::new(|v| batch("percent-batch", v.len())));
                }
                let mut c = Chart::with_custom_behavior(
                    HorzScaleBehaviorTime::default,
                    Default::default(),
                    Default::default(),
                    localization,
                )
                .unwrap();
                let mut format = custom_format();
                if mask & 1 == 0
                    && let PriceFormat::Custom(ref mut f) = format
                {
                    f.tickmarks_formatter = None;
                }
                if custom {
                    let mut opts: feather_charts::model::series_options::CustomSeriesOptions =
                        Default::default();
                    opts.common.price_format = format;
                    let h = c
                        .add_custom_series(
                            support::View::<Time, TimePoint>::default(),
                            opts,
                            0,
                            PriceScalePosition::Right,
                        )
                        .unwrap();
                    c.set_custom_data(
                        &h,
                        vec![
                            support::datum(time(1.), 10.),
                            support::datum(time(2.), 20.),
                            support::datum(time(3.), 30.),
                        ],
                    )
                    .unwrap();
                } else {
                    let mut opts = LineSeriesOptions::default();
                    opts.common.price_format = format;
                    let id = c
                        .add_series(SeriesOptionsMap::Line(opts), 0, PriceScalePosition::Right)
                        .unwrap();
                    c.set_data(id, vec![row(1., 10.), row(2., 20.), row(3., 30.)])
                        .unwrap();
                }
                c.apply_price_scale_options(
                    0,
                    &PriceScalePosition::Right,
                    PriceScaleOptionsPatch {
                        mode: Some(mode),
                        ..Default::default()
                    },
                )
                .unwrap();
                c.fit_content();
                c.prepare_snapshot(iced::Size::new(800., 500.), 1., Instant::now())
                    .unwrap();
                let scale = c.model().panes()[0].right_price_scale();
                let scalar = scale.format_logical(20.);
                let labels = scale.format_logical_tickmarks(&[10., 20., 30.]);
                let absolute = scale.format_price_absolute(20.);
                println!(
                    "FORMATTING_REFERENCE {{\"custom\":{custom},\"mode\":{mode_number},\"mask\":{mask},\"scalar\":{scalar:?},\"batch\":{labels:?},\"absolute\":{absolute:?}}}"
                );
                let price = if mask & 2 != 0 {
                    "global-price"
                } else {
                    "series"
                };
                let percentage = if mask & 8 != 0 {
                    "global-percent"
                } else {
                    "20.00%"
                };
                let expected_scalar = match mode {
                    PriceScaleMode::Percentage => percentage,
                    PriceScaleMode::IndexedTo100 => "20.00",
                    _ => price,
                };
                let expected_batch: Vec<String> = match mode {
                    PriceScaleMode::IndexedTo100 => {
                        ["10.00", "20.00", "30.00"].map(String::from).to_vec()
                    }
                    PriceScaleMode::Percentage if mask & 16 != 0 => batch("percent-batch", 3),
                    PriceScaleMode::Percentage if mask & 8 != 0 => vec!["global-percent".into(); 3],
                    PriceScaleMode::Percentage => {
                        ["10.00%", "20.00%", "30.00%"].map(String::from).to_vec()
                    }
                    _ if mask & 4 != 0 => batch("global-batch", 3),
                    _ if mask & 2 != 0 => vec!["global-price".into(); 3],
                    _ if mask & 1 != 0 => batch("series-batch", 3),
                    _ => vec!["series".into(); 3],
                };
                assert_eq!(
                    scalar, expected_scalar,
                    "custom={custom} mode={mode_number} mask={mask}"
                );
                assert_eq!(
                    labels, expected_batch,
                    "custom={custom} mode={mode_number} mask={mask}"
                );
                assert_eq!(
                    absolute, price,
                    "custom={custom} mode={mode_number} mask={mask}"
                );
            }
        }
    }
}

#[test]
fn yield_default_is_global_and_preserves_explicit_series_formatting() {
    use feather_charts::api::{Maturity, YieldCurveChart, YieldCurveOptions};
    let mut c = YieldCurveChart::new(YieldCurveOptions::default()).unwrap();
    let id = c
        .add_line_series(Default::default(), 0, PriceScalePosition::Left)
        .unwrap();
    let rows = [(1., 2.), (12., 3.)]
        .into_iter()
        .map(|(m, value)| {
            BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                time: Maturity::from_months(m).unwrap(),
                value,
                color: None,
                custom_values: None,
            }))
        })
        .collect();
    c.set_data(id, rows).unwrap();
    c.fit_content();
    c.prepare_snapshot(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    assert_eq!(
        c.model()
            .series(id)
            .unwrap()
            .formatter()
            .format_tickmarks(&[2., 3.]),
        vec!["2.00", "3.00"]
    );
    assert_eq!(
        c.model().panes()[0]
            .left_price_scale()
            .format_price_absolute(3.),
        "3.000%"
    );
}

#[test]
fn global_policy_reaches_new_panes_overlays_and_last_value_labels() {
    use feather_charts::model::{
        horz_scale_behavior_time::horz_scale_behavior_time::HorzScaleBehaviorTime,
        localization_options::LocalizationOptions, range_impl::RangeImpl,
        time_data::TimePointIndex,
    };
    let mut localization = LocalizationOptions::new("en-US", "dd MMM 'yy");
    localization.base.price_formatter = Some(Rc::new(|_| "absolute-global".into()));
    localization.base.percentage_formatter = Some(Rc::new(|_| "percentage-global".into()));
    localization.base.tickmarks_price_formatter = Some(Rc::new(|v| batch("axis-global", v.len())));
    let mut c = Chart::with_custom_behavior(
        HorzScaleBehaviorTime::default,
        Default::default(),
        Default::default(),
        localization,
    )
    .unwrap();
    let pane = c.add_pane();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            pane,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(id, vec![row(1., 10.), row(2., 20.), row(3., 30.)])
        .unwrap();
    let overlay = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            pane,
            PriceScalePosition::Overlay("named".into()),
        )
        .unwrap();
    c.set_data(overlay, vec![row(1., 10.), row(2., 20.), row(3., 30.)])
        .unwrap();
    c.fit_content();
    let snapshot = c
        .prepare_snapshot(iced::Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let pane = &c.model().panes()[pane];
    for (id, scale) in [
        (id, pane.right_price_scale()),
        (overlay, pane.price_scale_by_id("named").unwrap()),
    ] {
        assert_eq!(scale.format_price_absolute(20.), "absolute-global");
        assert_eq!(
            scale.format_logical_tickmarks(&[10., 20.]),
            batch("axis-global", 2)
        );
        let visible = RangeImpl::new(TimePointIndex::new(0.), TimePointIndex::new(2.));
        let last = c
            .model()
            .series(id)
            .unwrap()
            .last_value_data(scale, Some(&visible), true)
            .unwrap();
        assert_eq!(last.text, "absolute-global");
        assert_eq!(last.absolute, "absolute-global");
        assert_eq!(last.percentage, "percentage-global");
    }
    assert!(snapshot.panes[1].right.commands.iter().any(
        |cmd| matches!(cmd,PriceAxisCommand::Text{label,..} if label.starts_with("axis-global"))
    ));
}
