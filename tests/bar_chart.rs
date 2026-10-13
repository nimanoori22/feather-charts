use feather_charts::{
    api::Chart,
    model::{
        data_consumer::{BarData, BarDataItem, BuiltInSeriesDataItem},
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{BarSeriesOptions, SeriesOptionsMap},
    },
};
fn row(i: f64, close: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Bar(BarDataItem::Data(BarData {
        time: Time::from(UtcTimestamp::new(i)),
        open: close - 2.,
        high: close + 10.,
        low: close - 10.,
        close,
        color: Some("#ffff00".into()),
        custom_values: None,
    }))
}
fn prepare(chart: &mut Chart) {
    chart
        .prepare(iced::Size::new(800., 500.), 1.25, std::time::Instant::now())
        .unwrap();
}
#[test]
fn bars_public_ohlc_options_labels_and_lifecycle() {
    let mut c = Chart::default();
    let id = c
        .add_series(
            SeriesOptionsMap::Bar(BarSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(id, vec![row(0., 100.), row(1., 110.), row(2., 105.)])
        .unwrap();
    c.fit_content();
    prepare(&mut c);
    assert_eq!(c.frame().plot.candles.len(), 9);
    assert!(!c.frame().plot.price_lines.is_empty());
    assert_eq!(
        c.frame().plot.axes.as_ref().unwrap().right.labels[0]
            .axis
            .text,
        "105.00"
    );
    let before = c.frame().plot.candles.clone();
    let mut options = BarSeriesOptions::default();
    options.style.open_visible = false;
    options.style.thin_bars = false;
    c.replace_series_options(id, SeriesOptionsMap::Bar(options))
        .unwrap();
    prepare(&mut c);
    assert_eq!(c.frame().plot.candles.len(), 6);
    assert!(c.frame().plot.candles[0].size.width > before[0].size.width);
    c.update_data(id, row(3., 115.), false).unwrap();
    c.update_data(id, row(0., 95.), true).unwrap();
    c.fit_content();
    prepare(&mut c);
    assert_eq!(c.frame().plot.candles.len(), 8);
    let mut hidden = BarSeriesOptions::default();
    hidden.common.visible = false;
    c.replace_series_options(id, SeriesOptionsMap::Bar(hidden))
        .unwrap();
    prepare(&mut c);
    assert!(c.frame().plot.candles.is_empty());
    c.replace_series_options(id, SeriesOptionsMap::Bar(BarSeriesOptions::default()))
        .unwrap();
    let mut invalid = BarSeriesOptions::default();
    invalid.style.up_color = "invalid".into();
    assert!(
        c.replace_series_options(id, SeriesOptionsMap::Bar(invalid))
            .is_err()
    );
    let mut bad = row(4., 120.);
    if let BuiltInSeriesDataItem::Bar(BarDataItem::Data(r)) = &mut bad {
        r.color = Some("invalid".into());
    }
    assert!(c.set_data(id, vec![bad]).is_err());
    assert_eq!(c.model().series(id).unwrap().bars().rows().len(), 4);
    c.set_data(id, vec![]).unwrap();
    prepare(&mut c);
    assert!(c.frame().plot.candles.is_empty());
    assert!(c.frame().plot.price_lines.is_empty());
    c.set_data(id, vec![row(0., 100.)]).unwrap();
    c.fit_content();
    prepare(&mut c);
    assert_eq!(c.frame().plot.candles.len(), 3);
    c.remove_series(id).unwrap();
}
