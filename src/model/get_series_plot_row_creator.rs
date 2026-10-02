//! Conversion from typed public series input into indexed model plot rows.

use crate::model::{
    data_consumer::{
        BarDataItem, BuiltInSeriesDataItem, CandlestickDataItem, HistogramDataItem, LineDataItem,
    },
    icustom_series::{CustomData, CustomSeriesDataItem},
    plot_data::PlotRow,
    series_data::{
        AreaPlotRow, BarPlotRow, BaselinePlotRow, CandlestickPlotRow, CustomPlotRow,
        HistogramPlotRow, LinePlotRow, SeriesDataRow, SeriesPlotRow, WhitespacePlotRow,
    },
    series_options::SeriesType,
    time_data::TimePointIndex,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlotRowCreationError {
    SeriesTypeMismatch {
        expected: SeriesType,
        actual: SeriesType,
    },
    EmptyCustomValues,
}

pub trait CustomPlotRowBuilder<Data, Item, Metadata> {
    fn values(&self, data: &Data) -> Vec<f64>;
    fn is_whitespace(&self, data: &CustomSeriesDataItem<Data, Item, Metadata>) -> bool;
}

fn base<I, O, M>(
    time: I,
    index: TimePointIndex,
    original_time: O,
    custom_values: Option<M>,
    value: [f64; 4],
) -> PlotRow<I, O, M> {
    PlotRow {
        index,
        time,
        original_time,
        value,
        custom_values,
        original_data_count: None,
    }
}

pub fn create_line_row<I, O, M>(
    time: I,
    index: TimePointIndex,
    item: LineDataItem<O, M>,
    original_time: O,
) -> SeriesDataRow<I, O, (), M> {
    match item {
        LineDataItem::Whitespace(item) => SeriesDataRow::Whitespace(WhitespacePlotRow {
            index,
            time,
            original_time,
            custom_values: item.custom_values,
        }),
        LineDataItem::Data(item) => SeriesDataRow::Value(SeriesPlotRow::Line(LinePlotRow {
            base: base(
                time,
                index,
                original_time,
                item.custom_values,
                [item.value; 4],
            ),
            color: item.color,
        })),
    }
}

pub fn create_histogram_row<I, O, M>(
    time: I,
    index: TimePointIndex,
    item: HistogramDataItem<O, M>,
    original_time: O,
) -> SeriesDataRow<I, O, (), M> {
    match item {
        HistogramDataItem::Whitespace(item) => SeriesDataRow::Whitespace(WhitespacePlotRow {
            index,
            time,
            original_time,
            custom_values: item.custom_values,
        }),
        HistogramDataItem::Data(item) => {
            SeriesDataRow::Value(SeriesPlotRow::Histogram(HistogramPlotRow {
                base: base(
                    time,
                    index,
                    original_time,
                    item.custom_values,
                    [item.value; 4],
                ),
                color: item.color,
            }))
        }
    }
}

pub fn create_area_row<I, O, M>(
    time: I,
    index: TimePointIndex,
    item: crate::model::data_consumer::AreaDataItem<O, M>,
    original_time: O,
) -> SeriesDataRow<I, O, (), M> {
    match item {
        crate::model::data_consumer::AreaDataItem::Whitespace(item) => {
            SeriesDataRow::Whitespace(WhitespacePlotRow {
                index,
                time,
                original_time,
                custom_values: item.custom_values,
            })
        }
        crate::model::data_consumer::AreaDataItem::Data(item) => {
            SeriesDataRow::Value(SeriesPlotRow::Area(AreaPlotRow {
                base: base(
                    time,
                    index,
                    original_time,
                    item.custom_values,
                    [item.value; 4],
                ),
                line_color: item.line_color,
                top_color: item.top_color,
                bottom_color: item.bottom_color,
            }))
        }
    }
}

pub fn create_baseline_row<I, O, M>(
    time: I,
    index: TimePointIndex,
    item: crate::model::data_consumer::BaselineDataItem<O, M>,
    original_time: O,
) -> SeriesDataRow<I, O, (), M> {
    match item {
        crate::model::data_consumer::BaselineDataItem::Whitespace(item) => {
            SeriesDataRow::Whitespace(WhitespacePlotRow {
                index,
                time,
                original_time,
                custom_values: item.custom_values,
            })
        }
        crate::model::data_consumer::BaselineDataItem::Data(item) => {
            SeriesDataRow::Value(SeriesPlotRow::Baseline(BaselinePlotRow {
                base: base(
                    time,
                    index,
                    original_time,
                    item.custom_values,
                    [item.value; 4],
                ),
                top_fill_color1: item.top_fill_color1,
                top_fill_color2: item.top_fill_color2,
                top_line_color: item.top_line_color,
                bottom_fill_color1: item.bottom_fill_color1,
                bottom_fill_color2: item.bottom_fill_color2,
                bottom_line_color: item.bottom_line_color,
            }))
        }
    }
}

pub fn create_bar_row<I, O, M>(
    time: I,
    index: TimePointIndex,
    item: BarDataItem<O, M>,
    original_time: O,
) -> SeriesDataRow<I, O, (), M> {
    match item {
        BarDataItem::Whitespace(item) => SeriesDataRow::Whitespace(WhitespacePlotRow {
            index,
            time,
            original_time,
            custom_values: item.custom_values,
        }),
        BarDataItem::Data(item) => SeriesDataRow::Value(SeriesPlotRow::Bar(BarPlotRow {
            base: base(
                time,
                index,
                original_time,
                item.custom_values,
                [item.open, item.high, item.low, item.close],
            ),
            color: item.color,
        })),
    }
}

pub fn create_candlestick_row<I, O, M>(
    time: I,
    index: TimePointIndex,
    item: CandlestickDataItem<O, M>,
    original_time: O,
) -> SeriesDataRow<I, O, (), M> {
    match item {
        CandlestickDataItem::Whitespace(item) => SeriesDataRow::Whitespace(WhitespacePlotRow {
            index,
            time,
            original_time,
            custom_values: item.custom_values,
        }),
        CandlestickDataItem::Data(item) => {
            SeriesDataRow::Value(SeriesPlotRow::Candlestick(CandlestickPlotRow {
                base: base(
                    time,
                    index,
                    original_time,
                    item.custom_values,
                    [item.open, item.high, item.low, item.close],
                ),
                color: item.color,
                border_color: item.border_color,
                wick_color: item.wick_color,
            }))
        }
    }
}

/// Runtime dispatch used by DataLayer. Mismatches are invariant failures that
/// cannot occur through a typed public series API.
pub fn create_builtin_plot_row<I, O, M>(
    series_type: SeriesType,
    time: I,
    index: TimePointIndex,
    item: BuiltInSeriesDataItem<O, M>,
    original_time: O,
) -> Result<SeriesDataRow<I, O, (), M>, PlotRowCreationError> {
    let actual = match &item {
        BuiltInSeriesDataItem::Line(_) => SeriesType::Line,
        BuiltInSeriesDataItem::Histogram(_) => SeriesType::Histogram,
        BuiltInSeriesDataItem::Area(_) => SeriesType::Area,
        BuiltInSeriesDataItem::Baseline(_) => SeriesType::Baseline,
        BuiltInSeriesDataItem::Bar(_) => SeriesType::Bar,
        BuiltInSeriesDataItem::Candlestick(_) => SeriesType::Candlestick,
    };
    if series_type != actual {
        return Err(PlotRowCreationError::SeriesTypeMismatch {
            expected: series_type,
            actual,
        });
    }
    Ok(match item {
        BuiltInSeriesDataItem::Line(item) => create_line_row(time, index, item, original_time),
        BuiltInSeriesDataItem::Histogram(item) => {
            create_histogram_row(time, index, item, original_time)
        }
        BuiltInSeriesDataItem::Area(item) => create_area_row(time, index, item, original_time),
        BuiltInSeriesDataItem::Baseline(item) => {
            create_baseline_row(time, index, item, original_time)
        }
        BuiltInSeriesDataItem::Bar(item) => create_bar_row(time, index, item, original_time),
        BuiltInSeriesDataItem::Candlestick(item) => {
            create_candlestick_row(time, index, item, original_time)
        }
    })
}

pub fn create_custom_plot_row<I, O, D, M>(
    time: I,
    index: TimePointIndex,
    item: CustomSeriesDataItem<D, O, M>,
    original_time: O,
    builder: &impl CustomPlotRowBuilder<D, O, M>,
) -> Result<SeriesDataRow<I, O, D, M>, PlotRowCreationError>
where
    D: CustomData<Item = O>,
{
    if builder.is_whitespace(&item) {
        return Ok(match item {
            CustomSeriesDataItem::Whitespace(item) => {
                SeriesDataRow::Whitespace(WhitespacePlotRow {
                    index,
                    time,
                    original_time,
                    custom_values: item.custom_values,
                })
            }
            CustomSeriesDataItem::Data(_) => SeriesDataRow::Whitespace(WhitespacePlotRow {
                index,
                time,
                original_time,
                custom_values: None,
            }),
        });
    }
    match item {
        CustomSeriesDataItem::Whitespace(item) => {
            Ok(SeriesDataRow::Whitespace(WhitespacePlotRow {
                index,
                time,
                original_time,
                custom_values: item.custom_values,
            }))
        }
        CustomSeriesDataItem::Data(data) => {
            let values = builder.values(&data);
            let Some(&last) = values.last() else {
                return Err(PlotRowCreationError::EmptyCustomValues);
            };
            let high = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let low = values.iter().copied().fold(f64::INFINITY, f64::min);
            let color = data.color().map(str::to_owned);
            Ok(SeriesDataRow::Value(SeriesPlotRow::Custom(CustomPlotRow {
                base: base(time, index, original_time, None, [last, high, low, last]),
                data,
                color,
            })))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        data_consumer::{
            AreaData, AreaDataItem, BarData, BaselineData, BaselineDataItem, CandlestickData,
            CandlestickDataItem, HistogramData, HistogramDataItem, LineData, WhitespaceData,
        },
        icustom_series::CustomData,
        plot_data::MutablePlotRow,
    };

    #[derive(Clone, Debug, PartialEq)]
    struct Datum {
        time: u64,
        values: Vec<f64>,
    }
    impl crate::model::data_consumer::TimedData for Datum {
        type Item = u64;
        fn time(&self) -> &u64 {
            &self.time
        }
        fn time_mut(&mut self) -> &mut u64 {
            &mut self.time
        }
    }
    impl CustomData for Datum {}
    struct Builder;
    impl CustomPlotRowBuilder<Datum, u64, String> for Builder {
        fn values(&self, data: &Datum) -> Vec<f64> {
            data.values.clone()
        }
        fn is_whitespace(&self, data: &CustomSeriesDataItem<Datum, u64, String>) -> bool {
            matches!(data, CustomSeriesDataItem::Whitespace(_))
        }
    }

    #[test]
    fn builds_single_value_styles_and_preserves_metadata() {
        let row = create_line_row(
            20_u64,
            3.0.into(),
            LineDataItem::Data(LineData {
                time: 2_u64,
                value: 4.0,
                color: Some("#f00".into()),
                custom_values: Some("note"),
            }),
            2,
        );
        let SeriesDataRow::Value(SeriesPlotRow::Line(row)) = row else {
            panic!("line row")
        };
        assert_eq!(row.base.value, [4.0; 4]);
        assert_eq!(row.color.as_deref(), Some("#f00"));
        assert_eq!(row.base.custom_values.as_deref(), Some("note"));
        let whitespace = create_area_row(
            30,
            4.0.into(),
            AreaDataItem::Whitespace(WhitespaceData {
                time: 3,
                custom_values: Some("gap"),
            }),
            3,
        );
        assert!(whitespace.is_whitespace());
        assert_eq!(whitespace.values(), None);
    }

    #[test]
    fn builds_ohlc_and_rejects_runtime_mismatches() {
        let item = BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(CandlestickData {
            time: 3,
            open: 1.,
            high: 4.,
            low: 0.,
            close: 2.,
            color: Some("#f00".into()),
            border_color: Some("#0f0".into()),
            wick_color: Some("#00f".into()),
            custom_values: None::<()>,
        }));
        let row =
            create_builtin_plot_row(SeriesType::Candlestick, 30, 5.0.into(), item, 3).unwrap();
        let SeriesDataRow::Value(SeriesPlotRow::Candlestick(row)) = row else {
            panic!("candle row")
        };
        assert_eq!(row.base.value, [1., 4., 0., 2.]);
        assert_eq!(row.wick_color.as_deref(), Some("#00f"));
        let bad = BuiltInSeriesDataItem::Bar(BarDataItem::Whitespace(WhitespaceData {
            time: 3,
            custom_values: None::<()>,
        }));
        assert_eq!(
            create_builtin_plot_row(SeriesType::Line, 30, 5.0.into(), bad, 3),
            Err(PlotRowCreationError::SeriesTypeMismatch {
                expected: SeriesType::Line,
                actual: SeriesType::Bar
            })
        );
    }

    #[test]
    fn builds_every_remaining_builtin_style() {
        let area = create_area_row(
            1,
            0.0.into(),
            AreaDataItem::Data(AreaData {
                time: 1,
                value: 2.,
                line_color: Some("line".into()),
                top_color: Some("top".into()),
                bottom_color: Some("bottom".into()),
                custom_values: None::<()>,
            }),
            1,
        );
        let SeriesDataRow::Value(SeriesPlotRow::Area(area)) = area else {
            panic!("area row")
        };
        assert_eq!(area.base.value, [2.; 4]);
        assert_eq!(area.top_color.as_deref(), Some("top"));
        let baseline = create_baseline_row(
            2,
            1.0.into(),
            BaselineDataItem::Data(BaselineData {
                time: 2,
                value: 3.,
                top_fill_color1: Some("a".into()),
                top_fill_color2: None,
                top_line_color: None,
                bottom_fill_color1: None,
                bottom_fill_color2: None,
                bottom_line_color: Some("b".into()),
                custom_values: None::<()>,
            }),
            2,
        );
        let SeriesDataRow::Value(SeriesPlotRow::Baseline(baseline)) = baseline else {
            panic!("baseline row")
        };
        assert_eq!(baseline.base.value, [3.; 4]);
        assert_eq!(baseline.bottom_line_color.as_deref(), Some("b"));
        let histogram = create_histogram_row(
            3,
            2.0.into(),
            HistogramDataItem::Data(HistogramData {
                time: 3,
                value: 4.,
                color: Some("#abc".into()),
                custom_values: None::<()>,
            }),
            3,
        );
        let SeriesDataRow::Value(SeriesPlotRow::Histogram(histogram)) = histogram else {
            panic!("histogram row")
        };
        assert_eq!(histogram.base.value, [4.; 4]);
        let bar = create_bar_row(
            4,
            3.0.into(),
            BarDataItem::Data(BarData {
                time: 4,
                open: 1.,
                high: 5.,
                low: 0.,
                close: 4.,
                color: None,
                custom_values: None::<()>,
            }),
            4,
        );
        let SeriesDataRow::Value(SeriesPlotRow::Bar(bar)) = bar else {
            panic!("bar row")
        };
        assert_eq!(bar.base.value, [1., 5., 0., 4.]);
    }

    #[test]
    fn custom_rows_use_source_ohlc_projection_and_reject_empty_values() {
        let row = create_custom_plot_row(
            10,
            2.0.into(),
            CustomSeriesDataItem::Data(Datum {
                time: 1,
                values: vec![3., 7., 2.],
            }),
            1,
            &Builder,
        )
        .unwrap();
        let SeriesDataRow::Value(SeriesPlotRow::Custom(row)) = row else {
            panic!("custom row")
        };
        assert_eq!(row.base.value, [2., 7., 2., 2.]);
        let empty = create_custom_plot_row(
            10,
            2.0.into(),
            CustomSeriesDataItem::Data(Datum {
                time: 1,
                values: vec![],
            }),
            1,
            &Builder,
        );
        assert_eq!(empty, Err(PlotRowCreationError::EmptyCustomValues));
    }
}
