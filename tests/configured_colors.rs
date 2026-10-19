use feather_charts::{
    api::{Chart, ChartError},
    model::{
        chart_model::ChartModelOptions,
        colors::{ColorParser, Rgba},
        horz_scale_behavior_time::horz_scale_behavior_time::HorzScaleBehaviorTime,
        localization_options::LocalizationOptions,
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
const DARK: &str = "color(display-p3 0.2 0.3 0.4 / 0.5)";
const LIGHT: &str = "color(display-p3 0.8 0.9 1 / 0.25)";
fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}
#[test]
fn source_parser_matrix() {
    let calls = Rc::new(RefCell::new(Vec::<String>::new()));
    let mut p = ColorParser::new(
        vec![
            Box::new({
                let c = calls.clone();
                move |v| {
                    c.borrow_mut().push(format!("first:{v}"));
                    None
                }
            }),
            Box::new({
                let c = calls.clone();
                move |v| {
                    c.borrow_mut().push(format!("second:{v}"));
                    match v {
                        DARK => Some(Rgba::new(20, 40, 60, 0.5)),
                        LIGHT => Some(Rgba::new(220, 230, 240, 0.25)),
                        _ => None,
                    }
                }
            }),
            Box::new({
                let c = calls.clone();
                move |v| {
                    c.borrow_mut().push(format!("third:{v}"));
                    None
                }
            }),
        ],
        None,
    );
    let gray = p.color_string_to_grayscale(DARK).unwrap();
    let contrast = p.generate_contrast_colors(DARK).unwrap();
    let alpha = p.apply_alpha(DARK, 0.4).unwrap();
    let transparent = p.apply_alpha("transparent", 0.3).unwrap();
    let gradient = p.gradient_color_at_percent(DARK, LIGHT, 0.5).unwrap();
    let light = p.generate_contrast_colors(LIGHT).unwrap();
    let css = p.generate_contrast_colors("#fff").unwrap();
    p.color_string_to_grayscale(DARK).unwrap();
    let mut failures = 0;
    for _ in 0..2 {
        if p.parse_color("color(display-p3 0 1 0)").is_err() {
            failures += 1;
        }
    }
    let contrast_json = |c: feather_charts::model::colors::ContrastColors| {
        format!(
            "{{\"foreground\":{},\"background\":{}}}",
            quote(&c.foreground),
            quote(&c.background)
        )
    };
    println!(
        "CONFIGURED_COLORS_REFERENCE {{\"gray\":{gray},\"contrast\":{},\"alpha\":{},\"transparent\":{},\"gradient\":{},\"light\":{},\"css\":{},\"failures\":{failures},\"calls\":[{}]}}",
        contrast_json(contrast),
        quote(&alpha),
        quote(&transparent),
        quote(&gradient),
        contrast_json(light),
        contrast_json(css),
        calls
            .borrow()
            .iter()
            .map(|v| quote(v))
            .collect::<Vec<_>>()
            .join(",")
    );
    assert_eq!(failures, 2);
    assert_eq!(calls.borrow().len(), 10);
}
#[test]
fn checked_chart_invokes_its_configured_parser() {
    let calls = Rc::new(Cell::new(0));
    let mut options = ChartModelOptions::default();
    options.layout.text_color = DARK.into();
    options.layout.color_parsers.push(Box::new({
        let calls = calls.clone();
        move |v| {
            calls.set(calls.get() + 1);
            (v == DARK).then_some(Rgba::new(20, 40, 60, 0.5))
        }
    }));
    let result: Result<Chart, ChartError> = Chart::with_custom_behavior(
        HorzScaleBehaviorTime::default,
        options,
        Default::default(),
        LocalizationOptions::new("en-US", "dd MMM yyyy"),
    );
    assert!(
        result.is_ok(),
        "configured parser rejected: {:?}",
        result.err()
    );
    assert!(calls.get() > 0);
}

#[test]
fn cloned_resolver_shares_cache_and_releases_callbacks_after_final_owner() {
    let calls = Rc::new(Cell::new(0));
    let lifetime = Rc::new(());
    let weak = Rc::downgrade(&lifetime);
    let mut options = ChartModelOptions::default();
    options.layout.color_parsers.push(Box::new({
        let calls = calls.clone();
        move |v| {
            let _keep = &lifetime;
            calls.set(calls.get() + 1);
            (v == DARK).then_some(Rgba::new(20, 40, 60, 0.5))
        }
    }));
    let chart = Chart::new(options);
    let mut a = chart.color_parser();
    let mut b = chart.color_parser();
    assert_eq!(a.parse_color(DARK).unwrap(), b.parse_color(DARK).unwrap());
    assert_eq!(calls.get(), 1);
    drop(chart);
    drop(a);
    assert!(weak.upgrade().is_some());
    assert!(b.parse_color(DARK).is_ok());
    assert_eq!(calls.get(), 1);
    drop(b);
    assert!(weak.upgrade().is_none());
}
#[test]
fn callback_can_reenter_a_shared_resolver_and_invalid_alpha_is_never_cached() {
    let slot = Rc::new(RefCell::new(None::<ColorParser>));
    let calls = Rc::new(Cell::new(0));
    let mut p = ColorParser::new(
        vec![Box::new({
            let slot = Rc::downgrade(&slot);
            let calls = calls.clone();
            move |_| {
                calls.set(calls.get() + 1);
                assert!(
                    slot.upgrade()
                        .unwrap()
                        .borrow_mut()
                        .as_mut()
                        .unwrap()
                        .parse_color("white")
                        .is_ok()
                );
                Some(Rgba::new(1, 2, 3, f32::NAN))
            }
        })],
        None,
    );
    *slot.borrow_mut() = Some(p.clone());
    assert!(p.parse_color(DARK).is_err());
    assert!(p.parse_color(DARK).is_err());
    assert_eq!(calls.get(), 2);
}

fn configured_chart() -> (Chart, Rc<Cell<usize>>) {
    let calls = Rc::new(Cell::new(0));
    let mut o = ChartModelOptions::default();
    o.layout.text_color = DARK.into();
    o.layout.background = feather_charts::model::layout_options::Background::Solid {
        color: LIGHT.into(),
    };
    o.layout.panes.separator_color = DARK.into();
    o.layout.panes.separator_hover_color = LIGHT.into();
    o.grid.vert_lines.color = DARK.into();
    o.grid.horz_lines.color = DARK.into();
    o.layout.color_parsers.push(Box::new({
        let calls = calls.clone();
        move |v| {
            calls.set(calls.get() + 1);
            match v {
                DARK => Some(Rgba::new(20, 40, 60, 0.5)),
                LIGHT => Some(Rgba::new(220, 230, 240, 0.25)),
                _ => None,
            }
        }
    }));
    (Chart::new(o), calls)
}
fn options(kind: usize, color: &str) -> feather_charts::model::series_options::SeriesOptionsMap {
    use feather_charts::model::series_options::*;
    let mut o = match kind {
        0 => {
            let mut o = LineSeriesOptions::default();
            o.style.color = color.into();
            o.style.last_price_animation = LastPriceAnimationMode::Continuous;
            SeriesOptionsMap::Line(o)
        }
        1 => {
            let mut o = AreaSeriesOptions::default();
            o.style.line_color = color.into();
            o.style.top_color = color.into();
            o.style.bottom_color = LIGHT.into();
            SeriesOptionsMap::Area(o)
        }
        2 => {
            let mut o = BaselineSeriesOptions::default();
            o.style.top_line_color = color.into();
            o.style.bottom_line_color = color.into();
            o.style.top_fill_color1 = color.into();
            o.style.top_fill_color2 = LIGHT.into();
            o.style.bottom_fill_color1 = color.into();
            o.style.bottom_fill_color2 = LIGHT.into();
            SeriesOptionsMap::Baseline(o)
        }
        3 => {
            let mut o = HistogramSeriesOptions::default();
            o.style.color = color.into();
            SeriesOptionsMap::Histogram(o)
        }
        4 => {
            let mut o = BarSeriesOptions::default();
            o.style.up_color = color.into();
            o.style.down_color = color.into();
            SeriesOptionsMap::Bar(o)
        }
        _ => {
            let mut o = CandlestickSeriesOptions::default();
            o.style.up_color = color.into();
            o.style.down_color = color.into();
            o.style.border_color = color.into();
            o.style.border_up_color = color.into();
            o.style.border_down_color = color.into();
            o.style.wick_color = color.into();
            o.style.wick_up_color = color.into();
            o.style.wick_down_color = color.into();
            SeriesOptionsMap::Candlestick(o)
        }
    };
    let common = match &mut o {
        SeriesOptionsMap::Line(o) => &mut o.common,
        SeriesOptionsMap::Area(o) => &mut o.common,
        SeriesOptionsMap::Baseline(o) => &mut o.common,
        SeriesOptionsMap::Histogram(o) => &mut o.common,
        SeriesOptionsMap::Bar(o) => &mut o.common,
        SeriesOptionsMap::Candlestick(o) => &mut o.common,
        SeriesOptionsMap::Custom(o) => &mut o.common,
    };
    common.base_line_color = DARK.into();
    common.price_line_color = DARK.into();
    o
}
fn row(
    kind: usize,
    i: usize,
    color: &str,
) -> feather_charts::model::data_consumer::BuiltInSeriesDataItem<
    feather_charts::model::horz_scale_behavior_time::types::Time,
> {
    use feather_charts::model::{data_consumer::*, horz_scale_behavior_time::types::UtcTimestamp};
    let time = UtcTimestamp::new(1700000000. + i as f64 * 60.).into();
    let color = Some(color.into());
    match kind {
        0 => BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time,
            value: i as f64 + 2.,
            color,
            custom_values: None,
        })),
        1 => BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
            time,
            value: i as f64 + 2.,
            line_color: color.clone(),
            top_color: color,
            bottom_color: Some(LIGHT.into()),
            custom_values: None,
        })),
        2 => BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(BaselineData {
            time,
            value: i as f64 + 2.,
            top_fill_color1: color.clone(),
            top_fill_color2: color.clone(),
            top_line_color: color.clone(),
            bottom_fill_color1: color.clone(),
            bottom_fill_color2: color.clone(),
            bottom_line_color: color,
            custom_values: None,
        })),
        3 => BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
            time,
            value: i as f64 + 2.,
            color,
            custom_values: None,
        })),
        4 => BuiltInSeriesDataItem::Bar(BarDataItem::Data(BarData {
            time,
            open: 2.,
            high: 5.,
            low: 1.,
            close: i as f64 + 2.,
            color,
            custom_values: None,
        })),
        _ => BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(CandlestickData {
            time,
            open: 2.,
            high: 5.,
            low: 1.,
            close: i as f64 + 2.,
            color: color.clone(),
            border_color: color.clone(),
            wick_color: color,
            custom_values: None,
        })),
    }
}
#[test]
fn all_six_series_and_row_colors_resolve_once_preserve_inputs_and_reject_invalid_changes() {
    use feather_charts::model::pane::PriceScalePosition;
    use std::time::Instant;
    for kind in 0..6 {
        let (mut chart, calls) = configured_chart();
        let id = chart
            .add_series(options(kind, DARK), 0, PriceScalePosition::Right)
            .unwrap();
        let rows = (0..3).map(|i| row(kind, i, DARK)).collect::<Vec<_>>();
        chart.set_data(id, rows.clone()).unwrap();
        chart.fit_content();
        chart
            .prepare(iced::Size::new(500., 300.), 1.25, Instant::now())
            .unwrap();
        assert_eq!(chart.series_data(id).unwrap(), rows);
        let count = calls.get();
        assert_eq!(
            count, 2,
            "only two configured tokens should invoke callback"
        );
        chart
            .prepare(iced::Size::new(300., 90.), 1.5, Instant::now())
            .unwrap();
        chart
            .prepare(iced::Size::new(500., 300.), 1.25, Instant::now())
            .unwrap();
        assert_eq!(
            calls.get(),
            count,
            "preparing another frame must reuse chart cache"
        );
        let shot = chart.screenshot_of_prepared(Default::default()).unwrap();
        assert!(
            chart
                .set_data(id, vec![row(kind, 0, "invalid-color")])
                .is_err()
        );
        assert_eq!(chart.series_data(id).unwrap(), rows);
        assert!(
            chart
                .update_data(id, row(kind, 2, "invalid-color"), false)
                .is_err()
        );
        assert_eq!(chart.series_data(id).unwrap(), rows);
        assert!(
            chart
                .replace_series_options(id, options(kind, "invalid-color"))
                .is_err()
        );
        assert!(chart.screenshot_of_prepared(Default::default()).is_ok());
        drop(chart);
        drop(shot);
    }
}

