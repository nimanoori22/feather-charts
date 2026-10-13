//! Crowded public-API scene and reproducible input/capture states.
use super::*;
use feather_charts::api::{MarkerPosition, MarkerShape, SeriesMarker};

pub fn add_overlays(chart: &mut Chart) {
    for (kind, offset) in [
        (SeriesType::Area, 2.),
        (SeriesType::Baseline, -2.),
        (SeriesType::Line, 4.),
    ] {
        let id = chart
            .add_series(sample_options(kind, false), 0, PriceScalePosition::Right)
            .unwrap();
        chart
            .set_data(
                id,
                (0..120)
                    .map(|i| {
                        sample_row(
                            kind,
                            Time::from(UtcTimestamp::new(1_700_000_000. + i as f64 * 60.)),
                            100. + offset + (i as f64 * 0.1 + offset).sin() * 8.,
                        )
                    })
                    .collect(),
            )
            .unwrap();
    }
}
pub fn setup_capture(app: &mut App, scenario: &str) {
    let kind = match scenario {
        "hover-area" | "hover-area-disabled" | "hover-fill-only" | "hover-fill-only-disabled" => {
            SeriesType::Area
        }
        "hover-baseline" | "hover-baseline-disabled" => SeriesType::Baseline,
        "hover-bar" => SeriesType::Bar,
        "hover-candle" => SeriesType::Candlestick,
        "hover-histogram" => SeriesType::Histogram,
        _ => SeriesType::Line,
    };
    let mut chart = Chart::default();
    let mut options = sample_options(kind, false);
    hide_last_line(&mut options);
    if let SeriesOptionsMap::Line(o) = &mut options {
        o.style.color = "#00AA00".into();
    }
    if let SeriesOptionsMap::Area(o) = &mut options {
        o.style.line_color = "#00AA00".into();
        o.style.top_color = "rgba(0,170,0,0.5)".into();
        o.style.bottom_color = "rgba(0,170,0,0.5)".into();
        o.style.line_visible = !scenario.starts_with("hover-fill-only");
    }
    let lower = chart
        .add_series(options, 0, PriceScalePosition::Right)
        .unwrap();
    let upper_kind = if matches!(kind, SeriesType::Area | SeriesType::Baseline) {
        SeriesType::Area
    } else {
        SeriesType::Line
    };
    let mut options = sample_options(upper_kind, false);
    hide_last_line(&mut options);
    match &mut options {
        SeriesOptionsMap::Line(o) => {
            o.style.color = "#0000FF".into();
        }
        SeriesOptionsMap::Area(o) => {
            o.style.line_color = "#0000FF".into();
            o.style.top_color = "rgba(0,0,255,0.5)".into();
            o.style.bottom_color = "rgba(0,0,255,0.5)".into();
        }
        _ => unreachable!(),
    }
    let upper = chart
        .add_series(options, 0, PriceScalePosition::Right)
        .unwrap();
    for (id, kind, reverse) in [(lower, kind, false), (upper, upper_kind, true)] {
        chart
            .set_data(
                id,
                (0..10)
                    .map(|i| {
                        sample_row(
                            kind,
                            Time::from(UtcTimestamp::new(1_700_000_000. + i as f64 * 60.)),
                            if reverse {
                                110. - i as f64
                            } else {
                                100. + i as f64
                            },
                        )
                    })
                    .collect(),
            )
            .unwrap();
    }
    let mut level = PriceLineOptions::new(105.);
    level.color = "#FF0000".into();
    level.external_id = Some("crossing level".into());
    let line = chart.add_price_line(lower, level).unwrap();
    let mut marker = SeriesMarker::new(
        Time::from(UtcTimestamp::new(1_700_000_000. + 180.)),
        MarkerPosition::AtPriceMiddle(103.),
        MarkerShape::Circle,
        "#FF0000",
    );
    marker.id = Some("trade".into());
    chart
        .add_series_markers(
            lower,
            vec![marker],
            SeriesMarkersOptions {
                auto_scale: false,
                ..Default::default()
            },
        )
        .unwrap();
    chart.add_pane();
    app.chart = chart;
    app.price = lower;
    app.volume = upper;
    app.levels = vec![line];
    if scenario == "hover-narrow" {
        app.size = Size::new(400., 250.);
    }
    app.chart.fit_content();
    app.prepare();
    let point = position(&app.chart, 1., 101.);
    app.chart
        .update(ChartMessage::Click {
            pane: 0,
            position: point,
        })
        .unwrap();
    assert_eq!(app.chart.frame().plot.hovered_series, Some(lower));
    if scenario.ends_with("disabled") {
        app.chart.set_hovered_series_on_top(false);
    }
    if scenario == "hover-crossover" {
        let point = position(&app.chart, 6., 104.);
        // Remove the Point-priority level before crossing into the upper outline.
        app.chart.remove_price_line(line).unwrap();
        app.chart
            .update(ChartMessage::Click {
                pane: 0,
                position: point,
            })
            .unwrap();
        assert_eq!(app.chart.frame().plot.hovered_series, Some(upper));
    }
    if scenario == "hover-removed" {
        app.chart.remove_series(lower).unwrap();
        app.chart
            .update(ChartMessage::Tick(Instant::now()))
            .unwrap();
        assert!(app.chart.frame().plot.hovered_series.is_none());
        assert!(!app.chart.is_animating());
    }
}
fn position(chart: &Chart, index: f64, price: f64) -> iced::Point {
    iced::Point::new(
        chart
            .model()
            .time_scale()
            .index_to_coordinate(index.floor().into())
            .value() as f32
            + (index.fract() * chart.model().time_scale().bar_spacing()) as f32,
        chart.model().panes()[0]
            .right_price_scale()
            .price_to_coordinate(price, 100.)
            .value() as f32,
    )
}

