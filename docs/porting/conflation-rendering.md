# Conflation rendering implementation

Status: implementation and source parity delivered. Physical mouse checks pass
on software and GPU after the dense software zoom fix.
Plan: `tasks/conflation-plan.md`.
The historical timing exception remains open and deferred.

## Source component and consumers

The complete `model/data-conflater.ts` defines recursive pair aggregation and
remainder folding, derived-row counts/identity, custom reducer contexts and
last-chunk updates. Constants define powers 2..512. Source hashes, including
current local edits to Series/DataLayer/TimeScale, are frozen in
[conflation-source.json](conflation-source.json).

Direct current-source reads supplement graph traces: generic/interface calls
from LinePaneViewBase, BarsPaneViewBase and CustomPaneView to conflatedBars are
missing graph edges. These three consumers cover the six built-ins and custom
rendering; their shared base selects visible derived rows and projects coordinates.
Color lookup still reads the canonical row at an aggregate's initial index.
Autoscale, first/last prices, magnet and API event data read canonical rows.
Series owns per-factor caches; DataLayer invokes latest repair and prohibits
historical changes when the effective per-series factor exceeds one. Custom
series without a reducer draw raw rows, even though the time-scale factor is
still supplied to their renderer. Optional cache warmup uses browser task APIs.
The upstream integration tests mock active-state checks, so actual Series source
determines the option-enabled/factor-1 boundary.

## Behavioral contract

Preserve threshold precedence (series/global/1), DPR-aware floored power-of-two
selection, 512 cap, recursive pair order and folded odd remainders. Canonical
rows, time indexing, source colors and exact-index events stay unchanged.
Line-family aggregates select the extreme farther from close, low on a tie;
OHLC families project first open/high/low/final close. Original counts accumulate.
The aggregate retains its first source index; grouping is independent of viewport.
Custom contexts are typed and callbacks cannot be invoked under a model borrow.
All prohibited public updates must fail before mutation. Retained frame geometry
must remain immutable after cache changes, source replacement and removal.

## Rust design, alternatives and ownership

Two designs were considered: caches in each pane view (easy initial projection
integration but duplicate state and separate low-level paths), or in Series
(one owner/invalidation path shared by public and low-level consumers). Choose
Series-owned derived summaries, revising the plan's initial preference after
reading actual `ChartModel::prepare_*` consumers. Memoization uses RefCell like
existing series read caches; it is synchronous, not shared across threads.

```text
Chart -> model -> existing Rc<RefCell<Series>> -> canonical rows
                                           -> owned conflation summaries
      -> prepared frame -> owned projected geometry
Chart -> custom adapter -> typed cached payloads + existing owned view
Retained custom handle -> model, weak view; no new owner cycles
```

Private API: `factor(spacing, dpr, threshold) -> usize`,
`ConflationCache::ensure(rows, level)` and bounded `step`, and
`Series::render_rows(time)` yielding a borrowed raw/cached drawing selection.
Summaries retain source positions, logical indices, counts and OHLC; original
rows supply styles. No new public data handles, callback identity APIs, WeakMap
hashes, unsafe TS casts or browser scheduling objects are reproduced. Custom
adapters own typed caches and release canonical model borrows before reducers.
Cache removal follows owner Drop; no strong closure back-references are added.

Implementation is opt-in; the default nonconflated rendering path remains available.

## First slice checkpoint (historical)

The actual-TS oracle passes 140 aggregation/count/index/remainder cases. Core
tests cover bounded construction, warm reuse and latest repair versus fresh
results for lengths 1..49 and levels 2..512. Dense public Line retains 8,192
canonical rows while reducing projected points; historical-update rejection is
transactional and depends on the effective factor. 400 library tests and the
two public conflation tests passed at that checkpoint. The remaining implementation
and automated verification are covered below; physical acceptance is separate.

## Native scheduling and invalidation

`Chart::prepare_snapshot` advances optional warmup, and pending work participates
in `Chart::is_animating`. The existing native widget requests a redraw deadline
and publishes Tick, matching the exact Iced Action/Shell APIs. Flowsurface's
heatmap widget forwards the same action to its Shell; no separate task runtime
is needed here. Priorities map to total per-preparation pair budgets: Background
128, UserVisible 512, UserBlocking 2048, shared across series. This bounds pair
work, not arbitrary application reducer execution time. On-demand cold rendering
still finishes its requested level synchronously. Low-level model users may build
on demand; automatic warmup is supplied by the public native Chart adapter.

