use feather_charts::{
    api::Chart,
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{LastPriceAnimationMode, LineSeriesOptions, SeriesOptionsMap},
        time_data::LogicalRange,
        time_scale_options::HorzScaleOptionsPatch,
    },
};
use std::time::Instant;
fn row(i: usize, value: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: UtcTimestamp::new(1_700_000_000. + i as f64 * 60.).into(),
        value,
        color: None,
        custom_values: None,
    }))
}
#[test]
fn enabling_last_price_animation_changes_prepared_output() {
    let make = |mode| {
        let mut chart = Chart::default();
        let mut options = LineSeriesOptions::default();
        options.style.last_price_animation = mode;
        let id = chart
            .add_series(
                SeriesOptionsMap::Line(options),
                0,
                PriceScalePosition::Right,
            )
            .unwrap();
        chart
            .set_data(id, (0..32).map(|i| row(i, i as f64)).collect())
            .unwrap();
        chart.fit_content();
        chart
            .prepare(iced::Size::new(800., 500.), 1., Instant::now())
            .unwrap();
        chart
    };
    assert_ne!(
        pixels(&make(LastPriceAnimationMode::Disabled)),
        pixels(&make(LastPriceAnimationMode::Continuous)),
        "enabled animation must draw a marker"
    );
}

fn pixels(chart: &Chart) -> Vec<u8> {
    frame_pixels(chart.frame())
}
fn frame_pixels(snapshot: &feather_charts::ui::chart_frame::IcedChartFrame) -> Vec<u8> {
    use iced::advanced::{
        graphics::geometry::Renderer as _,
        renderer::{Headless, Renderer as _},
    };
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some("tiny-skia"),
    ))
    .unwrap();
    let bounds = iced::Rectangle::with_size(iced::Size::new(800., 500.));
    renderer.reset(bounds);
    let mut canvas = iced::widget::canvas::Frame::new(&renderer, bounds.size());
    snapshot.draw(&mut canvas);
    renderer.draw_geometry(canvas.into_geometry());
    renderer.screenshot(iced::Size::new(800, 500), 1., iced::Color::BLACK)
}

