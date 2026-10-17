# Development plan: locate and profile slow frames

Date: 2026-10-17. Status: selected CPU investigation executed; historical cause and timing
qualification remain INCONCLUSIVE. No production engine changes.
Scope: steps 1 and 2 of the recommended investigation. Checklist:
[frame-phase-profiling-todo.md](frame-phase-profiling-todo.md).
Existing historical plans and acceptance policies remain intact.

## Outcome

Produce a reproducible slow-frame capture with an identified phase, a time-aligned
profile of that phase, and a supported explanation or an explicit unresolved gap.
A valid investigation can end INCONCLUSIVE; it must not invent a fix or rerun
acceptance until it passes. Optimization and acceptance are subsequent work.

## Existing evidence and scope

The retained lazy-autoscale comparison contains 24 uninstrumented executions:
six baseline/candidate pairs at 1,200 and 100,000 starting rows, 16 Histogram
sources, fixed viewport/last-price formatting, 20 warmups, 2,000 measured samples.
The correction is source parity; its performance rule was not met.

Pair 4 at 100,000 rows has total p95 1.259479 -> 1.663795 ms (+32.10%).
Snapshot p95 is 1.175904 -> 1.535181 ms. This makes snapshot preparation a strong
initial investigation target, but separate phase percentiles do not establish
which phase caused the total percentile. The same total-tail sample IDs must be
examined. A roughly 1.66 ms CPU pipeline is also a different symptom from the
historical >33.333334 ms native presentation latency.

Reuse these existing boundaries:

| Fixture | Existing measured boundaries | Important limit |
|---|---|---|
| `examples/preparation_profile.rs` | public update, snapshot preparation, glyph/frame conversion | CPU only; excludes draw, GPU and presentation |
| `examples/support/preparation_diagnostics.rs` | CPU time, allocations, faults, switches, hardware counters | currently wraps update only; counter reads add overhead |
| `examples/frame_latency_profile/native.rs` | input/update, prepare, geometry, compositor present call, actual presentation feedback | present call return is not GPU completion or presentation |
| `examples/frame_latency_profile/presentation.rs` | advertised presentation clock, thread CPU clock, feedback identity | clocks must be checked before merging traces |
| `examples/support/runtime_profile_spans.rs` | bounded records in an isolated instrumented Iced build | owner-thread coverage; overflow must be reported |
| `scripts/trace_hyprland_surface.py` | compositor surface/render/output observations | privileged and pinned to exact binary hashes/offsets |

Codebase-memory search, direct-call trace, snippet and exact-file coverage checks
were used for these fixture files. The graph generation is 2026-10-15T20:37:48Z;
coverage reported matching metadata. Critical timing boundaries were also read
directly. This is task-directed coverage, not a complete engine audit.

## Step 1: locate the phase

### 1A. Reconstruct the existing regression without running the chart

Read the original ordered arrays in `target/lazy-autoscale/comparison/*.log`.
Join update/snapshot/glyph by execution and sample index. Preserve the raw
index-to-workload mapping (measured index 0 follows the 20 warmups).

For each sample calculate `total = update + snapshot + glyph`; select its p95
using the existing sorted index floor(n*.95), and inspect all samples in its
slowest 5%. Report those same samples' phase durations, plus median and slowest
samples as context. Do not add phase p95s or compare unrelated tail samples.
Compare pair 4 with its paired baseline and the other five pairs. Preserve run
order, distributions and per-run results; do not pool executions into one p95.

Deliver `phase-attribution.csv`, a sample-index timeline, per-run distribution
plots, and a concise finding. Test the parser with shuffled durations, missing
samples and a case where independent phase p95s give a misleading answer.
The arrays can locate a broad phase; they cannot reconstruct old stacks or
kernel events that were not recorded.

### 1B. Freeze the reproduction protocol

