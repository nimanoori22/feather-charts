//! Series-specific plot-row metadata and storage aliases.

use crate::model::{
    plot_data::{PlotRow, PlotRowLike},
    plot_list::PlotList,
    series_options::SeriesType,
};

#[derive(Clone, Debug, PartialEq)]
pub struct LinePlotRow<I, O> {
    pub base: PlotRow<I, O>,
    pub color: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HistogramPlotRow<I, O> {
    pub base: PlotRow<I, O>,
    pub color: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BarPlotRow<I, O> {
    pub base: PlotRow<I, O>,
    pub color: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandlestickPlotRow<I, O> {
    pub base: PlotRow<I, O>,
    pub color: Option<String>,
    pub border_color: Option<String>,
    pub wick_color: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AreaPlotRow<I, O> {
    pub base: PlotRow<I, O>,
    pub line_color: Option<String>,
    pub top_color: Option<String>,
    pub bottom_color: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BaselinePlotRow<I, O> {
    pub base: PlotRow<I, O>,
    pub top_fill_color1: Option<String>,
    pub top_fill_color2: Option<String>,
    pub top_line_color: Option<String>,
    pub bottom_fill_color1: Option<String>,
    pub bottom_fill_color2: Option<String>,
    pub bottom_line_color: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CustomPlotRow<I, O, D> {
    pub base: PlotRow<I, O>,
    pub data: D,
    pub color: Option<String>,
}
macro_rules! rowlike { ($t:ident<$($g:ident),*>, $field:ident) => { impl<$($g),*> PlotRowLike for $t<$($g),*> { fn index(&self)->crate::model::time_data::TimePointIndex {self.$field.index} fn values(&self)->&crate::model::plot_data::PlotRowValue {&self.$field.value} } }; }
rowlike!(LinePlotRow<I,O>,base);
rowlike!(HistogramPlotRow<I,O>,base);
rowlike!(BarPlotRow<I,O>,base);
rowlike!(CandlestickPlotRow<I,O>,base);
rowlike!(AreaPlotRow<I,O>,base);
rowlike!(BaselinePlotRow<I,O>,base);
rowlike!(CustomPlotRow<I,O,D>,base);
pub type SeriesPlotList<Row> = PlotList<Row>;
pub fn create_series_plot_list<Row: PlotRowLike + Clone>() -> PlotList<Row> {
    PlotList::new()
}
pub enum SeriesPlotRow<I, O, D = ()> {
    Bar(BarPlotRow<I, O>),
    Candlestick(CandlestickPlotRow<I, O>),
    Area(AreaPlotRow<I, O>),
    Baseline(BaselinePlotRow<I, O>),
    Line(LinePlotRow<I, O>),
    Histogram(HistogramPlotRow<I, O>),
    Custom(CustomPlotRow<I, O, D>),
}
impl<I, O, D> SeriesPlotRow<I, O, D> {
    pub fn series_type(&self) -> SeriesType {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_series_specific_colors_and_classifies_the_row() {
        let row = SeriesPlotRow::<(), ()>::Candlestick(CandlestickPlotRow {
            base: PlotRow {
                index: 7.0.into(),
                time: (),
                original_time: (),
                value: [10.0, 13.0, 8.0, 12.0],
                custom_values: None,
                original_data_count: Some(1),
            },
            color: Some("#00ff00".into()),
            border_color: Some("#008800".into()),
            wick_color: Some("#004400".into()),
        });

        assert_eq!(row.series_type(), SeriesType::Candlestick);
        let SeriesPlotRow::Candlestick(candlestick) = row else {
            panic!("expected candlestick row");
        };
        assert_eq!(candlestick.base.value[3], 12.0);
        assert_eq!(candlestick.wick_color.as_deref(), Some("#004400"));
    }
}
