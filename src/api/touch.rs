//! Public synchronous gesture messages; native finger recognition belongs to Canvas.
use iced::Point;
use std::time::Instant;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TrackingExitMode {
    #[default]
    OnNextTap,
    OnTouchEnd,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TouchNavigationOptions {
    pub horizontal_drag: bool,
    pub vertical_drag: bool,
    pub pinch: bool,
    pub tracking_exit: TrackingExitMode,
}
impl Default for TouchNavigationOptions {
    fn default() -> Self {
        Self {
            horizontal_drag: true,
            vertical_drag: true,
            pinch: true,
            tracking_exit: TrackingExitMode::OnNextTap,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KineticScrollOptions {
    pub mouse: bool,
    pub touch: bool,
}
impl Default for KineticScrollOptions {
    fn default() -> Self {
        Self {
            mouse: false,
            touch: true,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct TouchInteraction {
    pub pane: usize,
    pub now: Instant,
    pub action: TouchAction,
}
#[derive(Clone, Copy, Debug)]
pub enum TouchAction {
    Press,
    Pan {
        start: Option<Point>,
        position: Point,
    },
    Pinch {
        anchor: Point,
        zoom: f64,
    },
    Track {
        position: Point,
    },
    Tap {
        position: Point,
    },
    DoubleTap {
        position: Point,
    },
    End,
    ExitTracking,
    Cancel,
}
impl<B: super::ChartBehavior> super::Chart<B> {
    pub fn touch_navigation_options(&self) -> TouchNavigationOptions {
        self.touch_options
    }
    pub fn kinetic_scroll_options(&self) -> KineticScrollOptions {
        self.kinetic_options
    }
    pub fn set_touch_navigation_options(&mut self, options: TouchNavigationOptions) {
        if self.touch_options != options {
            self.cancel_interaction();
            self.touch_options = options;
        }
    }
    pub fn set_kinetic_scroll_options(&mut self, options: KineticScrollOptions) {
        if self.kinetic_options != options {
            self.cancel_interaction();
            self.kinetic_options = options;
        }
    }
}
