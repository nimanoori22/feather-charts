//! Filtering owned geometry leaves the ordinary screen draw path unchanged.
use super::{multi_pane_frame::IcedMultiPaneChartFrame, price_axis::AxisPrimitive};
use crate::api::{PrimitiveLayer, ScreenshotLayers};
use crate::model::series_markers::MarkerZOrder;

#[derive(Clone, Copy, Debug)]
pub(crate) enum ContentLayer {
    Base,
    Top,
    Crosshair,
}
impl ContentLayer {
    fn included(self, layers: ScreenshotLayers) -> bool {
        match self {
            Self::Base => true,
            Self::Top => layers.include_top_layer,
            Self::Crosshair => layers.include_top_layer && layers.include_crosshair,
        }
    }
}
fn filter_axis(
    primitives: &mut Vec<AxisPrimitive>,
    content: &mut Vec<ContentLayer>,
    layers: ScreenshotLayers,
) {
    let mut index = 0;
    primitives.retain(|_| {
        let keep = content
            .get(index)
            .copied()
            .expect("prepared axis layer metadata")
            .included(layers);
        index += 1;
        keep
    });
    content.retain(|layer| layer.included(layers));
}
impl IcedMultiPaneChartFrame {
    pub(crate) fn filter_screenshot(&mut self, layers: ScreenshotLayers) {
        self.attribution = None;
        for pane in &mut self.panes {
            let plot = &mut pane.plot;
            if !layers.include_top_layer || !layers.include_crosshair {
                plot.crosshair.clear();
                plot.crosshair_markers.clear();
            }
            if !layers.include_top_layer {
                plot.last_price_animations.clear();
                plot.annotation_geometry
                    .retain(|a| a.z_order != MarkerZOrder::Top);
                for a in plot.primitives.iter_mut().chain(&mut plot.time_primitives) {
                    a.output.views.retain(|v| v.layer != PrimitiveLayer::Top);
                    a.output
                        .price_views
                        .retain(|v| v.layer != PrimitiveLayer::Top);
                    a.output
                        .time_views
                        .retain(|v| v.layer != PrimitiveLayer::Top);
                }
            }
            for axis in [&mut pane.prices.left, &mut pane.prices.right] {
                filter_axis(&mut axis.primitives, &mut axis.screenshot_layers, layers);
            }
            filter_axis(
                &mut pane.time.primitives,
                &mut pane.time.screenshot_layers,
                layers,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{
        chart_frame::IcedChartFrame, line_chart::CanvasStroke,
        series_annotations::CanvasAnnotations,
    };
    #[test]
    fn base_annotations_survive_crosshair_and_top_filtering() {
        let mut source = IcedMultiPaneChartFrame::default();
        let mut pane = IcedChartFrame::default();
        pane.plot.crosshair.push(CanvasStroke {
            path: vec![],
            color: iced::Color::BLACK,
            width: 1.,
        });
        for z_order in [
            MarkerZOrder::Normal,
            MarkerZOrder::AboveSeries,
            MarkerZOrder::Top,
        ] {
            pane.plot.annotation_geometry.push(CanvasAnnotations {
                series_layer: 0,
                z_order,
                paths: vec![],
            });
        }
        source.panes.push(pane);
        for (top, crosshair) in [(false, false), (false, true), (true, false), (true, true)] {
            let mut frame = source.clone();
            frame.filter_screenshot(ScreenshotLayers {
                include_top_layer: top,
                include_crosshair: crosshair,
            });
            assert_eq!(
                frame.panes[0].plot.crosshair.len(),
                usize::from(top && crosshair)
            );
            assert_eq!(
                frame.panes[0].plot.annotation_geometry.len(),
                if top { 3 } else { 2 }
            );
            assert_eq!(source.panes[0].plot.annotation_geometry.len(), 3);
        }
    }
    #[test]
    fn price_and_time_crosshair_labels_require_both_flags() {
        use crate::{
            model::axis_snapshots::{FloatingAxisLabel, PriceAxisSide},
            renderers::{grid_renderer::PixelRatio, price_axis_renderer::*, time_axis_renderer::*},
            ui::{
                price_axis::{AxisFontResolver, IcedPriceAxis},
                time_axis::IcedTimeAxis,
            },
        };
        let fonts = AxisFontResolver::default();
        let label = FloatingAxisLabel {
            coordinate: 40.,
            text: "Crosshair".into(),
            background: "red".into(),
            color: "white".into(),
        };
        let mut price =
            crate::renderers::price_axis_renderer::tests::snapshot(PriceAxisSide::Right);
        price.crosshair_label = Some(label.clone());
        let ratio = PixelRatio {
            horizontal: 1.25,
            vertical: 1.25,
        };
        let bounds = AxisBounds {
            width: 120.,
            height: 100.,
        };
        let mut measurer =
            crate::ui::price_axis::IcedAxisTextMeasurer::with_font(iced::Font::DEFAULT, 12.)
                .unwrap();
        let price = prepare_price_axis(
            &measure_price_axis(&price, &mut measurer).unwrap(),
            bounds,
            ratio,
        )
        .unwrap();
        assert!(!price.screenshot_crosshair.is_empty());
        assert!(
            price
                .screenshot_crosshair
                .clone()
                .all(|i| !matches!(price.commands[i], PriceAxisCommand::Background(_)))
        );
        let mut time = crate::renderers::time_axis_renderer::tests::snapshot();
        time.crosshair_label = Some(label);
        time.primitive_labels
            .push(crate::model::axis_snapshots::PrimitiveAxisLabel::new(
                20.,
                "Primitive",
                "blue",
                "white",
            ));
        let time = prepare_time_axis(
            &measure_time_axis(
                &time,
                &mut measurer,
                &mut crate::ui::price_axis::IcedAxisTextMeasurer::with_font(
                    iced::Font::DEFAULT,
                    12.,
                )
                .unwrap(),
            )
            .unwrap(),
            bounds,
            ratio,
        )
        .unwrap();
        assert!(time.screenshot_top_labels.start < time.screenshot_crosshair.start);
        let mut frame = IcedMultiPaneChartFrame::default();
        let mut pane = IcedChartFrame::default();
        pane.prices.right = IcedPriceAxis::from_commands(&price, &fonts).unwrap();
        pane.time = IcedTimeAxis::from_commands(&time, &fonts).unwrap();
        frame.panes.push(pane);
        let counts = |frame: &IcedMultiPaneChartFrame| {
            (
                frame.panes[0].prices.right.primitives.len(),
                frame.panes[0].time.primitives.len(),
            )
        };
        let full = counts(&frame);
        let mut top = frame.clone();
        top.filter_screenshot(ScreenshotLayers {
            include_top_layer: true,
            include_crosshair: false,
        });
        let mut base = frame.clone();
        base.filter_screenshot(Default::default());
        assert!(counts(&base).0 < full.0);
        assert_eq!(counts(&base).0, counts(&top).0);
        assert!(counts(&base).1 < counts(&top).1);
        assert!(counts(&top).1 < full.1);
    }
}
