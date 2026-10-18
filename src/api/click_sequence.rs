//! Successful plot releases only; callers cancel when gesture ownership changes.
use iced::Point;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct ClickSequence {
    first: Option<(usize, Point, Instant)>,
}
impl ClickSequence {
    pub fn cancel(&mut self) {
        self.first = None;
    }
    pub fn release(
        &mut self,
        pane: usize,
        start: Point,
        pressed: Instant,
        position: Point,
        now: Instant,
        distance: f32,
    ) -> bool {
        if let Some((owner, anchor, began)) = self.first.take()
            && owner == pane
            && now.saturating_duration_since(began) < Duration::from_millis(500)
            && (position.x - anchor.x).abs() + (position.y - anchor.y).abs() < distance
        {
            return true;
        }
        self.first = Some((pane, start, pressed));
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pairs_use_first_press_timeout_distance_owner_and_cancellation() {
        let now = Instant::now();
        let p = Point::new(10., 10.);
        let mut s = ClickSequence::default();
        let release = |s: &mut ClickSequence, pane, ms, point| {
            s.release(
                pane,
                p,
                now + Duration::from_millis(ms),
                point,
                now + Duration::from_millis(ms + 10),
                5.,
            )
        };
        assert!(!release(&mut s, 0, 0, p));
        assert!(release(&mut s, 0, 100, p));
        assert!(!release(&mut s, 0, 200, p));
        assert!(!release(&mut s, 0, 700, p));
        assert!(!release(&mut s, 1, 800, p));
        s.cancel();
        assert!(!release(&mut s, 1, 900, p));
        assert!(!release(&mut s, 1, 1000, Point::new(15., 10.)));
        s.cancel();
        assert!(!s.release(0, p, now, p, now, 30.));
        assert!(s.release(
            0,
            p,
            now,
            Point::new(35., 10.),
            now + Duration::from_millis(100),
            30.
        ));
    }
}
