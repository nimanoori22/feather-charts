use super::*;
use iced::advanced::{layout::Limits, renderer::Headless, widget::Tree};

#[test]
fn hover_status_keeps_chart_bounds_stable_in_narrow_windows() {
    let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(16.),
        Some("tiny-skia"),
    ))
    .expect("headless renderer");
    for width in [640., 480., 360.] {
        let mut app = App::new(SeriesType::Candlestick);
        app.chart
            .add_series_markers(
                app.price,
                sample_markers(false),
                SeriesMarkersOptions::default(),
            )
            .unwrap();
        let window = Size::new(width, 480.);
        let mut tree = Tree::new(app.view().as_widget());
        let mut expected = None;
        for hovered_marker in [false, true, false, true, false] {
            let bounds = {
                let mut view = app.view();
                tree.diff(view.as_widget());
                let layout = view.as_widget_mut().layout(
                    &mut tree,
                    &renderer,
                    &Limits::new(Size::ZERO, window),
                );
                layout.children().last().unwrap().bounds()
            };
            if let Some(previous) = expected {
                assert_eq!(
                    bounds, previous,
                    "hover resized the chart at window width {width}"
                );
            } else {
                expected = Some(bounds);
            }
            let _ = app.update(Message::Chart(ChartMessage::Bounds(bounds.size())));
            let position = if hovered_marker {
                let m = &app.chart.frame().plot.annotations[0].items[0];
                iced::Point::new(m.center.x as f32, m.center.y as f32)
            } else {
                iced::Point::new(1., 1.)
            };
            let _ = app.update(Message::Chart(ChartMessage::Interaction(
                feather_charts::ui::chart_input::ChartInteraction {
                    position: Some(position),
                    navigation: None,
                    capture: true,
                },
            )));
            assert_eq!(app.chart.marker_hit(0, position).is_some(), hovered_marker);
            assert_eq!(app.status.contains("marker Some("), hovered_marker);
            assert_eq!(app.status.contains("target Some("), hovered_marker);
        }
    }
}

#[test]
fn primitive_controls_and_hover_keep_narrow_chart_bounds_stable() {
    let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(16.),
        Some("tiny-skia"),
    ))
    .unwrap();
    for width in [640., 480., 360.] {
        let mut app = App::new(SeriesType::Candlestick);
        app.primitive_scene = Some(primitive_demo::Scene::new(&mut app.chart, app.price));
        let mut tree = Tree::new(app.view().as_widget());
        let mut expected = None;
        for action in [
            Message::PrimitiveLayer,
            Message::MovePrimitive,
            Message::TogglePrimitive,
            Message::TogglePrimitive,
        ] {
            let _ = app.update(action);
            for status in [
                "No selection",
                "primitive Some(PrimitiveHit { owner: Series, external_id: region boundary, cursor: Pointer })",
            ] {
                app.status = status.into();
                let mut view = app.view();
                tree.diff(view.as_widget());
                let layout = view.as_widget_mut().layout(
                    &mut tree,
                    &renderer,
                    &Limits::new(Size::ZERO, Size::new(width, 480.)),
                );
                let bounds = layout.children().last().unwrap().bounds();
                if let Some(expected) = expected {
                    assert_eq!(
                        bounds, expected,
                        "primitive status changed plot bounds at {width}"
                    );
                } else {
                    expected = Some(bounds);
                }
            }
        }
    }
}

#[test]
fn live_feed_ticks_append_only_while_running_and_ignore_old_sessions() {
    let mut app = App::new(SeriesType::Line);
    app.size = Size::new(400., 300.);
    app.prepare();
    let initial = app.latest;
    let _ = app.update(Message::FeedTick(Instant::now()));
    assert_eq!(app.latest, initial);
    let _ = app.update(Message::ToggleLive);
    let first_session = app.live_since.unwrap();
    let _ = app.update(Message::FeedTick(first_session));
    assert_eq!(app.latest, initial + 1);
    assert_eq!(
        app.chart
            .model()
            .series(app.price)
            .unwrap()
            .bars()
            .rows()
            .len(),
        initial + 2
    );
    assert_eq!(
        app.chart
            .model()
            .series(app.volume)
            .unwrap()
            .bars()
            .rows()
            .len(),
        initial + 2
    );
    let _ = app.update(Message::ToggleLive);
    assert!(app.live_since.is_none());
    let _ = app.update(Message::FeedTick(Instant::now()));
    assert_eq!(app.latest, initial + 1);
    let _ = app.update(Message::ToggleLive);
    let stale = app.live_since.unwrap() - std::time::Duration::from_secs(1);
    let _ = app.update(Message::FeedTick(stale));
    assert_eq!(app.latest, initial + 1);
    let _ = app.update(Message::FeedTick(Instant::now()));
    assert_eq!(app.latest, initial + 2);
}

