use feather_charts::{
    api::*,
    model::{data_consumer::*, pane::PriceScalePosition, series_options::*},
};
use iced::Size;
use std::time::Instant;
fn key(x: f64) -> HorizontalPrice {
    HorizontalPrice::new(x).unwrap()
}
#[test]
fn prepared_numeric_geometry_is_proportional_through_navigation() {
    let mut chart = OptionsChart::new_numeric(2).unwrap();
    let id = chart
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(
            id,
            [1., 2., 100.]
                .map(|x| {
                    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                        time: key(x),
                        value: x,
                        color: None,
                        custom_values: None,
                    }))
                })
                .to_vec(),
        )
        .unwrap();
    chart
        .prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    chart.fit_content();
    chart
        .prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    for width in [800., 1200., 400.] {
        chart.zoom_time(200_f64.into(), 0.1);
        chart
            .prepare(Size::new(width, 500.), 1., Instant::now())
            .unwrap();
        let feather_charts::ui::hit_testing::HitGeometry::Line { data, .. } =
            &chart.frame().plot.hit_series[0].geometry
        else {
            panic!()
        };
        assert_eq!(data.items.len(), 3);
        let projected = &data.items;
        assert!(
            ((projected[2].x - projected[1].x) / (projected[1].x - projected[0].x) - 98.).abs()
                < 1e-8
        );
        assert!(
            ((projected[2].y - projected[1].y) / (projected[1].y - projected[0].y) - 98.).abs()
                < 1e-8
        );
        let xs = [1., 2., 100.].map(|x| chart.horizontal_to_coordinate(&key(x)).unwrap().unwrap());
        assert!(
            ((xs[2] - xs[1]) / (xs[1] - xs[0]) - 98.).abs() < 1e-8,
            "{xs:?}"
        );
        for x in [-1., 1.5, 50.] {
            let px = chart.numeric_to_coordinate(x).unwrap();
            assert!((chart.coordinate_to_numeric(px).unwrap() - x).abs() < 1e-10);
        }
    }
}