Record source/executable hashes, compiler/build flags, dependency lock hash,
kernel, CPU and allowed affinity, GPU/driver, renderer, dimensions/scale,
monitor refresh, compositor identity, power state and observed background load.
Keep CPU 5 for the matched headless comparison if it is still available; if not,
record a new protocol rather than silently changing the historical one.
Do not pin the native chart or compositor unless testing affinity explicitly.
Run builds and measurements serially. Keep normal desktop behavior; record
frequency/temperature where available rather than changing governors or priority.

Preserve the original baseline and candidate executables. Build a separate
optimized diagnostic artifact from the same source, with line debug information;
try DWARF unwinding first. If frame pointers are needed, label that build change
and retain the original artifact. Rust profiling requires usable symbols and
stacks; a debug-only build is not representative of release behavior.
See the [Rust Performance Book](https://nnethercote.github.io/perf-book/profiling.html).

Before collection, record a finite diagnostic batch: three matched pairs in
AB/BA/AB order, both histories, 20 warmups and 2,000 measured samples. Capture
all sessions. This batch diagnoses reproduction; it is not acceptance and does
not replace its six-pair procedure. Do not extend it until a favorable result
appears. If it does not reproduce, retain the negative result and specify the
missing workload or next testable hypothesis before another batch.

### 1C. Add only the missing diagnostic boundaries

Reuse the current fixture and counter/recorder helpers. Add sample ID, phase,
PID/TID, monotonic start/end and per-phase thread CPU time in an isolated build.
Extend observation to snapshot/glyph only if 1A points there. Report process or
thread scope explicitly; worker-thread cost is not represented by main-thread
CPU time. Buffer records in preallocated memory and flush after measurement.
Record dropped/overflowed data, avoid per-bar timestamps and console/file I/O in
the measured interval, and measure timer/counter overhead.

Use a matched diagnostic-off/on control batch fixed before capture. Report its
perturbation; a >5% total-pipeline median or p95 increase requires reducing
instrumentation before drawing timing conclusions. Even below that diagnostic
guard, flag any microsecond phase comparable to measurement overhead. This is
an instrumentation check, not a change to product acceptance budgets.

For native investigation reuse frame IDs, real presentation feedback and the
existing timing fields. Add CPU readings at phase boundaries only if missing.
Do not initially repeat the full acceptance matrix: reproduce the previously
failing backend/history/size/gesture as a labeled diagnostic cohort.

Checkpoint: identify whether the tail is in update, preparation, drawing,
submission, or after submission. If wall time is large but thread CPU time is
small, investigate waiting/scheduling; if both grow, investigate execution.
The difference is a clue, not an exact off-CPU accounting without a scheduler
trace. If no phase explains the tail, check missing spans and clock alignment.

## Step 2: profile the identified phase

### Tool selection

Availability checked in this session: `vmstat` is on PATH. `perf`, `samply`,
`cargo-flamegraph`, `bpftrace`, `trace-cmd`, `heaptrack`, `valgrind`, `hotspot`,
RenderDoc tools, `gpuvis`, `strace`, `pidstat` and Perfetto's trace processor were
not found on PATH. This is not proof they cannot be accessed through another
host environment. Existing in-repository native/counter profilers remain usable
subject to device/counter permissions. No tools were installed during planning.

| Tool | Question it answers | When to use |
|---|---|---|
| Existing ordered samples + Python | Which measured phase dominates the same slow samples? | First, without new instrumentation |
| `perf stat` | More instructions/cycles, faults or context switches? | Aggregate control; not individual-frame attribution |
| `perf record`, `report`, `annotate` | Which stacks/functions execute in the selected slow interval? | CPU-heavy phase; filter out startup and normal-frame samples |
| `perf sched timehist` or trace-cmd/ftrace | Was the thread runnable but delayed, blocked, or executing? | Wall/CPU gap or suspected kernel interference |
| Perfetto UI + timestamped spans | How do app, scheduler and compositor events line up? | Cross-layer critical path; existing CSV/JSON conversion suffices initially |
| Heaptrack or DHAT | Which call sites allocate/reallocate or copy excessively? | Only if allocation evidence points there |
| Existing Wayland observer; GPU-specific profiling if needed | Where is the interval after submission spent? | Native presentation delay; GPU timestamps only when supported |

Linux documents [counter collection](https://raw.githubusercontent.com/torvalds/linux/master/tools/perf/Documentation/perf-stat.txt),
[stack sampling](https://raw.githubusercontent.com/torvalds/linux/master/tools/perf/Documentation/perf-record.txt)
and [scheduling analysis](https://github.com/torvalds/linux/blob/master/tools/perf/Documentation/perf-sched.txt).
Perfetto supports [external event formats](https://perfetto.dev/docs/getting-started/other-formats)
and [scheduler events](https://perfetto.dev/docs/data-sources/cpu-scheduling).
Start with perf plus a timeline viewer; do not install every optional profiler.
Check tool/kernel compatibility, event availability and access before recording.
Scoped privileged collection may be necessary; do not lower global security
settings or alter desktop configuration to make a trace work.

### 2A. CPU execution branch

Collect an initial stack profile at a modest frequency (e.g. 199 Hz). Include
measurement-window markers and PID/TID, and filter analysis to those windows;
otherwise initial construction of 16 x 100,000 rows can dominate the profile.
Compare slow-frame windows against normal-frame windows in the same execution
and paired baseline windows. Show both inclusive and self cost. A whole-process
flame graph alone can hide rare tail events, and a 6-microsecond update is too
short for reliable individual-call attribution at that frequency. Accumulate
matching windows or use coarse spans/counters; do not just raise sampling rate.

Start with few hardware events: cycles/instructions, faults and switches in
separate bounded passes if necessary. Inspect enabled/running time and losses;
unsupported or heavily multiplexed events cannot establish precise deltas.
Stable instruction counts with more cycles/cache misses suggest a hypothesis,
not proof of a particular field, cache line or function defect.

### 2B. Waiting/kernel branch

Collect sched_switch and wakeup events covering the chart and relevant worker
threads; add bounded fault/syscall/interrupt observation only when indicated.
Account for on-CPU execution, runnable queue delay, blocked intervals and event
loss within the selected slow frame. Context-switch totals alone cannot tell
which state consumed its time. Pair scheduler evidence with call stacks or wait
sites. Check whether the delay belongs to our process or another component.

### 2C. Native post-submit branch

Correlate input/frame ID -> preparation -> geometry -> submit -> compositor
surface commit -> monitor render -> output commit -> presentation feedback.
Validate clock IDs and conversion, process start identity, surface identity and
monitor association. Missing identity or incompatible clocks make attribution
inconclusive. GPU submission, GPU completion and presentation are distinct.
If GPU execution is suspected, verify backend/adapter support for timestamp
queries and obtain separate diagnostic GPU evidence; a CPU present-call duration
does not measure it.

Before reusing the compositor observer, verify the currently running compositor
and library hashes and its supported probe identities. Never reuse the old PID
1172 or old offsets after an upgrade. Stop on mismatch and re-establish the
observer contract. Its GPU-sync observation is not a general GPU execution
profiler. Preserve unresolved/discarded frames rather than filtering them away.

## Completion and decision

Each branch produces one evidence packet: symptom and exact reproduction,
source/binary/environment manifest, full raw samples, validated identity/clock
mapping, phase attribution, selected slow and normal trace windows, resolved
stacks or wait states, overhead/loss report and remaining uncertainty.

A cause is supported when the extra time is accounted for in the matching frame
and a controlled, one-variable diagnostic intervention changes the predicted
operation with unchanged work/output. List competing explanations. If this
cannot be established, report the bounded finding and the next discriminating
experiment. Do not promise that any profiler will reveal one unique cause.

Subsequent work can implement one demonstrated remedy, verify exact rendering
and behavior, compare frozen uninstrumented binaries, then rerun the unchanged
[acceptance policy](../docs/porting/frame-acceptance-policy.md). Current budgets
remain 16.666667 ms work and 33.333334 ms actual presentation p95; update growth
remains <=10%. Investigation completion does not close acceptance or authorize
conflation as accepted work. Device coverage remains separate.

Execution evidence and limitations: [report](../docs/porting/frame-phase-profiling.md).
