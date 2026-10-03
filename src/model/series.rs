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
        price_scale_visible_range::PriceScaleVisibleRange,
        range_impl::RangeImpl,
        series_data::{CustomPlotRow, SeriesPlotRow},
        series_options::{
            CustomSeriesOptions, PriceFormat, PriceFormatBuiltIn, PriceFormatBuiltInType,
            SeriesOptionsCommon, SeriesOptionsMap, SeriesType,
        },
        time_data::TimePointIndex,
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

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
    z_order: Cell<i32>,
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
            z_order: Cell::new(0),
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
    /// Computes the built-in price range without consulting a user provider.
    fn built_in_autoscale_info(
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
    /// Returns the price range for an integer logical interval.
    ///
    /// A nonempty series with no rows in an otherwise valid interval returns
    /// `Some` with no price range. A configured provider receives that normal
    /// calculation in raw/public form and may replace or suppress it.
    pub fn autoscale_info(
        &self,
        start: TimePointIndex,
        end: TimePointIndex,
    ) -> Option<AutoscaleInfoImpl> {
        if let Some(provider) = common(&self.options).autoscale_info_provider.as_ref() {
            let raw = provider(&|| {
                self.built_in_autoscale_info(start, end)
                    .map(|info| info.to_raw())
            });
            return AutoscaleInfoImpl::from_raw(raw);
        }

        self.built_in_autoscale_info(start, end)
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
        self.z_order.get()
    }
    pub fn set_z_order(&self, z_order: i32) {
        self.z_order.set(z_order);
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

/// Shared built-in target updating the same Series used by Pane adapters.
pub struct BuiltInSeriesTarget<I, O, M = ()> {
    handle: Rc<RefCell<Series<I, O, M>>>,
}
impl<I, O, M> BuiltInSeriesTarget<I, O, M> {
    pub fn new(handle: Rc<RefCell<Series<I, O, M>>>) -> Self {
        Self { handle }
    }
}
impl<I: Clone, O: Clone, M: Clone> SeriesUpdateTarget<I, O, M> for BuiltInSeriesTarget<I, O, M> {
    fn invalidate_pane_data(&mut self) {
        self.handle.borrow_mut().invalidate_pane_data();
    }
    fn apply_built_in_rows(
        &mut self,
        rows: Vec<SeriesPlotRow<I, O, (), M>>,
        info: Option<SeriesUpdateInfo>,
    ) {
        self.handle.borrow_mut().set_data(rows, info);
    }
    fn fulfilled_indices(&self) -> Vec<TimePointIndex> {
        self.handle.borrow().fulfilled_indices().to_vec()
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
    fn set_z_order(&mut self, z_order: i32) {
        self.series.set_z_order(z_order);
    }
    fn visible(&self) -> bool {
        self.series.visible()
    }
    fn first_value(&self) -> Option<FirstValue> {
        self.series.first_value(self.visible)
    }
    fn format_price(&self, price: f64) -> String {
        self.series.format_price(price)
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
    options: CustomSeriesOptions,
    data: Vec<CustomPlotRow<I, O, D, M>>,
    formatter: SeriesValueFormatter,
    pane_data_generation: u64,
    z_order: i32,
    last_update_info: Option<SeriesUpdateInfo>,
}

impl<I, O, D, M> CustomSeries<I, O, D, M> {
    pub fn new(id: SeriesId, options: CustomSeriesOptions) -> Self {
        let formatter = formatter_for(&options.common.price_format);
        Self {
            id,
            options,
            data: vec![],
            formatter,
            pane_data_generation: 0,
            z_order: 0,
            last_update_info: None,
        }
    }
    pub fn rows(&self) -> &[CustomPlotRow<I, O, D, M>] {
        &self.data
    }
    pub const fn id(&self) -> SeriesId {
        self.id
    }
    pub fn options(&self) -> &CustomSeriesOptions {
        &self.options
    }
    pub fn replace_options(&mut self, options: CustomSeriesOptions) {
        self.formatter = formatter_for(&options.common.price_format);
        self.options = options;
        self.invalidate_pane_data();
    }
    pub fn set_data(
        &mut self,
        rows: Vec<CustomPlotRow<I, O, D, M>>,
        info: Option<SeriesUpdateInfo>,
    ) {
        self.data = rows;
        self.last_update_info = info;
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
    /// Consumes the typed DataLayer handoff so applying an update never
    /// requires cloning the custom payload.
    pub fn apply_update(&mut self, update: CustomDataUpdateResponse<I, O, D, M>) {
        let CustomDataUpdateResponse {
            custom_rows,
            custom,
            is_full_replacement,
            changed_index,
            ..
        } = update;
        if is_full_replacement {
            self.data = custom_rows;
        } else if let Some(index) = changed_index {
            self.data.retain(|row| row.base.index != index);
            self.data.extend(custom_rows);
            self.data
                .sort_by(|left, right| left.base.index.partial_cmp(&right.base.index).unwrap());
        }
        if let Some(changes) = custom.get(&self.id) {
            self.apply_indices(changes);
            self.last_update_info = changes.info;
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
    /// Computes the custom low/high envelope from its DataLayer-created OHLC
    /// projection. The typed custom payload is intentionally untouched.
    fn built_in_autoscale_info(
        &self,
        start: TimePointIndex,
        end: TimePointIndex,
    ) -> Option<AutoscaleInfoImpl> {
        if !start.is_integer() || !end.is_integer() || self.data.is_empty() {
            return None;
        }

        let mut range: Option<PriceRangeImpl> = None;
        for row in &self.data {
            if row.base.index < start || row.base.index > end {
                continue;
            }
            for value in [
                row.base.value[PlotRowValueIndex::Low as usize],
                row.base.value[PlotRowValueIndex::High as usize],
            ] {
                if value.is_nan() {
                    continue;
                }
                range = Some(match range {
                    Some(current) => current.merge(Some(&PriceRangeImpl::new(value, value))),
                    None => PriceRangeImpl::new(value, value),
                });
            }
        }

        Some(AutoscaleInfoImpl::new(range, None))
    }
    pub fn autoscale_info(
        &self,
        start: TimePointIndex,
        end: TimePointIndex,
    ) -> Option<AutoscaleInfoImpl> {
        if let Some(provider) = self.options.common.autoscale_info_provider.as_ref() {
            let raw = provider(&|| {
                self.built_in_autoscale_info(start, end)
                    .map(|info| info.to_raw())
            });
            return AutoscaleInfoImpl::from_raw(raw);
        }
        self.built_in_autoscale_info(start, end)
    }
    pub fn formatter(&self) -> &dyn PriceValueFormatter {
        &self.formatter
    }
    pub fn format_price(&self, value: f64) -> String {
        self.formatter.format(value)
    }
    pub fn base(&self) -> f64 {
        match &self.options.common.price_format {
            PriceFormat::BuiltIn(format) => format.base.unwrap_or(1.0 / format.min_move),
            PriceFormat::Custom(format) => format.base.unwrap_or(1.0 / format.min_move),
        }
    }
    pub const fn visible(&self) -> bool {
        self.options.common.visible
    }
    pub const fn z_order(&self) -> i32 {
        self.z_order
    }
    pub fn set_z_order(&mut self, z_order: i32) {
        self.z_order = z_order;
    }
    pub const fn last_update_info(&self) -> Option<SeriesUpdateInfo> {
        self.last_update_info
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

/// Long-lived custom source adapter. The type-erased PriceScale boundary owns
/// this adapter, while the adapter itself retains the typed `D` handle.
pub struct CustomSeriesPriceScaleSource<I, O, D, M = ()> {
    series: CustomSeriesHandle<I, O, D, M>,
    visible: PriceScaleVisibleRange,
}

impl<I, O, D, M> CustomSeriesPriceScaleSource<I, O, D, M> {
    pub fn new(series: CustomSeriesHandle<I, O, D, M>, visible: PriceScaleVisibleRange) -> Self {
        Self { series, visible }
    }
}

impl<I, O, D, M> PriceScaleDataSource for CustomSeriesPriceScaleSource<I, O, D, M>
where
    I: Clone + 'static,
    O: Clone + 'static,
    D: 'static,
    M: 'static,
{
    fn z_order(&self) -> i32 {
        self.series.borrow().z_order()
    }
    fn set_z_order(&mut self, z_order: i32) {
        self.series.borrow_mut().set_z_order(z_order);
    }
    fn visible(&self) -> bool {
        self.series.borrow().visible()
    }
    fn first_value(&self) -> Option<FirstValue> {
        let visible = self.visible.get();
        self.series.borrow().first_value(visible.as_ref())
    }
    fn format_price(&self, price: f64) -> String {
        self.series.borrow().format_price(price)
    }
    fn base(&self) -> f64 {
        self.series.borrow().base()
    }
    fn autoscale_info(
        &self,
        start: TimePointIndex,
        end: TimePointIndex,
    ) -> Option<AutoscaleInfoImpl> {
        self.series.borrow().autoscale_info(start, end)
    }
    fn update_all_views(&mut self) {}
}

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
            autoscale_info_impl::AutoscaleInfo,
            data_layer::{
                CustomDataUpdateResponse, CustomSeriesChanges, SeriesId, TimeScaleChanges,
            },
            layout_options::{Background, ColorSpace, LayoutOptions, LayoutPanesOptions},
            plot_data::PlotRow,
            price_range_impl::PriceRange,
            price_scale::{PriceScale, PriceScaleOptions, PriceSourceHandle},
            series_data::{BarPlotRow, HistogramPlotRow, LinePlotRow},
            series_options::{
                AutoscaleInfoProvider, BarStyleOptions, CustomStyleOptions, HistogramStyleOptions,
                LastPriceAnimationMode, LineStyleOptions, PriceAxisLastValueMode, PriceLineSource,
                SeriesOptions, SeriesOptionsCommon,
            },
        },
        renderers::draw_line::{LineStyle, LineType, LineWidth},
    };
    use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

    struct OwnedSeriesPriceSource {
        series: Series<(), (), ()>,
        visible: RangeImpl<TimePointIndex>,
    }

    impl PriceScaleDataSource for OwnedSeriesPriceSource {
        fn z_order(&self) -> i32 {
            self.series.z_order()
        }
        fn set_z_order(&mut self, z_order: i32) {
            self.series.set_z_order(z_order);
        }
        fn visible(&self) -> bool {
            self.series.visible()
        }
        fn first_value(&self) -> Option<FirstValue> {
            self.series.first_value(Some(&self.visible))
        }
        fn format_price(&self, price: f64) -> String {
            self.series.format_price(price)
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

    fn line_options_with_provider(provider: AutoscaleInfoProvider) -> SeriesOptionsMap {
        let mut options = line_options();
        let SeriesOptionsMap::Line(line) = &mut options else {
            unreachable!("line_options always constructs line options");
        };
        line.common.autoscale_info_provider = Some(provider);
        options
    }

    fn layout_options() -> LayoutOptions {
        LayoutOptions {
            background: Background::Solid {
                color: String::new(),
            },
            text_color: String::new(),
            font_size: 12.,
            font_family: String::new(),
            panes: LayoutPanesOptions {
                enable_resize: true,
                separator_color: String::new(),
                separator_hover_color: String::new(),
            },
            attribution_logo: false,
            color_space: ColorSpace::Srgb,
            color_parsers: vec![],
        }
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

    fn custom_options() -> CustomSeriesOptions {
        SeriesOptions {
            common: common_options(),
            style: CustomStyleOptions {
                color: String::new(),
            },
        }
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

    fn histogram_row(index: f64, values: [f64; 4]) -> SeriesPlotRow<(), (), (), ()> {
        SeriesPlotRow::Histogram(HistogramPlotRow {
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
        let provider: AutoscaleInfoProvider = Box::new(|_| {
            Some(AutoscaleInfo {
                price_range: Some(PriceRange {
                    min_value: 2.,
                    max_value: 8.,
                }),
                margins: None,
            })
        });
        let mut series = Series::new(
            SeriesId::new(1),
            SeriesType::Line,
            line_options_with_provider(provider),
        )
        .unwrap();
        series.set_data(vec![row(0., 5.)], None);
        let source = series.price_scale_source(None);
        assert_eq!(
            source
                .autoscale_info(0.0.into(), 0.0.into())
                .unwrap()
                .price_range(),
            Some(PriceRangeImpl::new(2., 8.))
        );
    }

    #[test]
    fn price_scale_source_preserves_builtin_range_and_empty_interval_semantics() {
        let mut line = Series::new(SeriesId::new(1), SeriesType::Line, line_options()).unwrap();
        line.set_data(vec![row_with_values(0., [2., 100., -50., 10.])], None);
        let sparse = RangeImpl::new(5.0.into(), 6.0.into());
        let line_source = line.price_scale_source(Some(&sparse));
        assert_eq!(
            line_source
                .autoscale_info(0.0.into(), 0.0.into())
                .unwrap()
                .price_range(),
            Some(PriceRangeImpl::new(10., 10.))
        );
        assert_eq!(
            line_source
                .autoscale_info(5.0.into(), 6.0.into())
                .unwrap()
                .price_range(),
            None
        );

        let mut bar = Series::new(SeriesId::new(2), SeriesType::Bar, bar_options()).unwrap();
        bar.set_data(vec![bar_row(0., [10., 50., -4., 20.])], None);
        assert_eq!(
            bar.price_scale_source(None)
                .autoscale_info(0.0.into(), 0.0.into())
                .unwrap()
                .price_range(),
            Some(PriceRangeImpl::new(-4., 50.))
        );

        let mut histogram = Series::new(
            SeriesId::new(3),
            SeriesType::Histogram,
            histogram_options(7.),
        )
        .unwrap();
        histogram.set_data(vec![histogram_row(0., [0., 100., -50., 10.])], None);
        assert_eq!(
            histogram
                .price_scale_source(None)
                .autoscale_info(0.0.into(), 0.0.into())
                .unwrap()
                .price_range(),
            Some(PriceRangeImpl::new(7., 10.))
        );

        let empty =
            Series::<(), (), ()>::new(SeriesId::new(4), SeriesType::Line, line_options()).unwrap();
        assert!(
            empty
                .price_scale_source(None)
                .autoscale_info(0.0.into(), 1.0.into())
                .is_none()
        );
    }

    #[test]
    fn autoscale_provider_can_adjust_or_suppress_the_default_result() {
        let adjuster: AutoscaleInfoProvider = Box::new(|default| {
            let mut result = default()?;
            assert_eq!(result.price_range, default()?.price_range);
            let range = result.price_range.as_mut()?;
            range.min_value -= 2.;
            range.max_value += 3.;
            Some(result)
        });
        let mut adjusted = Series::new(
            SeriesId::new(1),
            SeriesType::Line,
            line_options_with_provider(adjuster),
        )
        .unwrap();
        adjusted.set_data(vec![row(0., 5.)], None);
        assert_eq!(
            adjusted
                .autoscale_info(0.0.into(), 0.0.into())
                .unwrap()
                .price_range(),
            Some(PriceRangeImpl::new(3., 8.))
        );

        let suppressor: AutoscaleInfoProvider = Box::new(|_| None);
        let mut suppressed = Series::new(
            SeriesId::new(2),
            SeriesType::Line,
            line_options_with_provider(suppressor),
        )
        .unwrap();
        suppressed.set_data(vec![row(0., 5.)], None);
        assert!(suppressed.autoscale_info(0.0.into(), 0.0.into()).is_none());
    }

    #[test]
    fn autoscale_provider_may_supply_a_range_when_the_default_is_none() {
        let provider: AutoscaleInfoProvider = Box::new(|default| {
            assert!(default().is_none());
            Some(AutoscaleInfo {
                price_range: Some(PriceRange {
                    min_value: -1.,
                    max_value: 1.,
                }),
                margins: None,
            })
        });
        let series = Series::<(), (), ()>::new(
            SeriesId::new(1),
            SeriesType::Line,
            line_options_with_provider(provider),
        )
        .unwrap();
        assert_eq!(
            series
                .autoscale_info(0.5.into(), 1.0.into())
                .unwrap()
                .price_range(),
            Some(PriceRangeImpl::new(-1., 1.))
        );
    }

    #[test]
    fn price_scale_consumes_the_provider_range_from_an_owned_series_source() {
        let provider: AutoscaleInfoProvider = Box::new(|_| {
            Some(AutoscaleInfo {
                price_range: Some(PriceRange {
                    min_value: -5.,
                    max_value: 15.,
                }),
                margins: None,
            })
        });
        let mut series = Series::new(
            SeriesId::new(1),
            SeriesType::Line,
            line_options_with_provider(provider),
        )
        .unwrap();
        series.set_data(vec![row(0., 5.)], None);
        let source: PriceSourceHandle = Rc::new(RefCell::new(OwnedSeriesPriceSource {
            series,
            visible: RangeImpl::new(0.0.into(), 0.0.into()),
        }));
        let mut scale = PriceScale::new("right", PriceScaleOptions::default(), &layout_options());
        scale.add_data_source(source);
        scale.recalculate_price_range(&RangeImpl::new(0.0.into(), 0.0.into()));
        assert_eq!(scale.price_range(), Some(PriceRangeImpl::new(-5., 15.)));
    }

    #[test]
    fn custom_series_keeps_typed_rows_and_returns_popped_rows() {
        let mut custom = CustomSeries::<(), (), &str, ()>::new(SeriesId::new(7), custom_options());
        custom.set_data(
            vec![CustomPlotRow {
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
            }],
            None,
        );
        let changes = CustomSeriesChanges::new(vec![4.0.into()], None);
        custom.apply_indices(&changes);
        let popped = custom.pop_indices(&[4.0.into()]);
        assert_eq!(popped[0].data, "payload");
        assert!(custom.rows().is_empty());
    }

    #[test]
    fn custom_options_refresh_formatter_visibility_and_pane_data() {
        let mut custom = CustomSeries::<(), (), (), ()>::new(SeriesId::new(7), custom_options());
        assert!(custom.visible());
        assert_eq!(custom.base(), 100.);
        assert_eq!(custom.format_price(1.2), "1.20");

        let mut replacement = custom_options();
        replacement.common.visible = false;
        replacement.common.price_format = PriceFormat::BuiltIn(PriceFormatBuiltIn {
            kind: PriceFormatBuiltInType::Percent,
            precision: 2,
            min_move: 0.01,
            base: Some(100.),
        });
        custom.replace_options(replacement);
        custom.set_z_order(4);

        assert!(!custom.visible());
        assert_eq!(custom.z_order(), 4);
        assert_eq!(custom.format_price(1.2), "1.20%");
        assert_eq!(custom.pane_data_generation(), 1);
    }

    #[test]
    fn custom_series_uses_sparse_first_value_and_provider_result() {
        let provider: AutoscaleInfoProvider = Box::new(|default| {
            let mut info = default()?;
            let range = info.price_range.as_mut()?;
            range.min_value -= 2.;
            range.max_value += 3.;
            Some(info)
        });
        let mut options = custom_options();
        options.common.autoscale_info_provider = Some(provider);
        let mut custom = CustomSeries::<(), (), (), ()>::new(SeriesId::new(7), options);
        custom.set_data(
            vec![
                CustomPlotRow {
                    base: PlotRow {
                        index: 1.0.into(),
                        time: (),
                        original_time: (),
                        value: [10., 20., 5., 10.],
                        custom_values: None,
                        original_data_count: None,
                    },
                    data: (),
                    color: None,
                },
                CustomPlotRow {
                    base: PlotRow {
                        index: 4.0.into(),
                        time: (),
                        original_time: (),
                        value: [30., 40., 25., 30.],
                        custom_values: None,
                        original_data_count: None,
                    },
                    data: (),
                    color: None,
                },
            ],
            None,
        );
        let visible = RangeImpl::new(2.0.into(), 5.0.into());
        assert_eq!(custom.first_value(Some(&visible)).unwrap().value, 30.);
        assert_eq!(
            custom
                .autoscale_info(1.0.into(), 4.0.into())
                .unwrap()
                .price_range(),
            Some(PriceRangeImpl::new(3., 43.))
        );
    }

    #[test]
    fn custom_update_moves_non_clone_payload_without_copying_it() {
        struct NonClonePayload(&'static str);

        let id = SeriesId::new(7);
        let mut custom = CustomSeries::<(), (), NonClonePayload, ()>::new(id, custom_options());
        custom.apply_update(CustomDataUpdateResponse {
            series: BTreeMap::new(),
            custom_rows: vec![CustomPlotRow {
                base: PlotRow {
                    index: 2.0.into(),
                    time: (),
                    original_time: (),
                    value: [3.; 4],
                    custom_values: None,
                    original_data_count: None,
                },
                data: NonClonePayload("moved"),
                color: None,
            }],
            custom: BTreeMap::from([(id, CustomSeriesChanges::new(vec![2.0.into()], None))]),
            time_scale: TimeScaleChanges {
                points: None,
                first_changed_point_index: None,
                base_index: None,
            },
            is_full_replacement: true,
            changed_index: None,
        });
        assert_eq!(custom.rows()[0].data.0, "moved");
    }

    #[test]
    fn custom_autoscale_uses_projected_low_high_without_touching_payloads() {
        struct NonClonePayload(&'static str);

        let mut custom =
            CustomSeries::<(), (), NonClonePayload, ()>::new(SeriesId::new(7), custom_options());
        custom.set_data(
            vec![
                CustomPlotRow {
                    base: PlotRow {
                        index: 0.0.into(),
                        time: (),
                        original_time: (),
                        value: [10., 20., 5., 10.],
                        custom_values: None,
                        original_data_count: None,
                    },
                    data: NonClonePayload("first"),
                    color: None,
                },
                CustomPlotRow {
                    base: PlotRow {
                        index: 1.0.into(),
                        time: (),
                        original_time: (),
                        value: [15., f64::NAN, -3., 15.],
                        custom_values: None,
                        original_data_count: None,
                    },
                    data: NonClonePayload("second"),
                    color: None,
                },
            ],
            None,
        );

        assert_eq!(custom.rows()[0].data.0, "first");
        assert_eq!(
            custom
                .autoscale_info(0.0.into(), 1.0.into())
                .unwrap()
                .price_range(),
            Some(PriceRangeImpl::new(-3., 20.))
        );
        assert!(custom.autoscale_info(0.5.into(), 1.0.into()).is_none());
        assert_eq!(
            custom
                .autoscale_info(3.0.into(), 4.0.into())
                .unwrap()
                .price_range(),
            None
        );

        let empty =
            CustomSeries::<(), (), NonClonePayload, ()>::new(SeriesId::new(8), custom_options());
        assert!(empty.autoscale_info(0.0.into(), 1.0.into()).is_none());
    }
}
