//! Series-owned update markers; drawing owns a frozen snapshot, never this state.
use super::{
    Chart, ChartError, DrawingPrimitive, PreparedPrimitive, PrimitiveContext, PrimitiveId,
    PrimitiveLayer, PrimitiveOwner, PrimitiveView,
};
use crate::model::data_consumer::{AreaDataItem, BuiltInSeriesDataItem, LineDataItem, TimedData};
use crate::model::{
    data_layer::SeriesId,
    horz_scale_behavior_time::{horz_scale_behavior_time::TimeKey, types::Time},
    series_options::SeriesOptionsMap,
};
use crate::renderers::up_down_markers_renderer::{UpDownRenderer, geometry, in_view};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    time::{Duration, Instant},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerSign {
    Negative,
    Neutral,
    Positive,
}
#[derive(Clone, Debug, PartialEq)]
pub struct UpDownMarker<T = Time> {
    pub time: T,
    pub value: f64,
    pub sign: MarkerSign,
}
#[derive(Clone, Debug, PartialEq)]
pub struct UpDownMarkersOptions {
    pub positive_color: String,
    pub negative_color: String,
    /// Zero expires immediately, matching executed source timers; manual markers have no deadline.
    pub update_visibility_duration: Duration,
}
impl Default for UpDownMarkersOptions {
    fn default() -> Self {
        Self {
            positive_color: "#22AB94".into(),
            negative_color: "#F7525F".into(),
            update_visibility_duration: Duration::from_secs(5),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct UpDownMarkersId(PrimitiveId);
impl UpDownMarkersId {
    pub fn primitive_id(self) -> PrimitiveId {
        self.0
    }
}
#[derive(Clone, Debug)]
struct Active<T = Time, K = TimeKey> {
    key: K,
    marker: UpDownMarker<T>,
    expires: Option<Instant>,
}
#[derive(Debug)]
struct State<T = Time, K = TimeKey> {
    options: UpDownMarkersOptions,
    colors: [iced::Color; 2],
    active: Vec<Active<T, K>>,
    next: Option<Instant>,
}
#[derive(Debug)]
pub(crate) struct Controller<T = Time, K = TimeKey> {
    pub series: SeriesId,
    state: Rc<RefCell<State<T, K>>>,
    baseline: BTreeMap<K, f64>,
}
#[derive(Debug)]
struct Primitive<T = Time, K = TimeKey>(Rc<RefCell<State<T, K>>>);
fn key<B: super::ChartBehavior>(behavior: &B, time: &B::Item) -> Result<B::Key, ChartError> {
    Ok(behavior.key(&behavior.to_internal(time).map_err(B::conversion_error)?))
}
fn colors(options: &UpDownMarkersOptions) -> Result<[iced::Color; 2], ChartError> {
    let parse = |value: &str| {
        crate::ui::line_chart::color(value).map_err(|_| ChartError::InvalidUpDownOptions)
    };
    Ok([
        parse(&options.positive_color)?,
        parse(&options.negative_color)?,
    ])
}
fn value<T, M>(item: &BuiltInSeriesDataItem<T, M>) -> Option<f64> {
    match item {
        BuiltInSeriesDataItem::Line(LineDataItem::Data(d)) => Some(d.value),
        BuiltInSeriesDataItem::Area(AreaDataItem::Data(d)) => Some(d.value),
        _ => None,
    }
}
impl<B: super::ChartBehavior, M: Clone + 'static> Chart<B, M> {
    pub fn add_up_down_markers(
        &mut self,
        series: SeriesId,
        options: UpDownMarkersOptions,
    ) -> Result<UpDownMarkersId, ChartError> {
        let owner = self
            .model
            .series(series)
            .ok_or(ChartError::UnknownSeries(series))?;
        if !matches!(
            owner.options(),
            SeriesOptionsMap::Line(_) | SeriesOptionsMap::Area(_)
        ) {
            return Err(ChartError::UnsupportedSeries);
        }
        drop(owner);
        let state = Rc::new(RefCell::new(State {
            colors: colors(&options)?,
            options,
            active: vec![],
            next: None,
        }));
        let id = self.add_series_primitive(series, Primitive(state.clone()))?;
        self.up_down.insert(
            id,
            Controller {
                series,
                state,
                baseline: BTreeMap::new(),
            },
        );
        Ok(UpDownMarkersId(id))
    }
    /// Seed comparison values. Existing markers are not implicitly cleared.
    pub fn set_up_down_data(
        &mut self,
        id: UpDownMarkersId,
        data: Vec<BuiltInSeriesDataItem<B::Item, M>>,
    ) -> Result<(), ChartError> {
        let series = self
            .up_down
            .get(&id.0)
            .ok_or(ChartError::UnknownPrimitive(id.0))?
            .series;
        let baseline = data
            .iter()
            .filter_map(|item| {
                value(item).map(|v| {
                    key(self.model.time_scale().behavior(), item.time()).map(|key| (key, v))
                })
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        self.set_data(series, data)?;
        self.up_down.get_mut(&id.0).unwrap().baseline = baseline;
        Ok(())
    }
    /// Compare to the last managed full data set, not the preceding update.
    pub fn update_up_down_data(
        &mut self,
        id: UpDownMarkersId,
        item: BuiltInSeriesDataItem<B::Item, M>,
        historical: bool,
        now: Instant,
    ) -> Result<(), ChartError> {
        let key = key(self.model.time_scale().behavior(), item.time())?;
        let new_value = value(&item);
        let time = item.time().clone();
        let (series, previous, duration) = {
            let controller = self
                .up_down
                .get(&id.0)
                .ok_or(ChartError::UnknownPrimitive(id.0))?;
            (
                controller.series,
                controller.baseline.get(&key).copied(),
                controller.state.borrow().options.update_visibility_duration,
            )
        };
        let deadline = if previous.is_some() && new_value.is_some() {
            Some(
                now.checked_add(duration)
                    .ok_or(ChartError::InvalidUpDownDeadline)?,
            )
        } else {
            None
        };
        self.update_data(series, item, historical)?;
        let controller = self.up_down.get_mut(&id.0).unwrap();
        if let (Some(old), Some(new), Some(expires)) = (previous, new_value, deadline) {
            let sign = if new == old {
                MarkerSign::Neutral
            } else if new > old {
                MarkerSign::Positive
            } else {
                MarkerSign::Negative
            };
            let mut state = controller.state.borrow_mut();
            // shortcut: refresh scans active markers; add indexed order if large active sets dominate profiles.
            state.active.retain(|entry| entry.key != key);
            state.active.push(Active {
                key,
                marker: UpDownMarker {
                    time,
                    value: new,
                    sign,
                },
                expires: Some(expires),
            });
            state.next = state.active.iter().filter_map(|a| a.expires).min();
        } else if new_value.is_none() {
            controller.baseline.remove(&key);
        }
        self.request_primitive_update(id.0)
    }
    /// Expiration is a deadline, not a reason to enable continuous animation.
    pub fn next_up_down_expiration(&self) -> Option<Instant> {
        self.up_down
            .values()
            .filter_map(|controller| controller.state.borrow().next)
            .min()
    }
    pub fn set_up_down_markers(
        &mut self,
        id: UpDownMarkersId,
        markers: Vec<UpDownMarker<B::Item>>,
    ) -> Result<(), ChartError> {
        self.up_down
            .get(&id.0)
            .ok_or(ChartError::UnknownPrimitive(id.0))?;
        let mut active: Vec<Active<B::Item, B::Key>> = vec![];
        for (index, marker) in markers.into_iter().enumerate() {
            if !marker.value.is_finite() || marker.value.abs() > 90_071_992_547_409.9 {
                return Err(ChartError::InvalidMarker { index });
            }
            let key = key(self.model.time_scale().behavior(), &marker.time)?;
            active.retain(|entry| entry.key != key);
            active.push(Active {
                key,
                marker,
                expires: None,
            });
        }
        {
            let mut state = self.up_down[&id.0].state.borrow_mut();
            state.active = active;
            state.next = None;
        }
        self.request_primitive_update(id.0)
    }
    pub fn up_down_markers(
        &mut self,
        id: UpDownMarkersId,
        now: Instant,
    ) -> Result<Vec<UpDownMarker<B::Item>>, ChartError> {
        self.up_down
            .get(&id.0)
            .ok_or(ChartError::UnknownPrimitive(id.0))?;
        self.expire_up_down_markers(now);
        let controller = self
            .up_down
            .get(&id.0)
            .ok_or(ChartError::UnknownPrimitive(id.0))?;
        Ok(controller
            .state
            .borrow()
            .active
            .iter()
            .map(|a| a.marker.clone())
            .collect())
    }
    pub(super) fn expire_up_down_markers(&mut self, now: Instant) {
        if self
            .next_up_down_expiration()
            .is_none_or(|deadline| deadline > now)
        {
            return;
        }
        let mut changed = false;
        for controller in self.up_down.values() {
            let mut state = controller.state.borrow_mut();
            if state.next.is_some_and(|deadline| deadline <= now) {
                state
                    .active
                    .retain(|a| a.expires.is_none_or(|expires| expires > now));
                state.next = state.active.iter().filter_map(|a| a.expires).min();
                changed = true;
            }
        }
        if changed {
            self.model.full_update();
        }
    }
    pub fn clear_up_down_markers(&mut self, id: UpDownMarkersId) -> Result<(), ChartError> {
        self.set_up_down_markers(id, vec![])
    }
    pub fn replace_up_down_marker_options(
        &mut self,
        id: UpDownMarkersId,
        options: UpDownMarkersOptions,
    ) -> Result<(), ChartError> {
        let controller = self
            .up_down
            .get(&id.0)
            .ok_or(ChartError::UnknownPrimitive(id.0))?;
        let colors = colors(&options)?;
        {
            let mut state = controller.state.borrow_mut();
            state.options = options;
            state.colors = colors;
        }
        self.request_primitive_update(id.0)
    }
    pub fn remove_up_down_markers(&mut self, id: UpDownMarkersId) -> Result<(), ChartError> {
        self.up_down
            .get(&id.0)
            .ok_or(ChartError::UnknownPrimitive(id.0))?;
        self.remove_primitive(id.0)
    }
}
impl<B: super::ChartBehavior, M: Clone + 'static> DrawingPrimitive<B, M>
    for Primitive<B::Item, B::Key>
{
    fn prepare(&self, context: &PrimitiveContext<'_, B, M>) -> PreparedPrimitive {
        let PrimitiveOwner::Series(series) = context.owner else {
            return PreparedPrimitive::default();
        };
        let Some(series) = context.model.series(series) else {
            return PreparedPrimitive::default();
        };
        if !series.common_options().visible {
            return PreparedPrimitive::default();
        }
        let neutral = match series.options() {
            SeriesOptionsMap::Line(o) => &o.style.color,
            SeriesOptionsMap::Area(o) => &o.style.line_color,
            _ => return PreparedPrimitive::default(),
        };
        let Ok(neutral) = crate::ui::line_chart::color(neutral) else {
            return PreparedPrimitive::default();
        };
        let ratio = f64::from(context.pixel_ratio);
        let state = self.0.borrow();
        let points = state
            .active
            .iter()
            .filter_map(|entry| {
                let m = &entry.marker;
                let x = context.time_to_coordinate(&m.time)?;
                let y = context.price_to_coordinate(m.value)?;
                if !in_view(x, y, context.size, ratio) {
                    return None;
                }
                let color = match m.sign {
                    MarkerSign::Positive => state.colors[0],
                    MarkerSign::Negative => state.colors[1],
                    MarkerSign::Neutral => neutral,
                };
                Some((geometry(x, y, m.sign, ratio, ratio), color))
            })
            .collect::<Vec<_>>();
        if points.is_empty() {
            return PreparedPrimitive::default();
        }
        PreparedPrimitive {
            views: vec![PrimitiveView::new(
                PrimitiveLayer::Normal,
                UpDownRenderer { points, ratio },
            )],
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
use crate::model::horz_scale_behavior_time::time_utils::convert_time;
