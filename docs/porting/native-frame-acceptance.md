# Native frame acceptance and shared pane preparation

Historical study. See the [current follow-up](render-update-follow-up.md) for the
software drawing fix and subsequent acceptance results; original failures below
remain part of the record.

Follow-up to [the timing investigation](histogram-window-cause.md), 2026-10-15.
The [acceptance policy](frame-acceptance-policy.md) fixes the decision rules before
the acceptance runs. The historical 23/24 screen and all failed pilots remain
recorded. Physical-device feel is a separate, unfinished acceptance item.

## Measurement

`examples/frame_latency_profile.rs` runs the public ChartProgram event/draw path
and the exact Iced renderer/compositor used by the demos in a minimal winit host.
It records update/input, preparation, geometry, render/present call, thread CPU
time and actual Wayland `wp_presentation` feedback. Submission return and redraw
events are not substituted for presentation. A changing 4×4 stamp guarantees
damage when an offscreen append otherwise leaves the displayed chart unchanged.
This does not measure the demos' surrounding controls or physical input delivery.

The Linux-only example uses already-locked winit/Wayland/libc packages as direct
development dependencies. No packages were added to the lockfile and no runtime
dependencies or chart-engine async/threading requirements changed.

The presentation probe is a guest of winit's system Wayland connection. It owns
its event queue and presentation proxy; a retained `Arc<Window>` outlives every
guest object and keeps the foreign surface/display alive. UI-thread callbacks
drain feedback synchronously. There is no ownership cycle. The two unsafe FFI
conversions use the raw handles from that retained window. Missing protocol or
feedback invalidates measurement instead of estimating presentation.

The first pilots used a 60 Hz monitor, scale 1 and a 917×871 physical surface.
All 480 feedback events resolved, with no discarded frames. The software pilot's
render/present wall time and thread CPU time were both approximately 22 ms,
establishing real CPU work rather than buffer waiting as its dominant cost.
Stage probes attributed almost all preparation time to snapshot preparation,
not glyph conversion; axis measurement alone was a small part of that snapshot.
Temporary counters were removed after diagnosis. The host waits for redraw
delivery instead of spinning on an expired frame deadline.

## Reproduced defect and design

`prepare_series_plot` generated complete pane-wide axes, grid, all price lines
and crosshair geometry for every source. The composed owner retained those fields
only from its first source. Extra sources' copies were immediately discarded.
The repeated all-source decoration work amplified the cost as sources increased.

Upstream `src/gui/pane-widget.ts` paints the pane grid separately from its ordered
source groups and draws crosshair content separately. The existing Rust consumers
are the single-pane frame owner, standalone line-chart preparation and composed
multi-pane preparation. The first two require complete pane content. Composed
preparation requires that content once, plus each source's geometry, markers and
hit targets. Custom sources use the same composition boundary.

Two viable designs were a retained pane-content cache and a distinct scope for
additional source preparation. The latter is smaller and needs no new cache keys,
invalidation, reference sharing or lifecycle rules. Internal `PlotContent::Pane`
and `PlotContent::Series` select that scope in the existing preparation function.
Every preparation still reads fresh model state. The first source owns complete
pane content; subsequent sources contribute owned immutable geometry and hits.
Drawing order, coordinate conversion, clipping, hover raising, primitives and
public APIs remain unchanged. No TypeScript class/identity or callback machinery
was introduced. This fixes a Rust composition inefficiency, not a missing port.

The regression disables redundant sources' labels and observes a shared scale
formatter. The old path made 2,212 calls with 16 Line sources versus 22 with one.
The corrected path makes 262, within a linear-work bound of 16×22. The test permits
legitimate per-source formatting rather than specifying incidental exact counts.
Both Line and Histogram cases retain all source geometry/hit identities and the
pane's axes/grid. RED and GREEN logs are retained.

The matched regular-size wgpu pilots show p95 values in milliseconds:

| Phase | Preparation before / after | Frame work before / after | Presentation before / after |
| --- | ---: | ---: | ---: |
| Append | 5.281 / 1.408 | 7.024 / 3.117 | 33.082 / 24.361 |
| Zoom | 5.314 / 1.309 | 12.528 / 4.519 | 38.911 / 33.043 |
| Pan | 5.943 / 1.300 | 15.820 / 5.270 | 38.910 / 33.043 |
| Price axis | 5.331 / 1.230 | 13.260 / 4.475 | 38.902 / 33.047 |

Pilots are diagnostic evidence, not the final matrix. Artifacts, including the
failed pilots and stage-counter logs, are under `target/frame-acceptance/`.

## Reproduction

Build once before measuring, then run serially without competing builds, tests,
captures or profilers. The native command requires Wayland presentation feedback
and opens chart windows. Use fresh output directories; existing evidence is never
overwritten by these runners.

```sh
cargo build --offline --release --example frame_latency_profile --example preparation_profile
python3 scripts/profile_chart_frames.py --output target/native-frame-new
```

The native runner records binary/protocol hashes before the first run, completes
each eight-execution set and stops before further sets if any cohort fails or is
inconclusive. PASS requires all three sets. Warmups, discarded and unresolved
feedback are retained; discarded frames count as infinite latency.

For the fixed update guard, place the release `preparation_profile` executable in
a fresh directory as `current-profile`, then run:

```sh
python3 scripts/profile_incremental_append.py --histogram-tail --directory target/update-acceptance-new --cpu 5
python3 scripts/frame_acceptance.py --updates target/update-acceptance-new
```

The first command retains its original strict nonzero exit on an exceedance.
The second evaluates complete raw 2,000-sample pairs and complete same-history
controls under the explicit policy. It does not alter the raw strict screen.
Choose an available CPU affinity; this example does not change system policy.

