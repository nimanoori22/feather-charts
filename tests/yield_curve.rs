use feather_charts::model::time_scale_host::TimeScaleHost;
use feather_charts::{
    api::{
        ChartError, ChartEvent, ChartMessage, MarkerSign, Maturity, UpDownMarker, YieldCurveChart,
        YieldCurveOptions,
    },
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        pane::PriceScalePosition,
        time_data::TimePointIndex,
    },
};
use iced::Size;
use std::time::{Duration, Instant};
fn m(v: f64) -> Maturity {
    Maturity::from_months(v).unwrap()
}
fn row(t: f64, v: f64) -> BuiltInSeriesDataItem<Maturity> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: m(t),
        value: v,
        color: None,
        custom_values: None,
    }))
}
fn chart() -> (YieldCurveChart, feather_charts::model::data_layer::SeriesId) {
    let mut c = YieldCurveChart::new(Default::default()).unwrap();
    let id = c
        .add_line_series(Default::default(), 0, PriceScalePosition::Left)
        .unwrap();
    c.set_data(
        id,
        vec![row(1., 5.), row(6., 4.8), row(12., 4.5), row(120., 4.)],
    )
    .unwrap();
    c.fit_content();
    (c, id)
}
#[test]
fn sparse_curve_draws_and_source_defaults_keep_private_scaffold_out_of_public_data() {
    let (mut c, id) = chart();
    c.prepare(Size::new(800., 500.), 1.25, Instant::now())
        .unwrap();
    assert_eq!(c.series_ids().collect::<Vec<_>>(), vec![id]);
    assert!(!c.frame().plot.lines.is_empty());
    assert_eq!(c.model().series(id).unwrap().bars().rows().len(), 4);
    assert!(
        c.model()
            .time_scale()
            .time_to_index(&m(60.), false)
            .is_some()
    );
    assert!(!c.model().fulfilled_time_indices().contains(&60));
    let owner = c.model().series(id).unwrap();
    assert!(!owner.common_options().last_value_visible);
    assert!(!owner.common_options().price_line_visible);
}
#[test]
fn failed_writes_preserve_data_domain_and_marker_deadlines() {
    let (mut c, id) = chart();
    let now = Instant::now();
    let markers = c.add_up_down_markers(id, Default::default()).unwrap();
    c.set_up_down_data(markers, vec![row(1., 5.), row(120., 4.)])
        .unwrap();
    c.update_up_down_data(markers, row(1., 5.1), true, now)
        .unwrap();
    let before = c.up_down_markers(markers, now).unwrap();
    let deadline = c.next_up_down_expiration();
    for rows in [
        vec![row(2., 5.), row(1., 5.)],
        vec![row(200000., 4.)],
        vec![row(1., f64::NAN)],
    ] {
        assert!(c.set_up_down_data(markers, rows).is_err());
        assert_eq!(c.up_down_markers(markers, now).unwrap(), before);
        assert_eq!(c.next_up_down_expiration(), deadline);
        assert_eq!(c.model().series(id).unwrap().bars().rows().len(), 2);
    }
    assert!(matches!(
        c.replace_yield_curve_options(YieldCurveOptions {
            base_resolution: f64::NAN,
            ..Default::default()
        }),
        Err(ChartError::YieldCurve(_))
    ));
    c.prepare(Size::new(800., 500.), 1., now).unwrap();
    assert!(
        c.model()
            .time_scale()
            .time_to_index(&m(200000.), false)
            .is_none()
    );
}
#[test]
fn pointer_payloads_are_numeric_and_crosshair_ignores_scaffold_only_maturities() {
    let (mut c, _) = chart();
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    for maturity in [6., 8.] {
        let scale = c.model().time_scale();
        let index = scale.time_to_index(&m(maturity), false).unwrap();
        let x = scale.index_to_coordinate(index).value() as f32;
        let events = c
            .update(ChartMessage::Click {
                pane: 0,
                position: iced::Point::new(x, 20.),
            })
            .unwrap();
        let event = events
            .iter()
            .find_map(|e| {
                if let ChartEvent::Click(p) = e {
                    Some(p)
                } else {
                    None
                }
            })
            .unwrap();
        assert!(event.time.is_some());
        if maturity == 6. {
            assert_eq!(event.time, Some(m(6.)));
        }
        assert_ne!(
            event.time,
            Some(m(8.)),
            "support-only points are not crosshair targets"
        );
        assert_eq!(event.series_data.len(), 1);
    }
}
#[test]
fn numeric_markers_expire_exactly_and_manual_markers_persist() {
    let (mut c, id) = chart();
    let now = Instant::now();
    let markers = c.add_up_down_markers(id, Default::default()).unwrap();
    c.set_up_down_data(markers, vec![row(1., 5.), row(6., 4.8), row(12., 4.5)])
        .unwrap();
    c.update_up_down_data(markers, row(6., 4.9), true, now)
        .unwrap();
    assert_eq!(
        c.up_down_markers(markers, now).unwrap()[0].sign,
        MarkerSign::Positive
    );
    c.prepare(Size::new(800., 500.), 1., now).unwrap();
    assert!(!c.frame().plot.primitives.is_empty());
    assert_eq!(
        c.up_down_markers(markers, now + Duration::from_millis(4999))
            .unwrap()
            .len(),
        1
    );
    c.update(ChartMessage::Tick(now + Duration::from_secs(5)))
        .unwrap();
    assert!(
        c.up_down_markers(markers, now + Duration::from_secs(5))
            .unwrap()
            .is_empty()
    );
    c.set_up_down_markers(
        markers,
        vec![UpDownMarker {
            time: m(6.),
            value: 4.8,
            sign: MarkerSign::Neutral,
        }],
    )
    .unwrap();
    assert_eq!(
        c.up_down_markers(markers, now + Duration::from_secs(100))
            .unwrap()
            .len(),
        1
    );
    c.remove_series(id).unwrap();
    assert!(c.next_up_down_expiration().is_none());
    assert!(c.up_down_markers(markers, now).is_err());
    assert!(
        c.model()
            .time_scale()
            .index_to_time(TimePointIndex::new(120.))
            .is_some()
    );
}