#[test]
fn prepared_lines_ticks_crosshair_and_columns_share_numeric_geometry() {
    use feather_charts::renderers::line_renderer::Point;
    use feather_charts::ui::hit_testing::HitGeometry;
    let mut chart = OptionsChart::new_numeric(2).unwrap();
    let line = chart
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let hist = chart
        .add_series(
            SeriesOptionsMap::Histogram(HistogramSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart.set_numeric_histogram_width(hist, 0.5).unwrap();
    chart
        .set_data(
            line,
            [1., 2., 100.]
                .map(|x| {
                    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                        time: key(x),
                        value: 2.,
                        color: None,
                        custom_values: None,
                    }))
                })
                .to_vec(),
        )
        .unwrap();
    chart
        .set_data(
            hist,
            [-1., 2., 100.]
                .map(|x| {
                    BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
                        time: key(x),
                        value: 1.,
                        color: None,
                        custom_values: None,
                    }))
                })
                .to_vec(),
        )
        .unwrap();
    chart
        .prepare(Size::new(1000., 600.), 1., Instant::now())
        .unwrap();
    chart.fit_content();
    chart
        .prepare(Size::new(1000., 600.), 1., Instant::now())
        .unwrap();
    let HitGeometry::Line { data, .. } = &chart
        .frame()
        .plot
        .hit_series
        .iter()
        .find(|s| s.series == line)
        .unwrap()
        .geometry
    else {
        panic!()
    };
    let xs: Vec<_> = data.items.iter().map(|p| p.x).collect();
    assert_eq!(xs.len(), 3);
    use feather_charts::renderers::line_renderer::PathOperation;
    let drawn: Vec<_> = chart
        .frame()
        .plot
        .lines
        .iter()
        .flat_map(|l| l.path.iter())
        .filter_map(|p| match p {
            PathOperation::MoveTo(p) | PathOperation::LineTo(p) => Some(p.x),
            _ => None,
        })
        .collect();
    for x in &xs {
        assert!(drawn.iter().any(|drawn| (*drawn - *x).abs() < 1e-5));
    }

    assert!(((xs[2] - xs[1]) / (xs[1] - xs[0]) - 98.).abs() < 1e-8);
    let HitGeometry::Columns { items, width } = &chart
        .frame()
        .plot
        .hit_series
        .iter()
        .find(|s| s.series == hist)
        .unwrap()
        .geometry
    else {
        panic!()
    };
    assert!((items[1].x - xs[1]).abs() < 1e-10);
    let expected =
        chart.numeric_to_coordinate(2.25).unwrap() - chart.numeric_to_coordinate(1.75).unwrap();
    assert!((*width - expected).abs() < 1e-10);
    for (r, p) in chart.frame().plot.candles.iter().zip(items) {
        assert!((f64::from(r.size.width) - expected).abs() < 1e-4);
        assert!((f64::from(r.origin.x) + expected / 2. - p.x).abs() < 1e-4);
    }
    let p = &items[1];
    let y = (p.top + p.bottom) / 2.;
    assert_eq!(
        chart
            .hit_test(0, iced::Point::new(p.x as f32, y as f32))
            .unwrap()
            .series,
        hist
    );
    assert!(
        chart
            .hit_test(0, iced::Point::new((p.x + 20.) as f32, y as f32))
            .is_none()
    );
    let axes = chart.frame().plot.axes.as_ref().unwrap();
    assert!(
        axes.time
            .ticks
            .iter()
            .any(|t| ![1., 2., 100.].contains(&t.mark.label.parse::<f64>().unwrap()))
    );
    for tick in &axes.time.ticks {
        let value = tick.mark.label.parse::<f64>().unwrap();
        assert!((chart.numeric_to_coordinate(value).unwrap() - tick.mark.coordinate).abs() < 1e-8);
    }
    let x = chart.numeric_to_coordinate(50.).unwrap();
    chart
        .set_crosshair_position(0, Some(Point { x, y: 100. }))
        .unwrap();
    chart
        .prepare(Size::new(1000., 600.), 1., Instant::now())
        .unwrap();
    assert!((chart.model().crosshair().position().unwrap().x - x).abs() < 1e-8);
    assert_eq!(
        chart
            .frame()
            .plot
            .axes
            .as_ref()
            .unwrap()
            .time
            .crosshair_label
            .as_ref()
            .unwrap()
            .text,
        "50.00"
    );
}

#[test]
fn numeric_rejections_preserve_data_and_large_finite_y_is_supported() {
    let mut c = OptionsChart::new_numeric(3).unwrap();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let item = |x, y| {
        BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time: key(x),
            value: y,
            color: None,
            custom_values: None,
        }))
    };
    c.set_data(id, vec![item(1., 1e20), item(2., 2e20)])
        .unwrap();
    let original = c.series_data(id).unwrap();
    assert!(
        c.set_data(id, vec![item(-f64::MAX, 1.), item(f64::MAX, 2.)])
            .is_err()
    );
    assert_eq!(c.series_data(id).unwrap(), original);
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    c.fit_content();
    c.prepare(Size::new(800., 500.), 1., Instant::now())
        .unwrap();
    let a = c.price_to_coordinate(id, 1e20).unwrap().unwrap();
    let b = c.price_to_coordinate(id, 2e20).unwrap().unwrap();
    assert!(a.is_finite() && b.is_finite() && a > b);
    assert!((c.coordinate_to_price(id, a).unwrap().unwrap() / 1e20 - 1.).abs() < 1e-10);
    let left = c.horizontal_to_index(&key(1.5), true).unwrap().unwrap();
    assert_eq!(
        c.data_by_index(
            id,
            left,
            feather_charts::model::plot_list::MismatchDirection::None
        )
        .unwrap()
        .unwrap(),
        original[0]
    );
}
