//! Borrowed common series behavior; the existing typed owner retains all data.
use super::*;
use crate::model::{
    ipane_primitive::PrimitiveAutoscaleProvider,
    price_line::{LastValueData, PriceLineId, PriceLineOptions},
    price_scale::PriceScale,
    series_markers::MarkerCollection,
};

pub(crate) struct ProjectedRow<'a> {
    index: TimePointIndex,
    values: &'a [f64; 4],
}
impl PlotRowLike for ProjectedRow<'_> {
    fn index(&self) -> TimePointIndex {
        self.index
    }
    fn values(&self) -> &[f64; 4] {
        self.values
    }
}
pub(crate) trait SeriesProjection<O> {
    fn id(&self) -> SeriesId;
    fn common_options(&self) -> &SeriesOptionsCommon;
    fn set_price_scale_id(&mut self, id: String);
    fn series_type(&self) -> SeriesType;
    fn visible(&self) -> bool {
        self.common_options().visible
    }
    fn first_value(&self, range: Option<&RangeImpl<TimePointIndex>>) -> Option<FirstValue>;
    fn first_row(&self) -> Option<ProjectedRow<'_>>;
    fn row(&self, index: TimePointIndex, direction: MismatchDirection) -> Option<ProjectedRow<'_>>;
    fn last_value_data(
        &self,
        scale: &PriceScale,
        range: Option<&RangeImpl<TimePointIndex>>,
        global: bool,
    ) -> Option<LastValueData>;
    fn pane_data_generation(&self) -> u64;
    fn price_lines(&self) -> &Vec<(PriceLineId, PriceLineOptions)>;
    fn price_lines_mut(&mut self) -> &mut Vec<(PriceLineId, PriceLineOptions)>;
    fn marker_collections(&self) -> &Vec<MarkerCollection<O>>;
    fn marker_collections_mut(&mut self) -> &mut Vec<MarkerCollection<O>>;
    fn marker_spacing(&self) -> &Cell<f64>;
    fn primitive_autoscale_mut(&mut self) -> &mut Vec<(u64, PrimitiveAutoscaleProvider)>;
}
impl<I: Clone, O: Clone, M: Clone> SeriesProjection<O> for Series<I, O, M> {
    fn id(&self) -> SeriesId {
        self.id()
    }
    fn common_options(&self) -> &SeriesOptionsCommon {
        self.common_options()
    }
    fn set_price_scale_id(&mut self, id: String) {
        self.options.common_mut().price_scale_id = Some(id);
        self.invalidate_pane_data();
    }
    fn series_type(&self) -> SeriesType {
        self.series_type()
    }
    fn first_value(&self, range: Option<&RangeImpl<TimePointIndex>>) -> Option<FirstValue> {
        self.first_value(range)
    }
    fn first_row(&self) -> Option<ProjectedRow<'_>> {
        let row = self.bars().rows().first()?;
        Some(ProjectedRow {
            index: row.index(),
            values: row.values(),
        })
    }
    fn row(&self, index: TimePointIndex, direction: MismatchDirection) -> Option<ProjectedRow<'_>> {
        let row = self.bars().search(index, direction)?;
        Some(ProjectedRow {
            index: row.index(),
            values: row.values(),
        })
    }
    fn last_value_data(
        &self,
        scale: &PriceScale,
        range: Option<&RangeImpl<TimePointIndex>>,
        global: bool,
    ) -> Option<LastValueData> {
        self.last_value_data(scale, range, global)
    }
    fn pane_data_generation(&self) -> u64 {
        self.pane_data_generation()
    }
    fn price_lines(&self) -> &Vec<(PriceLineId, PriceLineOptions)> {
        &self.price_lines
    }
    fn price_lines_mut(&mut self) -> &mut Vec<(PriceLineId, PriceLineOptions)> {
        &mut self.price_lines
    }
    fn marker_collections(&self) -> &Vec<MarkerCollection<O>> {
        &self.marker_collections
    }
    fn marker_collections_mut(&mut self) -> &mut Vec<MarkerCollection<O>> {
        &mut self.marker_collections
    }
    fn marker_spacing(&self) -> &Cell<f64> {
        &self.marker_spacing
    }
    fn primitive_autoscale_mut(&mut self) -> &mut Vec<(u64, PrimitiveAutoscaleProvider)> {
        &mut self.primitive_autoscale
    }
}
impl<I, O, D, M> SeriesProjection<O> for CustomSeries<I, O, D, M> {
    fn id(&self) -> SeriesId {
        self.id()
    }
    fn common_options(&self) -> &SeriesOptionsCommon {
        &self.options.common
    }
    fn set_price_scale_id(&mut self, id: String) {
        self.options.common.price_scale_id = Some(id);
        self.invalidate_pane_data();
    }
    fn series_type(&self) -> SeriesType {
        SeriesType::Custom
    }
    fn first_value(&self, range: Option<&RangeImpl<TimePointIndex>>) -> Option<FirstValue> {
        self.first_value(range)
    }
    fn first_row(&self) -> Option<ProjectedRow<'_>> {
        let row = self.rows().first()?;
        Some(ProjectedRow {
            index: row.base.index,
            values: &row.base.value,
        })
    }
    fn row(&self, index: TimePointIndex, direction: MismatchDirection) -> Option<ProjectedRow<'_>> {
        let at = self.data.partition_point(|row| row.base.index < index);
        let at = if self.data.get(at).is_some_and(|row| row.base.index == index) {
            at
        } else {
            match direction {
                MismatchDirection::None => return None,
                MismatchDirection::NearestLeft => at.checked_sub(1)?,
                MismatchDirection::NearestRight => at,
            }
        };
        let row = self.data.get(at)?;
        Some(ProjectedRow {
            index: row.base.index,
            values: &row.base.value,
        })
    }
    fn last_value_data(
        &self,
        scale: &PriceScale,
        range: Option<&RangeImpl<TimePointIndex>>,
        global: bool,
    ) -> Option<LastValueData> {
        if scale.is_empty() {
            return None;
        }
        let range = range?;
        let first = self.first_value(Some(range))?;
        let at = if global {
            self.data.len().checked_sub(1)?
        } else {
            self.data
                .partition_point(|row| row.base.index <= range.right())
                .checked_sub(1)?
        };
        let row = &self.data[at];
        let price = row.base.value[3];
        let coordinate = scale.price_to_coordinate(price, first.value).value();
        if !coordinate.is_finite() {
            return None;
        }
        Some(LastValueData {
            price,
            index: row.base.index,
            coordinate,
            color: row
                .color
                .clone()
                .unwrap_or_else(|| self.options.style.color.clone()),
            text: scale.format_price(price, first.value),
            absolute: scale.format_price_absolute(price),
            percentage: scale.format_price_percentage(price, first.value),
        })
    }
    fn pane_data_generation(&self) -> u64 {
        self.pane_data_generation()
    }
    fn price_lines(&self) -> &Vec<(PriceLineId, PriceLineOptions)> {
        &self.price_lines
    }
    fn price_lines_mut(&mut self) -> &mut Vec<(PriceLineId, PriceLineOptions)> {
        &mut self.price_lines
    }
    fn marker_collections(&self) -> &Vec<MarkerCollection<O>> {
        &self.marker_collections
    }
    fn marker_collections_mut(&mut self) -> &mut Vec<MarkerCollection<O>> {
        &mut self.marker_collections
    }
    fn marker_spacing(&self) -> &Cell<f64> {
        &self.marker_spacing
    }
    fn primitive_autoscale_mut(&mut self) -> &mut Vec<(u64, PrimitiveAutoscaleProvider)> {
        &mut self.primitive_autoscale
    }
}
