//! Shared user-thread measurement primitives for diagnostic examples.
use std::{
    ffi::{c_int, c_long},
    fs::File,
    io::{self, Read},
    mem::MaybeUninit,
    os::fd::FromRawFd,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Timeval {
    pub(crate) seconds: c_long,
    pub(crate) microseconds: c_long,
}
#[repr(C)]
pub(crate) struct Timespec {
    pub(crate) seconds: c_long,
    pub(crate) nanoseconds: c_long,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Rusage {
    pub(crate) user: Timeval,
    pub(crate) system: Timeval,
    pub(crate) max_rss: c_long,
    pub(crate) shared_rss: c_long,
    pub(crate) unshared_data: c_long,
    pub(crate) unshared_stack: c_long,
    pub(crate) minor_faults: c_long,
    pub(crate) major_faults: c_long,
    pub(crate) swaps: c_long,
    pub(crate) input_blocks: c_long,
    pub(crate) output_blocks: c_long,
    pub(crate) sent_messages: c_long,
    pub(crate) received_messages: c_long,
    pub(crate) signals: c_long,
    pub(crate) voluntary_switches: c_long,
    pub(crate) involuntary_switches: c_long,
}
unsafe extern "C" {
    fn getrusage(who: c_int, usage: *mut Rusage) -> c_int;
    fn clock_gettime(clock: c_int, time: *mut Timespec) -> c_int;
    fn syscall(number: c_long, ...) -> c_long;
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct HardwareCount {
    pub(crate) count: u64,
    pub(crate) enabled_ns: u64,
    pub(crate) running_ns: u64,
}
impl HardwareCount {
    pub(crate) fn since(self, before: Self) -> Self {
        Self {
            count: self.count.checked_sub(before.count).unwrap(),
            enabled_ns: self.enabled_ns.checked_sub(before.enabled_ns).unwrap(),
            running_ns: self.running_ns.checked_sub(before.running_ns).unwrap(),
        }
    }
}
pub(crate) const EVENTS: [(&str, u64); 3] =
    [("instructions", 1), ("cycles", 0), ("cache-misses", 3)];
pub(crate) struct HardwareCounter(File);
impl HardwareCounter {
    pub(crate) fn open(event: u64) -> io::Result<Self> {
        // linux/perf_event.h version0, user-only hardware event, time-enabled/running read format.
        let mut attr = [0_u64; 8];
        attr[0] = 64_u64 << 32;
        attr[1] = event;
        attr[4] = 3;
        attr[5] = (1 << 5) | (1 << 6);
        // asm/unistd_64.h syscall298: calling thread, any CPU, no group, CLOEXEC.
        let fd = unsafe {
            syscall(
                298 as c_long,
                attr.as_ptr(),
                0 as c_long,
                -1 as c_long,
                -1 as c_long,
                8 as c_long,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(unsafe { File::from_raw_fd(fd.try_into().unwrap()) }))
    }
    pub(crate) fn read(&mut self) -> HardwareCount {
        let mut bytes = [0; 24];
        self.0
            .read_exact(&mut bytes)
            .expect("perf hardware counter read failed");
        HardwareCount {
            count: u64::from_ne_bytes(bytes[0..8].try_into().unwrap()),
            enabled_ns: u64::from_ne_bytes(bytes[8..16].try_into().unwrap()),
            running_ns: u64::from_ne_bytes(bytes[16..24].try_into().unwrap()),
        }
    }
}
pub(crate) fn usage() -> Rusage {
    let mut result = MaybeUninit::<Rusage>::uninit();
    // Linux x86_64 RUSAGE_THREAD=1; successful libc initializes the complete C struct.
    assert_eq!(unsafe { getrusage(1, result.as_mut_ptr()) }, 0);
    unsafe { result.assume_init() }
}
pub(crate) fn thread_cpu_ns() -> u64 {
    let mut result = MaybeUninit::<Timespec>::uninit();
    // Linux CLOCK_THREAD_CPUTIME_ID=3, with the native 64-bit timespec layout.
    assert_eq!(unsafe { clock_gettime(3, result.as_mut_ptr()) }, 0);
    let result = unsafe { result.assume_init() };
    assert!(result.seconds >= 0 && (0..1_000_000_000).contains(&result.nanoseconds));
    result.seconds as u64 * 1_000_000_000 + result.nanoseconds as u64
}
