# Live-append acceptance investigation

Follow-up to [incremental append](incremental-append.md), 2026-10-15.
Starting engine `27b543e`, evidence/checklist checkpoint `066fda5`.
Live-feed checkpoint `79eb94c`; shared-cache checkpoint `f69bd11`.
The original 10% screens and all failed samples remain unchanged and retained.

## Diagnosis before changes

The Linux-only `update_latency_probe` uses the exact public-route profiler fixture,
20 warmups and 1,000 diagnostic samples. Inputs and counter reads are outside the
update wall interval; snapshot/glyph preparation follows each update. It records
thread CPU time, faults and context switches. Optional own-thread, user-space
hardware counters expose retired instructions, cycles, generic cache misses and
perf enabled/running time. Unsupported platforms report the diagnostic unavailable.
No production instrumentation, dependency or system-setting change is required.
Counters perturb the process; these are diagnostics, not acceptance timings.

Two alternating-history diagnostic pairs show 100k Line append p95 growth in one
pair but not the other. All 2,000 large-history samples have zero faults/context switches;
wall/CPU correlation exceeds .995. Latest-Candlestick p95 is 44–46 µs in these runs;
middle-Line p95 is 8.8–9.0 ms. Faults explain only some middle outliers.

In the hardware-counter run, 100k versus 1,200 histories retire 3.7% more instructions,
while wall time grows 52.2%, CPI grows 45.6%, and effective cycles/CPU-nanosecond grows
only 1.3%. All events run without multiplexing. The slowest 5% of large-history appends retire
essentially unchanged instructions, with about 3× CPI and 4× generic cache misses.
This supports memory/cache costs in those measured tails; it does not identify a
specific cache level or explain every previous outlier.

A separate model-phase diagnostic finds normalization 0.934/0.953 µs at 1,200/100k,
model application 10.267/11.313 µs and settlement 4.541/5.006 µs. That diagnostic uses
the existing 800×500 model fixture, not the public 917×870.80005 acceptance fixture.
The autoscale path reads the visible OHLC rows again when unrelated tail data
changes, even if the requested logical range and its contents remain unchanged.

Raw diagnostics, immutable existing profilers and native captures are retained in
`target/incremental-append/latency-diagnosis/`.

## Candidate design: raw autoscale reuse

The complete upstream `plot-list.ts`, its min/max tests and actual
`Series._autoscaleInfoImpl` caller were read. Graph metadata is stale upstream;
the graph also misses the generic call edge, so current source/literal references
are the evidence. Rust Series/PlotList metadata matched and the caller was read.
Required behavior is inclusive range min/max over Close or Low/High, NaN skipping,
empty/gapped rows, and correct replacement. Histogram base, marker margins,
primitive callbacks and user autoscale providers must continue to be evaluated.

Two viable designs:

1. Reproduce upstream per-plot chunk maps in PlotList. This supports reusable
   chunks across moving windows, but adds invalidation/storage and interior cache
   state to a public container that currently preserves its automatic Sync trait.
2. Retain one raw min/max query in Series, whose existing state already uses Cell.
   Cache only requested bounds and copied raw min/max; preserve it for row changes
   outside those bounds, invalidate intersecting updates/full replacement, and
   leave all decorations/providers outside the cache.

Choose (2) as the smallest candidate for the observed repeated visible-range work.
It owns only copied numbers, adds no references/cycles/threads and keeps public
Chart/PlotList APIs and trait guarantees unchanged. Series owns both its rows and
cache; dropping it requires no special cleanup. No JS shared identity is needed.
Bounds change naturally replaces the single entry; memory use is constant.
The candidate is retained only if behavioral checks pass and isolated measurements
show a gain beyond run-to-run variation. No new cache is justified by its name alone.

## Hands-free demo feed

