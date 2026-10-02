//! Non-rendering series state: plot rows, formatting, and invalidation.

use crate::{
    formatters::{
        iprice_formatter::PriceValueFormatter, percentage_formatter::PercentageFormatter,
        price_formatter::PriceFormatter, volume_formatter::VolumeFormatter,
    },
    model::{
        autoscale_info_impl::AutoscaleInfoImpl,
        bar::BarPrice,
        data_layer::{CustomDataUpdateResponse, CustomSeriesChanges, SeriesId, SeriesUpdateInfo},
        iprice_data_source::{FirstValue, PriceScaleDataSource},
        plot_data::{PlotRowLike, PlotRowValueIndex},
        plot_list::{MismatchDirection, PlotList},
        range_impl::RangeImpl,
        series_data::{CustomPlotRow, SeriesPlotRow},
        series_options::{
            PriceFormat, PriceFormatBuiltIn, PriceFormatBuiltInType, SeriesOptionsCommon,
            SeriesOptionsMap, SeriesType,
        },
        time_data::TimePointIndex,
    },
};
use std::rc::Rc;

#[derive(Debug, Eq, PartialEq)]
pub enum SeriesConstructionError {
    OptionsDoNotMatchSeriesType { series_type: SeriesType },
}

enum SeriesValueFormatter {
    Price(PriceFormatter),
    Percentage(PercentageFormatter),
    Volume(VolumeFormatter),
    Custom(Rc<dyn Fn(BarPrice) -> String>),
}

impl PriceValueFormatter for SeriesValueFormatter {
    fn format(&self, value: f64) -> String {
        match self {
            Self::Price(formatter) => formatter.format(value),
            Self::Percentage(formatter) => formatter.format(value),
            Self::Volume(formatter) => formatter.format(value),
            Self::Custom(formatter) => formatter(BarPrice::new(value)),
        }
    }
}

fn formatter_for(format: &PriceFormat) -> SeriesValueFormatter {
    match format {
        PriceFormat::Custom(format) => SeriesValueFormatter::Custom(format.formatter.clone()),
        PriceFormat::BuiltIn(PriceFormatBuiltIn {
            kind: PriceFormatBuiltInType::Price,
            min_move,
            base,
            ..
        }) => SeriesValueFormatter::Price(
            PriceFormatter::new(*base, Some(*min_move))
                .unwrap_or_else(|_| PriceFormatter::default()),
        ),
        PriceFormat::BuiltIn(PriceFormatBuiltIn {
            kind: PriceFormatBuiltInType::Percent,
            base,
            ..
        }) => SeriesValueFormatter::Percentage(
            PercentageFormatter::new(*base).unwrap_or_else(|_| PercentageFormatter::default()),
        ),
        PriceFormat::BuiltIn(PriceFormatBuiltIn {
            kind: PriceFormatBuiltInType::Volume,
            precision,
            ..
        }) => SeriesValueFormatter::Volume(VolumeFormatter::new(*precision)),
    }
}

fn matches_type(series_type: SeriesType, options: &SeriesOptionsMap) -> bool {
    matches!(
        (series_type, options),
        (SeriesType::Bar, SeriesOptionsMap::Bar(_))
            | (SeriesType::Candlestick, SeriesOptionsMap::Candlestick(_))
            | (SeriesType::Area, SeriesOptionsMap::Area(_))
            | (SeriesType::Baseline, SeriesOptionsMap::Baseline(_))
            | (SeriesType::Line, SeriesOptionsMap::Line(_))
            | (SeriesType::Histogram, SeriesOptionsMap::Histogram(_))
            | (SeriesType::Custom, SeriesOptionsMap::Custom(_))
    )
}

fn common(options: &SeriesOptionsMap) -> &SeriesOptionsCommon {
    match options {
        SeriesOptionsMap::Bar(options) => &options.common,
        SeriesOptionsMap::Candlestick(options) => &options.common,
        SeriesOptionsMap::Area(options) => &options.common,
        SeriesOptionsMap::Baseline(options) => &options.common,
        SeriesOptionsMap::Line(options) => &options.common,
        SeriesOptionsMap::Histogram(options) => &options.common,
        SeriesOptionsMap::Custom(options) => &options.common,
    }
}

