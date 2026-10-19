use super::{Chart, ChartBehavior, ChartError, KineticScrollOptions, TouchNavigationOptions};
pub use crate::model::chart_configuration::{
    ChartModelOptionsPatch, GridLineOptionsPatch, GridOptionsPatch, LayoutAppearance,
    LayoutOptionsPatch, PaneLayoutOptionsPatch, RuntimeModelOptions,
};
pub use crate::model::localization_options::LocalizationOptionsPatch;
pub use crate::model::pane::DefaultVisiblePriceScale;
pub use crate::ui::chart_input::MouseNavigationOptions;

/// Independent axis dragging and double-click/double-tap reset settings.
/// Plot wheel/pinch and the native price-axis wheel extension are separate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AxisInteractionOptions {
    pub time_drag: bool,
    pub time_reset: bool,
    pub price_drag: bool,
    pub price_reset: bool,
}
impl Default for AxisInteractionOptions {
    fn default() -> Self {
        Self {
            time_drag: true,
            time_reset: true,
            price_drag: true,
            price_reset: true,
        }
    }
}
/// Model presentation patches and component input settings are validated together.
#[derive(Clone, Debug, Default)]
pub struct ChartOptionsPatch {
    pub model: ChartModelOptionsPatch,
    pub axis_interaction: Option<AxisInteractionOptions>,
    pub mouse_navigation: Option<MouseNavigationOptions>,
    pub touch_navigation: Option<TouchNavigationOptions>,
    pub kinetic_scroll: Option<KineticScrollOptions>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeChartOptions {
    pub model: RuntimeModelOptions,
    pub axis_interaction: AxisInteractionOptions,
    pub mouse_navigation: MouseNavigationOptions,
    pub touch_navigation: TouchNavigationOptions,
    pub kinetic_scroll: KineticScrollOptions,
}
impl<B: ChartBehavior, M: Clone + 'static> Chart<B, M> {
    pub fn runtime_options(&self) -> RuntimeChartOptions {
        RuntimeChartOptions {
            model: self.model.runtime_options(),
            axis_interaction: self.axis_options,
            mouse_navigation: self.mouse_options,
            touch_navigation: self.touch_options,
            kinetic_scroll: self.kinetic_options,
        }
    }
    pub fn apply_chart_options(&mut self, patch: ChartOptionsPatch) -> Result<(), ChartError> {
        let (next, model_changed, geometry) = self.model.preview_runtime_options(&patch.model)?;
        let axes = patch.axis_interaction.unwrap_or(self.axis_options);
        let axes_changed = axes != self.axis_options;
        let mouse = patch.mouse_navigation.unwrap_or(self.mouse_options);
        let touch = patch.touch_navigation.unwrap_or(self.touch_options);
        let kinetic = patch.kinetic_scroll.unwrap_or(self.kinetic_options);
        let input_changed = mouse != self.mouse_options
            || touch != self.touch_options
            || kinetic != self.kinetic_options;
        if !model_changed && !input_changed && !axes_changed {
            return Ok(());
        }
        if geometry || input_changed {
            self.cancel_viewport_interaction();
            self.revision = self.revision.wrapping_add(1);
        }
        self.set_axis_interaction_options(axes);
        self.mouse_options = mouse;
        self.touch_options = touch;
        self.kinetic_options = kinetic;
        self.sync_horizontal_interactions();
        if model_changed {
            self.background = match &next.layout.background {
                crate::model::layout_options::Background::Solid { color } => color.clone(),
                crate::model::layout_options::Background::VerticalGradient {
                    top_color, ..
                } => top_color.clone(),
            };
            self.separator = next.layout.panes.separator_color.clone();
            self.separator_hover = next.layout.panes.separator_hover_color.clone();
            self.resize_panes = next.layout.panes.enable_resize;
            self.model
                .apply_runtime_options(&mut self.layer, &patch.model, next);
        }
        Ok(())
    }
    pub(super) fn sync_horizontal_interactions(&mut self) {
        let a = self.axis_options;
        let m = self.mouse_options;
        let t = self.touch_options;
        self.model.set_time_interactions_enabled(
            a.time_drag
                || a.time_reset
                || m.wheel_zoom
                || m.wheel_scroll
                || m.pressed_mouse_move
                || t.horizontal_drag
                || t.vertical_drag
                || t.pinch,
        );
    }
    pub fn axis_interaction_options(&self) -> AxisInteractionOptions {
        self.axis_options
    }
    pub fn set_axis_interaction_options(&mut self, options: AxisInteractionOptions) {
        if options == self.axis_options {
            return;
        }
        if !options.time_drag {
            self.cancel_time_axis();
        }
        if !options.price_drag {
            self.cancel_price_axis();
        }
        self.axis_options = options;
        self.sync_horizontal_interactions();
    }
    pub fn mouse_navigation_options(&self) -> MouseNavigationOptions {
        self.mouse_options
    }
    pub fn set_mouse_navigation_options(&mut self, options: MouseNavigationOptions) {
        if options != self.mouse_options {
            self.cancel_viewport_interaction();
            self.mouse_options = options;
            self.sync_horizontal_interactions();
            self.revision = self.revision.wrapping_add(1);
        }
    }
}