Both public_chart and custom_series provide a Start/Stop live feed control and
`--live-feed` startup flag. Iced's standard timer delivers one append every 250 ms;
its Tokio timer feature is a dev dependency for examples/tests. Normal library
users acquire no new runtime feature from this declaration. Existing append paths
apply data; stopped/stale-session ticks are ignored, and custom removal stops its
feed. The fixed-width control avoids resizing the chart on start/stop.

The feed's tests were RED before implementation, then passed. Actual native demo
windows were opened with live feeds, captured twice and closed. Both mapped at
611×1,030; chart-region pixel changes confirmed updates continued without input.
These native smoke checks do not certify physical mouse/touchpad/touchscreen feel.


The RED operation tests reproduced 81 old visible rows read on repeated queries;
seven of eight failed before caching. All eight then passed. A separate randomized
oracle checks 600 mutations for each of six built-ins against direct uncached
min/max, including NaNs, sparse rows, full replacement and repeated/range changes.
The row-read observer is test-only; release code has no counter overhead.

The first uninstrumented pilot with 1,000 samples at 16 sources measured Line
append medians 21.596→10.423 µs at 1,200 rows and 20.104→9.475 µs at 100,000 rows.
p95 changed 60.093→16.016 and 42.491→13.832 µs respectively. Frozen profilers use
matching example timer features. The complete three-set matrix determines the
final acceptance result below; the pilot alone does not close any gate.


## Raw-range cache comparison

The first complete three-set comparison uses the incremental engine as baseline,
with the same dev timer feature and unchanged profiler sources in both artifacts.
It retains all 204 executions/61,200 samples under `latency-diagnosis/cache-acceptance/`.
The [phase CSV](raw-range-cache-measurements.csv) includes every median/p95.
The unchanged history screen passes 18/24; six failures remain (Line16 runs1/2/3,
Histogram16 run1, Custom16 runs1/3). No threshold or sample filter was changed.
Built-in latency improves substantially despite this open relative-growth gate.
Custom extrema/first-value queries are not yet cached in this first candidate.
Further investigation targets the still repeated first-visible-row binary searches.


## First-value and typed-custom follow-up design

PriceScale and pane/axis/price-line preparation repeatedly request the first
fulfilled row at or after the visible left edge. Original Series.firstBar/firstValue
and current Rust consumers were traced/read; the returned row can lie beyond the
visible right edge. A key based on both bounds would therefore do unnecessary work.

Retain a copied first value keyed only by the left edge. Invalidate it on a mutation
at/after that edge and at/before the cached first row, or any future qualifying row
if the cached result is missing. Whole replacement and reindex clear state; pop invalidates the removed indexes.
Raw min/max retains its separate inclusive-range invalidation. Two small Cell
entries share one private cache policy between built-in and typed custom owners;
closures compute missing values synchronously and are never retained. Native rows
and typed payloads stay with their existing owners. This also avoids changing the
public PlotList container's automatic Sync behavior or returning cached row borrows.

The alternative of retaining a whole history of search results/chunk extrema adds
storage and invalidation beyond the single active viewport needed here. Public
method signatures stay unchanged; only private synchronous cache helpers are added.
Histogram bases, marker margins, primitives and user providers remain outside both
raw caches and execute on every query. No data-builder callback is cached.


The first-value tests were RED (all eight failed) before the shared policy.
After implementation, all 37 series tests pass, including the nine raw-range
checks and 3,600-mutation oracle. A selective-pop guard caught an unnecessary
reset for a removed row beyond the cached first row; invalidation now checks the
removed logical index instead of clearing unaffected entries.

The second 1,000-sample pilot compares the raw-range-only candidate with the shared
policy. At 16 sources, Line update medians change 7.817→5.975 µs (1,200 points)
and 8.987→6.135 µs (100k); typed custom changes 23.804→5.811 µs and
25.816→5.770 µs. The complete follow-up matrix compares the original incremental
engine (before either cache) with the final shared policy; no measurement budget,
screen threshold, warmup count or outlier handling is changed.


## Final shared-cache comparison

