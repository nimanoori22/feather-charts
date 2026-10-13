//! Temporary source order borrows stable frame geometry; filled sources split passes.
use super::PlotSnapshot;
use iced::widget::canvas::Frame;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SeriesPassKind {
    Normal,
    Background,
    Foreground,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SeriesDrawPass {
    pub layer: usize,
    pub kind: SeriesPassKind,
}
impl PlotSnapshot {
    pub fn series_draw_passes(&self) -> impl Iterator<Item = SeriesDrawPass> + '_ {
        let raised = self.hovered_series.filter(|_| self.hovered_series_on_top);
        let layer_id = |i: usize| {
            if i == 0 {
                self.series_id
            } else {
                self.series_layers[i - 1].series_id
            }
        };
        let raised_layer =
            raised.and_then(|id| (0..=self.series_layers.len()).find(|&i| layer_id(i) == Some(id)));
        let filled =
            raised.is_some_and(|id| self.hit_series.iter().any(|h| h.series == id && h.filled));
        (0..=self.series_layers.len())
            .filter(move |&i| filled || Some(i) != raised_layer)
            .map(move |layer| SeriesDrawPass {
                layer,
                kind: if Some(layer) == raised_layer {
                    SeriesPassKind::Background
                } else {
                    SeriesPassKind::Normal
                },
            })
            .chain(raised_layer.map(move |layer| SeriesDrawPass {
                layer,
                kind: if filled {
                    SeriesPassKind::Foreground
                } else {
                    SeriesPassKind::Normal
                },
            }))
    }
    pub(super) fn draw_series(&self, frame: &mut Frame) {
        for pass in self.series_draw_passes() {
            let (id, fills, lines, gradient_lines, candles, markers, gradient_markers) =
                if pass.layer == 0 {
                    (
                        self.series_id,
                        &self.fills,
                        &self.lines,
                        &self.gradient_lines,
                        &self.candles,
                        &self.markers,
                        &self.gradient_markers,
                    )
                } else {
                    let l = &self.series_layers[pass.layer - 1];
                    (
                        l.series_id,
                        &l.fills,
                        &l.lines,
                        &l.gradient_lines,
                        &l.candles,
                        &l.markers,
                        &l.gradient_markers,
                    )
                };
            if pass.kind != SeriesPassKind::Foreground {
                self.draw_series_price_lines(frame, id, true);
                self.draw_fills(frame, fills);
            }
            if pass.kind != SeriesPassKind::Background {
                self.draw_strokes(frame, lines.iter());
                self.draw_gradient_strokes(frame, gradient_lines);
                for rect in candles {
                    frame.fill_rectangle(rect.origin, rect.size, rect.color);
                }
                self.draw_markers(frame, markers);
                self.draw_gradient_markers(frame, gradient_markers);
                self.draw_series_price_lines(frame, id, false);
            }
            if pass.kind != SeriesPassKind::Foreground {
                self.draw_annotations(
                    frame,
                    crate::model::series_markers::MarkerZOrder::Normal,
                    Some(pass.layer),
                );
            }
        }
    }
}
