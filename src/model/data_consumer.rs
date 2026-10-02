//! Public, typed input data for the built-in chart series.
//!
//! TypeScript represents whitespace through object shapes with omitted value
//! fields. Rust uses explicit enums so a value-bearing item cannot be confused
//! with whitespace after it enters the model.

/// Data carrying a horizontal-scale item.
pub trait TimedData {
    type Item;

    fn time(&self) -> &Self::Item;
    fn time_mut(&mut self) -> &mut Self::Item;
}

/// A data point without a plotted value.
#[derive(Clone, Debug, PartialEq)]
pub struct WhitespaceData<T, Metadata = ()> {
    pub time: T,
    pub custom_values: Option<Metadata>,
}

/// Common values for single-value series.
#[derive(Clone, Debug, PartialEq)]
pub struct SingleValueData<T, Metadata = ()> {
    pub time: T,
    pub value: f64,
    pub custom_values: Option<Metadata>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineData<T, Metadata = ()> {
    pub time: T,
    pub value: f64,
    pub color: Option<String>,
    pub custom_values: Option<Metadata>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HistogramData<T, Metadata = ()> {
    pub time: T,
    pub value: f64,
    pub color: Option<String>,
    pub custom_values: Option<Metadata>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AreaData<T, Metadata = ()> {
    pub time: T,
    pub value: f64,
    pub line_color: Option<String>,
    pub top_color: Option<String>,
    pub bottom_color: Option<String>,
    pub custom_values: Option<Metadata>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BaselineData<T, Metadata = ()> {
    pub time: T,
    pub value: f64,
    pub top_fill_color1: Option<String>,
    pub top_fill_color2: Option<String>,
    pub top_line_color: Option<String>,
    pub bottom_fill_color1: Option<String>,
    pub bottom_fill_color2: Option<String>,
    pub bottom_line_color: Option<String>,
    pub custom_values: Option<Metadata>,
}

/// Common OHLC fields for bar and candlestick data.
#[derive(Clone, Debug, PartialEq)]
pub struct OhlcData<T, Metadata = ()> {
    pub time: T,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub custom_values: Option<Metadata>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BarData<T, Metadata = ()> {
    pub time: T,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub color: Option<String>,
    pub custom_values: Option<Metadata>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandlestickData<T, Metadata = ()> {
    pub time: T,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub color: Option<String>,
    pub border_color: Option<String>,
    pub wick_color: Option<String>,
    pub custom_values: Option<Metadata>,
}

macro_rules! timed_data {
    ($($type:ident),+ $(,)?) => {$(
        impl<T, Metadata> TimedData for $type<T, Metadata> {
            type Item = T;
            fn time(&self) -> &T { &self.time }
            fn time_mut(&mut self) -> &mut T { &mut self.time }
        }
    )+};
}

timed_data!(
    WhitespaceData,
    SingleValueData,
    LineData,
    HistogramData,
    AreaData,
    BaselineData,
    OhlcData,
    BarData,
    CandlestickData
);

/// Input accepted by a line series.
#[derive(Clone, Debug, PartialEq)]
pub enum LineDataItem<T, Metadata = ()> {
    Data(LineData<T, Metadata>),
    Whitespace(WhitespaceData<T, Metadata>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum HistogramDataItem<T, Metadata = ()> {
    Data(HistogramData<T, Metadata>),
    Whitespace(WhitespaceData<T, Metadata>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum AreaDataItem<T, Metadata = ()> {
    Data(AreaData<T, Metadata>),
    Whitespace(WhitespaceData<T, Metadata>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum BaselineDataItem<T, Metadata = ()> {
    Data(BaselineData<T, Metadata>),
    Whitespace(WhitespaceData<T, Metadata>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum BarDataItem<T, Metadata = ()> {
    Data(BarData<T, Metadata>),
    Whitespace(WhitespaceData<T, Metadata>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum CandlestickDataItem<T, Metadata = ()> {
    Data(CandlestickData<T, Metadata>),
    Whitespace(WhitespaceData<T, Metadata>),
}

macro_rules! timed_item {
    ($($type:ident),+ $(,)?) => {$(
        impl<T, Metadata> TimedData for $type<T, Metadata> {
            type Item = T;
            fn time(&self) -> &T { match self { Self::Data(data) => data.time(), Self::Whitespace(data) => data.time() } }
            fn time_mut(&mut self) -> &mut T { match self { Self::Data(data) => data.time_mut(), Self::Whitespace(data) => data.time_mut() } }
        }
        impl<T, Metadata> $type<T, Metadata> {
            pub const fn is_whitespace(&self) -> bool { matches!(self, Self::Whitespace(_)) }
            pub const fn is_fulfilled(&self) -> bool { matches!(self, Self::Data(_)) }
        }
    )+};
}

timed_item!(
    LineDataItem,
    HistogramDataItem,
    AreaDataItem,
    BaselineDataItem,
    BarDataItem,
    CandlestickDataItem
);

/// Runtime-erased built-in input for a future data layer. Public APIs should
/// prefer the per-series enums above, which prevent mismatched data.
#[derive(Clone, Debug, PartialEq)]
pub enum BuiltInSeriesDataItem<T, Metadata = ()> {
    Line(LineDataItem<T, Metadata>),
    Histogram(HistogramDataItem<T, Metadata>),
    Area(AreaDataItem<T, Metadata>),
    Baseline(BaselineDataItem<T, Metadata>),
    Bar(BarDataItem<T, Metadata>),
    Candlestick(CandlestickDataItem<T, Metadata>),
}

impl<T, Metadata> BuiltInSeriesDataItem<T, Metadata> {
    pub const fn is_whitespace(&self) -> bool {
        match self {
            Self::Line(data) => data.is_whitespace(),
            Self::Histogram(data) => data.is_whitespace(),
            Self::Area(data) => data.is_whitespace(),
            Self::Baseline(data) => data.is_whitespace(),
            Self::Bar(data) => data.is_whitespace(),
            Self::Candlestick(data) => data.is_whitespace(),
        }
    }
    pub const fn is_fulfilled(&self) -> bool {
        !self.is_whitespace()
    }
}

/// Source-corresponding helpers for callers that intentionally work with a
/// runtime-erased built-in item rather than a per-series input enum.
pub const fn is_whitespace_data<T, Metadata>(data: &BuiltInSeriesDataItem<T, Metadata>) -> bool {
    data.is_whitespace()
}

pub const fn is_fulfilled_data<T, Metadata>(data: &BuiltInSeriesDataItem<T, Metadata>) -> bool {
    data.is_fulfilled()
}

pub const fn is_fulfilled_bar_data<T, Metadata>(data: &BarDataItem<T, Metadata>) -> bool {
    data.is_fulfilled()
}

pub const fn is_fulfilled_line_data<T, Metadata>(data: &LineDataItem<T, Metadata>) -> bool {
    data.is_fulfilled()
}

impl<T, Metadata> TimedData for BuiltInSeriesDataItem<T, Metadata> {
    type Item = T;
    fn time(&self) -> &T {
        match self {
            Self::Line(data) => data.time(),
            Self::Histogram(data) => data.time(),
            Self::Area(data) => data.time(),
            Self::Baseline(data) => data.time(),
            Self::Bar(data) => data.time(),
            Self::Candlestick(data) => data.time(),
        }
    }
    fn time_mut(&mut self) -> &mut T {
        match self {
            Self::Line(data) => data.time_mut(),
            Self::Histogram(data) => data.time_mut(),
            Self::Area(data) => data.time_mut(),
            Self::Baseline(data) => data.time_mut(),
            Self::Bar(data) => data.time_mut(),
            Self::Candlestick(data) => data.time_mut(),
        }
    }
}

/// Synchronous owner boundary used by public series APIs.
///
/// `Series` itself is deliberately not named here; DataLayer will supply a
/// stable Rust series identifier rather than rely on object identity.
pub trait DataUpdatesConsumer<SeriesId, Item, Row> {
    fn apply_new_data(&mut self, series: SeriesId, data: Vec<Item>);
    fn update_data(&mut self, series: SeriesId, data: Item, historical_update: bool);
    fn pop_data(&mut self, series: SeriesId, count: usize) -> Vec<Row>;
}

#[cfg(test)]
mod tests {
    use super::{
        BarData, BarDataItem, BuiltInSeriesDataItem, LineData, LineDataItem, TimedData,
        WhitespaceData, is_fulfilled_data, is_whitespace_data,
    };

    #[test]
    fn exposes_time_for_behavior_preprocessing() {
        let mut data = WhitespaceData {
            time: "2021-02-03",
            custom_values: None::<()>,
        };

        *data.time_mut() = "2021-02-04";
        assert_eq!(data.time(), &"2021-02-04");
    }

    #[test]
    fn classifies_whitespace_without_shape_checks() {
        let whitespace = LineDataItem::Whitespace(WhitespaceData {
            time: "2021-02-03",
            custom_values: Some("note"),
        });
        let value = LineDataItem::Data(LineData {
            time: "2021-02-04",
            value: 42.0,
            color: Some("#f00".into()),
            custom_values: None::<()>,
        });
        assert!(whitespace.is_whitespace());
        assert!(!value.is_whitespace());
        assert_eq!(value.time(), &"2021-02-04");
    }

    #[test]
    fn keeps_ohlc_values_and_metadata_typed() {
        let item = BuiltInSeriesDataItem::Bar(BarDataItem::Data(BarData {
            time: 10_u64,
            open: 1.0,
            high: 4.0,
            low: 0.5,
            close: 3.0,
            color: None,
            custom_values: Some(vec!["tag"]),
        }));
        assert!(is_fulfilled_data(&item));
        assert!(!is_whitespace_data(&item));
        assert_eq!(item.time(), &10);
    }
}