The final matrix retains 204 executions, 612 phase summaries and 61,200 warmed
samples in `latency-diagnosis/data-cache-acceptance/`. Both artifacts have the same
example timer features; baseline is the incremental engine before either cache.
The [complete CSV](series-data-cache-measurements.csv) includes all phases/runs.
All runs use CPU 5, 20 warmups, 100 samples and the original 10% growth screen.
No acceptance measurements overlap builds, capture runs or diagnostic probes.

**23/24 history screens pass**, versus 19/24 in the original engine comparison.
The remaining failure is Histogram, 16 sources, run 3: median grows
7.544→7.977 µs (+5.74%); p95 grows 12.176→19.432 µs (+59.59%).
That failure remains in the full matrix and keeps the strict performance gate open.

At 100,000 rows and 16 sources, update medians/p95 across the three sets are:

| Kind | Before median (µs) | After median (µs) | Before p95 (µs) | After p95 (µs) |
| --- | --- | --- | --- | --- |
| Line | 16.06–16.83 | 6.12–6.19 | 26.35–29.90 | 7.61–9.89 |
| Candlestick | 21.64–22.31 | 6.61–6.87 | 38.36–48.87 | 11.29–13.64 |
| Histogram | 17.44–38.41 | 6.49–7.98 | 32.20–84.03 | 10.25–19.43 |
| Typed custom | 25.76–26.57 | 5.77–6.06 | 40.00–44.56 | 9.95–11.00 |

Latest-Candlestick p95 is 9.745/11.331/10.532 µs after caching, compared with
37.149/35.532/39.610 µs in the matched baseline. Middle insertion p95 is
7.781/7.900/7.967 ms, versus 7.821/7.750/7.944 ms. The earlier latest/middle
outliers did not recur in these three complete sets. Reload remains near baseline;
latest/middle/reload update median/p95 changes stay within 10% here.
This does not prove that every possible latency tail has disappeared.

### Remaining Histogram tail

Three additional alternating 1,200/100k pairs use the exact immutable final profiler,
20 warmups and 100 samples, with no production changes. Their median/p95 growth is
−11.30%/−18.11%, −7.44%/+3.09%, and −16.07%/−19.40%: all pass.
These focused repeats are saved as `histo-repeat-*`; they do not replace or erase
the failed full-matrix screen.

A separate 1,000-sample hardware pair does not reproduce the history regression.
At 100k all samples have zero faults/context switches; no event is multiplexed.
The slowest 5% retire only 1.008× the instructions of the middle 50%, with 1.810×
CPI and 18.13× generic cache misses. This supports cache-related tail variation in
that diagnostic; it cannot by itself certify the unchanged acceptance gate.
Raw `histo-perf-*` evidence remains separate from uninstrumented timing evidence.

## Scope and remaining acceptance

Production changes are confined to the private Series/CustomSeries raw-range and
first-value cache in `src/model/series.rs`. `plot_list.rs` adds test-only observers.
Two separate test modules cover invalidation, current provider/decorator behavior,
missing values, boundaries, custom reindex/pop and randomized uncached equivalence.
The native example controls, profiler runner and Linux-only diagnostic are supporting
tools. Upstream plot-list chunk maps, callback identity, mutable JS references and
manual destruction are unnecessary for these copied single-entry values.

No public API is added. Important private signatures are
`invalidate(&mut self, Option<TimePointIndex>)`,
`first_value(&self, TimePointIndex, impl FnOnce() -> Option<FirstValue>)` and
`raw_range(&self, TimePointIndex, TimePointIndex, impl FnOnce() -> Option<MinMax>)`.
Existing model/scale/view callers continue using the same synchronous Series API.
The engine owns its data; cache entries own copied numbers; computation closures
are not stored. Full reindex/replacement invalidates caches conservatively.

Physical mouse/touchpad/touchscreen acceptance remains a separate human task.
Automated behavior/capture checks and real native feed smoke tests cover engineering
verification; they cannot judge the feel of a physical device. No physical test is
marked passed by inference. Conflation rendering remains a later, separate milestone.

