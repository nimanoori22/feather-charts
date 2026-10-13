//! Selection from owned, settled logical geometry, never from Canvas paths.
use super::line_chart::PlotSnapshot;
use crate::{
    model::{
        data_layer::SeriesId,
        hit_test::{ChartHit, HitTarget},
        internal_hit_test::{HitTestPriority, InternalHitTestCandidate, is_better_hit},
        time_data::SeriesItemsIndexesRange,
    },
    renderers::{
        draw_line::LineType,
        line_hit_test::line_hit,
        line_renderer::{LinePoint, LineRendererData},
    },
};
#[derive(Clone, Debug)]
pub struct PreparedSeriesHit {
    pub series: SeriesId,
    pub geometry: HitGeometry,
    pub filled: bool,
    pub tolerance: f64,
}
#[derive(Clone, Debug)]
pub enum HitGeometry {
    Line {
        data: LineRendererData,
        kind: LineType,
    },
    Range {
        items: Vec<crate::renderers::range_hit_test::RangePoint>,
        spacing: f64,
    },
}
impl PreparedSeriesHit {
    pub(crate) fn line(
        series: SeriesId,
        data: &LineRendererData,
        kind: LineType,
        tolerance: f64,
    ) -> Self {
        let from = data.visible_range.from.saturating_sub(1);
        let to = (data.visible_range.to + 1).min(data.items.len());
        Self {
            series,
            filled: false,
            tolerance,
            geometry: HitGeometry::Line {
                kind,
                data: LineRendererData {
                    items: data.items[from..to]
                        .iter()
                        .map(|p| LinePoint {
                            index: p.index,
                            x: p.x,
                            y: p.y,
                            color: String::new(),
                        })
                        .collect(),
                    visible_range: SeriesItemsIndexesRange {
                        from: data.visible_range.from - from,
                        to: data.visible_range.to - from,
                    },
                    line_type: data.line_type,
                    line_width: data.line_width,
                    line_style: data.line_style,
                    bar_width: data.bar_width,
                    point_markers_radius: data.point_markers_radius,
                },
            },
        }
    }
    fn hit(&self, x: f64, y: f64) -> Option<(ChartHit, InternalHitTestCandidate)> {
        let (distance, priority) = match &self.geometry {
            HitGeometry::Line { data, kind } => line_hit(
                &data.items,
                data.visible_range,
                x,
                y,
                *kind,
                if data.line_type.is_some() {
                    f64::from(data.line_width.pixels())
                } else {
                    1.
                },
                data.point_markers_radius,
                data.bar_width,
                self.tolerance,
            )?,
            HitGeometry::Range { items, spacing } => (
                crate::renderers::range_hit_test::range_hit(items, x, y, *spacing, self.tolerance)?,
                HitTestPriority::Range,
            ),
        };
        let target = match priority {
            HitTestPriority::Point => HitTarget::SeriesPoint,
            HitTestPriority::Line => HitTarget::SeriesLine,
            HitTestPriority::Range => HitTarget::SeriesRange,
        };
        Some((
            ChartHit {
                series: self.series,
                target,
            },
            InternalHitTestCandidate {
                distance,
                priority,
                cursor_style: None,
                external_id: None,
            },
        ))
    }
}
pub(crate) fn prepare<B, M>(
    model: &crate::model::chart_model::ChartModel<B, M>,
    id: SeriesId,
    view: &super::line_chart::PaneView<'_>,
) -> Option<PreparedSeriesHit>
where
    B: crate::model::ihorz_scale_behavior::HorzScaleBehavior,
    B::Item: 'static,
    B::InternalItem: 'static,
    B::Key: PartialOrd,
    B::CacheKey: Eq + std::hash::Hash,
    M: Clone + 'static,
{
    use super::line_chart::PaneView;
    use crate::model::{plot_data::PlotRowLike, series_options::SeriesOptionsMap};
    use crate::renderers::range_hit_test::RangePoint;
    let series = model.series(id)?;
    let tolerance = series.common_options().hit_test_tolerance;
    let line_kind = match series.options() {
        SeriesOptionsMap::Line(o) => o.style.line_type,
        SeriesOptionsMap::Area(o) => o.style.line_type,
        SeriesOptionsMap::Baseline(o) => o.style.line_type,
        _ => LineType::Simple,
    };
    let line = match view {
        PaneView::Line(v) => v.data(),
        PaneView::Area(v) => v.data().map(|d| &d.line),
        PaneView::Baseline(v) => v.data().map(|d| &d.line),
        _ => None,
    };
    if let Some(data) = line {
        let mut hit = PreparedSeriesHit::line(id, data, line_kind, tolerance);
        hit.filled = matches!(view, PaneView::Area(_) | PaneView::Baseline(_));
        return Some(hit);
    }
    let rows = series.bars().rows();
    let (items, spacing) = match view {
        PaneView::Bars(v) => {
            let d = v.data()?;
            let r = d.visible_range;
            (
                d.items[r.from..r.to]
                    .iter()
                    .enumerate()
                    .map(|(i, p)| RangePoint {
                        index: rows[r.from + i].index().value(),
                        x: p.x,
                        top: p.high_y,
                        bottom: p.low_y,
                    })
                    .collect(),
                d.bar_spacing,
            )
        }
        PaneView::Candlesticks(v) => {
            let d = v.data()?;
            let r = d.visible_range;
            (
                d.items[r.from..r.to]
                    .iter()
                    .enumerate()
                    .map(|(i, p)| RangePoint {
                        index: rows[r.from + i].index().value(),
                        x: p.x,
                        top: p.high_y,
                        bottom: p.low_y,
                    })
                    .collect(),
                d.bar_spacing,
            )
        }
        PaneView::Histogram(v) => {
            let d = v.data()?;
            let r = d.visible_range;
            (
                d.items[r.from..r.to]
                    .iter()
                    .map(|p| RangePoint {
                        index: p.index,
                        x: p.x,
                        top: p.y,
                        bottom: d.baseline,
                    })
                    .collect(),
                d.bar_spacing,
            )
        }
        _ => return None,
    };
    Some(PreparedSeriesHit {
        series: id,
        filled: false,
        tolerance,
        geometry: HitGeometry::Range { items, spacing },
    })
}
impl PlotSnapshot {
    pub fn hit_test(&self, point: iced::Point) -> Option<ChartHit> {
        if !point.x.is_finite()
            || !point.y.is_finite()
            || point.x < 0.
            || point.y < 0.
            || point.x >= self.size.width
            || point.y >= self.size.height
        {
            return None;
        }
        let marker = self.marker_hit(point);
        let mut best: Option<(ChartHit, InternalHitTestCandidate)> = None;
        for series in self.hit_series.iter().rev() {
            if let Some(m) = &marker
                && m.series == series.series
            {
                return best.map(|(h, _)| h).or_else(|| {
                    Some(ChartHit {
                        series: m.series,
                        target: HitTarget::Marker(m.clone()),
                    })
                });
            }
            if let Some(hit) = series.hit(f64::from(point.x), f64::from(point.y))
                && is_better_hit(&hit.1, best.as_ref().map(|(_, c)| c))
            {
                best = Some(hit);
            }
        }
        best.map(|(h, _)| h).or_else(|| {
            marker.map(|m| ChartHit {
                series: m.series,
                target: HitTarget::Marker(m),
            })
        })
    }
}
