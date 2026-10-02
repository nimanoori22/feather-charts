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
        price_range_impl::PriceRangeImpl,
        range_impl::RangeImpl,
        series_data::{CustomPlotRow, SeriesPlotRow},
        series_options::{
            PriceFormat, PriceFormatBuiltIn, PriceFormatBuiltInType, SeriesOptionsCommon,
            SeriesOptionsMap, SeriesType,
        },
        time_data::TimePointIndex,
    },
};
use std::{cell::RefCell, rc::Rc};

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

/// The built-in source uses close values for single-value series and the
/// low/high envelope for OHLC series. Custom-series autoscaling is a separate
/// typed path because its values come from the custom row builder.
fn autoscale_plots(series_type: SeriesType) -> Option<&'static [PlotRowValueIndex]> {
    match series_type {
        SeriesType::Line | SeriesType::Area | SeriesType::Baseline | SeriesType::Histogram => {
            Some(&[PlotRowValueIndex::Close])
        }
        SeriesType::Bar | SeriesType::Candlestick => {
            Some(&[PlotRowValueIndex::Low, PlotRowValueIndex::High])
        }
        SeriesType::Custom => None,
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

/// Object-safe chart-owner boundary. DataLayer intentionally does not know
/// this trait: it only produces stable `SeriesId`-keyed updates.
pub trait SeriesUpdateTarget<I, O, M = ()> {
    fn invalidate_pane_data(&mut self);
    fn apply_built_in_rows(
        &mut self,
        _rows: Vec<SeriesPlotRow<I, O, (), M>>,
        _info: Option<SeriesUpdateInfo>,
    ) {
    }
    fn apply_custom_indices(&mut self, _changes: &CustomSeriesChanges<I, O>) {}
    fn fulfilled_indices(&self) -> Vec<TimePointIndex>;
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
    /// Returns the built-in price range for an integer logical interval.
    ///
    /// A nonempty series with no rows in an otherwise valid interval returns
    /// `Some` with no price range. PriceScale relies on that distinction from
    /// an invalid request or an empty series, both of which return `None`.
    pub fn autoscale_info(
        &self,
        start: TimePointIndex,
        end: TimePointIndex,
    ) -> Option<AutoscaleInfoImpl> {
        if !start.is_integer() || !end.is_integer() || self.data.is_empty() {
            return None;
        }

        let plots = autoscale_plots(self.series_type)?;
        let mut range = self
            .data
            .min_max_on_range_cached(start, end, plots)
            .map(|values| PriceRangeImpl::new(values.min, values.max));

        if let SeriesOptionsMap::Histogram(options) = &self.options {
            let baseline = PriceRangeImpl::new(options.style.base, options.style.base);
            range = Some(match range {
                Some(values) => values.merge(Some(&baseline)),
                None => baseline,
            });
        }

        Some(AutoscaleInfoImpl::new(range, None))
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

impl<I, O, M> SeriesUpdateTarget<I, O, M> for Series<I, O, M>
where
    I: Clone,
    O: Clone,
    M: Clone,
{
    fn invalidate_pane_data(&mut self) {
        self.invalidate_pane_data();
    }
    fn apply_built_in_rows(
        &mut self,
        rows: Vec<SeriesPlotRow<I, O, (), M>>,
        info: Option<SeriesUpdateInfo>,
    ) {
        self.set_data(rows, info);
    }
    fn fulfilled_indices(&self) -> Vec<TimePointIndex> {
        self.fulfilled_indices().to_vec()
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
        start: TimePointIndex,
        end: TimePointIndex,
    ) -> Option<AutoscaleInfoImpl> {
        self.series.autoscale_info(start, end)
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
    pub const fn id(&self) -> SeriesId {
        self.id
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

impl<I, O, D, M> SeriesUpdateTarget<I, O, M> for CustomSeries<I, O, D, M> {
    fn invalidate_pane_data(&mut self) {
        self.invalidate_pane_data();
    }
    fn apply_custom_indices(&mut self, changes: &CustomSeriesChanges<I, O>) {
        self.apply_indices(changes);
    }
    fn fulfilled_indices(&self) -> Vec<TimePointIndex> {
        self.fulfilled_indices()
    }
}

/// Shared only at the chart-owner boundary: the typed custom API keeps this
/// handle to apply `CustomDataUpdateResponse<D>`, while the coordinator holds
/// a lightweight target adapter for invalidation and index synchronization.
pub type CustomSeriesHandle<I, O, D, M = ()> = Rc<RefCell<CustomSeries<I, O, D, M>>>;

pub struct CustomSeriesTarget<I, O, D, M = ()> {
    handle: CustomSeriesHandle<I, O, D, M>,
}

impl<I, O, D, M> CustomSeriesTarget<I, O, D, M> {
    pub fn new(handle: CustomSeriesHandle<I, O, D, M>) -> Self {
        Self { handle }
    }
}

impl<I, O, D, M> SeriesUpdateTarget<I, O, M> for CustomSeriesTarget<I, O, D, M> {
    fn invalidate_pane_data(&mut self) {
        self.handle.borrow_mut().invalidate_pane_data();
    }
    fn apply_custom_indices(&mut self, changes: &CustomSeriesChanges<I, O>) {
        self.handle.borrow_mut().apply_indices(changes);
    }
    fn fulfilled_indices(&self) -> Vec<TimePointIndex> {
        self.handle.borrow().fulfilled_indices()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::{
            data_layer::{CustomSeriesChanges, SeriesId},
            plot_data::PlotRow,
            series_data::{BarPlotRow, LinePlotRow},
            series_options::{
                BarStyleOptions, HistogramStyleOptions, LastPriceAnimationMode, LineStyleOptions,
                PriceAxisLastValueMode, PriceLineSource, SeriesOptions, SeriesOptionsCommon,
            },
        },
        renderers::draw_line::{LineStyle, LineType, LineWidth},
    };

    fn common_options() -> SeriesOptionsCommon {
        SeriesOptionsCommon {
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
        }
    }

    fn line_options() -> SeriesOptionsMap {
        SeriesOptionsMap::Line(SeriesOptions {
            common: common_options(),
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

    fn bar_options() -> SeriesOptionsMap {
        SeriesOptionsMap::Bar(SeriesOptions {
            common: common_options(),
            style: BarStyleOptions {
                up_color: String::new(),
                down_color: String::new(),
                open_visible: true,
                thin_bars: true,
            },
        })
    }

    fn histogram_options(base: f64) -> SeriesOptionsMap {
        SeriesOptionsMap::Histogram(SeriesOptions {
            common: common_options(),
            style: HistogramStyleOptions {
                color: String::new(),
                base,
            },
        })
    }

    fn row(index: f64, close: f64) -> SeriesPlotRow<(), (), (), ()> {
        row_with_values(index, [close; 4])
    }

    fn row_with_values(index: f64, values: [f64; 4]) -> SeriesPlotRow<(), (), (), ()> {
        SeriesPlotRow::Line(LinePlotRow {
            base: PlotRow {
                index: index.into(),
                time: (),
                original_time: (),
                value: values,
                custom_values: None,
                original_data_count: None,
            },
            color: None,
        })
    }

    fn bar_row(index: f64, values: [f64; 4]) -> SeriesPlotRow<(), (), (), ()> {
        SeriesPlotRow::Bar(BarPlotRow {
            base: PlotRow {
                index: index.into(),
                time: (),
                original_time: (),
                value: values,
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
    fn autoscale_uses_close_for_line_and_distinguishes_empty_intervals() {
        let mut series = Series::new(SeriesId::new(1), SeriesType::Line, line_options()).unwrap();
        series.set_data(vec![row_with_values(2., [2., 100., -50., 10.])], None);

        assert_eq!(
            series
                .autoscale_info(2.0.into(), 2.0.into())
                .unwrap()
                .price_range(),
            Some(PriceRangeImpl::new(10., 10.))
        );
        assert!(series.autoscale_info(2.5.into(), 3.0.into()).is_none());
        assert_eq!(
            series
                .autoscale_info(4.0.into(), 5.0.into())
                .unwrap()
                .price_range(),
            None
        );

        let empty =
            Series::<(), (), ()>::new(SeriesId::new(2), SeriesType::Line, line_options()).unwrap();
        assert!(empty.autoscale_info(0.0.into(), 1.0.into()).is_none());
    }

    #[test]
    fn autoscale_uses_low_and_high_for_bar_rows_and_skips_nan() {
        assert_eq!(
            autoscale_plots(SeriesType::Bar),
            autoscale_plots(SeriesType::Candlestick)
        );
        let mut series = Series::new(SeriesId::new(1), SeriesType::Bar, bar_options()).unwrap();
        series.set_data(
            vec![
                bar_row(0., [5., 100., -3., 10.]),
                bar_row(1., [5., f64::NAN, f64::NAN, 20.]),
            ],
            None,
        );

        assert_eq!(
            series
                .autoscale_info(0.0.into(), 1.0.into())
                .unwrap()
                .price_range(),
            Some(PriceRangeImpl::new(-3., 100.))
        );
    }

    #[test]
    fn histogram_autoscale_always_includes_its_baseline() {
        let mut series = Series::new(
            SeriesId::new(1),
            SeriesType::Histogram,
            histogram_options(0.),
        )
        .unwrap();
        series.set_data(vec![row(0., 10.)], None);
        assert_eq!(
            series
                .autoscale_info(0.0.into(), 0.0.into())
                .unwrap()
                .price_range(),
            Some(PriceRangeImpl::new(0., 10.))
        );
        assert_eq!(
            series
                .autoscale_info(5.0.into(), 6.0.into())
                .unwrap()
                .price_range(),
            Some(PriceRangeImpl::new(0., 0.))
        );
    }

    #[test]
    fn price_scale_source_forwards_series_autoscale_info() {
        let mut series = Series::new(SeriesId::new(1), SeriesType::Line, line_options()).unwrap();
        series.set_data(vec![row(0., 5.)], None);
        let source = series.price_scale_source(None);
        assert_eq!(
            source
                .autoscale_info(0.0.into(), 0.0.into())
                .unwrap()
                .price_range(),
            Some(PriceRangeImpl::new(5., 5.))
        );
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
