use crate::{
    model::{
        ihorz_scale_behavior::HorzScaleBehavior,
        plot_data::PlotRowLike,
        plot_list::MismatchDirection,
        price_scale::PriceScale,
        series::Series,
        series_markers::{MarkerHit, MarkerPosition, shape_height, shape_margin},
        series_options::SeriesType,
        time_scale::TimeScale,
    },
    renderers::{
        line_renderer::Point,
        series_markers_renderer::{MarkerText, PreparedMarker, PreparedMarkerCollection},
    },
};
use std::hash::Hash;

pub(crate) fn prepare<B, M>(
    series: &Series<B::InternalItem, B::Item, M>,
    time: &mut TimeScale<B>,
    scale: &PriceScale,
    font_size: f64,
    font_family: &str,
) -> Vec<PreparedMarkerCollection>
where
    B: HorzScaleBehavior,
    B::Key: PartialOrd,
    B::CacheKey: Eq + Hash,
    M: Clone,
{
    if series.marker_collections.is_empty()
        || !series.common_options().visible
        || time.is_empty()
        || scale.is_empty()
    {
        return vec![];
    }
    let Some(visible) = time.visible_strict_range() else {
        return vec![];
    };
    let Some(first) = series.first_value(Some(&visible)) else {
        return vec![];
    };
    let Some(first_row) = series.bars().rows().first() else {
        return vec![];
    };
    let spacing = time.bar_spacing();
    let margin = shape_margin(spacing);
    let ohlc = matches!(
        series.series_type(),
        SeriesType::Bar | SeriesType::Candlestick
    );
    let mut collections = vec![];
    for collection in &series.marker_collections {
        let mut cache = collection.index_cache.borrow_mut();
        let generation = (series.pane_data_generation(), time.data_generation());
        if cache.generation != Some(generation) {
            cache.indices = collection
                .markers
                .iter()
                .enumerate()
                .filter_map(|(i, marker)| {
                    let internal = time.behavior().to_internal(&marker.time).ok()?;
                    let index = time.time_to_index(&internal, true)?;
                    let mode = if index < first_row.index() {
                        MismatchDirection::NearestRight
                    } else {
                        MismatchDirection::NearestLeft
                    };
                    Some((i, series.bars().search(index, mode)?.index()))
                })
                .collect();
            cache.generation = Some(generation);
        }
        let indexed = &cache.indices;
        let from = indexed
            .partition_point(|(_, index)| *index < visible.left())
            .saturating_sub(1);
        let to = (indexed.partition_point(|(_, index)| *index <= visible.right()) + 1)
            .min(indexed.len());
        let mut items = vec![];
        let mut previous = None;
        let (mut above, mut below) = (margin, margin);
        for (i, index) in &indexed[from..to] {
            let marker = &collection.markers[*i];
            let index = *index;
            let Some(row) = series.bars().search(index, MismatchDirection::None) else {
                continue;
            };
            if previous != Some(index) {
                above = margin;
                below = margin;
                previous = Some(index);
            }
            let v = row.values();
            let position = marker.position;
            let price = match position {
                MarkerPosition::AtPriceTop(p)
                | MarkerPosition::AtPriceBottom(p)
                | MarkerPosition::AtPriceMiddle(p) => p,
                MarkerPosition::AboveBar if ohlc => {
                    if scale.options().invert_scale {
                        v[2]
                    } else {
                        v[1]
                    }
                }
                MarkerPosition::BelowBar if ohlc => {
                    if scale.options().invert_scale {
                        v[1]
                    } else {
                        v[2]
                    }
                }
                _ => v[3],
            };
            let size = shape_height(spacing) * marker.size.max(0.);
            let half = size / 2.;
            let mut y = scale.price_to_coordinate(price, first.value).value();
            let has_text = marker.text.as_ref().is_some_and(|t| !t.is_empty());
            let text_y = match position {
                MarkerPosition::AboveBar | MarkerPosition::AtPriceTop(_) => {
                    y -= half
                        + if position == MarkerPosition::AboveBar {
                            above
                        } else {
                            0.
                        };
                    if has_text {
                        above += font_size * 1.2;
                    }
                    if position == MarkerPosition::AboveBar {
                        above += size + margin;
                    }
                    y - half - font_size * 0.6
                }
                MarkerPosition::BelowBar | MarkerPosition::AtPriceBottom(_) => {
                    y += half
                        + if position == MarkerPosition::BelowBar {
                            below
                        } else {
                            0.
                        };
                    if has_text {
                        below += font_size * 1.2;
                    }
                    if position == MarkerPosition::BelowBar {
                        below += size + margin;
                    }
                    y + half + margin + font_size * 0.6
                }
                _ => y + half + margin + font_size * 0.6,
            };
            let x = time.index_to_coordinate(index).value();
            if [x, y, size, text_y]
                .iter()
                .any(|v| !v.is_finite() || v.abs() > f64::from(f32::MAX))
            {
                continue;
            }
            items.push(PreparedMarker {
                index,
                center: Point { x, y },
                size,
                shape: marker.shape,
                color: marker.color.clone(),
                hit: MarkerHit {
                    series: series.id(),
                    collection: collection.id,
                    index: *i,
                    external_id: marker.id.clone(),
                },
                text: has_text.then(|| MarkerText {
                    content: marker.text.clone().unwrap(),
                    x,
                    y: text_y,
                    width: 0.,
                    height: font_size,
                }),
            });
        }
        if !items.is_empty() {
            collections.push(PreparedMarkerCollection {
                series_layer: 0,
                series: series.id(),
                id: collection.id,
                z_order: collection.options.z_order,
                items,
                font_size,
                font_family: font_family.into(),
            });
        }
    }
    collections
}
