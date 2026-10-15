//! Opt-in observations around the acceptance profiler's actual public update.
use crate::{
    update_allocations::{self, Counts},
    update_counters::*,
};
use std::time::Duration;

pub struct Diagnostics {
    counters: Vec<HardwareCounter>,
    records: Vec<Record>,
}
pub struct Before {
    usage: Rusage,
    hardware: [HardwareCount; 3],
    cpu: u64,
}
struct Record {
    sample: usize,
    wall: Duration,
    cpu: u64,
    allocations: Counts,
    minor: i64,
    major: i64,
    voluntary: i64,
    involuntary: i64,
    hardware: [HardwareCount; 3],
}
impl Diagnostics {
    pub fn new(enabled: bool, samples: usize) -> Self {
        println!(
            "diagnostic_counters={}",
            if enabled { "enabled" } else { "disabled" }
        );
        let counters = if enabled {
            EVENTS
                .iter()
                .map(|(name, event)| {
                    HardwareCounter::open(*event).unwrap_or_else(|error| {
                        panic!("diagnostic event {name} unavailable: {error}; no acceptance result")
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        Self {
            counters,
            records: Vec::with_capacity(if enabled { samples } else { 0 }),
        }
    }
    pub fn before(&mut self) -> Option<Before> {
        if self.counters.is_empty() {
            return None;
        }
        let usage = usage();
        let hardware = std::array::from_fn(|i| self.counters[i].read());
        let cpu = thread_cpu_ns();
        update_allocations::start();
        Some(Before {
            usage,
            hardware,
            cpu,
        })
    }
    pub fn after(&mut self, before: Option<Before>, sample: usize, wall: Duration) {
        let Some(before) = before else {
            return;
        };
        let allocations = update_allocations::finish();
        let cpu = thread_cpu_ns() - before.cpu;
        let hardware = std::array::from_fn(|i| self.counters[i].read().since(before.hardware[i]));
        let after = usage();
        if sample >= 20 {
            self.records.push(Record {
                sample,
                wall,
                cpu,
                allocations,
                minor: after.minor_faults - before.usage.minor_faults,
                major: after.major_faults - before.usage.major_faults,
                voluntary: after.voluntary_switches - before.usage.voluntary_switches,
                involuntary: after.involuntary_switches - before.usage.involuntary_switches,
                hardware,
            });
        }
    }
    pub fn report(&self) {
        for r in &self.records {
            println!(
                "diagnostic_update sample={} wall_ns={} cpu_ns={} allocations={} reallocations={} deallocations={} requested_bytes={} minor={} major={} voluntary={} involuntary={}",
                r.sample,
                r.wall.as_nanos(),
                r.cpu,
                r.allocations.allocations,
                r.allocations.reallocations,
                r.allocations.deallocations,
                r.allocations.requested_bytes,
                r.minor,
                r.major,
                r.voluntary,
                r.involuntary
            );
            for ((name, _), h) in EVENTS.iter().zip(r.hardware) {
                println!(
                    "diagnostic_hardware sample={} event={name} count={} enabled_ns={} running_ns={}",
                    r.sample, h.count, h.enabled_ns, h.running_ns
                );
            }
        }
    }
}