Built-ins repair completed levels after latest replacement by reducing their last
source span. Append, shared reindex, full replacement and clear invalidate levels
and partial work. Custom payload caches use the existing pane-data generation;
latest updates rebuild them conservatively. Unchanged warm frames reuse levels.
There are nine possible cached levels; their row counts form a geometric series,
with at most one unfinished level. Typed payload allocation sizes remain the
application's responsibility. Removal drops the cache with its existing owner.

An upstream consumer detail matters for non-associative custom reducers:
`ChartApi._sendUpdateToChart` calls `Series.setData`, which clears the per-factor
cache before public drawing. The helper's incremental left fold can differ from
fresh recursive reduction, but the public path rebuilds. Rust preserves that
observable recursive result after custom updates. The actual ChartApi source is
included in the source hash manifest. Typed `original_time` values are retained;
JS's cast from internal time to user time is not reproduced.

The public error enum adds `HistoricalUpdateWithConflation(SeriesId)`; downstream
exhaustive matches need the new arm. Old `UnsupportedConflation` preparation enum
variants remain for compatibility, but supported built-ins no longer return them.

## Automated behavioral coverage

The actual-source oracle executes DataConflater and LinePaneViewBase, checking
182 aggregation/index/count/remainder states and their line-family representative
prices; 60 spacing/DPR/threshold combinations use the source calculator. An
order-sensitive custom reducer checks initial and latest results (2107 and 2109).
No-reducer custom views receive raw rows and the source global renderer factor.

Public tests compare all six families' warm latest/append/reload output with
fresh charts and verify retained frames. Other focused checks cover per-series
thresholds, canonical pointer events, factor-one historical updates versus active
rejection, optional work budgets/cancellation, custom model reads inside reducer
callbacks, removal, and short-height recovery at DPR 1/1.25/1.5/2 across normal,
log, percentage and indexed modes. A source-style regression checks an aggregate
whose direction reverses while its first candle's body/border/wick colors remain
canonical. Core tests prove exactly 1,024 aggregates from 8,192 rows at factor 8,
warm reuse, bounded storage and latest repair at lengths 1..49/all nine levels.

## Rendering, validation and review

The dense public demo and volume pane passed **144 captures** across TinySkia and
WGPU: all six families, regular/narrow/live scenes, DPR 1/1.25/1.5/2, plus
inversion/curved/stepped fill scenes for Area/Baseline. Capture assertions check
repeatability and complete coverage independent of the clear color. An Area image
was also inspected visually. A further **16 mixed-scene captures** (markers plus drawing primitives on dense
Line/volume scenes) pass across both backends. The custom box demo passed
**48 captures** across
both backends, including dense initial drawing, append, latest replacement,
narrow, inverted and manual scales. Pixel checks require the custom box color
and repeatable retained drawing. These are native renderer checks, not browser
pixel equality or physical-input acceptance. Artifacts remain under
`target/conflation/captures/` and `target/conflation/custom-captures/`.

The custom example's reducer intentionally shows the last box's inner quartiles
and outliers with combined outer whisker extrema. This is an application example,
not a claim to recompute statistical quantiles. `--conflation` loads 8,192 rows in
both public and custom demos. Default small demos retain their previous behavior.

Validation on 2026-10-17:

- `cargo fmt --check`, `cargo check --offline --all-targets` and
  `cargo clippy --offline --all-targets -- -D warnings`: pass.
- `cargo test --offline --all-targets`: 601 tests pass under the final dev profile,
  including 7 public conflation tests, 9 typed custom tests and the custom example
  test.
- `node scripts/check_conflation_reference.cjs`: 182 actual-source aggregation /
  line-family states, 60 factor states and order-sensitive custom update parity pass.
- Existing Line, Candlestick, Bar, Histogram, Area, Baseline, custom, incremental
  append, hover selection, markers, primitives and interaction reference scripts:
  all pass (13 source scripts including conflation).
- `python scripts/profile_conflation.py --self-test`: retained subprocess output,
  nonzero exit status and per-child memory collection pass.
- Diff review covered first-source style mapping, raw event/autoscale consumers,
  update invalidation, completed-level publication, custom callback borrowing,
  owner cleanup, work/storage bounds and disabled paths. No dependencies were added or upgraded; dev build overrides are described below.

