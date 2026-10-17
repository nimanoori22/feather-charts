use super::*;
use crate::model::{
    data_consumer::{AreaData, AreaDataItem, LineData},
    time_data::{TimePointIndex, TimeScalePoint},
};
fn m(v: f64) -> Maturity {
    Maturity::from_months(v).unwrap()
}
fn row(area: bool, t: f64, v: Option<f64>) -> BuiltInSeriesDataItem<Maturity> {
    match (area, v) {
        (_, None) => BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(WhitespaceData {
            time: m(t),
            custom_values: None,
        })),
        (false, Some(value)) => BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
            time: m(t),
            value,
            color: None,
            custom_values: None,
        })),
        (true, Some(value)) => BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
            time: m(t),
            value,
            line_color: None,
            top_color: None,
            bottom_color: None,
            custom_values: None,
        })),
    }
}
fn snapshot(c: &mut YieldCurveChart, name: &str) -> String {
    c.settle().unwrap();
    let keys = (0..100_001)
        .map_while(|i| {
            c.model()
                .time_scale()
                .index_to_time(TimePointIndex::new(i as f64))
                .map(|t| t.months())
        })
        .collect::<Vec<_>>();
    let weights = weights(&keys);
    let support = c
        .scaffold
        .unwrap()
        .items()
        .map(|m| m.months())
        .collect::<Vec<_>>();
    format!(
        "{{\"name\":{name:?},\"keys\":{keys:?},\"weights\":{weights:?},\"support\":{support:?},\"rebuilds\":{}}}",
        c.rebuilds
    )
}
fn weights(values: &[f64]) -> Vec<i32> {
    let mut behavior = YieldCurveHorzScaleBehavior::default();
    let mut points = values
        .iter()
        .map(|v| TimeScalePoint {
            time: m(*v),
            original_time: m(*v),
            time_weight: Default::default(),
        })
        .collect::<Vec<_>>();
    behavior.fill_weights_for_points(&mut points, 0);
    points.iter().map(|p| p.time_weight.value()).collect()
}
#[test]
fn yield_curve_reference() {
    let samples = [
        -12., -0., 0., 0.25, 1., 2., 3., 6., 11.5, 12., 13., 18., 24., 36., 60., 120., 121.5, 240.,
        360., 420.,
    ];
    let labels = samples.map(|v| m(v).label());
    let weights = weights(&samples);
    let mut behavior = YieldCurveHorzScaleBehavior::default();
    let mut localization = LocalizationOptions::<Maturity>::new("en-US", "");
    localization.time_formatter = Some(Rc::new(|v| format!("X{}", v.months())));
    behavior.update_formatter(&localization);
    let custom = samples.map(|v| behavior.format_item(&m(v)));
    let scaffold = [
        (0., 120., 1., 0.),
        (3., 10., 1., 360.),
        (3., 10., 12., 360.),
        (-3., -5., 0.25, 6.),
        (20., 10., 1., 0.),
        (0., 3., 1., 2.5),
        (0.5, 5., 1.5, 7.),
        (0., 0., 1., 0.),
    ]
    .map(|(start, minimum, resolution, last)| {
        YieldCurveOptions {
            start_time_range: start,
            minimum_time_range: minimum,
            base_resolution: resolution,
        }
        .scaffold(last)
        .unwrap()
        .items()
        .map(|m| m.months())
        .collect::<Vec<_>>()
    });
    let mut c = YieldCurveChart::new(Default::default()).unwrap();
    let mut states = vec![snapshot(&mut c, "empty")];
    let line = c
        .add_line_series(Default::default(), 0, PriceScalePosition::Left)
        .unwrap();
    let area = c
        .add_area_series(Default::default(), 0, PriceScalePosition::Left)
        .unwrap();
    c.set_data(
        line,
        vec![
            row(false, 1., Some(5.)),
            row(false, 6., Some(4.)),
            row(false, 360., Some(3.)),
        ],
    )
    .unwrap();
    states.push(snapshot(&mut c, "line"));
    c.set_data(
        area,
        vec![
            row(true, 3., Some(5.)),
            row(true, 12., Some(4.)),
            row(true, 240., Some(3.)),
        ],
    )
    .unwrap();
    states.push(snapshot(&mut c, "area"));
    c.update_data(line, row(false, 6., Some(4.2)), true)
        .unwrap();
    states.push(snapshot(&mut c, "historical"));
    c.update_data(area, row(true, 420., Some(3.1)), false)
        .unwrap();
    states.push(snapshot(&mut c, "extend"));
    c.update_data(
        area,
        BuiltInSeriesDataItem::Area(AreaDataItem::Whitespace(WhitespaceData {
            time: m(420.),
            custom_values: None,
        })),
        false,
    )
    .unwrap();
    states.push(snapshot(&mut c, "whitespace"));
    c.set_data(
        line,
        vec![row(false, 1., Some(5.)), row(false, 6., Some(4.))],
    )
    .unwrap();
    states.push(snapshot(&mut c, "shorter"));
    c.remove_series(area).unwrap();
    states.push(snapshot(&mut c, "remove_longest"));
    c.set_data(line, vec![]).unwrap();
    states.push(snapshot(&mut c, "clear"));
    println!(
        "YIELD_REFERENCE {{\"labels\":{labels:?},\"weights\":{weights:?},\"scaffold\":{scaffold:?},\"custom\":{custom:?},\"states\":[{}]}}",
        states.join(",")
    );
}
#[test]
fn value_only_updates_and_interactions_do_not_rebuild_scaffold() {
    let mut c = YieldCurveChart::new(Default::default()).unwrap();
    let line = c
        .add_line_series(Default::default(), 0, PriceScalePosition::Left)
        .unwrap();
    c.set_data(
        line,
        vec![row(false, 1., Some(5.)), row(false, 12., Some(4.))],
    )
    .unwrap();
    let now = Instant::now();
    c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
    let rebuilds = c.rebuilds;
    for value in 0..25 {
        c.update_data(line, row(false, 12., Some(4. + value as f64 / 100.)), false)
            .unwrap();
        c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
        c.update(ChartMessage::Tick(now)).unwrap();
        c.set_right_offset(value as f64);
        c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
    }
    assert_eq!(c.rebuilds, rebuilds);
    c.set_data(line, vec![row(false, 420., Some(4.))]).unwrap();
    c.set_data(line, vec![row(false, 12., Some(4.))]).unwrap();
    c.prepare(iced::Size::new(800., 500.), 1., now).unwrap();
    assert_eq!(
        c.rebuilds, rebuilds,
        "unsettled transient extent must not persist"
    );
}
