use crate::ui::chart_layout::LayoutRect;
use iced::{Point, mouse, widget::canvas::Event};

#[derive(Clone, Copy, Debug)]
pub enum Navigation {
    Wheel { x: f64, zoom: f64, scroll: f64 },
    Scroll { start: Option<f64>, x: f64 },
    EndScroll,
}
#[derive(Clone, Copy, Debug)]
pub struct ChartInteraction {
    pub position: Option<Point>,
    pub navigation: Option<Navigation>,
    pub capture: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct MouseNavigationOptions {
    pub wheel_zoom: bool,
    pub wheel_scroll: bool,
    pub pressed_mouse_move: bool,
}
impl Default for MouseNavigationOptions {
    fn default() -> Self {
        Self {
            wheel_zoom: true,
            wheel_scroll: true,
            pressed_mouse_move: true,
        }
    }
}
#[derive(Default)]
pub struct ChartInput {
    pub options: MouseNavigationOptions,
    pressed: Option<Point>,
    scrolling: bool,
    plot: Option<LayoutRect>,
    last_cursor: Option<Point>,
}
impl ChartInput {
    pub fn update(
        &mut self,
        event: &Event,
        cursor: Option<Point>,
        plot: LayoutRect,
        enabled: bool,
    ) -> Option<ChartInteraction> {
        let local = cursor
            .filter(|p| p.x.is_finite() && p.y.is_finite())
            .map(|p| Point::new(p.x - plot.origin.x as f32, p.y - plot.origin.y as f32));
        let position = local.filter(|p| {
            p.x >= 0.
                && p.y >= 0.
                && f64::from(p.x) < plot.size.width
                && f64::from(p.y) < plot.size.height
        });
        let changed = self.plot.replace(plot).is_some_and(|old| old != plot);
        let leaving = matches!(
            event,
            Event::Mouse(mouse::Event::CursorLeft) | Event::Window(iced::window::Event::Unfocused)
        );
        if changed
            || (!enabled && self.pressed.is_some())
            || (!self.options.pressed_mouse_move && self.pressed.is_some())
            || leaving
        {
            self.pressed = None;
            self.scrolling = false;
            self.last_cursor = if !leaving && enabled { local } else { None };
            return Some(ChartInteraction {
                position: if !leaving && enabled { position } else { None },
                navigation: Some(Navigation::EndScroll),
                capture: false,
            });
        }
        if !enabled {
            return None;
        }
        let mut action = ChartInteraction {
            position,
            navigation: None,
            capture: false,
        };
        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta }) if position.is_some() => {
                let (x, y, factor) = match delta {
                    mouse::ScrollDelta::Lines { x, y } => (*x, *y, 32.),
                    mouse::ScrollDelta::Pixels { x, y } => (*x, *y, 1.),
                };
                if !x.is_finite() || !y.is_finite() || (x == 0. && y == 0.) {
                    return None;
                }
                if (y == 0. || !self.options.wheel_zoom) && (x == 0. || !self.options.wheel_scroll)
                {
                    return None;
                }
                self.pressed = None;
                self.scrolling = false;
                action.navigation = Some(Navigation::Wheel {
                    x: f64::from(position.unwrap().x),
                    zoom: if self.options.wheel_zoom {
                        (f64::from(y) * factor / 100.).clamp(-1., 1.)
                    } else {
                        0.
                    },
                    scroll: if self.options.wheel_scroll {
                        f64::from(x) * factor * 0.8
                    } else {
                        0.
                    },
                });
                action.capture = true;
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if position.is_some() && self.options.pressed_mouse_move =>
            {
                self.pressed = local;
                self.scrolling = false;
                action.capture = true;
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                // Native cursor dispatch can repeat the same position after a redraw.
                if self.last_cursor == local {
                    return None;
                }
                if let (Some(start), Some(point)) = (self.pressed, local) {
                    if self.scrolling || (point.x - start.x).abs() + (point.y - start.y).abs() >= 5.
                    {
                        action.navigation = Some(Navigation::Scroll {
                            start: (!self.scrolling).then_some(f64::from(point.x)),
                            x: f64::from(point.x),
                        });
                        self.scrolling = true;
                    }
                    action.capture = true;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if self.pressed.take().is_some() =>
            {
                self.scrolling = false;
                action.navigation = Some(Navigation::EndScroll);
                action.capture = true;
            }
            _ => return None,
        }
        self.last_cursor = local;
        Some(action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::{Point, mouse, widget::canvas::Event};

    fn plot() -> crate::ui::chart_layout::LayoutRect {
        crate::ui::chart_layout::LayoutRect {
            origin: crate::renderers::line_renderer::Point { x: 80., y: 0. },
            size: crate::renderers::price_axis_renderer::AxisBounds {
                width: 600.,
                height: 300.,
            },
        }
    }
    fn event(e: mouse::Event) -> Event {
        Event::Mouse(e)
    }
    #[test]
    fn wheel_units_and_axis_exclusion() {
        let mut state = ChartInput::default();
        let wheel = event(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0., y: 1. },
        });
        assert!(
            state
                .update(&wheel, Some(Point::new(40., 50.)), plot(), true)
                .is_none()
        );
        let action = state
            .update(&wheel, Some(Point::new(180., 50.)), plot(), true)
            .unwrap();
        assert_eq!(action.position, Some(Point::new(100., 50.)));
        assert!(
            matches!(action.navigation, Some(Navigation::Wheel { zoom, .. }) if (zoom-0.32).abs()<1e-6)
        );
    }
    #[test]
    fn empty_redraws_do_not_publish_forever() {
        let mut state = ChartInput::default();
        let redraw = Event::Window(iced::window::Event::RedrawRequested(
            std::time::Instant::now(),
        ));
        assert!(state.update(&redraw, None, plot(), false).is_none());
        assert!(state.update(&redraw, None, plot(), false).is_none());
    }
    #[test]
    fn stationary_cursor_replay_settles_after_enter_move_and_drag() {
        let mut state = ChartInput::default();
        let movement = event(mouse::Event::CursorMoved {
            position: Point::ORIGIN,
        });
        let point = Some(Point::new(180., 50.));
        assert!(state.update(&movement, point, plot(), true).is_some());
        for _ in 0..10 {
            assert!(
                state.update(&movement, point, plot(), true).is_none(),
                "Repeated CursorMoved events must not publish another message for unchanged input"
            );
        }
        let press = event(mouse::Event::ButtonPressed(mouse::Button::Left));
        state.update(&press, point, plot(), true).unwrap();
        let moved = Some(Point::new(200., 50.));
        assert!(state.update(&movement, moved, plot(), true).is_some());
        assert!(state.update(&movement, moved, plot(), true).is_none());
        let outside = Some(Point::new(750., 50.));
        assert!(state.update(&movement, outside, plot(), true).is_some());
        assert!(state.update(&movement, outside, plot(), true).is_none());
        let outside_moved = Some(Point::new(760., 50.));
        assert!(
            state
                .update(&movement, outside_moved, plot(), true)
                .is_some(),
            "a drag still tracks changing coordinates outside the plot"
        );
        state
            .update(&event(mouse::Event::CursorLeft), None, plot(), true)
            .unwrap();
        assert!(
            state.update(&movement, point, plot(), true).is_some(),
            "re-entry at the previous position restores the crosshair"
        );
    }
    #[test]
    fn independent_wheel_options_and_focus_loss_during_resize() {
        let mut state = ChartInput::default();
        state.options.wheel_zoom = false;
        let wheel = event(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 10., y: 20. },
        });
        let action = state
            .update(&wheel, Some(Point::new(180., 50.)), plot(), true)
            .unwrap();
        assert!(matches!(
            action.navigation,
            Some(Navigation::Wheel {
                zoom: 0.,
                scroll: 8.,
                ..
            })
        ));
        state.options.wheel_scroll = false;
        assert!(
            state
                .update(&wheel, Some(Point::new(180., 50.)), plot(), true)
                .is_none()
        );
        let mut resized = plot();
        resized.size.width = 500.;
        let action = state
            .update(
                &Event::Window(iced::window::Event::Unfocused),
                Some(Point::new(180., 50.)),
                resized,
                true,
            )
            .unwrap();
        assert!(action.position.is_none());
        assert!(matches!(action.navigation, Some(Navigation::EndScroll)));
    }
    #[test]
    fn drag_threshold_outside_release_and_cancellation() {
        let mut state = ChartInput::default();
        state.update(
            &event(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Some(Point::new(180., 50.)),
            plot(),
            true,
        );
        let movement = event(mouse::Event::CursorMoved {
            position: Point::ORIGIN,
        });
        assert!(
            state
                .update(&movement, Some(Point::new(183., 50.)), plot(), true)
                .unwrap()
                .navigation
                .is_none()
        );
        let start = state
            .update(&movement, Some(Point::new(186., 50.)), plot(), true)
            .unwrap();
        assert!(matches!(
            start.navigation,
            Some(Navigation::Scroll { start: Some(_), .. })
        ));
        let outside = state
            .update(&movement, Some(Point::new(700., 50.)), plot(), true)
            .unwrap();
        assert!(outside.position.is_none());
        assert!(matches!(
            outside.navigation,
            Some(Navigation::Scroll { .. })
        ));
        let release = state
            .update(
                &event(mouse::Event::ButtonReleased(mouse::Button::Left)),
                None,
                plot(),
                true,
            )
            .unwrap();
        assert!(matches!(release.navigation, Some(Navigation::EndScroll)));
        assert!(
            state
                .update(&event(mouse::Event::CursorLeft), None, plot(), true)
                .is_some()
        );
        assert!(
            state
                .update(&movement, Some(Point::new(200., 50.)), plot(), true)
                .unwrap()
                .navigation
                .is_none()
        );
    }
}
