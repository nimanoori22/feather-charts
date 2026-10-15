//! Diagnostic-only self-monitoring RDPMC snapshots, following Linux perf_event_mmap_page.
use super::{EVENTS, HardwareCount, HardwareCounter};
use std::{
    io,
    os::fd::AsRawFd,
    ptr,
    sync::atomic::{Ordering, compiler_fence},
    time::{Duration, Instant},
};

fn extend_count(offset: i64, pmc: u64, width: u16) -> io::Result<u64> {
    if !(1..=64).contains(&width) {
        return Err(io::Error::other("invalid PMU width"));
    }
    let shift = 64 - width;
    Ok(offset.wrapping_add(((pmc << shift) as i64) >> shift) as u64)
}
fn monotonic_ns() -> u64 {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time) },
        0
    );
    time.tv_sec as u64 * 1_000_000_000 + time.tv_nsec as u64
}
struct Counter {
    page: *mut u8,
    length: usize,
    counter: HardwareCounter,
}
impl Counter {
    fn new(event: u64) -> io::Result<Self> {
        let counter = HardwareCounter::open(event)?;
        let length = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if length < 4096 {
            return Err(io::Error::other("unsupported metadata page"));
        }
        let page = unsafe {
            libc::mmap(
                ptr::null_mut(),
                length as usize,
                libc::PROT_READ,
                libc::MAP_SHARED,
                counter.0.as_raw_fd(),
                0,
            )
        };
        if page == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        let mut this = Self {
            page: page.cast(),
            length: length as usize,
            counter,
        };
        let before = this.read()?;
        let kernel = this.counter.read();
        let after = this.read()?;
        if !(before.count <= kernel.count && kernel.count <= after.count) {
            return Err(io::Error::other("RDPMC/kernel counter disagreement"));
        }
        Ok(this)
    }
    fn read(&self) -> io::Result<HardwareCount> {
        for _ in 0..1000 {
            let sequence = unsafe { ptr::read_volatile(self.page.add(8).cast::<u32>()) };
            if sequence & 1 != 0 {
                std::hint::spin_loop();
                continue;
            }
            compiler_fence(Ordering::Acquire);
            let index = unsafe { ptr::read_volatile(self.page.add(12).cast::<u32>()) };
            let offset = unsafe { ptr::read_volatile(self.page.add(16).cast::<i64>()) };
            let enabled_ns = unsafe { ptr::read_volatile(self.page.add(24).cast::<u64>()) };
            let running_ns = unsafe { ptr::read_volatile(self.page.add(32).cast::<u64>()) };
            let capabilities = unsafe { ptr::read_volatile(self.page.add(40).cast::<u64>()) };
            let width = unsafe { ptr::read_volatile(self.page.add(48).cast::<u16>()) };
            if index == 0 || capabilities & 4 == 0 {
                return Err(io::Error::other(
                    "active user RDPMC unavailable; no fallback",
                ));
            }
            let lo: u32;
            let hi: u32;
            // Fences bound out-of-order execution; the kernel sequence validates the PMU index.
            unsafe {
                std::arch::asm!("lfence", "rdpmc", "lfence", in("ecx") index - 1,
                out("eax") lo, out("edx") hi, options(nostack, preserves_flags));
            }
            compiler_fence(Ordering::Acquire);
            let end = unsafe { ptr::read_volatile(self.page.add(8).cast::<u32>()) };
            if sequence == end {
                return Ok(HardwareCount {
                    count: extend_count(offset, (u64::from(hi) << 32) | u64::from(lo), width)?,
                    enabled_ns,
                    running_ns,
                });
            }
        }
        Err(io::Error::other("unstable PMU metadata"))
    }
}
impl Drop for Counter {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.page.cast(), self.length);
        }
    }
}