use feather_charts::{
    api::ChartMessage,
    model::{data_consumer::*, series_options::*},
};
use std::{cell::Cell, rc::Rc, time::Duration};
fn options(kind: usize, mode: LastPriceAnimationMode) -> SeriesOptionsMap {
    match kind {
        0 => {
            let mut o = LineSeriesOptions::default();
            o.style.last_price_animation = mode;
            SeriesOptionsMap::Line(o)
        }
        1 => {
            let mut o = AreaSeriesOptions::default();
            o.style.last_price_animation = mode;
            SeriesOptionsMap::Area(o)
        }
        _ => {
            let mut o = BaselineSeriesOptions::default();
            o.style.last_price_animation = mode;
            o.style.base_value = BaseValueType::Price(BaseValuePrice { price: 100. });
            SeriesOptionsMap::Baseline(o)
        }
    }
}
fn sample(kind: usize, i: usize, value: f64) -> BuiltInSeriesDataItem<Time> {
    let time = UtcTimestamp::new(1_700_000_000. + i as f64 * 60.).into();
    match kind {
        0 => row(i, value),
        1 => BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
            time,
            value,
            line_color: Some("rgba(255,0,0,0.5)".into()),
            top_color: None,
            bottom_color: None,
            custom_values: None,
        })),
        _ => BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(BaselineData {
            time,
            value,
            top_line_color: Some("purple".into()),
            bottom_line_color: Some("purple".into()),
            top_fill_color1: None,
            top_fill_color2: None,
            bottom_fill_color1: None,
            bottom_fill_color2: None,
            custom_values: None,
        })),
    }
}
fn prepare(chart: &mut Chart, now: Instant, ratio: f32) {
    chart
        .prepare(iced::Size::new(800., 500.), ratio, now)
        .unwrap();
}
#[test]
fn all_families_canonical_endpoint_dpr_expiry_and_retained_frames() {
    for kind in 0..3 {
        for ratio in [1., 1.25, 1.5, 2.] {
            let mut c = Chart::default();
            let id = c
                .add_series(
                    options(kind, LastPriceAnimationMode::OnDataUpdate),
                    0,
                    PriceScalePosition::Right,
                )
                .unwrap();
            c.set_data(
                id,
                (0..8192)
                    .map(|i| sample(kind, i, 100. + (i % 31) as f64))
                    .collect(),
            )
            .unwrap();
            c.apply_time_scale_options(HorzScaleOptionsPatch {
                enable_conflation: Some(true),
                min_bar_spacing: Some(0.001),
                ..Default::default()
            });
            c.fit_content();
            prepare(&mut c, Instant::now(), ratio);
            let a = &c.frame().plot.last_price_animations[0];
            assert_eq!(a.fill_color.a, 0.); // initial full population is not a realtime event
            assert!(!c.is_animating());
            c.update_data(id, sample(kind, 8191, 111.), false).unwrap();
            let now = Instant::now();
            prepare(&mut c, now, ratio);
            assert!(c.is_animating());
            let old = c.frame().clone();
            let a = &old.plot.last_price_animations[0];
            let scale = c.model().panes()[0].default_price_scale();
            let last = c
                .model()
                .series(id)
                .unwrap()
                .last_value_data(
                    scale,
                    Some(&feather_charts::model::range_impl::RangeImpl::new(
                        0.0.into(),
                        8191.0.into(),
                    )),
                    true,
                )
                .unwrap();
            assert!((a.center.y - last.coordinate as f32).abs() < 0.001);
            let x = c
                .model()
                .time_scale()
                .index_to_coordinate(8191.0.into())
                .value();
            let correction = (ratio.floor().max(1.) % 2.) / 2.;
            assert!(
                (a.center.x - (((x * f64::from(ratio) + 0.5).floor() as f32 + correction) / ratio))
                    .abs()
                    < 0.001
            );
            assert_eq!(a.stroke_width, ratio.floor().max(1.) / ratio);
            assert_eq!(a.dot_radius, 4.5);
            println!(
                "PRICE_ENDPOINT {{\"dpr\":{ratio},\"x\":{x},\"y\":{},\"center_x\":{},\"center_y\":{},\"dot\":{},\"radius\":{},\"stroke\":{}}}",
                last.coordinate, a.center.x, a.center.y, a.dot_radius, a.radius, a.stroke_width
            );
            if kind == 1 {
                assert_eq!(a.color, iced::Color::from_rgba(1., 0., 0., 0.5));
                assert!(a.fill_color.a <= 0.125);
            }
            if kind == 2 {
                assert_ne!(a.color, iced::Color::from_rgb8(128, 0, 128));
            } // upstream barColor uses baseline side, not row stroke overrides
            let before = a.radius;
            c.update(ChartMessage::Tick(now + Duration::from_millis(325)))
                .unwrap();
            assert!(c.frame().plot.last_price_animations[0].radius > before);
            assert_eq!(old.plot.last_price_animations[0].radius, before);
            c.update(ChartMessage::Tick(now + Duration::from_secs(3)))
                .unwrap();
            assert!(!c.is_animating());
            assert_eq!(c.frame().plot.last_price_animations[0].stroke_color.a, 0.);
            c.set_logical_range(LogicalRange {
                from: 0.0.into(),
                to: 100.0.into(),
            });
            prepare(&mut c, now, ratio);
            assert!(c.frame().plot.last_price_animations.is_empty());
            assert!(!c.is_animating());
            c.set_data(id, vec![]).unwrap();
            prepare(&mut c, now, ratio);
            assert!(c.frame().plot.last_price_animations.is_empty());
            c.remove_series(id).unwrap();
            prepare(&mut c, now, ratio);
            assert_eq!(old.plot.last_price_animations.len(), 1);
        }
    }
}
#[test]
fn animation_ticks_do_no_axis_formatting_and_keep_base_geometry() {
    for count in [8192, 100_000] {
        let mut c = Chart::default();
        let calls = Rc::new(Cell::new(0));
        let captured = calls.clone();
        let mut o = LineSeriesOptions::default();
        o.style.last_price_animation = LastPriceAnimationMode::Continuous;
        o.common.price_format = PriceFormat::Custom(PriceFormatCustom {
            formatter: Rc::new(move |p| {
                captured.set(captured.get() + 1);
                format!("{}", p.value())
            }),
            tickmarks_formatter: None,
            min_move: 0.01,
            base: Some(100.),
        });
        let id = c
            .add_series(SeriesOptionsMap::Line(o), 0, PriceScalePosition::Right)
            .unwrap();
        c.set_data(
            id,
            (0..count).map(|i| row(i, 100. + (i % 31) as f64)).collect(),
        )
        .unwrap();
        c.apply_time_scale_options(HorzScaleOptionsPatch {
            enable_conflation: Some(true),
            min_bar_spacing: Some(0.001),
            ..Default::default()
        });
        c.fit_content();
        let now = Instant::now();
        prepare(&mut c, now, 1.);
        c.update(ChartMessage::Tick(now)).unwrap();
        calls.set(0);
        let ptr = c.frame().plot.lines.as_ptr();
        for tick in 1..=100 {
            assert!(
                c.update(ChartMessage::Tick(
                    now + Duration::from_micros(tick * 32500)
                ))
                .unwrap()
                .is_empty()
            );
        }
        assert_eq!(calls.get(), 0, "stage ticks must not prepare axes");
        assert_eq!(
            ptr,
            c.frame().plot.lines.as_ptr(),
            "stage ticks must retain base geometry"
        );
        c.update_data(id, row(count - 1, 200.), false).unwrap();
        c.update(ChartMessage::Tick(now + Duration::from_secs(4)))
            .unwrap();
        assert!(
            calls.get() > 0,
            "data invalidation must bypass the fast path"
        );
    }
}

