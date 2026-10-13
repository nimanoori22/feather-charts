use super::*;
use crate::model::{
    axis_snapshots::{FloatingAxisLabel, SeriesAxisLabel},
    colors::ColorParser,
    price_line::{PriceLine, PriceLineId, PriceLineOptions},
    series_options::{PriceAxisLastValueMode, PriceLineSource},
};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_LINE: AtomicU64 = AtomicU64::new(1);
#[derive(Default)]
pub(crate) struct PriceDecorations {
    pub lines: Vec<PriceDecorationLine>,
    pub left: Vec<SeriesAxisLabel>,
    pub right: Vec<SeriesAxisLabel>,
    pub left_center: Option<f64>,
    pub right_center: Option<f64>,
}
pub(crate) struct PriceDecorationLine {
    pub series: SeriesId,
    pub line: PriceLine,
    pub kind: crate::model::hit_test::PriceLineKind,
    pub external_id: Option<String>,
}
impl<B, M> ChartModel<B, M>
where
    B: HorzScaleBehavior,
    B::Item: 'static,
    B::InternalItem: 'static,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone + 'static,
{
    pub fn add_price_line(
        &mut self,
        series: SeriesId,
        options: PriceLineOptions,
    ) -> Result<PriceLineId, ChartModelError> {
        if !options.valid() {
            return Err(ChartModelError::InvalidPriceLineOptions);
        }
        let handle = self
            .series
            .get(&series)
            .and_then(|s| s.built_in())
            .ok_or(ChartModelError::UnknownSeries(series))?;
        let id = NEXT_LINE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map(PriceLineId)
            .map_err(|_| ChartModelError::PriceLineIdExhausted)?;
        handle.borrow_mut().price_lines.push((id, options));
        self.light_update();
        self.flush_effects();
        Ok(id)
    }
    pub fn price_line_options(&self, id: PriceLineId) -> Result<PriceLineOptions, ChartModelError> {
        for entry in self.series.values() {
            if let Some(series) = entry.built_in()
                && let Some((_, options)) = series
                    .borrow()
                    .price_lines
                    .iter()
                    .find(|(key, _)| *key == id)
            {
                return Ok(options.clone());
            }
        }
        Err(ChartModelError::UnknownPriceLine(id))
    }
    pub fn replace_price_line_options(
        &mut self,
        id: PriceLineId,
        options: PriceLineOptions,
    ) -> Result<(), ChartModelError> {
        if !options.valid() {
            return Err(ChartModelError::InvalidPriceLineOptions);
        }
        for entry in self.series.values() {
            if let Some(handle) = entry.built_in() {
                let mut series = handle.borrow_mut();
                if let Some((_, old)) = series.price_lines.iter_mut().find(|(key, _)| *key == id) {
                    *old = options;
                    drop(series);
                    self.light_update();
                    self.flush_effects();
                    return Ok(());
                }
            }
        }
        Err(ChartModelError::UnknownPriceLine(id))
    }
    pub fn remove_price_line(&mut self, id: PriceLineId) -> Result<(), ChartModelError> {
        for entry in self.series.values() {
            if let Some(handle) = entry.built_in() {
                let mut series = handle.borrow_mut();
                if let Some(index) = series.price_lines.iter().position(|(key, _)| *key == id) {
                    series.price_lines.remove(index);
                    drop(series);
                    self.light_update();
                    self.flush_effects();
                    return Ok(());
                }
            }
        }
        Err(ChartModelError::UnknownPriceLine(id))
    }
    pub(crate) fn price_decorations(
        &mut self,
        pane: usize,
    ) -> Result<PriceDecorations, AxisSnapshotError> {
        let state = self
            .panes
            .get_mut(pane)
            .ok_or(AxisSnapshotError::InvalidPane(pane))?;
        let mut result = PriceDecorations::default();
        if self.time_scale.is_empty() {
            return Ok(result);
        }
        let visible = self.time_scale.visible_strict_range();
        let ids = state.ordered_source_ids().to_vec();
        let attached = state.attached_source_ids();
        let left_first = attached.iter().copied().find(|id| {
            state
                .price_scale_for_source(*id)
                .is_some_and(|s| s.id() == "left")
        });
        let right_first = attached.iter().copied().find(|id| {
            state
                .price_scale_for_source(*id)
                .is_some_and(|s| s.id() == "right")
        });
        let mut parser = ColorParser::default();
        let mut left_overlay = vec![];
        let mut right_overlay = vec![];
        for id in ids {
            let Some(handle) = self.series.get(&id).and_then(|s| s.built_in()) else {
                continue;
            };
            let series = handle.borrow();
            if !series.visible() {
                continue;
            }
            let Some(scale) = state.price_scale_for_source(id) else {
                continue;
            };
            let Some(first) = series.first_value(visible.as_ref()) else {
                continue;
            };
            if scale.is_empty() {
                continue;
            }
            let common = series.common_options();
            if common.base_line_visible
                && matches!(scale.id(), "left" | "right")
                && matches!(
                    scale.options().mode,
                    crate::model::price_scale::PriceScaleMode::Percentage
                        | crate::model::price_scale::PriceScaleMode::IndexedTo100
                )
            {
                result.lines.push(PriceDecorationLine {
                    series: id,
                    kind: crate::model::hit_test::PriceLineKind::RelativeBase,
                    external_id: None,
                    line: PriceLine {
                        coordinate: scale.price_to_coordinate(first.value, first.value).value(),
                        color: common.base_line_color.clone(),
                        width: common.base_line_width,
                        style: common.base_line_style,
                    },
                });
            }
            let last = series.last_value_data(scale, visible.as_ref(), false);
            let line_value = if common.price_line_source == PriceLineSource::LastBar {
                series.last_value_data(scale, visible.as_ref(), true)
            } else {
                last.clone()
            };
            if common.price_line_visible
                && let Some(value) = line_value
            {
                result.lines.push(PriceDecorationLine {
                    series: id,
                    kind: crate::model::hit_test::PriceLineKind::LastValue,
                    external_id: None,
                    line: PriceLine {
                        coordinate: value.coordinate,
                        color: if common.price_line_color.is_empty() {
                            value.color
                        } else {
                            common.price_line_color.clone()
                        },
                        width: common.price_line_width,
                        style: common.price_line_style,
                    },
                });
            }
            let target = match scale.id() {
                "left" => Some("left"),
                "right" => Some("right"),
                _ => state.default_visible_price_scale().map(|s| s.id()),
            };
            let mut labels = vec![];
            if let Some(value) = last {
                let both = common.series_last_value_mode
                    == Some(PriceAxisLastValueMode::LastPriceAndPercentageValue);
                let axis_text = if !common.last_value_visible {
                    String::new()
                } else if both {
                    if scale.is_percentage() {
                        value.percentage.clone()
                    } else {
                        value.absolute.clone()
                    }
                } else {
                    value.text
                };
                let extra = if common.last_value_visible && both {
                    if scale.is_percentage() {
                        value.absolute
                    } else {
                        value.percentage
                    }
                } else {
                    String::new()
                };
                let pane_text = format!("{} {}", common.title, extra).trim().to_owned();
                if !axis_text.is_empty() || !pane_text.is_empty() {
                    let bg = if common.price_line_color.is_empty() {
                        &value.color
                    } else {
                        &common.price_line_color
                    };
                    let colors = parser
                        .generate_contrast_colors(bg)
                        .map_err(|_| AxisSnapshotError::InvalidLabelColor)?;
                    labels.push(SeriesAxisLabel {
                        source: Some(id),
                        fixed_coordinate: None,
                        tick_visible: None,
                        additional_padding: 0.,
                        axis: FloatingAxisLabel {
                            coordinate: value.coordinate,
                            text: axis_text,
                            background: colors.background,
                            color: colors.foreground,
                        },
                        pane_text,
                        render_coordinate: value.coordinate,
                    });
                }
            }
            for (line_id, options) in &series.price_lines {
                let y = scale
                    .price_to_coordinate(options.price, first.value)
                    .value();
                if !y.is_finite() {
                    continue;
                }
                if options.line_visible {
                    result.lines.push(PriceDecorationLine {
                        series: id,
                        kind: crate::model::hit_test::PriceLineKind::Custom(*line_id),
                        external_id: options.external_id.clone(),
                        line: PriceLine {
                            coordinate: y,
                            color: options.color.clone(),
                            width: options.line_width,
                            style: options.line_style,
                        },
                    });
                }
                if options.axis_label_visible {
                    let bg = if options.axis_label_color.is_empty() {
                        &options.color
                    } else {
                        &options.axis_label_color
                    };
                    let colors = parser
                        .generate_contrast_colors(bg)
                        .map_err(|_| AxisSnapshotError::InvalidLabelColor)?;
                    labels.push(SeriesAxisLabel {
                        source: Some(id),
                        fixed_coordinate: None,
                        tick_visible: None,
                        additional_padding: 0.,
                        axis: FloatingAxisLabel {
                            coordinate: y,
                            text: scale.format_price(options.price, first.value),
                            background: colors.background,
                            color: if options.axis_label_text_color.is_empty() {
                                colors.foreground
                            } else {
                                options.axis_label_text_color.clone()
                            },
                        },
                        pane_text: options.title.clone(),
                        render_coordinate: y,
                    });
                }
            }
            match target {
                Some("left") => {
                    if Some(id) == left_first {
                        result.left_center = labels.first().map(|l| l.axis.coordinate);
                    }
                    if scale.id() == "left" {
                        result.left.extend(labels);
                    } else {
                        left_overlay.extend(labels);
                    }
                }
                Some("right") => {
                    if Some(id) == right_first {
                        result.right_center = labels.first().map(|l| l.axis.coordinate);
                    }
                    if scale.id() == "right" {
                        result.right.extend(labels);
                    } else {
                        right_overlay.extend(labels);
                    }
                }
                _ => {}
            }
        }
        result.left.extend(left_overlay);
        result.right.extend(right_overlay);
        Ok(result)
    }
}