impl<B: ChartBehavior, M: Clone + 'static> Chart<B, M> {
    /// Detached configuration readback. Clones strings and Rc callback handles only.
    pub fn localization_options(
        &self,
    ) -> crate::model::localization_options::LocalizationOptions<B::Item> {
        self.model.time_scale().localization().clone()
    }
    /// Refreshes price, percentage and horizontal formatters together. Empty/equal
    /// patches are inert; changing labels preserves data, queued events and gestures.
    /// Callback equality uses Rc identity. Unknown locale strings keep behavior-specific fallback.
    pub fn apply_localization_options(&mut self, patch: LocalizationOptionsPatch<B::Item>) {
        let mut next = self.localization_options();
        if !patch.apply_to(&mut next) {
            return;
        }
        self.model.set_localization_options(&mut self.layer, next);
        self.revision = self.revision.wrapping_add(1);
    }
}

#[cfg(test)]
mod axis_tests {
    use super::*;
    use crate::model::{
        data_consumer::*, horz_scale_behavior_time::types::UtcTimestamp, pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
    };
    #[test]
    fn disabling_all_navigation_aligns_edge_ticks_and_reenabling_axes_invalidates_them() {
        let mut c = Chart::default();
        let id = c
            .add_series(
                SeriesOptionsMap::Line(Default::default()),
                0,
                PriceScalePosition::Right,
            )
            .unwrap();
        c.set_data(
            id,
            (1..=100)
                .map(|i| {
                    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                        time: UtcTimestamp::new(i as f64).into(),
                        value: i as f64,
                        color: None,
                        custom_values: None,
                    }))
                })
                .collect(),
        )
        .unwrap();
        c.fit_content();
        c.prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
            .unwrap();
        let before = c.model.prepare_axis_snapshots(0).unwrap().time;
        assert!(!before.ticks.is_empty());
        assert!(before.ticks.iter().all(|t| !t.mark.need_align_coordinate));
        c.apply_chart_options(ChartOptionsPatch {
            axis_interaction: Some(AxisInteractionOptions {
                time_drag: false,
                time_reset: false,
                price_drag: false,
                price_reset: false,
            }),
            mouse_navigation: Some(MouseNavigationOptions {
                wheel_zoom: false,
                wheel_scroll: false,
                pressed_mouse_move: false,
            }),
            touch_navigation: Some(TouchNavigationOptions {
                horizontal_drag: false,
                vertical_drag: false,
                pinch: false,
                ..Default::default()
            }),
            ..Default::default()
        })
        .unwrap();
        let static_axis = c.model.prepare_axis_snapshots(0).unwrap().time;
        assert!(
            static_axis
                .ticks
                .iter()
                .any(|t| t.mark.need_align_coordinate)
        );
        c.set_axis_interaction_options(AxisInteractionOptions {
            time_reset: true,
            ..c.axis_interaction_options()
        });
        assert!(
            c.model
                .prepare_axis_snapshots(0)
                .unwrap()
                .time
                .ticks
                .iter()
                .all(|t| !t.mark.need_align_coordinate)
        );
        c.set_axis_interaction_options(AxisInteractionOptions {
            time_reset: false,
            ..c.axis_interaction_options()
        });
        assert!(
            c.model
                .prepare_axis_snapshots(0)
                .unwrap()
                .time
                .ticks
                .iter()
                .any(|t| t.mark.need_align_coordinate)
        );
        c.set_mouse_navigation_options(MouseNavigationOptions {
            wheel_zoom: true,
            wheel_scroll: false,
            pressed_mouse_move: false,
        });
        assert!(
            c.model
                .prepare_axis_snapshots(0)
                .unwrap()
                .time
                .ticks
                .iter()
                .all(|t| !t.mark.need_align_coordinate)
        );
    }
}