#[test]
fn source_update_flags() {
    let mut c = Chart::default();
    let id = c
        .add_series(
            options(0, LastPriceAnimationMode::OnDataUpdate),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    for (op, times, historical, whitespace) in [
        ("set", vec![0, 1, 2], false, false),
        ("set", vec![0, 1, 2], false, false),
        ("set", vec![1, 2, 3], false, false),
        ("set", vec![0, 1, 2], false, false),
        ("update", vec![1], true, false),
        ("update", vec![3], false, true),
        ("update", vec![3], false, false),
        ("set", vec![], false, false),
        ("set", vec![0, 1, 2], false, false),
    ] {
        let mut error = false;
        if op == "set" {
            c.set_data(id, times.iter().map(|i| row(*i, 100.)).collect())
                .unwrap();
        } else if whitespace {
            c.update_data(
                id,
                BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(WhitespaceData {
                    time: UtcTimestamp::new(1_700_000_000. + times[0] as f64 * 60.).into(),
                    custom_values: None,
                })),
                historical,
            )
            .unwrap();
        } else {
            error = c.update_data(id, row(times[0], 100.), historical).is_err();
        }
        assert!(!error);
        let flag = c
            .model()
            .series(id)
            .unwrap()
            .last_update_info()
            .map(|i| i.last_bar_updated_or_new_bars_added_to_the_right);
        println!(
            "PRICE_FLAG {{\"op\":\"{op}\",\"times\":{times:?},\"historical\":{historical},\"whitespace\":{whitespace},\"flag\":{}}}",
            flag.map_or("null".into(), |v| v.to_string())
        );
    }
}
#[test]
fn scale_modes_visibility_option_changes_and_multi_pane_ticks() {
    use feather_charts::model::{
        axis_snapshots::PriceAxisSide,
        price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
    };
    let mut c = Chart::default();
    let pane = c.add_pane();
    let mut ids = vec![];
    for (kind, pane, side) in [
        (0, 0, PriceScalePosition::Left),
        (1, 0, PriceScalePosition::Right),
        (2, pane, PriceScalePosition::Right),
    ] {
        let id = c
            .add_series(
                options(kind, LastPriceAnimationMode::Continuous),
                pane,
                side,
            )
            .unwrap();
        c.set_data(
            id,
            (0..32).map(|i| sample(kind, i, 100. + i as f64)).collect(),
        )
        .unwrap();
        ids.push(id);
    }
    c.fit_content();
    let now = Instant::now();
    for mode in [
        PriceScaleMode::Normal,
        PriceScaleMode::Logarithmic,
        PriceScaleMode::Percentage,
        PriceScaleMode::IndexedTo100,
    ] {
        for invert in [false, true] {
            for (pane, side) in [
                (0, PriceAxisSide::Left),
                (0, PriceAxisSide::Right),
                (1, PriceAxisSide::Right),
            ] {
                c.apply_price_axis_options(
                    pane,
                    side,
                    PriceScaleOptionsPatch {
                        mode: Some(mode),
                        invert_scale: Some(invert),
                        ..Default::default()
                    },
                )
                .unwrap();
            }
            prepare(&mut c, now, 1.25);
            let frame = c.multi_frame().unwrap();
            assert_eq!(frame.panes[0].plot.last_price_animations.len(), 2);
            assert_eq!(frame.panes[1].plot.last_price_animations.len(), 1);
            assert!(
                frame
                    .panes
                    .iter()
                    .flat_map(|p| &p.plot.last_price_animations)
                    .all(|a| a.center.x.is_finite() && a.center.y.is_finite())
            );
            c.update(ChartMessage::Tick(now + Duration::from_millis(325)))
                .unwrap();
        }
    }
    let mut hidden = LineSeriesOptions::default();
    hidden.common.visible = false;
    hidden.style.last_price_animation = LastPriceAnimationMode::Continuous;
    c.replace_series_options(ids[0], SeriesOptionsMap::Line(hidden))
        .unwrap();
    c.update(ChartMessage::Tick(now)).unwrap();
    assert_eq!(
        c.multi_frame().unwrap().panes[0]
            .plot
            .last_price_animations
            .len(),
        1
    );
    c.replace_series_options(ids[1], options(1, LastPriceAnimationMode::Disabled))
        .unwrap();
    c.update(ChartMessage::Tick(now)).unwrap();
    assert!(
        c.multi_frame().unwrap().panes[0]
            .plot
            .last_price_animations
            .is_empty()
    );
    c.prepare(iced::Size::new(300., 90.), 1., now).unwrap();
    c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
    assert_eq!(
        c.multi_frame().unwrap().panes[1]
            .plot
            .last_price_animations
            .len(),
        1
    );
    c.remove_series(ids[2]).unwrap();
    c.update(ChartMessage::Tick(now)).unwrap();
    assert!(!c.is_animating());
}