#[test]
fn changing_maturity_formatter_invalidates_axis_labels_and_can_restore_defaults() {
    use std::rc::Rc;
    let (mut c, _) = chart();
    let now = Instant::now();
    let size = Size::new(800., 500.);
    c.prepare(size, 1., now).unwrap();
    c.set_maturity_formatter(Some(Rc::new(|v| format!("Tenor {}", v.months()))));
    let custom = c.prepare_snapshot(size, 1., now).unwrap();
    let labels = custom.panes[0]
        .time
        .commands
        .iter()
        .filter_map(|command| match command {
            feather_charts::renderers::time_axis_renderer::TimeAxisCommand::Text {
                label, ..
            } => Some(label),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(!labels.is_empty());
    assert!(labels.iter().all(|s| s.starts_with("Tenor ")));
    c.set_maturity_formatter(None);
    let restored = c.prepare_snapshot(size, 1., now).unwrap();
    assert!(restored.panes[0].time.commands.iter().any(|command|matches!(command,feather_charts::renderers::time_axis_renderer::TimeAxisCommand::Text{label,..} if label.ends_with('M')||label.ends_with('Y'))));
}

#[test]
fn numeric_live_updates_keep_one_software_damage_region() {
    use iced::{
        advanced::{
            graphics::{damage, geometry::Renderer as _},
            renderer::{Headless, Renderer as _},
        },
        widget::canvas::Program,
    };
    for size in [
        Size::new(941., 1030.),
        Size::new(420., 320.),
        Size::new(300., 180.),
    ] {
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            iced::Pixels(12.),
            Some("tiny-skia"),
        ))
        .unwrap();
        let (mut c, id) = chart();
        let state = feather_charts::api::ChartState::default();
        let bounds = iced::Rectangle::with_size(size);
        let mut previous: Option<Vec<iced_tiny_skia::Layer>> = None;
        let mut previous_pixels = None;
        for offset in [0., 3., -4., 0.] {
            c.update_data(id, row(120., 4. + offset / 100.), false)
                .unwrap();
            c.set_right_offset(offset);
            c.prepare(size, 1., Instant::now()).unwrap();
            renderer.reset(bounds);
            for g in c.program(std::convert::identity).draw(
                &state,
                &renderer,
                &iced::Theme::Dark,
                bounds,
                iced::mouse::Cursor::Unavailable,
            ) {
                renderer.draw_geometry(g);
            }
            let iced::Renderer::Secondary(software) = &mut renderer else {
                panic!("software required")
            };
            let layers = software.layers().to_vec();
            if let Some(previous) = previous {
                assert_eq!(
                    damage::group(
                        damage::diff(
                            &previous,
                            &layers,
                            |layer| vec![layer.bounds],
                            iced_tiny_skia::Layer::damage
                        ),
                        bounds
                    )
                    .len(),
                    1
                );
            }
            previous = Some(layers);
            let pixels = renderer.screenshot(
                Size::new(size.width as u32, size.height as u32),
                1.,
                iced::Color::BLACK,
            );
            if let Some(old) = previous_pixels {
                assert_ne!(pixels, old, "stale numeric geometry");
            }
            previous_pixels = Some(pixels);
        }
    }
}