Two fixture issues were corrected without changing the engine: cache comparisons
now fix minimum axis width to avoid comparing different viewports after label
width retention; source-oracle output parsing handles the Rust test harness's
prefix. A missing external `/usr/bin/time` aborted measurement setup before any
chart process ran. The runner now uses Linux `wait4` directly; that setup manifest
is retained at `target/conflation/performance/`. The completed finite batch below
has 24 executions and no retries.

## Bounded performance study

Frozen protocol and all 2,400 warm samples are committed in
[evidence/conflation/protocol.json](evidence/conflation/protocol.json) and
[evidence/conflation/results.json](evidence/conflation/results.json). The protocol
records binary/fixture hashes, toolchain, OS, CPU affinity availability and the
exact alternating order. Raw process stdout/stderr remain under
`target/conflation/performance-batch/`. Reproduce with:

```sh
cargo build --offline --release --example conflation_profile
python scripts/profile_conflation.py target/conflation/new-batch
```

One fixed study: Line/Histogram, 8,192/100,000 rows, three alternating on/off
pairs; 24 serial release processes, each with 20 warmups and 100 warm samples.
Viewport 1,000×500, DPR 1, full history, precompute disabled, no renderer.
No builds, tests, graph indexing or captures overlapped it. Processes were not CPU
pinned and ordinary desktop load was not controlled; per-run load is recorded.
Cold preparation follows factor-one font/axis setup. Latest/append are one probe
per process after the warm window, not latency-distribution acceptance screens.
Memory is whole-process peak RSS, including canonical setup, not isolated cache
allocation. All measurements below are medians across the three processes.

| Series | Rows | Conflation | Projected | Warm µs | Cold µs | Latest update / prepare µs | Append update / prepare µs | Peak MiB |
|---|---:|---|---:|---:|---:|---:|---:|---:|
| Line | 8,192 | off | 8,192 | 1293.6 | 3991.2 | 10.1 / 1348.0 | 100.2 / 1944.0 | 39.7 |
| Line | 8,192 | factor 8 | 1,024 | 495.2 | 2997.8 | 10.2 / 626.3 | 236.3 / 1091.7 | 39.0 |
| Line | 100,000 | off | 100,000 | 14885.1 | 22008.6 | 11.6 / 16482.8 | 832.9 / 20233.9 | 390.2 |
| Line | 100,000 | factor 64 | 1,562 | 642.1 | 7730.9 | 13.3 / 2145.7 | 1565.5 / 6303.6 | 357.7 |
| Histogram | 8,192 | off | 8,192 | 1413.0 | 4129.8 | 10.4 / 1488.6 | 147.9 / 2002.6 | 39.1 |
| Histogram | 8,192 | factor 8 | 1,024 | 452.9 | 2911.8 | 10.8 / 581.3 | 284.3 / 1074.2 | 39.0 |
| Histogram | 100,000 | off | 100,000 | 15750.5 | 22970.5 | 12.4 / 17650.5 | 814.4 / 19595.7 | 382.5 |
| Histogram | 100,000 | factor 64 | 1,562 | 607.9 | 7852.3 | 13.0 / 2086.6 | 1480.1 / 6334.6 | 358.9 |

The on/off warm-process ranges are separated in all four cohorts. Median warm
ratios are 2.61×/23.18× for Line and 3.12×/25.91× for Histogram at the smaller/larger
histories. This supports a CPU preparation speed claim for these dense fixtures,
not a claim about display FPS, GPU presentation or every workload. Append update
alone is slower when dropping cached summaries; append preparation is faster.
Cold builds and append rebuilds remain history-dependent. No claim of constant-time
append, constant-time cold rendering or a fixed frame deadline is made.

## Changed files and remaining dependencies

Engine/model: `src/model/data_conflater.rs`, `mod.rs`, `series_data.rs`,
`time_scale.rs`, `series.rs`, `series/conflation.rs`, and `series/line_pane_view.rs`, `series/area_pane_view.rs`,
`series/baseline_pane_view.rs`, `series/bars_pane_view.rs`,
`series/candlesticks_pane_view.rs`, `series/histogram_pane_view.rs`. Public/native adapter: `src/api/chart.rs`, `mod.rs`, `conflation.rs`,
`custom_series.rs`, `custom_series/conflation.rs`, `src/ui/custom_series.rs`,
`multi_pane_frame.rs`. Behavioral tests: `tests/conflation.rs`,
`tests/public_custom_series.rs`, and `src/model/chart_model/tests.rs`,
`src/model/chart_model/tests/candlesticks.rs`,
`src/model/chart_model/tests/histogram.rs`. Fixtures/tooling: public/custom demos and custom capture/box
modules, `examples/public_chart/software_drag_profile.rs`,
`examples/conflation_profile.rs`, source oracle and profile runner. `Cargo.toml`
sets the software dependency dev profiles described below.
Documentation: README, this source/ownership/validation report, source hash
manifest, committed timing evidence and the plan execution checklist.

