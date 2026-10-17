//! Injected into the isolated Histogram16 diagnostic library only.
use std::{cell::Cell, time::Instant};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Counts {
    pub refreshes: u64,
    pub preparations: u64,
    pub points: u64,
    pub preparation_ns: u64,
}

thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static TIMING: Cell<bool> = const { Cell::new(false) };
    static COUNTS: [Cell<Counts>; 16] = const {
        [const { Cell::new(Counts { refreshes: 0, preparations: 0, points: 0, preparation_ns: 0 }) }; 16]
    };
}

pub fn begin() {
    assert!(!ENABLED.replace(true), "nested diagnostic capture");
    COUNTS.with(|rows| rows.iter().for_each(|r| r.set(Counts::default())));
}

pub fn end() -> [Counts; 16] {
    assert!(ENABLED.replace(false), "missing diagnostic capture");
    TIMING.set(false);
    COUNTS.with(|rows| std::array::from_fn(|i| rows[i].get()))
}

pub fn begin_timed() {
    begin();
    TIMING.set(true);
}

pub struct PreparationTimer {
    id: u64,
    start: Option<Instant>,
}

pub fn preparation_timer(id: u64) -> PreparationTimer {
    PreparationTimer {
        id,
        start: TIMING.get().then(Instant::now),
    }
}

impl Drop for PreparationTimer {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            add(
                self.id,
                Counts {
                    preparation_ns: start.elapsed().as_nanos().try_into().unwrap(),
                    ..Counts::default()
                },
            );
        }
    }
}

#[inline]
fn add(id: u64, delta: Counts) {
    if !ENABLED.get() {
        return;
    }
    assert!((1..=16).contains(&id), "requires fixture series IDs 1..=16");
    COUNTS.with(|rows| {
        let slot = &rows[id as usize - 1];
        let mut row = slot.get();
        row.refreshes += delta.refreshes;
        row.preparations += delta.preparations;
        row.points += delta.points;
        row.preparation_ns += delta.preparation_ns;
        slot.set(row);
    });
}

#[inline]
pub fn refresh(id: u64) {
    add(
        id,
        Counts {
            refreshes: 1,
            ..Counts::default()
        },
    );
}

#[inline]
pub fn preparation(id: u64) {
    add(
        id,
        Counts {
            preparations: 1,
            ..Counts::default()
        },
    );
}

#[inline]
pub fn projected(id: u64, points: usize) {
    add(
        id,
        Counts {
            points: points as u64,
            ..Counts::default()
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timed_scopes_record_on_drop_and_reset_without_timing_inactive_capture() {
        begin_timed();
        // Backdate a guard to test aggregation without a timing threshold or sleep.
        let timer = PreparationTimer {
            id: 1,
            start: Some(std::time::Instant::now() - std::time::Duration::from_millis(1)),
        };
        drop(timer);
        let rows = end();
        assert!(rows[0].preparation_ns >= 1_000_000);
        assert_eq!(rows[1].preparation_ns, 0);
        drop(preparation_timer(999));
        begin();
        drop(preparation_timer(1));
        assert_eq!(end(), [Counts::default(); 16]);
    }

    #[test]
    fn counts_are_per_series_reset_per_frame_and_disabled_outside_capture() {
        refresh(999);
        begin();
        refresh(1);
        refresh(1);
        preparation(1);
        projected(1, 83);
        preparation(16);
        projected(16, 20);
        let rows = end();
        assert_eq!(
            rows[0],
            Counts {
                refreshes: 2,
                preparations: 1,
                points: 83,
                ..Counts::default()
            }
        );
        assert_eq!(
            rows[15],
            Counts {
                refreshes: 0,
                preparations: 1,
                points: 20,
                ..Counts::default()
            }
        );
        assert!(rows[1..15].iter().all(|r| *r == Counts::default()));
        refresh(999);
        begin();
        assert_eq!(end(), [Counts::default(); 16]);
    }

    #[test]
    fn capture_is_thread_local() {
        begin();
        refresh(1);
        let other = std::thread::spawn(|| {
            begin();
            projected(2, 100);
            end()
        })
        .join()
        .unwrap();
        assert_eq!(other[1].points, 100);
        let rows = end();
        assert_eq!(rows[0].refreshes, 1);
        assert_eq!(rows[1].points, 0);
    }
}