#[test]
fn numeric_pulse_ticks_reuse_markers_until_the_expiration_deadline() {
    use feather_charts::model::series_options::{
        LastPriceAnimationMode, LineSeriesOptionsPatch, LineStyleOptionsPatch,
    };
    use std::rc::Rc;
    let mut c = YieldCurveChart::new(Default::default()).unwrap();
    let id = c
        .add_line_series(
            LineSeriesOptionsPatch {
                style: LineStyleOptionsPatch {
                    last_price_animation: Some(LastPriceAnimationMode::Continuous),
                    ..Default::default()
                },
                ..Default::default()
            },
            0,
            PriceScalePosition::Left,
        )
        .unwrap();
    let markers = c.add_up_down_markers(id, Default::default()).unwrap();
    let now = Instant::now();
    c.set_up_down_data(markers, vec![row(1., 5.), row(12., 4.8)])
        .unwrap();
    c.update_up_down_data(markers, row(12., 4.9), false, now)
        .unwrap();
    c.fit_content();
    c.prepare(Size::new(800., 500.), 1., now).unwrap();
    c.update(ChartMessage::Tick(now)).unwrap();
    let retained = c.frame().plot.primitives[0].output.views[0]
        .renderer
        .clone();
    for ms in [50, 100, 200, 1000] {
        c.update(ChartMessage::Tick(now + Duration::from_millis(ms)))
            .unwrap();
        assert!(Rc::ptr_eq(
            &retained,
            &c.frame().plot.primitives[0].output.views[0].renderer
        ));
    }
    c.update(ChartMessage::Tick(now + Duration::from_secs(5)))
        .unwrap();
    assert!(
        c.up_down_markers(markers, now + Duration::from_secs(5))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn numeric_headless_snapshot_expires_markers_without_a_widget_tick() {
    let (mut c, id) = chart();
    let now = Instant::now();
    let markers = c.add_up_down_markers(id, Default::default()).unwrap();
    c.set_up_down_data(markers, vec![row(1., 5.), row(12., 4.8)])
        .unwrap();
    c.update_up_down_data(markers, row(12., 4.9), false, now)
        .unwrap();
    let snapshot = c.prepare_snapshot(Size::new(800., 500.), 1., now).unwrap();
    assert!(!snapshot.panes[0].plot.primitives.is_empty());
    let expired = c
        .prepare_snapshot(Size::new(800., 500.), 1., now + Duration::from_secs(5))
        .unwrap();
    assert!(
        expired.panes[0]
            .plot
            .primitives
            .iter()
            .all(|a| a.output.views.is_empty()),
        "headless snapshots must advance marker deadlines"
    );
}

#[test]
fn yield_axis_uses_three_decimal_percent_defaults_including_support_owner() {
    let (mut c, _) = chart();
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    assert_eq!(
        c.model().panes()[0]
            .left_price_scale()
            .format_price(5.378, 5.),
        "5.378%"
    );
}
