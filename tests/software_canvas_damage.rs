use feather_charts::{
    api::{Chart, ChartState},
    model::{
        data_consumer::{BuiltInSeriesDataItem, HistogramData, HistogramDataItem},
        horz_scale_behavior_time::types::UtcTimestamp,
        pane::PriceScalePosition,
        series_options::{HistogramSeriesOptions, SeriesOptionsMap},
    },
};
use iced::{
    Color, Rectangle, Size, Theme,
    advanced::{
        graphics::{damage, geometry::Renderer as _},
        renderer::{Headless, Renderer as _},
    },
    widget::canvas::Program,
};
use std::time::Instant;

#[test]
fn changing_chart_damage_is_one_region_and_keeps_fresh_pixels() {
    check_damage(false, false, Size::new(941., 1030.));
}
#[test]
fn image_chart_keeps_one_damage_region_at_all_acceptance_sizes() {
    for size in [
        Size::new(941., 1030.),
        Size::new(420., 320.),
        Size::new(300., 180.),
    ] {
        check_damage(true, false, size);
    }
}
#[test]
fn up_down_updates_keep_one_damage_region_and_fresh_pixels() {
    for size in [
        Size::new(941., 1030.),
        Size::new(420., 320.),
        Size::new(300., 180.),
    ] {
        check_damage(true, true, size);
    }
}
fn check_damage(with_image: bool, with_updates: bool, size: Size) {
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some("tiny-skia"),
    ))
    .unwrap();
    let mut chart = Chart::default();
    let id = chart
        .add_series(
            SeriesOptionsMap::Histogram(HistogramSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(
            id,
            (0..120)
                .map(|i| {
                    BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
                        time: UtcTimestamp::new(1_700_000_000. + i as f64 * 60.).into(),
                        value: 100. + (i as f64 * 0.1).sin() * 10.,
                        color: None,
                        custom_values: None,
                    }))
                })
                .collect(),
        )
        .unwrap();
    if with_image {
        chart
            .add_pane_primitive(
                0,
                feather_charts::api::ImageWatermark::from_rgba(
                    1,
                    1,
                    vec![255, 0, 0, 128],
                    Default::default(),
                )
                .unwrap(),
            )
            .unwrap();
    }
    let marker_id = with_updates.then(|| {
        let line = chart
            .add_series(
                SeriesOptionsMap::Line(Default::default()),
                0,
                PriceScalePosition::Right,
            )
            .unwrap();
        let marker = chart.add_up_down_markers(line, Default::default()).unwrap();
        chart
            .set_up_down_data(marker, vec![update_row(100.)])
            .unwrap();
        marker
    });
    chart.fit_content();
    let state = ChartState::default();
    let bounds = Rectangle::with_size(size);
    let mut previous: Option<Vec<iced_tiny_skia::Layer>> = None;
    let mut previous_pixels = None;
    for offset in [0., 3., -4., 0.] {
        if let Some(id) = marker_id {
            chart
                .update_up_down_data(id, update_row(101. + offset), false, Instant::now())
                .unwrap();
        }
        chart.set_right_offset(offset);
        chart.prepare(size, 1., Instant::now()).unwrap();
        renderer.reset(bounds);
        for geometry in chart.program(std::convert::identity).draw(
            &state,
            &renderer,
            &Theme::Dark,
            bounds,
            iced::mouse::Cursor::Unavailable,
        ) {
            renderer.draw_geometry(geometry);
        }
        let iced::Renderer::Secondary(software) = &mut renderer else {
            panic!("software renderer required")
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
                bounds,
            );
            assert_eq!(
                regions.len(),
                1,
                "chart damage must not replay all geometry for each bar"
            );
        }
        previous = Some(layers);
        let pixels = renderer.screenshot(
            Size::new(size.width as u32, size.height as u32),
            1.,
            Color::BLACK,
        );
        if let Some(previous) = previous_pixels {
            assert_ne!(pixels, previous, "draw reused stale geometry");
        }
        previous_pixels = Some(pixels);
    }
}

fn update_row(
    value: f64,
) -> BuiltInSeriesDataItem<feather_charts::model::horz_scale_behavior_time::types::Time> {
    use feather_charts::model::data_consumer::{LineData, LineDataItem};
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: UtcTimestamp::new(1_700_000_000. + 119. * 60.).into(),
        value,
        color: None,
        custom_values: None,
    }))
}