pub struct FastDiagnostics {
    counters: [Counter; 3],
    before: Option<[HardwareCount; 3]>,
    records: Vec<(usize, Duration, [HardwareCount; 3])>,
    marks: Vec<(usize, &'static str, u64, [HardwareCount; 3])>,
}
impl FastDiagnostics {
    pub fn new(samples: usize) -> io::Result<Self> {
        let counters = [
            Counter::new(EVENTS[0].1)?,
            Counter::new(EVENTS[1].1)?,
            Counter::new(EVENTS[2].1)?,
        ];
        let this = Self {
            counters,
            before: None,
            records: Vec::with_capacity(samples + 20),
            marks: Vec::new(),
        };
        let start = Instant::now();
        for _ in 0..1000 {
            std::hint::black_box(this.snapshot()?);
        }
        println!(
            "diagnostic_fast_counters=true raw=true pinned=true snapshot_triplet_average_ns={}",
            start.elapsed().as_nanos() / 1000
        );
        Ok(this)
    }
    fn snapshot(&self) -> io::Result<[HardwareCount; 3]> {
        Ok([
            self.counters[0].read()?,
            self.counters[1].read()?,
            self.counters[2].read()?,
        ])
    }
    pub fn before(&mut self) {
        assert!(self.before.is_none(), "unpaired counter begin");
        self.before = Some(self.snapshot().expect("RDPMC snapshot"));
    }
    pub fn enable_phases(&mut self, samples: usize) {
        // Touch trace storage before timing so its own first writes cannot masquerade as engine faults.
        self.marks.resize(
            (samples + 20) * 4,
            (
                0,
                "",
                0,
                [HardwareCount {
                    count: 0,
                    enabled_ns: 0,
                    running_ns: 0,
                }; 3],
            ),
        );
        std::hint::black_box(self.marks.as_slice());
        self.marks.clear();
    }
    pub fn mark(&mut self, stage: &'static str) {
        let now = monotonic_ns();
        let counts = self.snapshot().expect("phase RDPMC snapshot");
        assert!(
            self.marks.len() < self.marks.capacity(),
            "phase trace overflow"
        );
        self.marks.push((self.records.len(), stage, now, counts));
    }
    pub fn after(&mut self, sample: usize, wall: Duration) {
        let end = self.snapshot().expect("RDPMC snapshot");
        let begin = self.before.take().expect("unpaired counter end");
        self.records.push((
            sample,
            wall,
            std::array::from_fn(|i| end[i].since(begin[i])),
        ));
    }
    pub fn report(&self) {
        assert!(self.before.is_none(), "unfinished counter interval");
        for (sample, wall, counts) in &self.records {
            println!("fast_update sample={sample} wall_ns={}", wall.as_nanos());
            for ((name, _), count) in EVENTS.iter().zip(counts) {
                println!(
                    "fast_hardware sample={sample} event={name} count={} enabled_ns={} running_ns={}",
                    count.count, count.enabled_ns, count.running_ns
                );
            }
        }
        for pair in self.marks.windows(2) {
            let (begin, end) = (&pair[0], &pair[1]);
            if begin.0 != end.0 {
                continue;
            }
            let counts: [HardwareCount; 3] = std::array::from_fn(|i| end.3[i].since(begin.3[i]));
            println!(
                "fast_phase sample={} phase={} wall_ns={} instructions={} cycles={} cache_misses={} start_ns={} end_ns={}",
                end.0,
                end.1,
                end.2 - begin.2,
                counts[0].count,
                counts[1].count,
                counts[2].count,
                begin.2,
                end.2
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_pmc_width_and_kernel_offset_are_combined() {
        assert_eq!(extend_count(1000, (1_u64 << 48) - 3, 48).unwrap(), 997);
        assert_eq!(extend_count(7, u64::MAX, 64).unwrap(), 6);
        assert_eq!(extend_count(-100, 1234, 48).unwrap(), 1134);
        assert_eq!(extend_count(0, 42, 8).unwrap(), 42);
        assert!(extend_count(0, 0, 0).is_err());
        assert!(extend_count(0, 0, 65).is_err());
    }
}