#[test]
fn configured_lines_watermarks_and_markers_validate_before_mutation() {
    use feather_charts::api::*;
    use feather_charts::model::{
        horz_scale_behavior_time::types::UtcTimestamp,
        pane::PriceScalePosition,
        series_markers::{MarkerPosition, MarkerShape, SeriesMarker},
    };
    use std::time::Instant;
    let (mut c, _) = configured_chart();
    let id = c
        .add_series(options(0, DARK), 0, PriceScalePosition::Right)
        .unwrap();
    c.set_data(id, (0..3).map(|i| row(0, i, DARK)).collect())
        .unwrap();
    c.fit_content();
    let mut line = PriceLineOptions::new(3.);
    line.color = DARK.into();
    line.axis_label_color = LIGHT.into();
    line.axis_label_text_color = DARK.into();
    let lid = c.add_price_line(id, line.clone()).unwrap();
    let mut bad = line.clone();
    bad.axis_label_text_color = "invalid-color".into();
    assert!(c.replace_price_line_options(lid, bad).is_err());
    assert_eq!(c.price_line_options(lid).unwrap(), line);
    let watermark = TextWatermarkOptions {
        lines: vec![TextWatermarkLine {
            text: "Color policy".into(),
            color: DARK.into(),
            ..Default::default()
        }],
        ..Default::default()
    };
    assert!(TextWatermark::new(watermark.clone()).is_err());
    let wm = TextWatermark::with_color_parser(watermark, c.color_parser()).unwrap();
    c.add_pane_primitive(0, wm).unwrap();
    let up = UpDownMarkersOptions {
        positive_color: LIGHT.into(),
        negative_color: DARK.into(),
        ..Default::default()
    };
    c.add_up_down_markers(id, up).unwrap();
    c.add_series_markers(
        id,
        vec![SeriesMarker {
            time: UtcTimestamp::new(1700000060.).into(),
            position: MarkerPosition::AboveBar,
            shape: MarkerShape::Circle,
            color: DARK.into(),
            id: None,
            text: Some("custom".into()),
            size: 1.,
        }],
        Default::default(),
    )
    .unwrap();
    c.prepare(iced::Size::new(500., 300.), 1.5, Instant::now())
        .unwrap();
    assert_eq!(c.price_line_options(lid).unwrap().color, DARK);
}