## Acceptance results and decision

**The exception remains OPEN. The milestone is not accepted.** The historical
original screen remains 23/24. Frozen artifacts, source hashes, compiler and
raw-file hashes are recorded in [the artifact manifest](frame-acceptance-artifacts.json).

The isolated six-pair update execution passes **4/6** complete 2,000-sample
comparisons. Its first two large-history update median/p95 values are
12.248/37.445 µs and 9.656/26.278 µs, versus paired small-history values
6.535/16.935 µs and 6.428/13.240 µs. The remaining four comparisons pass.
Complete same-history controls have no positive exceedance under the fixed rule,
so the update decision is **FAIL**, not INCONCLUSIVE. This does not establish
causality: the first consecutive large-history control improves rather than
demonstrating stable execution, and later large-history medians return to about
6.4 µs. The [six comparisons](frame-update-measurements.csv) and
`target/frame-acceptance/update-decision.json` retain exact results, complete
controls and descriptive short-window flags (59/120 history, 34/120 same-history).
No samples were filtered.

A preceding attempt accidentally overlapped image checksum verification. Its
entire six-pair result is excluded as contaminated, even though it passed 6/6.
All logs and the invalidation reason remain under `update-acceptance/`; the valid
attempt is `update-acceptance-isolated/`. The checksum job had finished before
the isolated attempt. Builds, tests, captures and other task profilers did not
overlap the isolated update, native-frame or subsequent counter measurements.

The native set contains **3,840 raw frames**, including warmups, and **32 measured
phase cohorts**. All feedback resolved, with zero discarded frames. Windows
reported scale 1 and monitor refresh 60,000 millihertz. Regular surfaces were
917×871 physical pixels; narrow surfaces were 420×320. There was no backend
fallback or unexpected post-warmup size change. **23/32 cohorts pass** both budgets.

The table reports the worst per-phase p95 in each execution, in milliseconds:

| Backend | Starting rows | Size | Work p95 | Presentation p95 | Passing phases |
| --- | ---: | --- | ---: | ---: | ---: |
| tiny-skia | 1,200 | Regular | 29.085 | 49.772 | 0/4 |
| tiny-skia | 1,200 | Narrow | 13.708 | 33.015 | 4/4 |
| tiny-skia | 100,000 | Regular | 28.286 | 47.953 | 0/4 |
| tiny-skia | 100,000 | Narrow | 16.327 | 35.465 | 3/4 |
| wgpu | 1,200 | Regular | 6.213 | 33.021 | 4/4 |
| wgpu | 1,200 | Narrow | 5.560 | 31.530 | 4/4 |
| wgpu | 100,000 | Regular | 6.134 | 33.059 | 4/4 |
| wgpu | 100,000 | Narrow | 8.622 | 33.043 | 4/4 |

The [phase CSV](native-frame-measurements.csv) retains exact nanosecond values.
The runner stops before sets two and three because set one fails. All 16 GPU
cohorts pass this first set; this is not three-set GPU acceptance. Regular-size
software render/present thread CPU p95 reaches about 24–25 ms, establishing a
remaining renderer CPU bottleneck. The snapshot fix does not remove that
rasterization cost. One narrow software zoom cohort also fails presentation
latency despite satisfying the work budget.

Failed update comparisons triggered a fixed, separate diagnostic study: four
executions ordered 1,200/100,000/100,000/1,200, each 2,000 samples, CPU 5, with
user-thread instruction/cycle/cache-miss counters. This is profiling, not a
replacement acceptance attempt. All four medians are 6.848–6.923 µs; the earlier
large-history median slowdown does not recur. In the two large-history runs, the
slowest 5% versus the middle 50% have instruction ratios 0.995/0.996,
cycles-per-instruction ratios 3.69/3.62, and cache-miss ratios 13.94/12.78. Both
large-history runs have zero recorded faults, switches and counter multiplexing.
Counter intervals include clock/counter-read overhead. These observations identify
execution/cache variation in this diagnostic; they do not establish which cache
line, allocation or external condition caused the earlier failed comparisons.
Raw samples, protocol and analysis are under `counter-follow-up/`.

Further work must profile software rasterization and correlate hardware counters
with a reproduced failing update execution. A later predefined acceptance attempt
is justified after a measured fix or identified environmental cause; repeating
until green is not acceptance. Physical mouse/touchpad/touchscreen feel and
input-to-photon checks remain separate and unfinished.

## Files and validation

Production changes are confined to `src/ui/line_chart.rs`,
`src/ui/chart_frame.rs` and `src/ui/multi_pane_frame.rs`. The internal scope enum
does not change public signatures or ownership. No related upstream port is
required before using this fix; software renderer optimization remains follow-on
work in the existing Iced/tiny-skia drawing path.

New Rust files are `examples/frame_latency_profile.rs`, its
`frame_latency_profile/native.rs` and `presentation.rs` modules, and
`tests/pane_preparation_work.rs`. Cargo manifests expose already-locked Linux
development packages. Python additions are `frame_acceptance.py`,
`profile_chart_frames.py` and their three test files in `scripts/`.

Validation passes: formatting, **559 Rust tests across 50 targets**, all-target
check, strict all-target Clippy, debug example builds, frozen release builds,
**26 Python tests**, and **20 original-source reference scripts**. All **984**
renderer regression captures pass (492 per backend), and every complete PPM file
is byte-identical to the saved pre-change capture. Final source/frozen binary
identities were rechecked. RED/GREEN, validation and capture-comparison logs are
under `target/frame-acceptance/`. No desktop or kernel configuration was changed.
