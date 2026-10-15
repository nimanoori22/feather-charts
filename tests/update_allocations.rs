#[path = "../examples/support/update_allocations.rs"]
mod update_allocations;

use std::{
    hint::black_box,
    sync::{Arc, Barrier},
};
use update_allocations::{Counts, finish, start};

fn bytes(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(black_box(size));
    data.resize(size, 0x5a);
    black_box(&data);
    data
}

#[test]
fn counts_real_allocations_reallocations_and_deallocations() {
    let initial_capacity = 4096;
    start();
    let mut data = bytes(initial_capacity);
    data.reserve_exact(black_box(8192));
    let grown_capacity = data.capacity();
    black_box(&data);
    drop(data);
    let counts = finish();

    assert!(grown_capacity > initial_capacity);
    assert!(counts.allocations >= 1, "{counts:?}");
    assert!(counts.reallocations >= 1, "{counts:?}");
    assert!(counts.deallocations >= 1, "{counts:?}");
    assert!(
        counts.requested_bytes >= (initial_capacity + grown_capacity) as u64,
        "requested bytes include the whole realloc destination, not just growth: {counts:?}"
    );
}

#[test]
fn finish_disables_tracking_and_new_sessions_reset_counts() {
    assert_eq!(finish(), Counts::default());
    start();
    let retained = bytes(4096);
    let first = finish();
    assert!(first.allocations >= 1 && first.requested_bytes >= 4096);

    drop(retained);
    drop(bytes(65536));
    assert_eq!(
        finish(),
        Counts::default(),
        "inactive calls must not repeat the preceding session"
    );
    start();
    assert_eq!(
        finish(),
        Counts::default(),
        "a fresh empty session starts at zero"
    );
}

#[test]
fn another_threads_large_allocation_is_excluded_from_the_active_owner() {
    let barrier = Arc::new(Barrier::new(2));
    let child_barrier = barrier.clone();
    let child = std::thread::spawn(move || {
        // Prime synchronization before the allocation-counting interval.
        child_barrier.wait();
        start();
        child_barrier.wait();
        child_barrier.wait();
        drop(bytes(4096));
        finish()
    });
    barrier.wait();
    barrier.wait();
    let other_thread_bytes = 1 << 20;
    drop(bytes(other_thread_bytes));
    barrier.wait();
    let counts = child.join().unwrap();

    assert!(
        counts.allocations >= 1 && counts.requested_bytes >= 4096,
        "{counts:?}"
    );
    assert!(
        counts.requested_bytes < other_thread_bytes as u64,
        "the active child session must exclude the parent's large allocation: {counts:?}"
    );
    assert_eq!(finish(), Counts::default());
}