/// Core state for a built-in Series. It deliberately has no model, pane, or
/// renderer reference; its owner applies invalidation and layout effects.
pub struct Series<I, O, M = ()> {
    id: SeriesId,
    series_type: SeriesType,
    options: SeriesOptionsMap,
    data: PlotList<SeriesPlotRow<I, O, (), M>>,
    formatter: SeriesValueFormatter,
    pane_data_generation: u64,
    z_order: i32,
    last_update_info: Option<SeriesUpdateInfo>,
}

impl<I, O, M> Series<I, O, M>
where
    I: Clone,
    O: Clone,
    M: Clone,
{
    pub fn new(
        id: SeriesId,
        series_type: SeriesType,
        options: SeriesOptionsMap,
    ) -> Result<Self, SeriesConstructionError> {
        if !matches_type(series_type, &options) {
            return Err(SeriesConstructionError::OptionsDoNotMatchSeriesType { series_type });
        }
        let formatter = formatter_for(&common(&options).price_format);
        Ok(Self {
            id,
            series_type,
            options,
            data: PlotList::new(),
            formatter,
            pane_data_generation: 0,
            z_order: 0,
            last_update_info: None,
        })
    }
    pub const fn id(&self) -> SeriesId {
        self.id
    }
    pub const fn series_type(&self) -> SeriesType {
        self.series_type
    }
    pub fn options(&self) -> &SeriesOptionsMap {
        &self.options
    }
    pub fn replace_options(
        &mut self,
        options: SeriesOptionsMap,
    ) -> Result<(), SeriesConstructionError> {
        if !matches_type(self.series_type, &options) {
            return Err(SeriesConstructionError::OptionsDoNotMatchSeriesType {
                series_type: self.series_type,
            });
        }
        self.formatter = formatter_for(&common(&options).price_format);
        self.options = options;
        self.invalidate_pane_data();
        Ok(())
    }
    pub fn bars(&self) -> &PlotList<SeriesPlotRow<I, O, (), M>> {
        &self.data
    }
    pub fn fulfilled_indices(&self) -> &[TimePointIndex] {
        self.data.indices()
    }
    pub fn set_data(
        &mut self,
        rows: Vec<SeriesPlotRow<I, O, (), M>>,
        info: Option<SeriesUpdateInfo>,
    ) {
        self.data.set_data(rows);
        self.last_update_info = info;
        self.invalidate_pane_data();
    }
    pub fn first_bar(
        &self,
        visible: Option<&RangeImpl<TimePointIndex>>,
    ) -> Option<&SeriesPlotRow<I, O, (), M>> {
        let visible = visible?;
        self.data
            .search(visible.left(), MismatchDirection::NearestRight)
    }
    pub fn first_value(&self, visible: Option<&RangeImpl<TimePointIndex>>) -> Option<FirstValue> {
        let row = self.first_bar(visible)?;
        Some(FirstValue {
            value: PlotRowLike::values(row)[PlotRowValueIndex::Close as usize],
            time_point: PlotRowLike::index(row),
        })
    }
    pub fn formatter(&self) -> &dyn PriceValueFormatter {
        &self.formatter
    }
    pub fn format_price(&self, value: f64) -> String {
        self.formatter.format(value)
    }
    pub fn base(&self) -> f64 {
        match &common(&self.options).price_format {
            PriceFormat::BuiltIn(format) => format.base.unwrap_or(1.0 / format.min_move),
            PriceFormat::Custom(format) => format.base.unwrap_or(1.0 / format.min_move),
        }
    }
    pub fn visible(&self) -> bool {
        common(&self.options).visible
    }
    pub const fn z_order(&self) -> i32 {
        self.z_order
    }
    pub fn set_z_order(&mut self, z_order: i32) {
        self.z_order = z_order;
    }
    pub fn invalidate_pane_data(&mut self) {
        self.pane_data_generation = self.pane_data_generation.wrapping_add(1);
    }
    pub const fn pane_data_generation(&self) -> u64 {
        self.pane_data_generation
    }
    pub fn last_update_info(&self) -> Option<SeriesUpdateInfo> {
        self.last_update_info
    }
}

/// A short-lived PriceScale view that makes the visible range explicit instead
/// of coupling Series to an owning chart model.
pub struct SeriesPriceScaleSource<'a, I, O, M> {
    series: &'a Series<I, O, M>,
    visible: Option<&'a RangeImpl<TimePointIndex>>,
}