### Other retained phase variation

The static update interval performs no chart update: it times empty control flow
and clock overhead. Its median changes 62→133 ns, 56→77 ns, 71→66 ns across the
three pairs; corresponding p95 values are 118→241 ns, 177→200 ns, 148→168 ns.
These raw numbers are retained, without treating them as chart-engine work.

Static glyph, run 2: median 0.071666→0.069648 ms; p95 0.079756→0.101868 ms. This separate phase variation remains visible in the CSV.

Reload glyph, run 3: median 0.057992→0.057438 ms; p95 0.062690→0.077208 ms. This separate phase variation remains visible in the CSV.


## Validation and review

- `cargo fmt --all -- --check`: passed.
- `cargo test --offline --all-targets`: 558 passed, 0 failed, across 48 targets.
- `cargo check --offline --all-targets`: passed.
- `cargo clippy --offline --all-targets -- -D warnings`: passed after fixing the
  custom demo's initializer; its focused example test also passed afterward.
- `cargo build --offline --examples`: passed.
- All 20 Node source-reference checks passed.
- Fresh complete renderer matrices passed: 492 tiny-skia and 492 wgpu captures
  (984 total), including all six built-ins, hover, markers, editing and custom
  series at the configured ratios/narrow sizes. Earlier engine captures remain
  separately preserved under `captures-engine27/`.
- Both native live-feed demos passed the final smoke check at 611×1,030;
  public/custom chart regions changed 42,326/41,686 pixels without input.
- The final measured artifact source hashes match the production cache files.
  All 612 checked-in CSV summaries match the raw JSON; all 204 logs retain their
  100 samples for each of three phases (61,200 samples).

Review covered each data mutation path and the scale/model/view consumers.
Range keys use inclusive logical indexes, first-value keys use only the visible
left edge, and missing answers are cached with the same selective invalidation.
Decorators/providers are evaluated outside the raw cache. No new library runtime,
public trait guarantee, data ownership or renderer-specific behavior is introduced.

Exact changed files for this follow-up:
`Cargo.toml`, `Cargo.lock`, `examples/public_chart.rs`,
`examples/public_chart/tests.rs`, `examples/custom_series.rs`,
`src/model/series.rs`, `src/model/plot_list.rs`,
`src/model/series/min_max_cache_tests.rs`,
`src/model/series/first_value_cache_tests.rs`, `examples/update_latency_probe.rs`,
`scripts/profile_incremental_append.py`, `README.md`,
`tasks/incremental-append-todo.md`, this report, the two measurement CSVs and
`docs/porting/incremental-append.md`.


## Reproduction

Run the diagnostic independently of acceptance timing or other CPU-heavy work:

```sh
cargo build --offline --release --example update_latency_probe
taskset -c 5 target/release/examples/update_latency_probe --case append-histogram --rows 100000 --sources 16 --samples 1000 --instructions
```

Use an available CPU on another machine. Omit `--instructions` for thread-only
diagnostics; permission failures report unavailable hardware events without changing
kernel settings. Compare both 1,200 and 100,000 rows. The other diagnostic cases
are `append-line`, `latest-candle`, and `middle-line`. Instrumented results are not
substitutes for the uninstrumented acceptance matrix.

Frozen final artifact SHA-256 identities (also in the local matrix manifest):

| Artifact | SHA-256 |
| --- | --- |
| Baseline | `dd1e5833bc5206a4b7aa888c2ed208fc7604c1a2154e274c1f0b19b2f057363a` |
| Baseline custom | `5aaf5fcee741dcd90f3d6c8fb891b8ab20bab7cfd13ef4b4abc830b342bc17fe` |
| Shared cache | `cb7d75974773e14d72c8921653a35a9b151968a2d8b5a5f2b65d9b8b66746b56` |
| Shared cache custom | `eb09ac3de35af4e9ffe31097377444e43b1a4dcb5200ae4e77dd27e30e47fe16` |
