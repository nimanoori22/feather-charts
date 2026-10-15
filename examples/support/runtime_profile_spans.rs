//! Injected only into an isolated diagnostic Iced build by profile_public_chart.py.
use std::{
    cell::RefCell,
    fs::OpenOptions,
    io::{BufWriter, Write},
};
const CAPACITY: usize = 200_000;
static OWNER: std::sync::OnceLock<std::thread::ThreadId> = std::sync::OnceLock::new();
fn is_owner() -> bool {
    *OWNER.get_or_init(|| std::thread::current().id()) == std::thread::current().id()
}
struct Record {
    stage: &'static str,
    id: u64,
    start: u64,
    end: u64,
    cpu: u64,
    a: u64,
    b: u64,
}
struct Recorder {
    records: Vec<Record>,
    overflow: usize,
    closed: bool,
}
thread_local! {
    static RECORDS: RefCell<Recorder> = RefCell::new(Recorder {
        records: Vec::with_capacity(CAPACITY), overflow: 0, closed: false
    });
}
fn clock(id: libc::clockid_t) -> u64 {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    assert_eq!(unsafe { libc::clock_gettime(id, &mut time) }, 0);
    time.tv_sec as u64 * 1_000_000_000 + time.tv_nsec as u64
}
fn push(record: Record) {
    if !is_owner() {
        return;
    }
    let _ = RECORDS.try_with(|state| {
        let mut state = state.borrow_mut();
        if state.closed {
            return;
        }
        if state.records.len() < CAPACITY {
            state.records.push(record);
        } else {
            state.overflow += 1;
        }
    });
}
pub fn diagnostic_record(stage: &'static str, id: u64, a: u64, b: u64) {
    let now = clock(libc::CLOCK_MONOTONIC);
    push(Record {
        stage,
        id,
        start: now,
        end: now,
        cpu: 0,
        a,
        b,
    });
}
pub fn diagnostic_flush() {
    RECORDS.with(|state| state.borrow_mut().flush());
}
impl Recorder {
    fn flush(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        let path =
            std::env::var_os("FEATHER_RUNTIME_TRACE").expect("diagnostic trace path required");
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .expect("fresh trace path");
        let mut out = BufWriter::new(file);
        writeln!(out, "stage,id,start_ns,end_ns,cpu_ns,a,b").unwrap();
        writeln!(out, "diagnostic_only,0,0,0,0,1,0").unwrap();
        writeln!(out, "overflow,0,0,0,0,{},0", self.overflow).unwrap();
        for r in &self.records {
            writeln!(
                out,
                "{},{},{},{},{},{},{}",
                r.stage, r.id, r.start, r.end, r.cpu, r.a, r.b
            )
            .unwrap();
        }
        out.flush().unwrap();
    }
}
impl Drop for Recorder {
    fn drop(&mut self) {
        if std::env::var_os("FEATHER_RUNTIME_TRACE").is_some() {
            self.flush();
        }
    }
}
#[derive(Debug)]
pub struct Span {
    stage: &'static str,
    start: u64,
    cpu: u64,
}
impl Span {
    pub fn new(stage: &'static str) -> Self {
        if !is_owner() {
            return Self {
                stage,
                start: 0,
                cpu: 0,
            };
        }
        RECORDS.with(|_| {});
        Self {
            stage,
            start: clock(libc::CLOCK_MONOTONIC),
            cpu: clock(libc::CLOCK_THREAD_CPUTIME_ID),
        }
    }
    pub fn finish(self) {}
}
impl Drop for Span {
    fn drop(&mut self) {
        if self.start == 0 {
            return;
        }
        let end = clock(libc::CLOCK_MONOTONIC);
        let cpu = clock(libc::CLOCK_THREAD_CPUTIME_ID) - self.cpu;
        push(Record {
            stage: self.stage,
            id: 0,
            start: self.start,
            end,
            cpu,
            a: 0,
            b: 0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn worker_spans_do_not_own_or_flush_the_main_trace() {
        let main = Span::new("boot");
        if let Some(path) = std::env::var_os("FEATHER_RUNTIME_TRACE") {
            assert!(
                !std::path::Path::new(&path).exists(),
                "trace flushed during initialization"
            );
        }
        std::thread::spawn(|| {
            Span::new("worker").finish();
        })
        .join()
        .unwrap();
        main.finish();
        RECORDS.with(|records| {
            let records = records.borrow();
            assert_eq!(records.records.len(), 1);
            assert_eq!(records.records[0].stage, "boot");
            assert!(records.records[0].end >= records.records[0].start);
        });
        let started = std::time::Instant::now();
        for _ in 0..10_000 {
            Span::new("calibration").finish();
        }
        println!(
            "diagnostic_span_average_ns={}",
            started.elapsed().as_nanos() / 10_000
        );
    }
}
