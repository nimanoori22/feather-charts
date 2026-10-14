//! Canvas-owned native finger state. No chart/model borrow survives an event.
use super::{TouchAction, TouchInteraction, TouchNavigationOptions, TrackingExitMode};
use crate::ui::chart_layout::{ChartLayout, LayoutRect};
use iced::{
    Point,
    touch::{Event as FingerEvent, Finger},
    widget::canvas::Event,
};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Default)]
enum Gesture {
    #[default]
    Idle,
    Pressed {
        origin: Point,
        deadline: Instant,
    },
    Pan,
    Pinch {
        anchor: Point,
        distance: f64,
        scale: f64,
    },
    Tracking {
        origin: Point,
        base: Option<Point>,
        next_tap: bool,
        moved: bool,
    },
    Suspended,
    AfterPinch {
        origin: Point,
    },
    Delegated,
}
pub(super) struct TouchOutput {
    pub input: TouchInteraction,
    pub capture: bool,
}
#[derive(Default)]
pub(super) struct TouchInput {
    fingers: Vec<(Finger, Point)>,
    primary: Option<Finger>,
    down: Option<Point>,
    owner: Option<(usize, LayoutRect)>,
    gesture: Gesture,
    pinch_eligible: bool,
    persistent_tracking: Option<usize>,
    suppress_mouse_until: Option<Instant>,
}
impl TouchInput {
    fn output(&self, now: Instant, action: TouchAction, capture: bool) -> Option<TouchOutput> {
        Some(TouchOutput {
            input: TouchInteraction {
                pane: self.owner?.0,
                now,
                action,
            },
            capture,
        })
    }
    fn local(&self, position: Point) -> Option<Point> {
        let plot = self.owner?.1;
        Some(Point::new(
            position.x - plot.origin.x as f32,
            position.y - plot.origin.y as f32,
        ))
    }
    pub fn is_active(&self) -> bool {
        self.owner.is_some() || self.persistent_tracking.is_some()
    }
    pub fn owner_matches(&self, layouts: &[ChartLayout]) -> bool {
        self.owner.is_none_or(|(pane, plot)| {
            layouts.get(pane).is_some_and(|layout| {
                layout.plot.origin.y == plot.origin.y && layout.plot.size.height == plot.size.height
            })
        })
    }
    pub fn suppresses_mouse(&self, now: Instant) -> bool {
        !self.fingers.is_empty() || self.suppress_mouse_until.is_some_and(|until| now < until)
    }
    pub fn deadline(&self) -> Option<Instant> {
        if let Gesture::Pressed { deadline, .. } = self.gesture {
            Some(deadline)
        } else {
            None
        }
    }
    pub fn cancel(&mut self, now: Instant) -> Option<TouchOutput> {
        let pane = self.owner.map(|owner| owner.0).or(self.persistent_tracking);
        self.fingers.clear();
        self.primary = None;
        self.down = None;
        self.owner = None;
        self.gesture = Gesture::Idle;
        self.persistent_tracking = None;
        self.pinch_eligible = false;
        pane.map(|pane| TouchOutput {
            input: TouchInteraction {
                pane,
                now,
                action: TouchAction::Cancel,
            },
            capture: false,
        })
    }
    fn pair(&self) -> Option<(Point, f64)> {
        if self.fingers.len() != 2 {
            return None;
        }
        let first = self.local(self.fingers[0].1)?;
        let second = self.local(self.fingers[1].1)?;
        let distance = (f64::from(first.x) - f64::from(second.x))
            .hypot(f64::from(first.y) - f64::from(second.y));
        (distance.is_finite() && distance > 0.).then_some((
            Point::new((first.x + second.x) * 0.5, (first.y + second.y) * 0.5),
            distance,
        ))
    }
    fn begin_pinch(&mut self, now: Instant) -> Option<TouchOutput> {
        if !self.pinch_eligible {
            return None;
        }
        let (anchor, distance) = self.pair()?;
        self.gesture = Gesture::Pinch {
            anchor,
            distance,
            scale: 1.,
        };
        self.output(now, TouchAction::Pinch { anchor, zoom: 0. }, true)
    }
    pub fn update(
        &mut self,
        event: &Event,
        bounds: iced::Rectangle,
        layouts: &[ChartLayout],
        options: TouchNavigationOptions,
        applied: Option<(usize, Point)>,
        now: Instant,
    ) -> Option<TouchOutput> {
        if let Event::Window(iced::window::Event::RedrawRequested(_)) = event {
            if let Gesture::Pressed { origin, deadline } = self.gesture
                && now >= deadline
                && self.fingers.len() == 1
            {
                self.pinch_eligible = false;
                self.gesture = Gesture::Tracking {
                    origin,
                    base: None,
                    next_tap: false,
                    moved: false,
                };
                self.persistent_tracking = self.owner.map(|owner| owner.0);
                return self.output(now, TouchAction::Track { position: origin }, true);
            }
            return None;
        }
        let Event::Touch(event) = event else {
            return None;
        };
        let (id, position) = match *event {
            FingerEvent::FingerPressed { id, position }
            | FingerEvent::FingerMoved { id, position }
            | FingerEvent::FingerLifted { id, position }
            | FingerEvent::FingerLost { id, position } => (id, position),
        };
        if !position.x.is_finite() || !position.y.is_finite() {
            return self.cancel(now);
        }
        let position = Point::new(position.x - bounds.x, position.y - bounds.y);
        match event {
            FingerEvent::FingerPressed { .. } => {
                if self.fingers.iter().any(|(finger, _)| *finger == id) {
                    return None;
                }
                if self.owner.is_none() {
                    let (pane, plot) = layouts.iter().enumerate().find_map(|(pane, layout)| {
                        inside(position, layout.plot).then_some((pane, layout.plot))
                    })?;
                    self.owner = Some((pane, plot));
                    self.primary = Some(id);
                    let local = self.local(position)?;
                    self.down = Some(local);
                    if self.persistent_tracking == Some(pane) {
                        self.gesture = Gesture::Tracking {
                            origin: local,
                            base: applied
                                .filter(|(owner, _)| *owner == pane)
                                .map(|(_, point)| point),
                            next_tap: true,
                            moved: false,
                        };
                        self.pinch_eligible = false;
                    } else {
                        self.persistent_tracking = None;
                        self.gesture = Gesture::Pressed {
                            origin: local,
                            deadline: now + Duration::from_millis(240),
                        };
                        self.pinch_eligible = true;
                    }
                } else if !inside(position, self.owner?.1) {
                    return None;
                }
                self.fingers.push((id, position));
                self.suppress_mouse_until = Some(now + Duration::from_millis(500));
                if self.fingers.len() == 2
                    && let Some(output) = self.begin_pinch(now)
                {
                    return Some(output);
                }
                if self.fingers.len() > 1
                    && !matches!(
                        self.gesture,
                        Gesture::Pan | Gesture::Tracking { .. } | Gesture::Delegated
                    )
                {
                    self.gesture = Gesture::Suspended;
                    return self.output(now, TouchAction::End, true);
                }
                if self.fingers.len() > 1 {
                    return None;
                }
                self.output(now, TouchAction::Press, false)
            }
            FingerEvent::FingerLost { .. } => {
                if !self.fingers.iter().any(|(finger, _)| *finger == id) {
                    return None;
                }
                self.suppress_mouse_until = Some(now + Duration::from_millis(500));
                self.cancel(now)
            }
            FingerEvent::FingerMoved { .. } => {
                let at = self.fingers.iter().position(|(finger, _)| *finger == id)?;
                if self.fingers[at].1 == position {
                    return None;
                }
                self.fingers[at].1 = position;
                self.suppress_mouse_until = Some(now + Duration::from_millis(500));
                if matches!(self.gesture, Gesture::Pinch { .. }) {
                    let Gesture::Pinch {
                        anchor,
                        distance,
                        scale,
                    } = self.gesture
                    else {
                        unreachable!()
                    };
                    let (_, current) = self.pair()?;
                    let next = current / distance;
                    let zoom = (next - scale) * 5.;
                    if !zoom.is_finite() {
                        return self.cancel(now);
                    }
                    self.gesture = Gesture::Pinch {
                        anchor,
                        distance,
                        scale: next,
                    };
                    return self.output(now, TouchAction::Pinch { anchor, zoom }, true);
                }
                if self.primary != Some(id) {
                    return None;
                }
                self.pinch_eligible = false;
                let local = self.local(position)?;
                match self.gesture {
                    Gesture::Pressed { origin, .. } | Gesture::AfterPinch { origin } => {
                        if manhattan(local, origin) < 5. {
                            return None;
                        }
                        let vertical =
                            (local.y - origin.y).abs() >= (local.x - origin.x).abs() * 0.5;
                        if if vertical {
                            !options.vertical_drag
                        } else {
                            !options.horizontal_drag
                        } {
                            self.gesture = Gesture::Delegated;
                            return None;
                        }
                        self.gesture = Gesture::Pan;
                        self.output(
                            now,
                            TouchAction::Pan {
                                start: Some(local),
                                position: local,
                            },
                            true,
                        )
                    }
                    Gesture::Pan => self.output(
                        now,
                        TouchAction::Pan {
                            start: None,
                            position: local,
                        },
                        true,
                    ),
                    Gesture::Tracking {
                        origin,
                        base,
                        next_tap,
                        moved,
                    } => {
                        if !moved && manhattan(local, origin) < 5. {
                            return None;
                        }
                        let pane = self.owner?.0;
                        let base = base
                            .or_else(|| {
                                applied
                                    .filter(|(owner, _)| *owner == pane)
                                    .map(|(_, point)| point)
                            })
                            .unwrap_or(origin);
                        let plot = self.owner?.1;
                        let point = Point::new(
                            (base.x + local.x - origin.x)
                                .clamp(0., (plot.size.width as f32 - 1.).max(0.)),
                            (base.y + local.y - origin.y)
                                .clamp(0., (plot.size.height as f32 - 1.).max(0.)),
                        );
                        self.gesture = Gesture::Tracking {
                            origin,
                            base: Some(base),
                            next_tap,
                            moved: true,
                        };
                        self.output(now, TouchAction::Track { position: point }, true)
                    }
                    _ => None,
                }
            }
            FingerEvent::FingerLifted { .. } => {
                let at = self.fingers.iter().position(|(finger, _)| *finger == id)?;
                self.fingers.remove(at);
                self.suppress_mouse_until = Some(now + Duration::from_millis(500));
                if self.fingers.len() == 2
                    && let Some(output) = self.begin_pinch(now)
                {
                    return Some(output);
                }
                if self.primary != Some(id) {
                    if self.fingers.is_empty() {
                        let output = self.output(now, TouchAction::End, true);
                        self.owner = None;
                        self.gesture = Gesture::Idle;
                        return output;
                    }
                    if matches!(self.gesture, Gesture::Pinch { .. } | Gesture::Suspended)
                        && self.fingers.len() == 1
                    {
                        self.pinch_eligible = true;
                        self.gesture = self
                            .down
                            .map_or(Gesture::Suspended, |origin| Gesture::AfterPinch { origin });
                        return self.output(now, TouchAction::End, true);
                    }
                    if matches!(self.gesture, Gesture::Pinch { .. }) {
                        self.gesture = Gesture::Suspended;
                        return self.output(now, TouchAction::End, true);
                    }
                    return None;
                }
                let local = self.local(position)?;
                let (action, capture) = match self.gesture {
                    Gesture::Pressed { origin, .. } => (
                        if manhattan(local, origin) < 5. {
                            TouchAction::Tap { position: local }
                        } else {
                            TouchAction::End
                        },
                        true,
                    ),
                    Gesture::Tracking {
                        next_tap, moved, ..
                    } => {
                        let exit = options.tracking_exit == TrackingExitMode::OnTouchEnd
                            || (next_tap && !moved);
                        if exit {
                            self.persistent_tracking = None;
                        }
                        (
                            if exit {
                                TouchAction::ExitTracking
                            } else {
                                TouchAction::End
                            },
                            true,
                        )
                    }
                    Gesture::Delegated => (TouchAction::End, false),
                    _ => (TouchAction::End, true),
                };
                let output = self.output(now, action, capture);
                self.primary = None;
                self.down = None;
                if self.fingers.is_empty() {
                    self.owner = None;
                    self.gesture = Gesture::Idle;
                } else {
                    self.gesture = Gesture::Suspended;
                }
                output
            }
        }
    }
}
fn inside(point: Point, plot: LayoutRect) -> bool {
    f64::from(point.x) >= plot.origin.x
        && f64::from(point.x) < plot.origin.x + plot.size.width
        && f64::from(point.y) >= plot.origin.y
        && f64::from(point.y) < plot.origin.y + plot.size.height
}
fn manhattan(first: Point, second: Point) -> f32 {
    (first.x - second.x).abs() + (first.y - second.y).abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderers::{line_renderer::Point as ModelPoint, price_axis_renderer::AxisBounds};
    fn layouts() -> [ChartLayout; 2] {
        [20., 240.].map(|y| ChartLayout {
            plot: LayoutRect {
                origin: ModelPoint { x: 30., y },
                size: AxisBounds {
                    width: 300.,
                    height: 200.,
                },
            },
            ..Default::default()
        })
    }
    fn send(
        input: &mut TouchInput,
        kind: u8,
        id: u64,
        x: f32,
        y: f32,
        now: Instant,
    ) -> Option<TouchOutput> {
        let id = Finger(id);
        let position = Point::new(x + 130., y + 70.);
        let event = match kind {
            0 => FingerEvent::FingerPressed { id, position },
            1 => FingerEvent::FingerMoved { id, position },
            2 => FingerEvent::FingerLifted { id, position },
            _ => FingerEvent::FingerLost { id, position },
        };
        input.update(
            &Event::Touch(event),
            iced::Rectangle::new(Point::new(100., 50.), iced::Size::new(600., 500.)),
            &layouts(),
            Default::default(),
            None,
            now,
        )
    }
    #[test]
    fn pinch_freezes_anchor_suspends_third_finger_and_restarts_pair() {
        let now = Instant::now();
        let mut input = TouchInput::default();
        send(&mut input, 0, 1, 50., 50., now);
        let first = send(&mut input, 0, 2, 150., 50., now).unwrap();
        assert!(
            matches!(first.input.action,TouchAction::Pinch { anchor, zoom: 0. } if anchor == Point::new(100.,50.))
        );
        let moved = send(&mut input, 1, 2, 200., 50., now).unwrap();
        assert!(
            matches!(moved.input.action,TouchAction::Pinch { anchor, zoom } if anchor == Point::new(100.,50.) && (zoom-2.5).abs()<1e-6)
        );
        assert!(matches!(
            send(&mut input, 0, 3, 250., 50., now).unwrap().input.action,
            TouchAction::End
        ));
        assert!(send(&mut input, 1, 2, 210., 50., now).is_none());
        assert!(
            matches!(send(&mut input,2,1,50.,50.,now).unwrap().input.action,TouchAction::Pinch { anchor, zoom: 0. } if anchor == Point::new(230.,50.))
        );
        send(&mut input, 2, 2, 210., 50., now);
        assert!(send(&mut input, 1, 3, 280., 50., now).is_none());
        send(&mut input, 2, 3, 280., 50., now);
        assert!(!input.is_active());
    }
    #[test]
    fn primary_move_prevents_later_pinch_and_remaining_primary_can_pan() {
        let now = Instant::now();
        let mut input = TouchInput::default();
        send(&mut input, 0, 1, 50., 50., now);
        send(&mut input, 1, 1, 51., 50., now);
        assert!(matches!(
            send(&mut input, 0, 2, 150., 50., now).unwrap().input.action,
            TouchAction::End
        ));
        assert!(send(&mut input, 1, 2, 200., 50., now).is_none());
        send(&mut input, 2, 2, 200., 50., now);
        assert!(matches!(
            send(&mut input, 1, 1, 80., 50., now).unwrap().input.action,
            TouchAction::Pan { start: Some(_), .. }
        ));
        input.cancel(now);
        send(&mut input, 0, 1, 50., 50., now);
        send(&mut input, 0, 2, 150., 50., now);
        send(&mut input, 2, 2, 150., 50., now);
        assert!(matches!(
            send(&mut input, 1, 1, 90., 50., now).unwrap().input.action,
            TouchAction::Pan { start: Some(_), .. }
        ));
    }
    #[test]
    fn zero_distance_loss_and_unknown_ids_do_not_invent_navigation() {
        let now = Instant::now();
        let mut input = TouchInput::default();
        assert!(send(&mut input, 1, 99, 50., 50., now).is_none());
        send(&mut input, 0, 1, 50., 50., now);
        assert!(matches!(
            send(&mut input, 0, 2, 50., 50., now).unwrap().input.action,
            TouchAction::End
        ));
        assert!(send(&mut input, 1, 2, 60., 50., now).is_none());
        assert!(matches!(
            send(&mut input, 3, 2, 60., 50., now).unwrap().input.action,
            TouchAction::Cancel
        ));
        assert!(!input.is_active());
        assert!(input.deadline().is_none());
        assert!(send(&mut input, 2, 1, 50., 50., now).is_none());
        send(&mut input, 0, 3, 50., 50., now);
        assert!(matches!(
            send(&mut input, 1, 3, f32::NAN, 50., now)
                .unwrap()
                .input
                .action,
            TouchAction::Cancel
        ));
        assert!(!input.is_active());
    }
    #[test]
    fn mouse_suppression_covers_held_fingers_and_expires_after_release() {
        let now = Instant::now();
        let mut input = TouchInput::default();
        send(&mut input, 0, 1, 50., 50., now);
        assert!(input.suppresses_mouse(now + Duration::from_secs(2)));
        send(&mut input, 2, 1, 50., 50., now + Duration::from_secs(2));
        assert!(input.suppresses_mouse(now + Duration::from_millis(2499)));
        assert!(!input.suppresses_mouse(now + Duration::from_millis(2500)));
    }
    #[test]
    fn tracking_uses_applied_origin_and_clamps_inside_owning_pane() {
        let now = Instant::now();
        let mut input = TouchInput::default();
        send(&mut input, 0, 1, 50., 270., now);
        let bounds = iced::Rectangle::new(Point::new(100., 50.), iced::Size::new(600., 500.));
        let track = input
            .update(
                &Event::Window(iced::window::Event::RedrawRequested(
                    now + Duration::from_millis(240),
                )),
                bounds,
                &layouts(),
                Default::default(),
                None,
                now + Duration::from_millis(240),
            )
            .unwrap();
        assert_eq!(track.input.pane, 1);
        let position = Point::new(1000., 1000.);
        let moved = input
            .update(
                &Event::Touch(FingerEvent::FingerMoved {
                    id: Finger(1),
                    position,
                }),
                bounds,
                &layouts(),
                Default::default(),
                Some((1, Point::new(80., 60.))),
                now + Duration::from_millis(250),
            )
            .unwrap();
        assert!(
            matches!(moved.input.action,TouchAction::Track { position } if position==Point::new(299.,199.))
        );
        assert!(input.deadline().is_none());
        input.cancel(now);
        assert!(input.deadline().is_none());
    }
}