#[test]
fn p3_targets_are_explicit_errors_at_checked_legacy_and_atomic_runtime_boundaries() {
    use feather_charts::{
        api::*,
        model::{chart_model::ChartModelError, layout_options::ColorSpace},
    };
    use std::time::Instant;
    let p3 = || {
        let mut o = ChartModelOptions::default();
        o.layout.color_space = ColorSpace::DisplayP3;
        o
    };
    let unsupported = |error: ChartError| {
        assert!(matches!(
            error,
            ChartError::Model(ChartModelError::UnsupportedColorSpace(
                ColorSpace::DisplayP3
            ))
        ))
    };
    unsupported(Chart::try_new(p3()).err().unwrap());
    let called = Cell::new(0);
    unsupported(
        Chart::<HorzScaleBehaviorTime>::with_custom_behavior(
            || {
                called.set(called.get() + 1);
                HorzScaleBehaviorTime::default()
            },
            p3(),
            Default::default(),
            LocalizationOptions::new("en-US", "dd MMM yyyy"),
        )
        .err()
        .unwrap(),
    );
    assert_eq!(called.get(), 0);
    unsupported(
        OptionsChart::with_options_chart_options(
            Default::default(),
            p3(),
            Default::default(),
            LocalizationOptions::new("en-US", ""),
        )
        .err()
        .unwrap(),
    );
    unsupported(
        YieldCurveChart::with_chart_options(
            Default::default(),
            p3(),
            LocalizationOptions::new("en-US", ""),
        )
        .err()
        .unwrap(),
    );
    let mut legacy = Chart::new(p3());
    unsupported(
        legacy
            .prepare_snapshot(iced::Size::new(300., 200.), 1., Instant::now())
            .err()
            .unwrap(),
    );
    unsupported(
        legacy
            .prepare(iced::Size::new(300., 200.), 1., Instant::now())
            .err()
            .unwrap(),
    );
    assert!(legacy.screenshot_of_prepared(Default::default()).is_err());
    legacy
        .apply_chart_options(ChartOptionsPatch {
            model: ChartModelOptionsPatch {
                layout: Some(LayoutOptionsPatch {
                    color_space: Some(ColorSpace::Srgb),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    legacy
        .prepare(iced::Size::new(300., 200.), 1., Instant::now())
        .unwrap();
    let (mut c, calls) = configured_chart();
    let before = c.runtime_options();
    let count = calls.get();
    unsupported(
        c.apply_chart_options(ChartOptionsPatch {
            model: ChartModelOptionsPatch {
                layout: Some(LayoutOptionsPatch {
                    color_space: Some(ColorSpace::DisplayP3),
                    text_color: Some(DARK.into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
        .err()
        .unwrap(),
    );
    assert_eq!(c.runtime_options(), before);
    assert_eq!(calls.get(), count);
}

#[path = "configured_colors/custom.rs"]
mod custom;

#[test]
fn browser_non_rgb_fallback_forms() {
    let tokens = [
        "color(display-p3 1 0 0)",
        "color(srgb 0.2 0.3 0.4)",
        "color(srgb-linear 0.2 0.3 0.4)",
        "lab(50% 0 0)",
        "lch(50% 0 0)",
        "oklab(50% 0 0)",
        "oklch(50% 0 0)",
        "hsl(120 50% 50%)",
        "hwb(120 10% 20%)",
        "#abcdef",
    ];
    let rows = tokens
        .into_iter()
        .enumerate()
        .map(|(index, token)| {
            let calls = Rc::new(Cell::new(0));
            let mut p = ColorParser::new(
                vec![Box::new({
                    let calls = calls.clone();
                    move |_| {
                        calls.set(calls.get() + 1);
                        Some(Rgba::new(10, 20, 30, 0.5))
                    }
                })],
                None,
            );
            let gray = p.color_string_to_grayscale(token).unwrap();
            assert_eq!(calls.get(), usize::from(index < 7), "{token}");
            format!(
                "{{\"token\":{},\"gray\":{gray},\"calls\":{}}}",
                quote(token),
                calls.get()
            )
        })
        .collect::<Vec<_>>();
    println!("CONFIGURED_COLORS_FORMS [{}]", rows.join(","));
}

#[test]
fn external_snapshot_conversion_accepts_the_same_chart_policy() {
    let (mut chart, _) = configured_chart();
    let id = chart
        .add_series(
            options(0, DARK),
            0,
            feather_charts::model::pane::PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(id, (0..3).map(|i| row(0, i, DARK)).collect())
        .unwrap();
    chart.fit_content();
    let snapshot = chart
        .prepare_snapshot(iced::Size::new(400., 300.), 1.25, std::time::Instant::now())
        .unwrap();
    feather_charts::ui::multi_pane_frame::IcedMultiPaneChartFrame::from_snapshot_with_colors(
        snapshot,
        &Default::default(),
        DARK,
        &chart.color_parser(),
    )
    .unwrap();
}
