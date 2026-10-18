/// Source canvas selection. Crosshair inclusion requires the top canvas too.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScreenshotLayers {
    pub include_top_layer: bool,
    pub include_crosshair: bool,
}

use super::{Chart, ChartBehavior, ChartError};
use crate::ui::multi_pane_frame::IcedMultiPaneChartFrame;
use std::time::Instant;

#[derive(Debug)]
pub enum ScreenshotError {
    NoPreparedFrame,
    InvalidRasterOptions,
    BackendUnavailable,
    InvalidReadback,
}
impl std::fmt::Display for ScreenshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ScreenshotError {}

/// Owned geometry. Custom renderers obey the same frozen-view contract as chart frames.
#[derive(Clone, Debug)]
pub struct ChartScreenshot {
    pub(crate) frame: IcedMultiPaneChartFrame,
    layers: ScreenshotLayers,
}
impl ChartScreenshot {
    pub fn logical_size(&self) -> iced::Size {
        let size = self.frame.layout.chart.chart_size;
        iced::Size::new(size.width as f32, size.height as f32)
    }
    pub fn draw(&self, frame: &mut iced::widget::canvas::Frame) {
        self.frame.draw(frame);
        if self.layers.include_top_layer {
            // Source labelPaneViews are painted on both canvases, above top views.
            for pane in &self.frame.panes {
                pane.pane_labels.left.draw_at(frame, pane.layout.plot);
                pane.pane_labels.right.draw_at(frame, pane.layout.plot);
            }
        }
    }
}
impl<B: ChartBehavior> Chart<B> {
    /// Captures the last successful displayed frame, even if model updates are pending.
    /// Does not settle data, sample animations, change options or consume host events.
    pub fn screenshot_of_prepared(
        &self,
        layers: ScreenshotLayers,
    ) -> Result<ChartScreenshot, ScreenshotError> {
        if !self.displayed_frame_valid {
            return Err(ScreenshotError::NoPreparedFrame);
        }
        let mut frame = self.frame.clone();
        frame.filter_screenshot(layers);
        Ok(ChartScreenshot { frame, layers })
    }
    /// Prepares the current viewport through the ordinary display path, then captures it.
    /// Preparation may emit normal range/size events; they remain queued for the host.
    pub fn prepare_screenshot(
        &mut self,
        layers: ScreenshotLayers,
        now: Instant,
    ) -> Result<ChartScreenshot, ChartError> {
        if !self.displayed_frame_valid {
            return Err(ChartError::Screenshot(ScreenshotError::NoPreparedFrame));
        }
        self.prepare(self.size, self.ratio, now)?;
        self.screenshot_of_prepared(layers)
            .map_err(ChartError::Screenshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::{ChartExporter, ScreenshotBackend},
        renderers::{
            line_renderer::Point,
            price_axis_renderer::{AxisBounds, AxisRect, PreparedPriceAxis, PriceAxisCommand},
        },
        ui::price_axis::{AxisFontResolver, IcedPriceAxis},
    };
    #[test]
    fn pane_labels_are_present_on_both_source_canvases() {
        let mut chart = Chart::default();
        chart
            .prepare(iced::Size::new(300., 200.), 1., Instant::now())
            .unwrap();
        let mut screenshot = chart.screenshot_of_prepared(Default::default()).unwrap();
        let plot = screenshot.frame.panes[0].layout.plot;
        screenshot.frame.panes[0].pane_labels.right = IcedPriceAxis::from_commands(
            &PreparedPriceAxis {
                bounds: plot.size,
                commands: vec![PriceAxisCommand::Rectangle {
                    rect: AxisRect {
                        origin: Point { x: 10., y: 10. },
                        size: AxisBounds {
                            width: 20.,
                            height: 20.,
                        },
                    },
                    color: "rgba(255,0,0,0.5)".into(),
                }],
                ..Default::default()
            },
            &AxisFontResolver::default(),
        )
        .unwrap();
        let mut exporter =
            iced::futures::executor::block_on(ChartExporter::new(ScreenshotBackend::Software))
                .unwrap();
        let base = exporter.render(&screenshot, Default::default()).unwrap();
        screenshot.layers.include_top_layer = true;
        let top = exporter.render(&screenshot, Default::default()).unwrap();
        let index = ((plot.origin.y as usize + 20) * 300 + plot.origin.x as usize + 20) * 4;
        assert!(
            top.rgba()[index + 1] < base.rgba()[index + 1],
            "top canvas must repaint pane labels"
        );
    }
}
