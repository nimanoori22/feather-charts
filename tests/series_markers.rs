use feather_charts::{
    api::{Chart, MarkerPosition, MarkerShape, SeriesMarker, SeriesMarkersOptions},
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{LineSeriesOptions, SeriesOptionsMap},
    },
};
fn time(t: f64) -> Time {
    Time::from(UtcTimestamp::new(t))
}
fn marker(t: f64) -> SeriesMarker<Time> {
    SeriesMarker::new(
        time(t),
        MarkerPosition::AboveBar,
        MarkerShape::Circle,
        "#f00",
    )
}
fn chart() -> (Chart, feather_charts::model::data_layer::SeriesId) {
    let mut c = Chart::default();
    let id = c
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    c.set_data(
        id,
        (0..10)
            .map(|i| {
                BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                    time: time(i as f64 * 10.),
                    value: 100. + i as f64,
                    color: None,
                    custom_values: None,
                }))
            })
            .collect(),
    )
    .unwrap();
    c.fit_content();
    (c, id)
}
#[test]
fn independent_marker_attachments_validate_replace_clear_and_detach() {
    let (mut c, series) = chart();
    let a = c
        .add_series_markers(
            series,
            vec![marker(0.), marker(0.)],
            SeriesMarkersOptions::default(),
        )
        .unwrap();
    let b = c
        .add_series_markers(series, vec![marker(10.)], SeriesMarkersOptions::default())
        .unwrap();
    let mut invalid = marker(20.);
    invalid.color = "invalid".into();
    assert!(c.set_series_markers(a, vec![invalid]).is_err());
    assert!(
        c.set_series_markers(a, vec![marker(10.), marker(0.)])
            .is_err()
    );
    assert_eq!(c.series_markers(a).unwrap().len(), 2);
    c.set_series_markers(a, vec![]).unwrap();
    assert_eq!(c.series_markers(b).unwrap().len(), 1);
    c.remove_series_markers(a).unwrap();
    assert!(c.series_markers(a).is_err());
    assert!(c.remove_series_markers(a).is_err());
    c.remove_series(series).unwrap();
    assert!(c.series_markers(b).is_err());
}
fn prepare(c: &mut Chart) {
    c.prepare(iced::Size::new(800., 500.), 1.25, std::time::Instant::now())
        .unwrap();
}
#[test]
fn marker_margins_add_to_configured_scale_padding_and_clear_restores_it() {
    let (mut c, series) = chart();
    prepare(&mut c);
    let scale = c.model().panes()[0].right_price_scale();
    let configured = scale.options().scale_margins;
    let before = scale.internal_height();
    let collection = c
        .add_series_markers(series, vec![marker(20.)], SeriesMarkersOptions::default())
        .unwrap();
    prepare(&mut c);
    let scale = c.model().panes()[0].right_price_scale();
    assert_eq!(scale.options().scale_margins, configured);
    assert!(scale.internal_height() < before);
    c.set_series_markers(collection, vec![]).unwrap();
    prepare(&mut c);
    assert_eq!(
        c.model().panes()[0].right_price_scale().internal_height(),
        before
    );
}
