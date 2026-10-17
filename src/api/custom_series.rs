//! Typed public custom owners and the heterogeneous, synchronous frame adapter.
mod conflation;
use super::{Chart, ChartBehavior, ChartError};
use crate::model::{
    data_consumer::TimedData,
    data_layer::SeriesId,
    get_series_plot_row_creator::CustomPlotRowBuilder,
    icustom_series::*,
    iprice_data_source::FirstValue,
    pane::PriceScalePosition,
    range_impl::RangeImpl,
    series::{CustomSeries, CustomSeriesHandle as ModelHandle},
    series_options::{CustomSeriesOptions, SeriesType},
    time_data::{TimePointIndex, ValueRange, visible_range_by},
};
use crate::ui::{
    custom_series::{CustomPaneView, CustomPreparation, PreparedCustomSeries},
    multi_pane_frame::SeriesPaneView,
};
use iced::widget::canvas::Frame;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::atomic::Ordering,
};

struct OwnedView<V: CustomSeriesPaneView<Frame>> {
    view: V,
    options: V::Options,
}
impl<V: CustomSeriesPaneView<Frame>> Drop for OwnedView<V> {
    fn drop(&mut self) {
        self.view.destroy();
    }
}

/// A typed identity and data owner. Chart operations reject detached/foreign
/// handles. Reading retained data does not keep the pane view attached.
///
/// Calendar handles cannot be passed to price-based chart mutators:
/// ```compile_fail
/// use feather_charts::{api::*, model::{icustom_series::CustomSeriesPaneView,
///     horz_scale_behavior_time::types::{Time, TimePoint}}};
/// use iced::widget::canvas::Frame;
/// fn wrong<V>(chart: &mut OptionsChart, handle: &CustomSeriesHandle<V>, options: V::Options)
/// where V: CustomSeriesPaneView<Frame, Item=Time, InternalTime=TimePoint, Metadata=()> {
///     chart.replace_custom_view_options(handle, options).unwrap();
/// }
/// ```
/// Price events cannot expose maturity payloads:
/// ```compile_fail
/// use feather_charts::{api::*, model::icustom_series::CustomSeriesPaneView};
/// use iced::widget::canvas::Frame;
/// fn wrong<V>(event: &PointerEvent<HorizontalPrice>, handle: &CustomSeriesHandle<V>)
/// where V: CustomSeriesPaneView<Frame, Item=Maturity, InternalTime=Maturity, Metadata=()>, V::Data: 'static {
///     event.custom_data(handle);
/// }
/// ```
pub struct CustomSeriesHandle<V>
where
    V: CustomSeriesPaneView<Frame, Metadata = ()>,
{
    id: SeriesId,
    model: ModelHandle<V::InternalTime, V::Item, V::Data>,
    view: Weak<RefCell<OwnedView<V>>>,
}
impl<V> Clone for CustomSeriesHandle<V>
where
    V: CustomSeriesPaneView<Frame, Metadata = ()>,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            model: self.model.clone(),
            view: self.view.clone(),
        }
    }
}
impl<V> std::fmt::Debug for CustomSeriesHandle<V>
where
    V: CustomSeriesPaneView<Frame, Metadata = ()>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomSeriesHandle")
            .field("id", &self.id)
            .finish()
    }
}
impl<V> CustomSeriesHandle<V>
where
    V: CustomSeriesPaneView<Frame, Metadata = ()>,
{
    pub const fn id(&self) -> SeriesId {
        self.id
    }
    pub fn data_at(&self, index: TimePointIndex) -> Option<V::Data> {
        let model = self.model.borrow();
        let at = model.rows().partition_point(|row| row.base.index < index);
        let row = model.rows().get(at).filter(|row| row.base.index == index)?;
        let mut data = row.data.clone();
        *data.time_mut() = row.base.original_time.clone();
        Some(data)
    }
    fn attached_view<B: ChartBehavior>(
        &self,
        chart: &Chart<B>,
    ) -> Result<Rc<RefCell<OwnedView<V>>>, ChartError>
    where
        V: CustomSeriesPaneView<
                Frame,
                Item = B::Item,
                InternalTime = B::InternalItem,
                Metadata = (),
            >,
    {
        if chart.kinds.get(&self.id) != Some(&SeriesType::Custom)
            || chart.model.pane_for_series(self.id).is_none()
        {
            return Err(ChartError::UnknownSeries(self.id));
        }
        self.view
            .upgrade()
            .ok_or(ChartError::UnknownSeries(self.id))
    }
}
struct Builder<V: CustomSeriesPaneView<Frame>>(Rc<RefCell<OwnedView<V>>>);
impl<V: CustomSeriesPaneView<Frame>> CustomPlotRowBuilder<V::Data, V::Item, V::Metadata>
    for Builder<V>
{
    fn values(&self, data: &V::Data) -> Vec<f64> {
        self.0
            .borrow()
            .view
            .price_value_builder(data)
            .into_iter()
            .map(|p| p.value())
            .collect()
    }
    fn is_whitespace(&self, data: &CustomSeriesDataItem<V::Data, V::Item, V::Metadata>) -> bool {
        self.0.borrow().view.is_whitespace(data)
    }
}

