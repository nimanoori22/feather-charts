//! Injected only into isolated preparation-profile builds.
use crate::update_counters::thread_cpu_ns;
use std::{
    fs::File,
    io::{BufWriter, Write},
};

#[derive(Clone, Copy)]
pub struct Stamp {
    wall: u64,
    cpu: u64,
}
pub struct Recorder {
    path: std::ffi::OsString,
    records: Vec<(usize, [Stamp; 4])>,
    capacity: usize,
    overflow: usize,
}
impl Recorder {
    pub fn new(samples: usize) -> Option<Self> {
        std::env::var_os("FEATHER_PHASE_TRACE").map(|path| Self {
            path,
            records: Vec::with_capacity(samples + 20),
            capacity: samples + 20,
            overflow: 0,
        })
    }
    pub fn stamp(&self) -> Stamp {
        let cpu = thread_cpu_ns();
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        assert_eq!(
            unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) },
            0
        );
        Stamp {
            wall: ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64,
            cpu,
        }
    }
    pub fn push(&mut self, sample: usize, stamps: [Stamp; 4]) {
        if self.records.len() == self.capacity {
            self.overflow += 1;
        } else {
            self.records.push((sample, stamps));
        }
    }
    pub fn finish(self) {
        assert_eq!(self.overflow, 0, "diagnostic record overflow");
        assert_eq!(
            self.records.len(),
            self.capacity,
            "incomplete diagnostic records"
        );
        let pid = std::process::id();
        let tid = unsafe { libc::gettid() };
        let mut out = BufWriter::new(File::create_new(self.path).unwrap());
        writeln!(out, "sample,phase,pid,tid,clock_id,warmup,start_ns,end_ns,cpu_ns,minor,major,voluntary,involuntary").unwrap();
        for (sample, stamps) in self.records {
            for (i, phase) in ["update", "snapshot", "glyph"].into_iter().enumerate() {
                let a = stamps[i];
                let b = stamps[i + 1];
                // Blank fields distinguish unavailable resource counters from measured zeroes.
                writeln!(
                    out,
                    "{sample},{phase},{pid},{tid},1,{},{},{},{},,,,",
                    sample < 20,
                    a.wall,
                    b.wall,
                    b.cpu.checked_sub(a.cpu).unwrap()
                )
                .unwrap();
            }
        }
    }
}