impl<I, O, M> Series<I, O, M>
where
    I: Clone,
    O: Clone,
    M: Clone,
{
    pub fn price_scale_source<'a>(
        &'a self,
        visible: Option<&'a RangeImpl<TimePointIndex>>,
    ) -> SeriesPriceScaleSource<'a, I, O, M> {
        SeriesPriceScaleSource {
            series: self,
            visible,
        }
    }
}

impl<I, O, M> PriceScaleDataSource for SeriesPriceScaleSource<'_, I, O, M>
where
    I: Clone,
    O: Clone,
    M: Clone,
{
    fn z_order(&self) -> i32 {
        self.series.z_order()
    }
    fn visible(&self) -> bool {
        self.series.visible()
    }
    fn first_value(&self) -> Option<FirstValue> {
        self.series.first_value(self.visible)
    }
    fn formatter(&self) -> &dyn PriceValueFormatter {
        self.series.formatter()
    }
    fn base(&self) -> f64 {
        self.series.base()
    }
    fn autoscale_info(
        &self,
        _start: TimePointIndex,
        _end: TimePointIndex,
    ) -> Option<AutoscaleInfoImpl> {
        None
    }
    fn update_all_views(&mut self) {}
}

/// Typed custom-series data retained by the Series rather than DataLayer.
pub struct CustomSeries<I, O, D, M = ()> {
    id: SeriesId,
    data: Vec<CustomPlotRow<I, O, D, M>>,
    pane_data_generation: u64,
}

impl<I, O, D, M> CustomSeries<I, O, D, M> {
    pub fn new(id: SeriesId) -> Self {
        Self {
            id,
            data: vec![],
            pane_data_generation: 0,
        }
    }
    pub fn rows(&self) -> &[CustomPlotRow<I, O, D, M>] {
        &self.data
    }
    pub fn set_data(&mut self, rows: Vec<CustomPlotRow<I, O, D, M>>) {
        self.data = rows;
        self.invalidate_pane_data();
    }
    pub fn apply_indices(&mut self, changes: &CustomSeriesChanges<I, O>) {
        for (row, index) in self.data.iter_mut().zip(&changes.indices) {
            row.base.index = *index;
        }
        self.invalidate_pane_data();
    }
    pub fn pop_indices(&mut self, indices: &[TimePointIndex]) -> Vec<CustomPlotRow<I, O, D, M>> {
        let mut removed = vec![];
        for index in indices {
            if let Some(position) = self.data.iter().position(|row| row.base.index == *index) {
                removed.push(self.data.remove(position));
            }
        }
        if !removed.is_empty() {
            self.invalidate_pane_data();
        }
        removed
    }
    pub fn apply_update(&mut self, update: &CustomDataUpdateResponse<I, O, D, M>)
    where
        I: Clone,
        O: Clone,
        D: Clone,
        M: Clone,
    {
        if update.is_full_replacement {
            self.data = update.custom_rows.clone();
        } else if let Some(index) = update.changed_index {
            self.data.retain(|row| row.base.index != index);
            self.data.extend(update.custom_rows.iter().cloned());
            self.data
                .sort_by(|left, right| left.base.index.partial_cmp(&right.base.index).unwrap());
        }
        if let Some(changes) = update.custom.get(&self.id) {
            self.apply_indices(changes);
        }
    }
    pub fn fulfilled_indices(&self) -> Vec<TimePointIndex> {
        self.data.iter().map(|row| row.base.index).collect()
    }
    pub fn first_value(&self, visible: Option<&RangeImpl<TimePointIndex>>) -> Option<FirstValue> {
        let visible = visible?;
        let row = self
            .data
            .iter()
            .find(|row| row.base.index >= visible.left())?;
        Some(FirstValue {
            value: row.base.value[PlotRowValueIndex::Close as usize],
            time_point: row.base.index,
        })
    }
    pub fn invalidate_pane_data(&mut self) {
        self.pane_data_generation = self.pane_data_generation.wrapping_add(1);
    }
    pub const fn pane_data_generation(&self) -> u64 {
        self.pane_data_generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::{
            data_layer::{CustomSeriesChanges, SeriesId},
            plot_data::PlotRow,
            series_data::LinePlotRow,
            series_options::{
                LastPriceAnimationMode, LineStyleOptions, PriceAxisLastValueMode, PriceLineSource,
                SeriesOptions, SeriesOptionsCommon,
            },
        },
        renderers::draw_line::{LineStyle, LineType, LineWidth},
    };

