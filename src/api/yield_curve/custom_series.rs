use super::*;
use crate::model::{
    icustom_series::{CustomSeriesDataItem, CustomSeriesPaneRenderer, CustomSeriesPaneView},
    series_options::CustomSeriesOptions,
};
use iced::widget::canvas::Frame;
impl<M: Clone + 'static> YieldCurveChart<M> {
    pub fn add_custom_series<V>(
        &mut self,
        view: V,
        options: CustomSeriesOptions,
        pane: usize,
        position: PriceScalePosition,
    ) -> Result<CustomSeriesHandle<V>, ChartError>
    where
        V: CustomSeriesPaneView<Frame, Item = Maturity, InternalTime = Maturity, Metadata = M>
            + 'static,
        V::Data: 'static,
        V::Options: 'static,
        V::Renderer: Clone + 'static,
        <V::Renderer as CustomSeriesPaneRenderer<Frame>>::HitTestData: Clone + 'static,
    {
        self.core.add_custom_series(view, options, pane, position)
    }
    pub fn pop_custom_data<V>(
        &mut self,
        handle: &CustomSeriesHandle<V>,
        count: usize,
    ) -> Result<Vec<V::Data>, ChartError>
    where
        V: CustomSeriesPaneView<Frame, Item = Maturity, InternalTime = Maturity, Metadata = M>
            + 'static,
        V::Data: 'static,
    {
        let removed = self.core.pop_custom_data(handle, count)?;
        self.dirty |= !removed.is_empty();
        Ok(removed)
    }
    pub fn set_custom_data<V>(
        &mut self,
        handle: &CustomSeriesHandle<V>,
        data: Vec<CustomSeriesDataItem<V::Data, Maturity, M>>,
    ) -> Result<(), ChartError>
    where
        V: CustomSeriesPaneView<Frame, Item = Maturity, InternalTime = Maturity, Metadata = M>
            + 'static,
        V::Data: 'static,
    {
        self.core.set_custom_data(handle, data)?;
        self.dirty = true;
        Ok(())
    }
    pub fn update_custom_data<V>(
        &mut self,
        handle: &CustomSeriesHandle<V>,
        data: CustomSeriesDataItem<V::Data, Maturity, M>,
        historical: bool,
    ) -> Result<(), ChartError>
    where
        V: CustomSeriesPaneView<Frame, Item = Maturity, InternalTime = Maturity, Metadata = M>
            + 'static,
        V::Data: 'static,
    {
        self.core.update_custom_data(handle, data, historical)?;
        self.dirty = true;
        Ok(())
    }
    pub fn replace_custom_view_options<V>(
        &mut self,
        handle: &CustomSeriesHandle<V>,
        options: V::Options,
    ) -> Result<(), ChartError>
    where
        V: CustomSeriesPaneView<Frame, Item = Maturity, InternalTime = Maturity, Metadata = M>,
    {
        self.core.replace_custom_view_options(handle, options)
    }
    pub fn replace_custom_series_options<V>(
        &mut self,
        handle: &CustomSeriesHandle<V>,
        options: CustomSeriesOptions,
    ) -> Result<(), ChartError>
    where
        V: CustomSeriesPaneView<Frame, Item = Maturity, InternalTime = Maturity, Metadata = M>,
    {
        self.core.replace_custom_series_options(handle, options)
    }
}