No missing related component blocks rendering the supported series. Later work
may improve append/custom-update rebuild costs if a measured workload needs it;
that is an optimization, not permission to change reduction order. Last-price
animation remains a separate feature. Physical mouse checks for dense zoom/live feed pass on both backends after the
software zoom fix below. Touchpad, touchscreen and high-DPI hardware coverage
remains untested. The historical timing exception stays OPEN/deferred,
with unchanged acceptance thresholds and no acceptance rerun in this feature study.

## Dense software zoom diagnosis and fix

The first physical software test found delayed/stuttering wheel zoom. Other
requested gestures worked. This was a new dense-demo regression investigation,
not a rerun or closure of the historical timing exception.

`public_chart --conflation --dense-zoom-profile` reproduces two panes, 8,192 rows,
regular/narrow sizes, live appends and repeated wheel-sized zoom steps across
factor boundaries. It measures model mutation, preparation, canvas geometry and
software raster/screenshot separately. All 160 frames per build are retained in
[evidence/conflation/zoom](evidence/conflation/zoom/summary.json), including the
intermediate configurations. Summaries omit the first ten frames per size equally
in each build. These are diagnostic comparisons, not new acceptance deadlines.

| Build | Size | Model ms | Prepare ms | Geometry ms | Raster/screenshot ms |
|---|---|---:|---:|---:|---:|
| unoptimized | 800×500 | 0.044 | 11.217 | 2.690 | 277.073 |
| unoptimized | 300×300 | 0.039 | 7.242 | 1.120 | 68.785 |
| release | 800×500 | 0.007 | 1.402 | 0.574 | 12.605 |
| release | 300×300 | 0.003 | 0.701 | 0.208 | 3.401 |
| final-dev | 800×500 | 0.039 | 11.278 | 1.231 | 15.102 |
| final-dev | 300×300 | 0.018 | 7.022 | 0.478 | 4.233 |

`perf record -F 99 -e cpu-clock:u --call-graph dwarf,16384` collected 626 CPU
samples without lost samples. The intermediate optimized build still spent much
of its CPU in TinySkia's `u16x16` shift/multiply/add and low-precision blending.
Exact installed 0.11.4 source shows sixteen-lane scalar arithmetic intended for
compiler inlining/vectorization. Optimizing TinySkia/path and Iced's software
adapter reduced the cost; using release-style integer overflow semantics in
TinySkia alone removed further per-lane overhead. The source and timing evidence
support compiler treatment of this raster arithmetic as the cause of the large
debug-build penalty. No aggregation algorithm change was needed.

Cargo now optimizes these three software-renderer dependencies in dev builds and
sets `overflow-checks = false` only for `tiny-skia`. Chart code remains unoptimized,
with its overflow checks, debug assertions and input validation enabled. TinySkia
uses its production release integer arithmetic semantics; its debug assertions
remain enabled. These workspace profile settings do not propagate to downstream
applications that depend on this library. Release builds and the frozen 24-run
CPU preparation study are unaffected.

The screenshot phase includes buffer allocation and RGBA conversion beyond native
rasterization. Its numbers locate a large software cost; they are not native
presentation latency. Final acceptance therefore used the real native window:
the user reported responsive software zoom at regular/narrow sizes after the fix,
and all GPU wheel zoom, diagonal pan and price-axis wheel/drag checks passed at
both sizes with live feed. The first software test had passed the other gestures.

All 192 before/after built-in and custom images are byte-identical under the
final dev profile. Final images remain under `target/conflation/final-captures/`. The raw perf data is
retained at `/tmp/feather-conflation-zoom.data`; the symbol report is committed with
the diagnostic CSVs. One addr2line warning limits source-stack attribution; the
reported hot symbols and measured pipeline stages are the evidence used here.
