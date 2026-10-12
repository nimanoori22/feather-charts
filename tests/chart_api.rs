use feather_charts::{
    api::{Chart, ChartError},
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{LineSeriesOptions, SeriesOptionsMap},
    },
};

fn row(time: f64, value: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: Time::from(UtcTimestamp::new(time)),
        value,
        color: None,
        custom_values: None,
    }))
}
fn add(chart: &mut Chart) -> feather_charts::model::data_layer::SeriesId {
    chart
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap()
}
#[test]
fn public_data_lifecycle_and_foreign_ids() {
    let mut chart = Chart::default();
    let id = add(&mut chart);
    chart.update_data(id, row(10., 100.), false).unwrap();
    chart.update_data(id, row(20., 120.), false).unwrap();
    chart.update_data(id, row(20., 125.), false).unwrap();
    assert_eq!(chart.model().series(id).unwrap().bars().rows().len(), 2);
    let mut other = Chart::default();
    let foreign = add(&mut other);
    assert!(matches!(
        chart.set_data(foreign, vec![row(1., 1.)]),
        Err(ChartError::UnknownSeries(_))
    ));
    chart.remove_series(id).unwrap();
    assert!(matches!(
        chart.update_data(id, row(30., 130.), false),
        Err(ChartError::UnknownSeries(_))
    ));
    assert!(chart.model().time_scale().is_empty());
}
#[test]
fn invalid_replacement_preserves_existing_data_and_future_updates() {
    let mut chart = Chart::default();
    let id = add(&mut chart);
    chart
        .set_data(id, vec![row(10., 100.), row(20., 120.)])
        .unwrap();
    for bad in [
        vec![row(20., 1.), row(10., 2.)],
        vec![row(10., 1.), row(10., 2.)],
        vec![row(30., f64::NAN)],
        vec![row(f64::NAN, 1.)],
    ] {
        assert!(chart.set_data(id, bad).is_err());
        assert_eq!(chart.model().series(id).unwrap().bars().rows().len(), 2);
    }
    assert!(chart.update_data(id, row(15., 115.), false).is_err());
    chart.update_data(id, row(10., 105.), true).unwrap();
    chart.update_data(id, row(30., 130.), false).unwrap();
    assert_eq!(chart.model().series(id).unwrap().bars().rows().len(), 3);
}