fn hide_last_line(options: &mut SeriesOptionsMap) {
    let common = match options {
        SeriesOptionsMap::Line(o) => &mut o.common,
        SeriesOptionsMap::Area(o) => &mut o.common,
        SeriesOptionsMap::Baseline(o) => &mut o.common,
        SeriesOptionsMap::Bar(o) => &mut o.common,
        SeriesOptionsMap::Candlestick(o) => &mut o.common,
        SeriesOptionsMap::Histogram(o) => &mut o.common,
        SeriesOptionsMap::Custom(o) => &mut o.common,
    };
    common.price_line_visible = false;
}

pub fn verify_pixels(app: &App, scenario: &str, rgba: &[u8], physical_width: usize) {
    if matches!(
        scenario,
        "hover-area" | "hover-area-disabled" | "hover-fill-only" | "hover-fill-only-disabled"
    ) {
        let plot = app.chart.frame().layout.plot;
        let sample = |p: iced::Point| {
            let x = ((f64::from(p.x) + plot.origin.x) * f64::from(app.ratio)).round() as usize;
            let y = ((f64::from(p.y) + plot.origin.y) * f64::from(app.ratio)).round() as usize;
            &rgba[(y * physical_width + x) * 4..][..3]
        };
        let inside = sample(position(&app.chart, 4.5, 99.3));
        for (actual, expected) in inside.iter().zip([63u8, 106, 191]) {
            assert!(
                actual.abs_diff(expected) <= 2,
                "{scenario}: fill must blend once per source: {inside:?}, point {:?}, size {:?}",
                position(&app.chart, 4.5, 99.3),
                app.chart.frame().plot.size
            );
        }
        if scenario == "hover-area" {
            let outline = sample(position(&app.chart, 2., 102.));
            assert!(
                outline[1] > 145 && outline[0] < 15 && outline[2] < 25,
                "hovered outline must cover upper fill: {outline:?}"
            );
        }
        if scenario == "hover-area-disabled" {
            let outline = sample(position(&app.chart, 2., 102.));
            assert!(
                outline[2] > 100,
                "normal outline must remain under upper fill: {outline:?}"
            );
        }
    }
}
