//! Event-sequence input/output for the original DataLayer and TickMarks oracle.
use feather_charts::model::{
    data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem, TimedData, WhitespaceData},
    data_layer::{DataLayer, DataUpdateResponse, SeriesId},
    get_series_plot_row_creator::CustomPlotRowBuilder,
    horz_scale_behavior_time::{
        horz_scale_behavior_time::HorzScaleBehaviorTime,
        types::{Time, TimePoint, UtcTimestamp},
    },
    icustom_series::{CustomData, CustomSeriesDataItem},
    plot_data::PlotRowLike,
    series_options::SeriesType,
    tick_marks::TickMarks,
    time_data::TimeScalePoint,
};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone)]
struct Datum {
    time: Time,
    value: f64,
}
impl TimedData for Datum {
    type Item = Time;
    fn time(&self) -> &Time {
        &self.time
    }
    fn time_mut(&mut self) -> &mut Time {
        &mut self.time
    }
}
impl CustomData for Datum {}
struct Builder;
impl CustomPlotRowBuilder<Datum, Time, ()> for Builder {
    fn values(&self, data: &Datum) -> Vec<f64> {
        vec![data.value]
    }
    fn is_whitespace(&self, data: &CustomSeriesDataItem<Datum, Time>) -> bool {
        matches!(data, CustomSeriesDataItem::Whitespace(_))
    }
}
fn line(time: f64, value: Option<f64>) -> BuiltInSeriesDataItem<Time> {
    let time: Time = UtcTimestamp::new(time).into();
    BuiltInSeriesDataItem::Line(value.map_or_else(
        || {
            LineDataItem::Whitespace(WhitespaceData {
                time: time.clone(),
                custom_values: None,
            })
        },
        |value| {
            LineDataItem::Data(LineData {
                time: time.clone(),
                value,
                color: None,
                custom_values: None,
            })
        },
    ))
}
fn custom(time: f64, value: Option<f64>) -> CustomSeriesDataItem<Datum, Time> {
    let time: Time = UtcTimestamp::new(time).into();
    value.map_or_else(
        || {
            CustomSeriesDataItem::Whitespace(
                feather_charts::model::icustom_series::CustomSeriesWhitespaceData {
                    time: time.clone(),
                    custom_values: None,
                },
            )
        },
        |value| {
            CustomSeriesDataItem::Data(Datum {
                time: time.clone(),
                value,
            })
        },
    )
}
fn main() {
    let mut layer: DataLayer<HorzScaleBehaviorTime> = DataLayer::new(Default::default());
    let mut points: Vec<TimeScalePoint<TimePoint, Time>> = vec![];
    let mut rows: BTreeMap<u64, Vec<(f64, f64)>> = BTreeMap::new();
    let mut output = Vec::new();
    // id, custom, replacement, historical, time, value/whitespace
    let events = [
        (1, false, true, false, 1_704_067_200., Some(10.)),
        (2, true, true, false, 1_704_067_200., Some(20.)),
        (1, false, false, false, 1_704_067_260., Some(11.)),
        (2, true, false, false, 1_704_067_320., None),
        (2, true, false, false, 1_704_067_320., Some(21.)),
        (1, false, false, false, 1_704_067_380., Some(12.)),
        (2, true, false, false, 1_704_067_350., Some(22.)),
        (1, false, false, true, 1_704_067_260., Some(13.)),
        (2, true, false, false, 1_704_153_600., Some(23.)),
        (1, false, false, false, 1_706_832_000., None),
        (1, false, false, false, 1_706_832_000., Some(14.)),
        (1, false, true, false, 1_706_918_400., Some(15.)),
        (2, true, false, false, 1_709_251_200., Some(24.)),
    ];
    for (step, (id, is_custom, replace, historical, time, value)) in events.into_iter().enumerate()
    {
        let series = SeriesId::new(id);
        let update = if is_custom {
            let update = if replace {
                layer.set_custom_series_data(series, vec![custom(time, value)], &Builder)
            } else {
                layer.update_custom_series_data(series, custom(time, value), &Builder, historical)
            }
            .unwrap();
            let owner = rows.entry(id).or_default();
            if update.is_full_replacement {
                owner.clear();
            }
            if let Some(index) = update.changed_index {
                owner.retain(|row| row.0 != index.value());
            }
            owner.extend(
                update
                    .custom_rows
                    .iter()
                    .map(|row| (row.base.index.value(), row.base.value[3])),
            );
            owner.sort_by(|a, b| a.0.total_cmp(&b.0));
            DataUpdateResponse {
                series: update.series,
                custom: update.custom,
                time_scale: update.time_scale,
            }
        } else {
            if replace {
                layer.set_series_data(series, SeriesType::Line, vec![line(time, value)])
            } else {
                layer.update_series_data(series, line(time, value), historical)
            }
            .unwrap()
        };
        if let Some(full) = update.time_scale.points {
            points = full;
        }
        if let Some(append) = update.time_scale.append {
            assert_eq!(append.index, points.len());
            points.push(append.point);
        }
        for (id, change) in update.series {
            let owner = rows.entry(id.value()).or_default();
            if let Some(index) = change.changed_index {
                owner.retain(|row| row.0 != index.value());
            } else {
                owner.clear();
            }
            owner.extend(
                change
                    .rows
                    .iter()
                    .map(|row| (row.index().value(), row.values()[3])),
            );
            owner.sort_by(|a, b| a.0.total_cmp(&b.0));
        }
        for (id, change) in update.custom {
            if !change.indices_unchanged {
                let owner = rows.get_mut(&id.value()).unwrap();
                assert_eq!(owner.len(), change.indices.len());
                for (row, index) in owner.iter_mut().zip(change.indices) {
                    row.0 = index.value();
                }
            }
        }
        let fulfilled: BTreeSet<usize> = rows
            .values()
            .flat_map(|owner| owner.iter().map(|row| row.0 as usize))
            .collect();
        let mut tick_cases = Vec::new();
        for uniform in [false, true] {
            for filter in [false, true] {
                for spacing in [1., 3., 13.] {
                    let mut ticks = TickMarks::default();
                    ticks.set_uniform_distribution(uniform);
                    ticks.set_time_scale_points(&points, 0);
                    let marks = ticks
                        .build(1., spacing, filter, &fulfilled, 0)
                        .iter()
                        .map(|mark| format!("[{},{}]", mark.index.value(), mark.weight.value()))
                        .collect::<Vec<_>>()
                        .join(",");
                    tick_cases.push(format!("[{}]", marks));
                }
            }
        }
        let shared = points
            .iter()
            .map(|point| {
                format!(
                    "[{},{}]",
                    point.time.timestamp.seconds(),
                    point.time_weight.value()
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let owners = rows
            .iter()
            .map(|(id, rows)| {
                format!(
                    "[{},[{}]]",
                    id,
                    rows.iter()
                        .map(|(index, value)| format!("[{index},{value}]"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        output.push(format!("{{\"step\":{step},\"points\":[{shared}],\"rows\":[{owners}],\"base\":{},\"ticks\":[{}]}}",update.time_scale.base_index.map_or("null".into(),|index|index.value().to_string()),tick_cases.join(",")));
    }
    println!("[{}]", output.join(","));
}
