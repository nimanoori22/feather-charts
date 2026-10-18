use super::{Chart, ChartBehavior, ChartError, KineticScrollOptions, TouchNavigationOptions};
pub use crate::model::chart_configuration::{
    ChartModelOptionsPatch, GridLineOptionsPatch, GridOptionsPatch, LayoutAppearance,
    LayoutOptionsPatch, PaneLayoutOptionsPatch, RuntimeModelOptions,
};
pub use crate::ui::chart_input::MouseNavigationOptions;

/// Model presentation patches and component input settings are validated together.
#[derive(Clone, Debug, Default)]
pub struct ChartOptionsPatch {
    pub model: ChartModelOptionsPatch,
    pub mouse_navigation: Option<MouseNavigationOptions>,
    pub touch_navigation: Option<TouchNavigationOptions>,
    pub kinetic_scroll: Option<KineticScrollOptions>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeChartOptions {
    pub model: RuntimeModelOptions,
    pub mouse_navigation: MouseNavigationOptions,
    pub touch_navigation: TouchNavigationOptions,
    pub kinetic_scroll: KineticScrollOptions,
}
impl<B: ChartBehavior> Chart<B> {
    pub fn runtime_options(&self) -> RuntimeChartOptions {
        RuntimeChartOptions {
            model: self.model.runtime_options(),
            mouse_navigation: self.mouse_options,
            touch_navigation: self.touch_options,
            kinetic_scroll: self.kinetic_options,
        }
    }
    pub fn apply_chart_options(&mut self, patch: ChartOptionsPatch) -> Result<(), ChartError> {
        let (next, model_changed, geometry) = self.model.preview_runtime_options(&patch.model)?;
        let mouse = patch.mouse_navigation.unwrap_or(self.mouse_options);
        let touch = patch.touch_navigation.unwrap_or(self.touch_options);
        let kinetic = patch.kinetic_scroll.unwrap_or(self.kinetic_options);
        let input_changed = mouse != self.mouse_options
            || touch != self.touch_options
            || kinetic != self.kinetic_options;
        if !model_changed && !input_changed {
            return Ok(());
        }
        if geometry || input_changed {
            self.cancel_viewport_interaction();
            self.revision = self.revision.wrapping_add(1);
        }
        self.mouse_options = mouse;
        self.touch_options = touch;
        self.kinetic_options = kinetic;
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
    pub fn mouse_navigation_options(&self) -> MouseNavigationOptions {
        self.mouse_options
    }
    pub fn set_mouse_navigation_options(&mut self, options: MouseNavigationOptions) {
        if options != self.mouse_options {
            self.cancel_viewport_interaction();
            self.mouse_options = options;
            self.revision = self.revision.wrapping_add(1);
        }
    }
}
