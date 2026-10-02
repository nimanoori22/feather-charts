//! Series plot rows, including DataLayer-owned whitespace rows.

use crate::model::{
    plot_data::{MutablePlotRow, PlotRow, PlotRowLike, PlotRowValue},
    plot_list::PlotList,
    series_options::SeriesType,
    time_data::TimePointIndex,
};

#[derive(Clone, Debug, PartialEq)]
pub struct LinePlotRow<I, O, M = ()> {
    pub base: PlotRow<I, O, M>,
    pub color: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct HistogramPlotRow<I, O, M = ()> {
    pub base: PlotRow<I, O, M>,
    pub color: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BarPlotRow<I, O, M = ()> {
    pub base: PlotRow<I, O, M>,
    pub color: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CandlestickPlotRow<I, O, M = ()> {
    pub base: PlotRow<I, O, M>,
    pub color: Option<String>,
    pub border_color: Option<String>,
    pub wick_color: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AreaPlotRow<I, O, M = ()> {
    pub base: PlotRow<I, O, M>,
    pub line_color: Option<String>,
    pub top_color: Option<String>,
    pub bottom_color: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BaselinePlotRow<I, O, M = ()> {
    pub base: PlotRow<I, O, M>,
    pub top_fill_color1: Option<String>,
    pub top_fill_color2: Option<String>,
    pub top_line_color: Option<String>,
    pub bottom_fill_color1: Option<String>,
    pub bottom_fill_color2: Option<String>,
    pub bottom_line_color: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CustomPlotRow<I, O, D, M = ()> {
    pub base: PlotRow<I, O, M>,
    pub data: D,
    pub color: Option<String>,
}

/// A time point belonging to a series but containing no value. It must remain
/// in DataLayer so the horizontal scale retains explicit whitespace.
#[derive(Clone, Debug, PartialEq)]
pub struct WhitespacePlotRow<I, O, M = ()> {
    pub index: TimePointIndex,
    pub time: I,
    pub original_time: O,
    pub custom_values: Option<M>,
}

macro_rules! row_traits {
    ($($type:ident<$($generic:ident),+>),+ $(,)?) => {$(
        impl<$($generic),+> PlotRowLike for $type<$($generic),+> {
            fn index(&self) -> TimePointIndex { self.base.index }
            fn values(&self) -> &PlotRowValue { &self.base.value }
        }
        impl<$($generic),+> MutablePlotRow for $type<$($generic),+> {
            type InternalTime = I;
            type OriginalTime = O;
            fn index(&self) -> TimePointIndex { self.base.index }
            fn set_index(&mut self, index: TimePointIndex) { self.base.index = index; }
            fn time(&self) -> &I { &self.base.time }
            fn original_time(&self) -> &O { &self.base.original_time }
            fn values(&self) -> Option<&PlotRowValue> { Some(&self.base.value) }
        }
    )+};
}

row_traits!(
    LinePlotRow<I, O, M>, HistogramPlotRow<I, O, M>, BarPlotRow<I, O, M>,
    CandlestickPlotRow<I, O, M>, AreaPlotRow<I, O, M>, BaselinePlotRow<I, O, M>,
    CustomPlotRow<I, O, D, M>,
);

impl<I, O, M> MutablePlotRow for WhitespacePlotRow<I, O, M> {
    type InternalTime = I;
    type OriginalTime = O;
    fn index(&self) -> TimePointIndex {
        self.index
    }
    fn set_index(&mut self, index: TimePointIndex) {
        self.index = index;
    }
    fn time(&self) -> &I {
        &self.time
    }
    fn original_time(&self) -> &O {
        &self.original_time
    }
    fn values(&self) -> Option<&PlotRowValue> {
        None
    }
}

pub type SeriesPlotList<Row> = PlotList<Row>;
pub fn create_series_plot_list<Row: PlotRowLike + Clone>() -> PlotList<Row> {
    PlotList::new()
}

#[derive(Clone, Debug, PartialEq)]
pub enum SeriesPlotRow<I, O, D = (), M = ()> {
    Bar(BarPlotRow<I, O, M>),
    Candlestick(CandlestickPlotRow<I, O, M>),
    Area(AreaPlotRow<I, O, M>),
    Baseline(BaselinePlotRow<I, O, M>),
    Line(LinePlotRow<I, O, M>),
    Histogram(HistogramPlotRow<I, O, M>),
    Custom(CustomPlotRow<I, O, D, M>),
}

impl<I, O, D, M> SeriesPlotRow<I, O, D, M> {
    pub const fn series_type(&self) -> SeriesType {
        match self {
            Self::Bar(_) => SeriesType::Bar,
            Self::Candlestick(_) => SeriesType::Candlestick,
            Self::Area(_) => SeriesType::Area,
            Self::Baseline(_) => SeriesType::Baseline,
            Self::Line(_) => SeriesType::Line,
            Self::Histogram(_) => SeriesType::Histogram,
            Self::Custom(_) => SeriesType::Custom,
        }
    }
}

impl<I, O, D, M> PlotRowLike for SeriesPlotRow<I, O, D, M> {
    fn index(&self) -> TimePointIndex {
        match self {
            Self::Bar(row) => PlotRowLike::index(row),
            Self::Candlestick(row) => PlotRowLike::index(row),
            Self::Area(row) => PlotRowLike::index(row),
            Self::Baseline(row) => PlotRowLike::index(row),
            Self::Line(row) => PlotRowLike::index(row),
            Self::Histogram(row) => PlotRowLike::index(row),
            Self::Custom(row) => PlotRowLike::index(row),
        }
    }
    fn values(&self) -> &PlotRowValue {
        match self {
            Self::Bar(row) => PlotRowLike::values(row),
            Self::Candlestick(row) => PlotRowLike::values(row),
            Self::Area(row) => PlotRowLike::values(row),
            Self::Baseline(row) => PlotRowLike::values(row),
            Self::Line(row) => PlotRowLike::values(row),
            Self::Histogram(row) => PlotRowLike::values(row),
            Self::Custom(row) => PlotRowLike::values(row),
        }
    }
}

impl<I, O, D, M> MutablePlotRow for SeriesPlotRow<I, O, D, M> {
    type InternalTime = I;
    type OriginalTime = O;
    fn index(&self) -> TimePointIndex {
        PlotRowLike::index(self)
    }
    fn set_index(&mut self, index: TimePointIndex) {
        match self {
            Self::Bar(row) => row.set_index(index),
            Self::Candlestick(row) => row.set_index(index),
            Self::Area(row) => row.set_index(index),
            Self::Baseline(row) => row.set_index(index),
            Self::Line(row) => row.set_index(index),
            Self::Histogram(row) => row.set_index(index),
            Self::Custom(row) => row.set_index(index),
        }
    }
    fn time(&self) -> &I {
        match self {
            Self::Bar(row) => MutablePlotRow::time(row),
            Self::Candlestick(row) => MutablePlotRow::time(row),
            Self::Area(row) => MutablePlotRow::time(row),
            Self::Baseline(row) => MutablePlotRow::time(row),
            Self::Line(row) => MutablePlotRow::time(row),
            Self::Histogram(row) => MutablePlotRow::time(row),
            Self::Custom(row) => MutablePlotRow::time(row),
        }
    }
    fn original_time(&self) -> &O {
        match self {
            Self::Bar(row) => MutablePlotRow::original_time(row),
            Self::Candlestick(row) => MutablePlotRow::original_time(row),
            Self::Area(row) => MutablePlotRow::original_time(row),
            Self::Baseline(row) => MutablePlotRow::original_time(row),
            Self::Line(row) => MutablePlotRow::original_time(row),
            Self::Histogram(row) => MutablePlotRow::original_time(row),
            Self::Custom(row) => MutablePlotRow::original_time(row),
        }
    }
    fn values(&self) -> Option<&PlotRowValue> {
        Some(PlotRowLike::values(self))
    }
}

/// The union DataLayer stores before filtering fulfilled rows into a PlotList.
#[derive(Clone, Debug, PartialEq)]
pub enum SeriesDataRow<I, O, D = (), M = ()> {
    Value(SeriesPlotRow<I, O, D, M>),
    Whitespace(WhitespacePlotRow<I, O, M>),
}

impl<I, O, D, M> SeriesDataRow<I, O, D, M> {
    pub const fn is_whitespace(&self) -> bool {
        matches!(self, Self::Whitespace(_))
    }
    pub const fn is_fulfilled(&self) -> bool {
        matches!(self, Self::Value(_))
    }
    pub fn into_value(self) -> Option<SeriesPlotRow<I, O, D, M>> {
        match self {
            Self::Value(row) => Some(row),
            Self::Whitespace(_) => None,
        }
    }
}

impl<I, O, D, M> MutablePlotRow for SeriesDataRow<I, O, D, M> {
    type InternalTime = I;
    type OriginalTime = O;
    fn index(&self) -> TimePointIndex {
        match self {
            Self::Value(row) => MutablePlotRow::index(row),
            Self::Whitespace(row) => MutablePlotRow::index(row),
        }
    }
    fn set_index(&mut self, index: TimePointIndex) {
        match self {
            Self::Value(row) => row.set_index(index),
            Self::Whitespace(row) => row.set_index(index),
        }
    }
    fn time(&self) -> &I {
        match self {
            Self::Value(row) => row.time(),
            Self::Whitespace(row) => row.time(),
        }
    }
    fn original_time(&self) -> &O {
        match self {
            Self::Value(row) => row.original_time(),
            Self::Whitespace(row) => row.original_time(),
        }
    }
    fn values(&self) -> Option<&PlotRowValue> {
        match self {
            Self::Value(row) => MutablePlotRow::values(row),
            Self::Whitespace(row) => MutablePlotRow::values(row),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_and_whitespace_rows_share_mutable_index_access() {
        let mut row = SeriesDataRow::<(), (), (), String>::Whitespace(WhitespacePlotRow {
            index: 1.0.into(),
            time: (),
            original_time: (),
            custom_values: Some("tag".into()),
        });
        row.set_index(7.0.into());
        assert_eq!(row.index().value(), 7.0);
        assert!(row.values().is_none());
        assert!(row.is_whitespace());
    }
}
