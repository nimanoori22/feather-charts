//! Synchronous allocation diagnostic: one sample appends one shared time to all sources.
use feather_charts::model::{
    data_consumer::{BuiltInSeriesDataItem, LineData, LineDataItem},
    data_layer::{DataLayer, SeriesId},
    horz_scale_behavior_time::{
        horz_scale_behavior_time::HorzScaleBehaviorTime,
        types::{Time, TimePoint, UtcTimestamp},
    },
    series_data::SeriesDataRow,
    series_options::SeriesType,
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
};

struct CountingAllocator;
static ENABLED: AtomicBool = AtomicBool::new(false);
static BYTES: AtomicUsize = AtomicUsize::new(0);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static REALLOCS: AtomicUsize = AtomicUsize::new(0);
static DEALLOCS: AtomicUsize = AtomicUsize::new(0);
static LARGEST: AtomicUsize = AtomicUsize::new(0);

fn allocated(bytes: usize, realloc: bool) {
    if ENABLED.load(Relaxed) {
        BYTES.fetch_add(bytes, Relaxed);
        if realloc {
            REALLOCS.fetch_add(1, Relaxed);
        } else {
            ALLOCS.fetch_add(1, Relaxed);
        }
        LARGEST.fetch_max(bytes, Relaxed);
    }
}

// Each method forwards the caller's allocation contract unchanged to System.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        allocated(layout.size(), false);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        allocated(layout.size(), false);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        allocated(new_size, true);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ENABLED.load(Relaxed) {
            DEALLOCS.fetch_add(1, Relaxed);
        }
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[derive(Clone, Copy, Default)]
struct Counts {
    bytes: usize,
    allocs: usize,
    reallocs: usize,
    deallocs: usize,
    largest: usize,
}
impl Counts {
    fn start() {
        for counter in [&BYTES, &ALLOCS, &REALLOCS, &DEALLOCS, &LARGEST] {
            counter.store(0, Relaxed);
        }
        ENABLED.store(true, Relaxed);
    }
    fn stop() -> Self {
        ENABLED.store(false, Relaxed);
        Self {
            bytes: BYTES.load(Relaxed),
            allocs: ALLOCS.load(Relaxed),
            reallocs: REALLOCS.load(Relaxed),
            deallocs: DEALLOCS.load(Relaxed),
            largest: LARGEST.load(Relaxed),
        }
    }
    fn add(&mut self, other: Self) {
        self.bytes += other.bytes;
        self.allocs += other.allocs;
        self.reallocs += other.reallocs;
        self.deallocs += other.deallocs;
        self.largest = self.largest.max(other.largest);
    }
}

fn row(index: usize) -> BuiltInSeriesDataItem<Time> {
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: UtcTimestamp::new(1_700_000_000. + index as f64 * 60.).into(),
        value: 100.,
        color: None,
        custom_values: None,
    }))
}

fn append(
    layer: &mut DataLayer<HorzScaleBehaviorTime>,
    sources: usize,
    index: usize,
) -> (Counts, Counts) {
    let mut total = Counts::default();
    let mut first = Counts::default();
    for source in 0..sources {
        let input = row(index);
        Counts::start();
        let response = layer
            .update_series_data(SeriesId::new(source as u64), input, false)
            .unwrap();
        let counts = Counts::stop();
        assert_eq!(response.series.len(), 1);
        assert_eq!(response.series.values().next().unwrap().rows.len(), 1);
        assert_eq!(response.time_scale.base_index, Some((index as f64).into()));
        if source == 0 {
            first = counts;
        }
        total.add(counts);
        drop(response);
    }
    (total, first)
}

fn run(rows: usize, sources: usize) {
    let mut layer = DataLayer::new(HorzScaleBehaviorTime::default());
    for source in 0..sources {
        layer
            .set_series_data(
                SeriesId::new(source as u64),
                SeriesType::Line,
                (0..rows).map(row).collect(),
            )
            .unwrap();
    }
    let (cold, first) = append(&mut layer, sources, rows);
    println!(
        "rows={rows} sources={sources} cold bytes={} allocs={} reallocs={} deallocs={} largest={} first_owner_bytes={}",
        cold.bytes, cold.allocs, cold.reallocs, cold.deallocs, cold.largest, first.bytes
    );
    for index in rows + 1..=rows + 20 {
        append(&mut layer, sources, index);
    }
    let mut samples = [Counts::default(); 100];
    let mut first_owner = [0; 100];
    for (sample, index) in (rows + 21..rows + 121).enumerate() {
        let (total, first) = append(&mut layer, sources, index);
        samples[sample] = total;
        first_owner[sample] = first.bytes;
    }
    let realloc_samples = samples.iter().filter(|s| s.reallocs != 0).count();
    let mut allocations = samples.map(|s| s.allocs);
    allocations.sort_unstable();
    samples.sort_unstable_by_key(|s| s.bytes);
    first_owner.sort_unstable();
    println!(
        "rows={rows} sources={sources} warm bytes_median={} bytes_p95={} bytes_max={} allocs_median={} allocs_p95={} allocs_max={} realloc_samples={realloc_samples} reallocs_total={} largest={} first_owner_bytes_median={} first_owner_bytes_p95={} first_owner_bytes_max={}",
        samples[49].bytes,
        samples[94].bytes,
        samples[99].bytes,
        allocations[49],
        allocations[94],
        allocations[99],
        samples.iter().map(|s| s.reallocs).sum::<usize>(),
        samples.iter().map(|s| s.largest).max().unwrap(),
        first_owner[49],
        first_owner[94],
        first_owner[99]
    );
}

fn main() {
    println!(
        "SeriesDataRow<TimePoint,Time> size={} bytes; requested bytes count full new_size on realloc; each sample appends all sources; warmup=20 samples=100",
        std::mem::size_of::<SeriesDataRow<TimePoint, Time>>()
    );
    for rows in [1200, 100_000] {
        for sources in [1, 16] {
            run(rows, sources);
        }
    }
}
