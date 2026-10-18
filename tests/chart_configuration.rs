use feather_charts::{
    api::*,
    model::{
        layout_options::Background, pane::PriceScalePosition, price_scale::PriceScaleOptionsPatch,
    },
};

#[test]
fn rendering_uses_new_gradient_grid_and_typography_without_mutating_old_frames() {
    use feather_charts::model::{
        data_consumer::*, horz_scale_behavior_time::types::UtcTimestamp,
        series_options::SeriesOptionsMap,
    };
    let mut c = Chart::default();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(Default::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(
        id,
        (1..20)
            .map(|i| {
                BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                    time: UtcTimestamp::new(i as f64).into(),
                    value: i as f64,
                    color: None,
                    custom_values: None,
                }))
            })
            .collect(),
    )
    .unwrap();
    c.fit_content();
    let size = iced::Size::new(800., 500.);
    let now = std::time::Instant::now();
    c.prepare(size, 1., now).unwrap();
    let old = c.frame().clone();
    let old_width = old.layout.right_axis.size.width;
    c.apply_chart_options(ChartOptionsPatch {
        model: ChartModelOptionsPatch {
            layout: Some(LayoutOptionsPatch {
                background: Some(Background::VerticalGradient {
                    top_color: "red".into(),
                    bottom_color: "blue".into(),
                }),
                font_size: Some(24.),
                ..Default::default()
            }),
            grid: Some(GridOptionsPatch {
                vert_lines: Some(GridLineOptionsPatch {
                    visible: Some(false),
                    ..Default::default()
                }),
                horz_lines: Some(GridLineOptionsPatch {
                    visible: Some(false),
                    ..Default::default()
                }),
            }),
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
    c.prepare(size, 1., now).unwrap();
    assert_eq!(
        c.frame().plot.background_gradient,
        Some((
            iced::Color::from_rgb(1., 0., 0.),
            iced::Color::from_rgb(0., 0., 1.)
        ))
    );
    assert!(c.frame().layout.right_axis.size.width > old_width);
    assert_eq!(old.layout.right_axis.size.width, old_width);
    assert_eq!(old.plot.background_gradient, None);
    assert_eq!(c.series_data(id).unwrap().len(), 19);
}

#[test]
fn invalid_later_sections_preserve_all_state() {
    use feather_charts::model::{
        price_scale::PriceScaleMarginsPatch, time_scale_options::HorzScaleOptionsPatch,
    };
    let mut c = Chart::default();
    let before = c.runtime_options();
    for model in [
        ChartModelOptionsPatch {
            layout: Some(LayoutOptionsPatch {
                text_color: Some("red".into()),
                ..Default::default()
            }),
            time_scale: Some(HorzScaleOptionsPatch {
                bar_spacing: Some(f64::NAN),
                ..Default::default()
            }),
            ..Default::default()
        },
        ChartModelOptionsPatch {
            grid: Some(GridOptionsPatch {
                horz_lines: Some(GridLineOptionsPatch {
                    color: Some("not-a-color".into()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        ChartModelOptionsPatch {
            right_price_scale: Some(PriceScaleOptionsPatch {
                scale_margins: Some(PriceScaleMarginsPatch {
                    top: Some(0.9),
                    bottom: Some(0.9),
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
    ] {
        assert!(
            c.apply_chart_options(ChartOptionsPatch {
                model,
                mouse_navigation: Some(MouseNavigationOptions {
                    wheel_zoom: false,
                    wheel_scroll: false,
                    pressed_mouse_move: false
                }),
                ..Default::default()
            })
            .is_err()
        );
        assert_eq!(c.runtime_options(), before);
    }
}
#[test]
fn appearance_and_scale_patches_are_atomic_and_reach_future_panes() {
    let mut c = Chart::default();
    c.add_pane();
    let before = c.runtime_options();
    let patch = ChartOptionsPatch {
        model: ChartModelOptionsPatch {
            layout: Some(LayoutOptionsPatch {
                font_size: Some(17.),
                background: Some(Background::VerticalGradient {
                    top_color: "white".into(),
                    bottom_color: "blue".into(),
                }),
                ..Default::default()
            }),
            right_price_scale: Some(PriceScaleOptionsPatch {
                invert_scale: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    c.apply_chart_options(patch).unwrap();
    let after = c.runtime_options();
    assert_eq!(after.model.layout.font_size, 17.);
    assert_ne!(after, before);
    for p in [0, 1, c.add_pane()] {
        assert!(
            c.price_scale_options(p, &PriceScalePosition::Right)
                .unwrap()
                .invert_scale
        );
    }
    let before = c.runtime_options();
    assert!(
        c.apply_chart_options(ChartOptionsPatch {
            model: ChartModelOptionsPatch {
                layout: Some(LayoutOptionsPatch {
                    text_color: Some("red".into()),
                    font_size: Some(f64::NAN),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
        .is_err()
    );
    assert_eq!(c.runtime_options(), before);
    c.apply_price_scale_options(
        1,
        &PriceScalePosition::Right,
        PriceScaleOptionsPatch {
            invert_scale: Some(false),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        c.price_scale_options(0, &PriceScalePosition::Right)
            .unwrap()
            .invert_scale
    );
    let p = c.add_pane();
    assert!(
        !c.price_scale_options(p, &PriceScalePosition::Right)
            .unwrap()
            .invert_scale
    );
}
