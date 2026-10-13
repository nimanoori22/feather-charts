use feather_charts::{
    api::*,
    model::{
        data_consumer::*,
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::*,
    },
};
fn chart() -> (Chart, feather_charts::model::data_layer::SeriesId) {
    let mut c = Chart::default();
    let mut o = LineSeriesOptions::default();
    o.common.price_line_visible = false;
    let id = c
        .add_series(SeriesOptionsMap::Line(o), 0, PriceScalePosition::Right)
        .unwrap();
    c.set_data(
        id,
        (0..10)
            .map(|i| {
                BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                    time: Time::from(UtcTimestamp::new(1000. + i as f64 * 10.)),
                    value: 100. + i as f64,
                    color: None,
                    custom_values: None,
                }))
            })
            .collect(),
    )
    .unwrap();
    c.fit_content();
    c.prepare(iced::Size::new(640., 480.), 1.25, std::time::Instant::now())
        .unwrap();
    (c, id)
}
#[test]
fn settled_line_hit_is_owned_and_public_click_reports_it() {
    let (mut c, id) = chart();
    let stroke = &c.frame().plot.lines[0];
    let a = match stroke.path[0] {
        feather_charts::renderers::line_renderer::PathOperation::MoveTo(p) => p,
        _ => panic!("move"),
    };
    let p = iced::Point::new(a.x as f32, a.y as f32);
    let hit = c.hit_test(0, p).unwrap();
    assert_eq!(hit.series, id);
    assert!(matches!(
        hit.target,
        HitTarget::SeriesLine | HitTarget::SeriesPoint
    ));
    let events = c
        .update(ChartMessage::Click {
            pane: 0,
            position: p,
        })
        .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e,ChartEvent::Click(data) if data.hit.as_ref()==Some(&hit)))
    );
    c.set_data(id, vec![]).unwrap();
    c.prepare(iced::Size::new(640., 480.), 1.25, std::time::Instant::now())
        .unwrap();
    assert!(c.hit_test(0, p).is_none());
    assert_eq!(hit.series, id);
}
