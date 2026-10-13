//! Source kinetic curve in logical offsets and milliseconds, sampled synchronously.
use super::invalidate_mask::TimeScaleAnimation;
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
struct Sample {
    position: f64,
    time: Instant,
}

#[derive(Debug)]
pub struct KineticSampler {
    samples: [Option<Sample>; 4],
    min_speed: f64,
    max_speed: f64,
    decay: f64,
    min_move: f64,
}
impl KineticSampler {
    pub fn new(min_speed: f64, max_speed: f64, decay: f64, min_move: f64) -> Option<Self> {
        (min_speed.is_finite()
            && min_speed >= 0.
            && max_speed.is_finite()
            && max_speed >= min_speed
            && max_speed > 0.
            && decay.is_finite()
            && decay > 0.
            && decay < 1.
            && min_move.is_finite()
            && min_move >= 0.)
            .then_some(Self {
                samples: [None; 4],
                min_speed,
                max_speed,
                decay,
                min_move,
            })
    }
    pub fn add_position(&mut self, position: f64, now: Instant) {
        if !position.is_finite() {
            return;
        }
        if let Some(last) = &mut self.samples[0] {
            if now < last.time {
                return;
            }
            if now == last.time {
                last.position = position;
                return;
            }
            if (position - last.position).abs() < self.min_move {
                return;
            }
        }
        self.samples.rotate_right(1);
        self.samples[0] = Some(Sample {
            position,
            time: now,
        });
    }
    pub fn start(&self, position: f64, now: Instant) -> Option<KineticAnimation> {
        let latest = self.samples[0]?;
        if !position.is_finite()
            || now < latest.time
            || now.duration_since(latest.time).as_secs_f64() * 1_000. > 50.
        {
            return None;
        }
        let mut total_distance = 0.;
        let mut weighted_speed = 0.;
        let mut direction = None;
        for pair in self.samples.windows(2) {
            let (Some(newer), Some(older)) = (pair[0], pair[1]) else {
                break;
            };
            let distance = newer.position - older.position;
            let elapsed = newer.time.duration_since(older.time).as_secs_f64() * 1_000.;
            if elapsed <= 0. || distance == 0. {
                break;
            }
            let sign = distance.signum();
            if direction.is_some_and(|old| old != sign) {
                break;
            }
            direction = Some(sign);
            let speed = (distance / elapsed).clamp(-self.max_speed, self.max_speed);
            total_distance += distance.abs();
            weighted_speed += distance.abs() * speed;
        }
        if total_distance <= 0. {
            return None;
        }
        let speed = weighted_speed / total_distance;
        if !speed.is_finite() || speed.abs() < self.min_speed || speed == 0. {
            return None;
        }
        let log_decay = self.decay.ln();
        let duration_ms = ((-log_decay / speed.abs()).ln() / log_decay).max(0.);
        duration_ms.is_finite().then_some(KineticAnimation {
            position,
            started: now,
            speed,
            decay: self.decay,
            log_decay,
            duration_ms,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct KineticAnimation {
    position: f64,
    started: Instant,
    speed: f64,
    decay: f64,
    log_decay: f64,
    duration_ms: f64,
}
impl TimeScaleAnimation for KineticAnimation {
    fn sample(&self, now: Instant) -> (f64, bool) {
        let elapsed = now.saturating_duration_since(self.started).as_secs_f64() * 1_000.;
        (
            self.position + self.speed * (self.decay.powf(elapsed) - 1.) / self.log_decay,
            elapsed >= self.duration_ms,
        )
    }
}
