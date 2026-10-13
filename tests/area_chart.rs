use feather_charts::{
    api::Chart,
    model::{
        data_consumer::{AreaData, AreaDataItem, BuiltInSeriesDataItem},
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{AreaSeriesOptions, SeriesOptionsMap},
    },
};
fn row(i: f64, v: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
        time: Time::from(UtcTimestamp::new(i)),
        value: v,
        line_color: None,
        top_color: None,
        bottom_color: None,
        custom_values: None,
    }))
}
#[test]
fn area_public_lifecycle_and_fill_visibility() {
    let mut chart = Chart::default();
    let id = chart
        .add_series(
            SeriesOptionsMap::Area(AreaSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(id, vec![row(0., 100.), row(1., 110.), row(2., 105.)])
        .unwrap();
    chart.fit_content();
    chart
        .prepare(iced::Size::new(800., 500.), 1.25, std::time::Instant::now())
        .unwrap();
    assert!(!chart.frame().plot.lines.is_empty());
    assert!(!chart.frame().plot.fills.is_empty());
    assert!(!chart.frame().plot.price_lines.is_empty());
    assert_eq!(
        chart.frame().plot.axes.as_ref().unwrap().right.labels[0]
            .axis
            .text,
        "105.00"
    );
    let mut options = AreaSeriesOptions::default();
    options.style.line_visible = false;
    chart
        .replace_series_options(id, SeriesOptionsMap::Area(options))
        .unwrap();
    chart
        .prepare(iced::Size::new(800., 500.), 1.25, std::time::Instant::now())
        .unwrap();
    assert!(chart.frame().plot.lines.is_empty());
    assert!(!chart.frame().plot.fills.is_empty());
    chart.update_data(id, row(3., 115.), false).unwrap();
    chart.set_data(id, vec![]).unwrap();
    chart
        .prepare(iced::Size::new(800., 500.), 1.25, std::time::Instant::now())
        .unwrap();
    assert!(chart.frame().plot.price_lines.is_empty());
    assert!(chart.frame().plot.fills.is_empty());
    chart.remove_series(id).unwrap();
}

#[test]
fn area_invalid_options_and_row_colors_preserve_working_state() {
    let mut chart = Chart::default();
    let id = chart
        .add_series(
            SeriesOptionsMap::Area(AreaSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(id, vec![row(0., 100.), row(1., 110.)])
        .unwrap();
    let mut options = AreaSeriesOptions::default();
    options.style.top_color = "invalid".into();
    assert!(
        chart
            .replace_series_options(id, SeriesOptionsMap::Area(options))
            .is_err()
    );
    assert!(
        matches!(chart.model().series(id).unwrap().options(),SeriesOptionsMap::Area(o) if o.style.top_color==AreaSeriesOptions::default().style.top_color)
    );
    let mut bad = row(2., 120.);
    if let BuiltInSeriesDataItem::Area(AreaDataItem::Data(data)) = &mut bad {
        data.bottom_color = Some("invalid".into());
    }
    assert!(chart.set_data(id, vec![bad]).is_err());
    assert_eq!(chart.model().series(id).unwrap().bars().rows().len(), 2);
    chart.update_data(id, row(2., 120.), false).unwrap();
}

#[test]
fn area_row_style_markers_and_scale_modes_follow_public_frames() {
    use feather_charts::{
        api::ChartMessage,
        model::{
            axis_snapshots::PriceAxisSide,
            price_scale::{PriceScaleMode, PriceScaleOptionsPatch},
        },
        ui::chart_input::ChartInteraction,
    };
    let mut chart = Chart::default();
    let id = chart
        .add_series(
            SeriesOptionsMap::Area(AreaSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let mut last = row(1., 110.);
    if let BuiltInSeriesDataItem::Area(AreaDataItem::Data(data)) = &mut last {
        data.line_color = Some("red".into());
        data.top_color = Some("blue".into());
    }
    chart.set_data(id, vec![row(0., 100.), last]).unwrap();
    chart.fit_content();
    for mode in [
        PriceScaleMode::Normal,
        PriceScaleMode::Logarithmic,
        PriceScaleMode::Percentage,
        PriceScaleMode::IndexedTo100,
    ] {
        chart
            .apply_price_axis_options(
                0,
                PriceAxisSide::Right,
                PriceScaleOptionsPatch {
                    mode: Some(mode),
                    ..Default::default()
                },
            )
            .unwrap();
        chart
            .prepare(iced::Size::new(800., 500.), 1.5, std::time::Instant::now())
            .unwrap();
        let label = &chart.frame().plot.axes.as_ref().unwrap().right.labels[0];
        assert_eq!(
            label.axis.text,
            if mode == PriceScaleMode::Percentage {
                "10.00%"
            } else {
                "110.00"
            }
        );
        assert_eq!(label.axis.background, "rgb(255, 0, 0)");
        assert!(!chart.frame().plot.fills.is_empty());
    }
    let x = chart
        .model()
        .time_scale()
        .index_to_coordinate(1.0.into())
        .value() as f32;
    let events = chart
        .update(ChartMessage::Interaction(ChartInteraction {
            position: Some(iced::Point::new(x, 100.)),
            navigation: None,
            capture: false,
        }))
        .unwrap();
    assert!(!events.is_empty());
    assert!(!chart.frame().plot.crosshair_markers.is_empty());
}
