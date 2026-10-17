use feather_charts::{
    api::Chart,
    model::{
        data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
        horz_scale_behavior_time::types::{Time, UtcTimestamp},
        pane::PriceScalePosition,
        series_options::{LineSeriesOptions, SeriesOptionsMap},
        time_data::LogicalRange,
        time_scale_options::HorzScaleOptionsPatch,
    },
};
use std::time::Instant;
fn row(i: usize, value: f64) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: UtcTimestamp::new(1_700_000_000. + i as f64 * 60.).into(),
        value,
        color: None,
        custom_values: None,
    }))
}
#[test]
fn dense_line_reduces_geometry_without_changing_canonical_rows() {
    let mut chart = Chart::default();
    let id = chart
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(id, (0..8192).map(|i| row(i, (i % 31) as f64)).collect())
        .unwrap();
    chart.apply_time_scale_options(HorzScaleOptionsPatch {
        min_bar_spacing: Some(0.001),
        enable_conflation: Some(true),
        ..Default::default()
    });
    chart.set_logical_range(LogicalRange {
        from: 0.0.into(),
        to: 8191.0.into(),
    });
    chart
        .prepare(iced::Size::new(1000., 500.), 1., Instant::now())
        .unwrap();
    let feather_charts::ui::hit_testing::HitGeometry::Line { data, .. } =
        &chart.frame().plot.hit_series[0].geometry
    else {
        panic!("expected line")
    };
    assert!(data.items.len() <= 2050);
    assert_eq!(chart.model().series(id).unwrap().bars().rows().len(), 8192);
}

#[test]
fn historical_rejection_uses_effective_level_and_preserves_data() {
    let mut chart = Chart::default();
    let id = chart
        .add_series(
            SeriesOptionsMap::Line(LineSeriesOptions::default()),
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    chart
        .set_data(id, (0..32).map(|i| row(i, i as f64)).collect())
        .unwrap();
    chart.apply_time_scale_options(HorzScaleOptionsPatch {
        min_bar_spacing: Some(0.001),
        bar_spacing: Some(0.125),
        enable_conflation: Some(true),
        ..Default::default()
    });
    assert!(chart.update_data(id, row(1, 500.), true).is_err());
    use feather_charts::model::plot_data::PlotRowLike;
    assert_eq!(
        chart.model().series(id).unwrap().bars().rows()[1].values()[3],
        1.
    );
    chart.apply_time_scale_options(HorzScaleOptionsPatch {
        bar_spacing: Some(6.),
        ..Default::default()
    });
    chart.update_data(id, row(1, 500.), true).unwrap();
    assert_eq!(
        chart.model().series(id).unwrap().bars().rows()[1].values()[3],
        500.
    );
}