struct Adapter<V>
where
    V: CustomSeriesPaneView<Frame, Metadata = ()>,
{
    conflation: conflation::Cache<V>,
    model: ModelHandle<V::InternalTime, V::Item, V::Data>,
    view: Rc<RefCell<OwnedView<V>>>,
}
impl<V> crate::ui::custom_series::CustomPreparationAdapter for Adapter<V>
where
    V: CustomSeriesPaneView<Frame, Metadata = ()> + 'static,
    V::Data: 'static,
    V::Options: 'static,
    V::Renderer: Clone + 'static,
    <V::Renderer as CustomSeriesPaneRenderer<Frame>>::HitTestData: Clone + 'static,
{
    fn needs_precompute(&self, levels: &[usize]) -> bool {
        let model = self.model.borrow();
        !model.rows().is_empty()
            && levels.iter().any(|&level| {
                level > 1
                    && !self
                        .conflation
                        .contains(model.pane_data_generation(), level)
            })
    }
    fn precompute(&mut self, levels: &[usize], budget: usize) -> usize {
        let model = self.model.borrow();
        let source = (model.pane_data_generation(), model.rows().len());
        let level = levels
            .iter()
            .copied()
            .find(|&level| level > 1 && !self.conflation.contains(source.0, level));
        drop(model);
        level.map_or(0, |level| {
            self.conflation.step(
                source,
                level,
                budget,
                |i| self.model.borrow().rows()[i].clone(),
                &self.view.borrow().view,
            )
        })
    }
    fn hit_tolerance(&self) -> f64 {
        self.model.borrow().options().common.hit_test_tolerance
    }
    fn data_at(&self, index: TimePointIndex) -> Option<CustomSeriesEventData> {
        let model = self.model.borrow();
        let at = model.rows().partition_point(|row| row.base.index < index);
        let row = model.rows().get(at).filter(|row| row.base.index == index)?;
        let mut data = row.data.clone();
        *data.time_mut() = row.base.original_time.clone();
        Some(CustomSeriesEventData {
            series: model.id(),
            value: Rc::new(data),
        })
    }
    fn first_value(&self, range: Option<&RangeImpl<TimePointIndex>>) -> Option<FirstValue> {
        self.model.borrow().first_value(range)
    }
    fn prepare(&mut self, context: CustomPreparation<'_>) -> Option<PreparedCustomSeries> {
        let mut model = self.model.borrow();
        if !model.visible() {
            return None;
        }
        let level = if context.conflation_enabled {
            crate::model::data_conflater::factor(
                context.bar_spacing,
                context.pixel_ratio,
                model
                    .options()
                    .common
                    .conflation_threshold_factor
                    .or(context.conflation_threshold)
                    .unwrap_or(1.),
            )
        } else {
            1
        };
        if level > 1
            && !self
                .conflation
                .contains(model.pane_data_generation(), level)
        {
            let generation = model.pane_data_generation();
            let length = model.rows().len();
            drop(model);
            self.conflation.ensure(
                (generation, length),
                level,
                |i| self.model.borrow().rows()[i].clone(),
                &self.view.borrow().view,
            );
            model = self.model.borrow();
        }
        let rows = if level > 1 {
            self.conflation.rows(level).unwrap_or_else(|| model.rows())
        } else {
            model.rows()
        };
        let range = context
            .visible
            .as_ref()
            .map(|range| visible_range_by(rows, range, true, |row| row.base.index));
        let storage = range
            .map(|r| ValueRange {
                from: r.from.saturating_sub(1),
                to: (r.to + 1).min(rows.len()),
            })
            .unwrap_or(ValueRange { from: 0, to: 0 });
        let bars = rows[storage.from..storage.to]
            .iter()
            .map(|row| {
                let mut original_data = row.data.clone();
                *original_data.time_mut() = row.base.original_time.clone();
                CustomBarItemData {
                    x: (context.coordinate)(row.base.index),
                    time: row.base.index,
                    original_data,
                    bar_color: row
                        .color
                        .clone()
                        .unwrap_or_else(|| model.options().style.color.clone()),
                }
            })
            .collect();
        let data = PaneRendererCustomData {
            bars,
            bar_spacing: context.bar_spacing,
            visible_range: range.map(|r| ValueRange {
                from: r.from - storage.from,
                to: r.to - storage.from,
            }),
            conflation_factor: context.conflation_factor as f64,
            pixel_ratio: context.pixel_ratio,
        };
        let fallback = context
            .price
            .map(|price| {
                range
                    .map(|range| {
                        rows[range.from..range.to]
                            .iter()
                            .map(|row| crate::renderers::range_hit_test::RangePoint {
                                index: row.base.index.value(),
                                x: (context.coordinate)(row.base.index).value(),
                                top: price.price_to_coordinate(row.base.value[1]).value(),
                                bottom: price.price_to_coordinate(row.base.value[2]).value(),
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        drop(model);
        let mut owned = self.view.borrow_mut();
        let OwnedView { view, options } = &mut *owned;
        view.update(data, options);
        context.price.map(|price| {
            PreparedCustomSeries::new(
                view.renderer().clone(),
                price,
                fallback,
                context.bar_spacing,
            )
        })
    }
}
impl<B: ChartBehavior> Chart<B> {
    /// Renderer clones must own frozen visible geometry and must not share a
    /// mutable live view. They are retained independently by prepared frames.
    pub fn add_custom_series<V>(
        &mut self,
        view: V,
        options: CustomSeriesOptions,
        pane: usize,
        position: PriceScalePosition,
    ) -> Result<CustomSeriesHandle<V>, ChartError>
    where
        V: CustomSeriesPaneView<
                Frame,
                Item = B::Item,
                InternalTime = B::InternalItem,
                Metadata = (),
            > + 'static,
        V::Data: 'static,
        V::Options: 'static,
        V::Renderer: Clone + 'static,
        <V::Renderer as CustomSeriesPaneRenderer<Frame>>::HitTestData: Clone + 'static,
    {
        validate_options(&options)?;
        let id = super::chart::NEXT_SERIES
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map(SeriesId::new)
            .map_err(|_| ChartError::IdExhausted)?;
        let owned = Rc::new(RefCell::new(OwnedView {
            options: view.default_options(),
            view,
        }));
        let model = Rc::new(RefCell::new(CustomSeries::new(id, options)));
        self.model
            .register_custom_series(model.clone(), pane, position)?;
        let update = self
            .layer
            .set_custom_series_data(id, vec![], &Builder(owned.clone()))
            .map_err(B::data_error)?;
        self.model.apply_custom_data_update(&model, update)?;
        self.kinds.insert(id, SeriesType::Custom);
        self.views[pane].push((
            id,
            SeriesPaneView::Custom(CustomPaneView::new(Adapter {
                conflation: Default::default(),
                model: model.clone(),
                view: owned.clone(),
            })),
        ));
        self.revision += 1;
        Ok(CustomSeriesHandle {
            id,
            model,
            view: Rc::downgrade(&owned),
        })
    }
    pub fn set_custom_data<V>(
        &mut self,
        handle: &CustomSeriesHandle<V>,
        data: Vec<CustomSeriesDataItem<V::Data, B::Item>>,
    ) -> Result<(), ChartError>
    where
        V: CustomSeriesPaneView<
                Frame,
                Item = B::Item,
                InternalTime = B::InternalItem,
                Metadata = (),
            > + 'static,
        V::Data: 'static,
    {
        let view = handle.attached_view(self)?;
        if !data.is_empty() {
            B::validation_converter(self.layer.behavior(), &data).map_err(B::conversion_error)?;
        }
        let empty = data.is_empty();
        let update = self
            .layer
            .set_custom_series_data(handle.id, data, &Builder(view))
            .map_err(B::data_error)?;
        if empty {
            self.cancel_interaction();
        }
        self.model.apply_custom_data_update(&handle.model, update)?;
        self.revision += 1;
        Ok(())
    }
    pub fn update_custom_data<V>(
        &mut self,
        handle: &CustomSeriesHandle<V>,
        data: CustomSeriesDataItem<V::Data, B::Item>,
        historical: bool,
    ) -> Result<(), ChartError>
    where
        V: CustomSeriesPaneView<
                Frame,
                Item = B::Item,
                InternalTime = B::InternalItem,
                Metadata = (),
            > + 'static,
        V::Data: 'static,
    {
        let view = handle.attached_view(self)?;
        let threshold = handle
            .model
            .borrow()
            .options()
            .common
            .conflation_threshold_factor;
        self.check_historical_conflation(handle.id, historical, threshold)?;
        B::validation_converter(self.layer.behavior(), std::slice::from_ref(&data))
            .map_err(B::conversion_error)?;
        let update = self
            .layer
            .update_custom_series_data(handle.id, data, &Builder(view), historical)
            .map_err(B::data_error)?;
        self.model.apply_custom_data_update(&handle.model, update)?;
        Ok(())
    }
    pub fn replace_custom_view_options<V>(
        &mut self,
        handle: &CustomSeriesHandle<V>,
        options: V::Options,
    ) -> Result<(), ChartError>
    where
        V: CustomSeriesPaneView<
                Frame,
                Item = B::Item,
                InternalTime = B::InternalItem,
                Metadata = (),
            >,
    {
        handle.attached_view(self)?.borrow_mut().options = options;
        handle.model.borrow_mut().invalidate_pane_data();
        self.model.light_update();
        self.revision += 1;
        Ok(())
    }
    pub fn replace_custom_series_options<V>(
        &mut self,
        handle: &CustomSeriesHandle<V>,
        options: CustomSeriesOptions,
    ) -> Result<(), ChartError>
    where
        V: CustomSeriesPaneView<
                Frame,
                Item = B::Item,
                InternalTime = B::InternalItem,
                Metadata = (),
            >,
    {
        handle.attached_view(self)?;
        validate_options(&options)?;
        handle.model.borrow_mut().replace_options(options);
        // Reapply the typed owner handoff so pane autoscale and crosshair settle.
        self.model.custom_options_changed(handle.id);
        self.revision += 1;
        Ok(())
    }
}
fn validate_options(options: &CustomSeriesOptions) -> Result<(), ChartError> {
    let mut parser = crate::model::colors::ColorParser::default();
    let common = &options.common;
    let valid_format = match &common.price_format {
        crate::model::series_options::PriceFormat::BuiltIn(f) => {
            f.min_move.is_finite()
                && f.min_move > 0.
                && f.base.is_none_or(|v| v.is_finite() && v > 0.)
        }
        crate::model::series_options::PriceFormat::Custom(f) => {
            f.min_move.is_finite()
                && f.min_move > 0.
                && f.base.is_none_or(|v| v.is_finite() && v > 0.)
        }
    };
    if !valid_format
        || !common.hit_test_tolerance.is_finite()
        || common.hit_test_tolerance < 0.
        || parser.parse_color(&options.style.color).is_err()
        || parser.parse_color(&common.base_line_color).is_err()
        || (!common.price_line_color.is_empty()
            && parser.parse_color(&common.price_line_color).is_err())
    {
        return Err(ChartError::InvalidData { index: 0 });
    }
    Ok(())
}

/// An owned custom datum copied at the pointer event's exact logical index.
#[derive(Clone)]
pub struct CustomSeriesEventData {
    pub series: SeriesId,
    pub(crate) value: Rc<dyn std::any::Any>,
}
impl std::fmt::Debug for CustomSeriesEventData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomSeriesEventData")
            .field("series", &self.series)
            .finish_non_exhaustive()
    }
}
/// Owned renderer hit payload, kept separate from public hit identity/equality.
#[derive(Clone)]
pub struct CustomHitData(pub(crate) Rc<dyn std::any::Any>);
impl std::fmt::Debug for CustomHitData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomHitData").finish_non_exhaustive()
    }
}
impl<T> super::PointerEvent<T> {
    pub fn custom_data<V>(&self, handle: &CustomSeriesHandle<V>) -> Option<&V::Data>
    where
        V: CustomSeriesPaneView<Frame, Item = T, Metadata = ()>,
        V::Data: 'static,
    {
        self.custom_series_data
            .iter()
            .find(|data| data.series == handle.id)?
            .value
            .downcast_ref()
    }
    pub fn custom_hit_data<V>(
        &self,
        handle: &CustomSeriesHandle<V>,
    ) -> Option<&<V::Renderer as CustomSeriesPaneRenderer<Frame>>::HitTestData>
    where
        V: CustomSeriesPaneView<Frame, Item = T, Metadata = ()>,
        <V::Renderer as CustomSeriesPaneRenderer<Frame>>::HitTestData: 'static,
    {
        if self.hit.as_ref()?.series != handle.id {
            return None;
        }
        self.custom_hit.as_ref()?.0.downcast_ref()
    }
}
