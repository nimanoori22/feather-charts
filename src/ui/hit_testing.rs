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
    pub geometry: LineRendererData,
    pub kind: LineType,
    pub tolerance: f64,
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
            kind,
            tolerance,
            geometry: LineRendererData {
                items: data.items[from..to]
                    .iter()
                    .map(|p| LinePoint {
                        color: String::new(),
                        index: p.index,
                        x: p.x,
                        y: p.y,
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
        }
    }
    fn hit(&self, x: f64, y: f64) -> Option<(ChartHit, InternalHitTestCandidate)> {
        let data = &self.geometry;
        let (distance, priority) = line_hit(
            &data.items,
            data.visible_range,
            x,
            y,
            self.kind,
            if data.line_type.is_some() {
                f64::from(data.line_width.pixels())
            } else {
                1.
            },
            data.point_markers_radius,
            data.bar_width,
            self.tolerance,
        )?;
        Some((
            ChartHit {
                series: self.series,
                target: if priority == HitTestPriority::Point {
                    HitTarget::SeriesPoint
                } else {
                    HitTarget::SeriesLine
                },
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