#[test]
fn top_markers_cover_animation_and_do_not_change_hits() {
    use feather_charts::api::{
        MarkerPosition, MarkerShape, MarkerZOrder, SeriesMarker, SeriesMarkersOptions,
    };
    let mut c = Chart::default();
    let id = c
        .add_series(
            options(0, LastPriceAnimationMode::Continuous),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(id, (0..32).map(|i| row(i, 100. + (i % 7) as f64)).collect())
        .unwrap();
    c.add_series_markers(
        id,
        vec![SeriesMarker::new(
            UtcTimestamp::new(1_700_000_000. + 31. * 60.).into(),
            MarkerPosition::InBar,
            MarkerShape::Circle,
            "red",
        )],
        SeriesMarkersOptions {
            auto_scale: false,
            z_order: MarkerZOrder::Top,
        },
    )
    .unwrap();
    c.fit_content();
    prepare(&mut c, Instant::now(), 1.);
    let old = c.frame().clone();
    let a = &old.plot.last_price_animations[0];
    let px = (a.center.x + old.layout.plot.origin.x as f32).round() as usize;
    let py = (a.center.y + old.layout.plot.origin.y as f32).round() as usize;
    let offset = (py * 800 + px) * 4;
    let mut reference = old.clone();
    reference.plot.last_price_animations.clear();
    let marker = frame_pixels(&reference);
    assert_eq!(&marker[offset..offset + 3], &[255, 0, 0]);
    assert_eq!(
        &frame_pixels(&old)[offset..offset + 4],
        &marker[offset..offset + 4],
        "top marker must cover the last-price dot"
    );
    let hit = c.hit_test(0, a.center);
    c.update(ChartMessage::Tick(
        Instant::now() + Duration::from_millis(325),
    ))
    .unwrap();
    assert_eq!(hit, c.hit_test(0, a.center));
}

#[test]
fn price_pulses_do_not_starve_scroll_or_conflation_warmup() {
    let mut c = Chart::default();
    let id = c
        .add_series(
            options(0, LastPriceAnimationMode::Continuous),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(
        id,
        (0..8192).map(|i| row(i, 100. + (i % 31) as f64)).collect(),
    )
    .unwrap();
    c.apply_time_scale_options(HorzScaleOptionsPatch {
        enable_conflation: Some(true),
        min_bar_spacing: Some(0.001),
        precompute_conflation_on_init: Some(true),
        ..Default::default()
    });
    c.fit_content();
    let now = Instant::now();
    prepare(&mut c, now, 1.);
    c.scroll_to_offset_animated(10., Duration::from_secs(1));
    c.update(ChartMessage::Tick(now)).unwrap();
    let initial = c.model().time_scale().right_offset();
    c.update(ChartMessage::Tick(now + Duration::from_millis(500)))
        .unwrap();
    let middle = c.model().time_scale().right_offset();
    assert!(middle > initial);
    c.update(ChartMessage::Tick(now + Duration::from_secs(1)))
        .unwrap();
    assert_eq!(c.model().time_scale().right_offset(), 10.);
    // Background warmup is finite even while the continuous pulse keeps requesting ticks.
    for i in 0..150 {
        c.update(ChartMessage::Tick(
            now + Duration::from_millis(1100 + i * 16),
        ))
        .unwrap();
    }
    c.replace_series_options(id, options(0, LastPriceAnimationMode::Disabled))
        .unwrap();
    c.update(ChartMessage::Tick(now + Duration::from_secs(5)))
        .unwrap();
    assert!(
        !c.is_animating(),
        "warmup should have completed alongside the pulse"
    );
}

#[test]
fn rejected_updates_and_shared_reindex_preserve_the_existing_pulse() {
    let mut c = Chart::default();
    let id = c
        .add_series(
            options(0, LastPriceAnimationMode::OnDataUpdate),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(id, (0..32).map(|i| row(i, 100. + i as f64)).collect())
        .unwrap();
    c.fit_content();
    prepare(&mut c, Instant::now(), 1.);
    c.update_data(id, row(31, 120.), false).unwrap();
    let now = Instant::now();
    prepare(&mut c, now, 1.);
    let before = c.frame().plot.last_price_animations[0].clone();
    assert!(c.update_data(id, row(31, f64::NAN), false).is_err());
    assert!(c.update_data(id, row(1, 150.), false).is_err());
    c.update(ChartMessage::Tick(now + Duration::from_millis(325)))
        .unwrap();
    let expected = c.frame().plot.last_price_animations[0].radius;
    assert!(expected > before.radius);
    let other = c
        .add_series(
            options(0, LastPriceAnimationMode::Disabled),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(
        other,
        vec![BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time: UtcTimestamp::new(1_699_999_000.).into(),
            value: 99.,
            color: None,
            custom_values: None,
        }))],
    )
    .unwrap();
    prepare(&mut c, now + Duration::from_millis(325), 1.);
    assert_eq!(
        c.frame().plot.last_price_animations[0].radius,
        expected,
        "shared index shift must not trigger another pulse"
    );
    // Historical fulfilled updates qualify, but a rejected conflated update cannot restart activity.
    c.update_data(id, row(1, 130.), true).unwrap();
    c.apply_time_scale_options(HorzScaleOptionsPatch {
        enable_conflation: Some(true),
        min_bar_spacing: Some(0.001),
        bar_spacing: Some(0.125),
        ..Default::default()
    });
    assert!(c.update_data(id, row(1, 140.), true).is_err());
    c.apply_time_scale_options(HorzScaleOptionsPatch {
        bar_spacing: Some(6.),
        ..Default::default()
    });
    c.fit_content();
    prepare(&mut c, Instant::now() + Duration::from_secs(4), 1.);
    assert!(!c.is_animating());
    c.set_data(id, (0..32).map(|i| row(i, 100. + i as f64)).collect())
        .unwrap();
    prepare(&mut c, Instant::now(), 1.);
    assert!(
        c.is_animating(),
        "replacement with matching fulfilled bounds should pulse"
    );
    c.set_data(id, vec![]).unwrap();
    prepare(&mut c, Instant::now(), 1.);
    assert!(c.frame().plot.last_price_animations.is_empty());
    c.set_data(id, (0..32).map(|i| row(i, 100. + i as f64)).collect())
        .unwrap();
    prepare(&mut c, Instant::now(), 1.);
    assert!(
        !c.is_animating(),
        "population after clear has no previous fulfilled bounds"
    );
    assert_eq!(c.frame().plot.last_price_animations.len(), 1);
}

#[test]
fn dot_follows_line_width_even_when_the_line_is_hidden() {
    use feather_charts::renderers::draw_line::LineWidth;
    for (width, radius) in [
        (LineWidth::One, 2.),
        (LineWidth::Two, 3.),
        (LineWidth::Three, 4.5),
        (LineWidth::Four, 6.),
    ] {
        let mut c = Chart::default();
        let mut o = LineSeriesOptions::default();
        o.style.last_price_animation = LastPriceAnimationMode::Continuous;
        o.style.line_visible = false;
        o.style.line_width = width;
        let id = c
            .add_series(SeriesOptionsMap::Line(o), 0, PriceScalePosition::Right)
            .unwrap();
        c.set_data(id, vec![row(0, 100.), row(1, 101.)]).unwrap();
        c.fit_content();
        prepare(&mut c, Instant::now(), 1.5);
        assert!(c.frame().plot.lines.is_empty());
        assert_eq!(c.frame().plot.last_price_animations[0].dot_radius, radius);
    }
}
