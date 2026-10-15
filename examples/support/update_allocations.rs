//! Example-only requested allocation counts for the calling thread.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub allocations: u64,
    pub reallocations: u64,
    pub deallocations: u64,
    pub requested_bytes: u64,
}
thread_local! {
    static COUNTS: Cell<Option<Counts>> = const { Cell::new(None) };
}
pub fn start() {
    COUNTS.with(|counts| {
        assert!(counts.get().is_none(), "nested allocation measurement");
        counts.set(Some(Counts::default()));
    });
}
pub fn finish() -> Counts {
    COUNTS.with(|counts| counts.replace(None).unwrap_or_default())
}
fn record(update: impl FnOnce(&mut Counts)) {
    // TLS destruction may allocate; such work is outside an active measurement.
    let _ = COUNTS.try_with(|cell| {
        if let Some(mut counts) = cell.get() {
            update(&mut counts);
            cell.set(Some(counts));
        }
    });
}
struct TrackingAllocator;
#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;
// Each operation delegates the original pointer/layout contract to System.
unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            record(|c| {
                c.allocations += 1;
                c.requested_bytes += layout.size() as u64;
            });
        }
        ptr
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            record(|c| {
                c.allocations += 1;
                c.requested_bytes += layout.size() as u64;
            });
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        record(|c| c.deallocations += 1);
        unsafe { System.dealloc(ptr, layout) };
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let ptr = unsafe { System.realloc(ptr, layout, size) };
        if !ptr.is_null() {
            record(|c| {
                c.reallocations += 1;
                c.requested_bytes += size as u64;
            });
        }
        ptr
    }
}