    fn line_options() -> SeriesOptionsMap {
        SeriesOptionsMap::Line(SeriesOptions {
            common: SeriesOptionsCommon {
                last_value_visible: true,
                title: String::new(),
                price_scale_id: None,
                series_last_value_mode: Some(PriceAxisLastValueMode::LastValueAccordingToScale),
                visible: true,
                hit_test_tolerance: 3.,
                price_line_visible: true,
                price_line_source: PriceLineSource::LastBar,
                price_line_width: LineWidth::One,
                price_line_color: String::new(),
                price_line_style: LineStyle::Solid,
                price_format: PriceFormat::BuiltIn(PriceFormatBuiltIn {
                    kind: PriceFormatBuiltInType::Price,
                    precision: 2,
                    min_move: 0.01,
                    base: Some(100.),
                }),
                base_line_visible: true,
                base_line_color: String::new(),
                base_line_width: LineWidth::One,
                base_line_style: LineStyle::Solid,
                autoscale_info_provider: None,
                conflation_threshold_factor: None,
            },
            style: LineStyleOptions {
                color: "#000".into(),
                line_style: LineStyle::Solid,
                line_width: LineWidth::One,
                line_type: LineType::Simple,
                line_visible: true,
                point_markers_visible: false,
                point_markers_radius: None,
                crosshair_marker_visible: true,
                crosshair_marker_radius: 4.,
                crosshair_marker_border_color: String::new(),
                crosshair_marker_background_color: String::new(),
                crosshair_marker_border_width: 2.,
                last_price_animation: LastPriceAnimationMode::Disabled,
            },
        })
    }

    fn row(index: f64, close: f64) -> SeriesPlotRow<(), (), (), ()> {
        SeriesPlotRow::Line(LinePlotRow {
            base: PlotRow {
                index: index.into(),
                time: (),
                original_time: (),
                value: [close; 4],
                custom_values: None,
                original_data_count: None,
            },
            color: None,
        })
    }

    #[test]
    fn validates_options_replaces_data_and_queries_the_first_visible_value() {
        assert!(matches!(
            Series::<(), (), ()>::new(SeriesId::new(1), SeriesType::Bar, line_options()),
            Err(SeriesConstructionError::OptionsDoNotMatchSeriesType { .. })
        ));
        let mut series = Series::new(SeriesId::new(1), SeriesType::Line, line_options()).unwrap();
        series.set_data(vec![row(1., 10.), row(3., 30.)], None);
        let visible = RangeImpl::new(2.0.into(), 4.0.into());
        assert_eq!(series.fulfilled_indices(), &[1.0.into(), 3.0.into()]);
        assert_eq!(series.first_value(Some(&visible)).unwrap().value, 30.);
        assert_eq!(series.format_price(1.2), "1.20");
        assert_eq!(series.pane_data_generation(), 1);
        assert!(series.first_value(None).is_none());
    }

    #[test]
    fn invalidation_and_price_scale_source_are_owner_independent() {
        let mut series = Series::new(SeriesId::new(1), SeriesType::Line, line_options()).unwrap();
        series.set_data(vec![row(0., 5.)], None);
        series.invalidate_pane_data();
        let visible = RangeImpl::new(0.0.into(), 1.0.into());
        let source = series.price_scale_source(Some(&visible));
        assert_eq!(source.first_value().unwrap().value, 5.);
        assert_eq!(source.base(), 100.);
        assert_eq!(series.pane_data_generation(), 2);
    }

    #[test]
    fn custom_series_keeps_typed_rows_and_returns_popped_rows() {
        let mut custom = CustomSeries::<(), (), &str, ()>::new(SeriesId::new(7));
        custom.set_data(vec![CustomPlotRow {
            base: PlotRow {
                index: 2.0.into(),
                time: (),
                original_time: (),
                value: [3.; 4],
                custom_values: None,
                original_data_count: None,
            },
            data: "payload",
            color: None,
        }]);
        let changes = CustomSeriesChanges::new(vec![4.0.into()], None);
        custom.apply_indices(&changes);
        let popped = custom.pop_indices(&[4.0.into()]);
        assert_eq!(popped[0].data, "payload");
        assert!(custom.rows().is_empty());
    }
}