#[test]
fn short_chart_resizes_and_recovers_during_live_append() {
    for kind in [
        SeriesType::Histogram,
        SeriesType::Line,
        SeriesType::Area,
        SeriesType::Baseline,
        SeriesType::Bar,
        SeriesType::Candlestick,
    ] {
        for ratio in [1., 1.25, 2.] {
            let mut app = App::new(kind);
            app.primitive_scene = Some(primitive_demo::Scene::new(&mut app.chart, app.price));
            for (id, volume) in [(app.price, false), (app.volume, true)] {
                app.chart
                    .add_series_markers(id, sample_markers(volume), SeriesMarkersOptions::default())
                    .unwrap();
            }
            for height in [100., 500., 180., 30., 2., 0., 100., 500.] {
                app.size = Size::new(300., height);
                app.ratio = ratio;
                app.chart
                    .prepare(app.size, ratio, Instant::now())
                    .unwrap_or_else(|error| {
                        panic!("{kind:?} height={height} ratio={ratio}: {error:?}")
                    });
                let _ = app.update(Message::Append);
                app.chart.prepare(app.size, ratio, Instant::now()).unwrap();
                let panes = app.chart.multi_frame().unwrap();
                assert_eq!(panes.panes.len(), 2);
                for pane in &panes.panes {
                    for rect in &pane.plot.candles {
                        assert!(rect.origin.x.is_finite() && rect.origin.y.is_finite());
                        assert!(rect.size.width.is_finite() && rect.size.height.is_finite());
                    }
                }
            }
            assert!(
                !app.chart.frame().plot.candles.is_empty()
                    || !app.chart.frame().plot.lines.is_empty()
                    || !app.chart.frame().plot.fills.is_empty()
            );
        }
    }
}

#[test]
fn short_chart_keeps_manual_price_modes_and_source_identity() {
    use feather_charts::model::{
        axis_snapshots::PriceAxisSide,
        price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
    };
    for mode in [
        PriceScaleMode::Normal,
        PriceScaleMode::Logarithmic,
        PriceScaleMode::Percentage,
        PriceScaleMode::IndexedTo100,
    ] {
        let mut app = App::new(SeriesType::Histogram);
        app.primitive_scene = Some(primitive_demo::Scene::new(&mut app.chart, app.price));
        app.size = Size::new(300., 500.);
        app.ratio = 1.25;
        app.prepare();
        app.chart
            .apply_price_axis_options(
                0,
                PriceAxisSide::Right,
                PriceScaleOptionsPatch {
                    mode: Some(mode),
                    ..Default::default()
                },
            )
            .unwrap();
        app.prepare();
        app.chart
            .apply_price_axis_options(
                0,
                PriceAxisSide::Right,
                PriceScaleOptionsPatch {
                    auto_scale: Some(false),
                    ..Default::default()
                },
            )
            .unwrap();
        for height in [180., 100., 30., 2., 500.] {
            app.size.height = height;
            let _ = app.update(Message::Append);
            app.chart
                .prepare(app.size, app.ratio, Instant::now())
                .unwrap();
            let frame = app.chart.multi_frame().unwrap();
            assert_eq!(frame.panes[0].plot.series_id, Some(app.price));
            assert_eq!(frame.panes[1].plot.series_id, Some(app.volume));
            assert!(
                !app.chart.model().panes()[0]
                    .right_price_scale()
                    .is_auto_scale()
            );
        }
        assert!(!app.chart.frame().plot.candles.is_empty());
    }
}

#[test]
fn live_demo_damage_does_not_fragment_chart_redraws() {
    use iced::advanced::{
        Layout,
        graphics::damage,
        renderer::{Renderer as _, Style},
    };
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(16.),
        Some("tiny-skia"),
    ))
    .unwrap();
    let mut app = App::new(SeriesType::Histogram);
    hover_demo::add_overlays(&mut app.chart);
    app.primitive_scene = Some(primitive_demo::Scene::new(&mut app.chart, app.price));
    let size = Size::new(941., 1030.);
    let viewport = iced::Rectangle::with_size(size);
    let mut tree = Tree::new(app.view().as_widget());
    let mut previous: Option<Vec<iced_tiny_skia::Layer>> = None;
    for step in 0..8 {
        let bounds = {
            let mut view = app.view();
            tree.diff(view.as_widget());
            let layout =
                view.as_widget_mut()
                    .layout(&mut tree, &renderer, &Limits::new(Size::ZERO, size));
            layout.children().last().unwrap().bounds()
        };
        let _ = app.update(Message::Chart(ChartMessage::Bounds(bounds.size())));
        let _ = app.update(Message::Append);
        app.chart.set_right_offset(step as f64 * 3.);
        app.prepare();
        app.status = format!(
            "Crosshair: time Some({step}), price Some(104.123456), 4 series, marker None, target None, primitive None"
        );
        renderer.reset(viewport);
        {
            let mut view = app.view();
            tree.diff(view.as_widget());
            let layout =
                view.as_widget_mut()
                    .layout(&mut tree, &renderer, &Limits::new(Size::ZERO, size));
            view.as_widget().draw(
                &tree,
                &mut renderer,
                &iced::Theme::Dark,
                &Style {
                    text_color: iced::Color::WHITE,
                },
                Layout::new(&layout),
                iced::mouse::Cursor::Unavailable,
                &viewport,
            );
        }
        let iced::Renderer::Secondary(software) = &mut renderer else {
            panic!("software required")
        };
        let layers = software.layers().to_vec();
        if let Some(previous) = previous {
            let regions = damage::group(
                damage::diff(
                    &previous,
                    &layers,
                    |layer| vec![layer.bounds],
                    iced_tiny_skia::Layer::damage,
                ),
                viewport,
            );
            assert!(
                regions.len() <= 3,
                "full-demo chart damage fragmented into {} regions",
                regions.len()
            );
        }
        previous = Some(layers);
    }
}
