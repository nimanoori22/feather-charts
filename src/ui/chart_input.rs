use crate::ui::chart_layout::LayoutRect;
use iced::{Point, mouse, widget::canvas::Event};

#[derive(Clone, Copy, Debug)]
pub enum Navigation {
    Wheel {
        x: f64,
        zoom: f64,
        scroll: f64,
    },
    Scroll {
        start: Option<f64>,
        x: f64,
    },
    Pan {
        start: Option<Point>,
        position: Point,
    },
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
    gesture_origin: Option<Point>,
    plot: Option<LayoutRect>,
    last_cursor: Option<Point>,
}
impl ChartInput {
    pub fn is_dragging(&self) -> bool {
        self.pressed.is_some()
    }
    pub fn update(
        &mut self,
        event: &Event,
        cursor: Option<Point>,
        plot: LayoutRect,
        enabled: bool,
    ) -> Option<ChartInteraction> {
        let cursor = cursor.filter(|p| p.x.is_finite() && p.y.is_finite());
        let local =
            cursor.map(|p| Point::new(p.x - plot.origin.x as f32, p.y - plot.origin.y as f32));
        let position = local.filter(|p| {
            p.x >= 0.
                && p.y >= 0.
                && f64::from(p.x) < plot.size.width
                && f64::from(p.y) < plot.size.height
        });
        let changed = self.plot.replace(plot).is_some_and(|old| {
            if self.pressed.is_some() {
                old.origin.y != plot.origin.y || old.size.height != plot.size.height
            } else {
                old != plot
            }
        });
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
            self.gesture_origin = None;
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
                // Native line units represent wheel steps; pixels retain source sensitivity.
                let (x, y, zoom_factor, scroll_factor) = match delta {
                    mouse::ScrollDelta::Lines { x, y } => (*x, *y, 100., 32.),
                    mouse::ScrollDelta::Pixels { x, y } => (*x, *y, 1., 1.),
                };
                if !x.is_finite() || !y.is_finite() || (x == 0. && y == 0.) {
                    return None;
                }
                if (y == 0. || !self.options.wheel_zoom) && (x == 0. || !self.options.wheel_scroll)
                {
                    return None;
                }
                self.pressed = None;
                self.gesture_origin = None;
                self.scrolling = false;
                action.navigation = Some(Navigation::Wheel {
                    x: f64::from(position.unwrap().x),
                    zoom: if self.options.wheel_zoom {
                        (f64::from(y) * zoom_factor / 100.).clamp(-1., 1.)
                    } else {
                        0.
                    },
                    scroll: if self.options.wheel_scroll {
                        f64::from(x) * scroll_factor * 0.8
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
                self.gesture_origin = Some(Point::new(plot.origin.x as f32, plot.origin.y as f32));
                self.scrolling = false;
                action.capture = true;
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                // Native cursor dispatch can repeat the same position after a redraw.
                if self.last_cursor == local {
                    return None;
                }
                let drag_point = cursor
                    .zip(self.gesture_origin)
                    .map(|(p, origin)| Point::new(p.x - origin.x, p.y - origin.y));
                if let (Some(start), Some(point)) = (self.pressed, drag_point) {
                    if self.scrolling || (point.x - start.x).abs() + (point.y - start.y).abs() >= 5.
                    {
                        action.navigation = Some(Navigation::Pan {
                            start: (!self.scrolling).then_some(point),
                            position: point,
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
                self.gesture_origin = None;
                action.navigation = Some(Navigation::EndScroll);
                action.capture = true;
            }
            _ => return None,
        }
        self.last_cursor = local;
        Some(action)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum MultiPaneInteraction {
    Plot {
        pane: usize,
        interaction: ChartInteraction,
    },
    Resize {
        top: usize,
        upper_stretch: f64,
        lower_stretch: f64,
    },
    Cancel,
}
impl MultiPaneInteraction {
    pub fn capture(self) -> bool {
        match self {
            Self::Plot { interaction, .. } => interaction.capture,
            Self::Resize { .. } => true,
            Self::Cancel => false,
        }
    }
}
struct DividerDrag {
    top: usize,
    start_y: f32,
    last_y: f32,
    initial_upper: f64,
    total_stretch: f64,
    factor: f64,
    minimum: f64,
}
#[derive(Default)]
pub struct MultiPaneInput {
    pub options: MouseNavigationOptions,
    input: ChartInput,
    selected: Option<usize>,
    gesture: Option<usize>,
    divider: Option<DividerDrag>,
    viewport: Option<(crate::renderers::price_axis_renderer::AxisBounds, usize)>,
    pub hovered_divider: Option<usize>,
}
impl MultiPaneInput {
    pub fn is_dragging(&self) -> bool {
        self.gesture.is_some() || self.divider.is_some()
    }
    pub fn update(
        &mut self,
        event: &Event,
        cursor: Option<Point>,
        layout: &crate::ui::multi_pane_layout::MultiPaneLayout,
        stretches: &[f64],
        enabled: bool,
        resize_enabled: bool,
    ) -> Option<MultiPaneInteraction> {
        let cursor = cursor.filter(|p| p.x.is_finite() && p.y.is_finite());
        let viewport = (layout.chart.available, layout.panes.len());
        let changed = self
            .viewport
            .replace(viewport)
            .is_some_and(|old| old != viewport);
        let leaving = matches!(
            event,
            Event::Mouse(mouse::Event::CursorLeft) | Event::Window(iced::window::Event::Unfocused)
        );
        self.hovered_divider = if resize_enabled {
            cursor.and_then(|p| {
                layout.separators.iter().position(|r| {
                    p.x >= 0.
                        && f64::from(p.x) < r.size.width
                        && f64::from(p.y) >= r.origin.y - 4.
                        && f64::from(p.y) < r.origin.y + 5.
                })
            })
        } else {
            None
        };
        if changed
            || leaving
            || (!enabled && self.gesture.is_some())
            || (!resize_enabled && self.divider.is_some())
        {
            self.gesture = None;
            self.divider = None;
            self.selected = None;
            self.input = ChartInput::default();
            if leaving {
                self.hovered_divider = None;
            }
            return Some(MultiPaneInteraction::Cancel);
        }
        if let Some(drag) = &mut self.divider {
            match event {
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    self.divider = None;
                    return Some(MultiPaneInteraction::Cancel);
                }
                Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                    let y = cursor?.y;
                    if y == drag.last_y {
                        return None;
                    }
                    drag.last_y = y;
                    let upper = (drag.initial_upper + f64::from(y - drag.start_y) * drag.factor)
                        .clamp(drag.minimum, drag.total_stretch - drag.minimum);
                    return Some(MultiPaneInteraction::Resize {
                        top: drag.top,
                        upper_stretch: upper,
                        lower_stretch: drag.total_stretch - upper,
                    });
                }
                _ => return None,
            }
        }
        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        ) && let Some(top) = self.hovered_divider
        {
            let upper = *stretches.get(top)?;
            let lower = *stretches.get(top + 1)?;
            let height =
                layout.panes[top].plot.size.height + layout.panes[top + 1].plot.size.height;
            if height <= 60.
                || !upper.is_finite()
                || !lower.is_finite()
                || upper <= 0.
                || lower <= 0.
            {
                return None;
            }
            let total_stretch = upper + lower;
            let factor = total_stretch / height;
            self.divider = Some(DividerDrag {
                top,
                start_y: cursor?.y,
                last_y: cursor?.y,
                initial_upper: upper,
                total_stretch,
                factor,
                minimum: 30. * factor,
            });
            self.gesture = None;
            self.input = ChartInput::default();
            self.selected = None;
            return Some(MultiPaneInteraction::Resize {
                top,
                upper_stretch: upper,
                lower_stretch: lower,
            });
        }
        let target = self.gesture.or_else(|| {
            cursor.and_then(|p| {
                layout.panes.iter().position(|pane| {
                    let r = pane.plot;
                    f64::from(p.x) >= r.origin.x
                        && f64::from(p.x) < r.origin.x + r.size.width
                        && f64::from(p.y) >= r.origin.y
                        && f64::from(p.y) < r.origin.y + r.size.height
                })
            })
        });
        let Some(pane) = target else {
            if self.selected.take().is_some() {
                self.input = ChartInput::default();
                return Some(MultiPaneInteraction::Cancel);
            }
            return None;
        };
        if self.selected != Some(pane) {
            self.input = ChartInput::default();
            self.selected = Some(pane);
        }
        self.input.options = self.options;
        let interaction =
            self.input
                .update(event, cursor, layout.panes.get(pane)?.plot, enabled)?;
        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        ) && interaction.capture
        {
            self.gesture = Some(pane);
        }
        if matches!(
            interaction.navigation,
            Some(Navigation::EndScroll) | Some(Navigation::Wheel { .. })
        ) {
            self.gesture = None;
        }
        Some(MultiPaneInteraction::Plot { pane, interaction })
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
            matches!(action.navigation, Some(Navigation::Wheel { zoom, .. }) if (zoom-1.).abs()<1e-6)
        );
    }
    #[test]
    fn wheel_zoom_preserves_fractional_input_pixels_and_horizontal_scroll() {
        for (delta, expected_zoom, expected_scroll) in [
            (mouse::ScrollDelta::Lines { x: 1., y: 1. }, 1., 25.6),
            (mouse::ScrollDelta::Lines { x: -1., y: -1. }, -1., -25.6),
            (mouse::ScrollDelta::Lines { x: 0., y: 0.125 }, 0.125, 0.),
            (mouse::ScrollDelta::Lines { x: 0., y: 4. }, 1., 0.),
            (mouse::ScrollDelta::Pixels { x: 10., y: 5. }, 0.05, 8.),
            (mouse::ScrollDelta::Pixels { x: 0., y: -5. }, -0.05, 0.),
        ] {
            let mut state = ChartInput::default();
            let action = state
                .update(
                    &event(mouse::Event::WheelScrolled { delta }),
                    Some(Point::new(180., 50.)),
                    plot(),
                    true,
                )
                .unwrap();
            let Some(Navigation::Wheel { zoom, scroll, .. }) = action.navigation else {
                panic!("wheel must navigate");
            };
            assert!((zoom - expected_zoom).abs() < 1e-6, "{delta:?}: {zoom}");
            assert!(
                (scroll - expected_scroll).abs() < 1e-6,
                "{delta:?}: {scroll}"
            );
        }
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
            Some(Navigation::Pan { start: Some(_), .. })
        ));
        let outside = state
            .update(&movement, Some(Point::new(700., 50.)), plot(), true)
            .unwrap();
        assert!(outside.position.is_none());
        assert!(matches!(outside.navigation, Some(Navigation::Pan { .. })));
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

    #[test]
    fn multi_pane_routes_once_and_keeps_drag_owner() {
        let layout = crate::ui::multi_pane_layout::allocate_multi_pane_layout(
            crate::renderers::price_axis_renderer::AxisBounds {
                width: 800.,
                height: 500.,
            },
            crate::ui::chart_layout::AxisSizeRequests {
                left_width: 60.,
                right_width: 80.,
                time_height: 28.,
            },
            crate::renderers::grid_renderer::PixelRatio {
                horizontal: 1.5,
                vertical: 1.5,
            },
            &[2., 1.],
        )
        .unwrap();
        let mut input = MultiPaneInput::default();
        let bottom = Point::new(160., layout.panes[1].plot.origin.y as f32 + 50.);
        let wheel = event(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0., y: 1. },
        });
        let action = input
            .update(&wheel, Some(bottom), &layout, &[2., 1.], true, true)
            .unwrap();
        assert!(matches!(
            action,
            MultiPaneInteraction::Plot {
                pane: 1,
                interaction: ChartInteraction {
                    position: Some(Point { x: 100., y: 50. }),
                    navigation: Some(Navigation::Wheel { .. }),
                    ..
                }
            }
        ));
        let press = event(mouse::Event::ButtonPressed(mouse::Button::Left));
        input
            .update(&press, Some(bottom), &layout, &[2., 1.], true, true)
            .unwrap();
        let movement = event(mouse::Event::CursorMoved {
            position: Point::ORIGIN,
        });
        let moved = Point::new(180., 20.);
        assert!(matches!(
            input.update(&movement, Some(moved), &layout, &[2., 1.], true, true),
            Some(MultiPaneInteraction::Plot {
                pane: 1,
                interaction: ChartInteraction {
                    navigation: Some(Navigation::Pan { .. }),
                    position: None,
                    ..
                }
            })
        ));
        assert!(
            input
                .update(&movement, Some(moved), &layout, &[2., 1.], true, true)
                .is_none()
        );
        assert!(matches!(
            input.update(
                &Event::Window(iced::window::Event::Unfocused),
                None,
                &layout,
                &[2., 1.],
                true,
                true
            ),
            Some(MultiPaneInteraction::Cancel)
        ));
    }

    #[test]
    fn divider_priority_clamps_minimum_and_survives_own_height_changes() {
        use crate::{
            renderers::{grid_renderer::PixelRatio, price_axis_renderer::AxisBounds},
            ui::{chart_layout::AxisSizeRequests, multi_pane_layout::allocate_multi_pane_layout},
        };
        let bounds = AxisBounds {
            width: 800.,
            height: 500.,
        };
        let ratio = PixelRatio {
            horizontal: 1.,
            vertical: 1.,
        };
        let layout =
            allocate_multi_pane_layout(bounds, AxisSizeRequests::default(), ratio, &[2., 1.])
                .unwrap();
        let mut input = MultiPaneInput::default();
        let point = Point::new(100., layout.separators[0].origin.y as f32 - 3.);
        let press = event(mouse::Event::ButtonPressed(mouse::Button::Left));
        assert!(matches!(
            input.update(&press, Some(point), &layout, &[2., 1.], true, true),
            Some(MultiPaneInteraction::Resize { top: 0, .. })
        ));
        let movement = event(mouse::Event::CursorMoved {
            position: Point::ORIGIN,
        });
        let Some(MultiPaneInteraction::Resize {
            upper_stretch,
            lower_stretch,
            ..
        }) = input.update(
            &movement,
            Some(Point::new(100., -1000.)),
            &layout,
            &[2., 1.],
            true,
            true,
        )
        else {
            panic!("divider must own gesture");
        };
        assert!((upper_stretch + lower_stretch - 3.).abs() < 1e-9);
        let resized = allocate_multi_pane_layout(
            bounds,
            AxisSizeRequests::default(),
            ratio,
            &[upper_stretch, lower_stretch],
        )
        .unwrap();
        assert_eq!(resized.panes[0].plot.size.height, 30.);
        assert!(matches!(
            input.update(
                &movement,
                Some(Point::new(100., 1000.)),
                &resized,
                &[upper_stretch, lower_stretch],
                true,
                true
            ),
            Some(MultiPaneInteraction::Resize { .. })
        ));
        assert!(matches!(
            input.update(
                &movement,
                Some(point),
                &resized,
                &[upper_stretch, lower_stretch],
                true,
                false
            ),
            Some(MultiPaneInteraction::Cancel)
        ));
        let tiny = allocate_multi_pane_layout(
            AxisBounds {
                width: 800.,
                height: 50.,
            },
            AxisSizeRequests::default(),
            ratio,
            &[2., 1.],
        )
        .unwrap();
        let mut input = MultiPaneInput::default();
        assert!(
            input
                .update(
                    &press,
                    Some(Point::new(100., tiny.separators[0].origin.y as f32)),
                    &tiny,
                    &[2., 1.],
                    true,
                    true
                )
                .is_none()
        );
    }
}

#[cfg(test)]
mod pan_tests {
    use super::*;
    use crate::renderers::{line_renderer::Point as EnginePoint, price_axis_renderer::AxisBounds};
    #[test]
    fn price_label_width_changes_do_not_cancel_or_shift_a_plot_pan() {
        let mut input = ChartInput::default();
        let mut plot = LayoutRect {
            origin: EnginePoint { x: 50., y: 0. },
            size: AxisBounds {
                width: 600.,
                height: 300.,
            },
        };
        let press = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let movement = Event::Mouse(mouse::Event::CursorMoved {
            position: Point::ORIGIN,
        });
        input
            .update(&press, Some(Point::new(200., 100.)), plot, true)
            .unwrap();
        input
            .update(&movement, Some(Point::new(210., 110.)), plot, true)
            .unwrap();
        plot.origin.x += 10.;
        plot.size.width -= 10.;
        let action = input
            .update(&movement, Some(Point::new(230., 130.)), plot, true)
            .unwrap();
        assert!(matches!(
            action.navigation,
            Some(Navigation::Pan {
                start: None,
                position: Point { x: 180., y: 130. }
            })
        ));
    }
}
