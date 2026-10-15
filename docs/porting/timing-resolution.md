# Verified timing attribution

Acceptance remains **OPEN**. This continuation fixes the observer handshake and a
phase-probe fault artifact, reproduces late frames with verified chart identity,
and measures their compositor stages. It does not claim a production engine fix
or an acceptance pass.

## Handshake correction

The earlier observer tolerated a missing PID file and could complete without
identifying the chart. The launcher now writes its owned, stopped child's PID
inside the repository and waits for an atomic observer acknowledgement containing
that exact client PID, compositor PID and clock ID. The observer requires a live
PID before attaching; absent, malformed and dead PID files fail immediately.
Scheduling rings attach after acknowledgement, avoiding overflow during the
human sudo step. Timeout kills only the owned chart child and retains evidence.

The observer also rejects a recording with no verified chart surface within five
seconds. It caches the successful identity check instead of repeatedly scanning
the growing event list. The actual runtime identity comes from numeric client PID
and wire-surface fields, not temporal similarity alone. Tests cover missing,
invalid, dead and mismatched identities. Atomic ready-file publication prevents
a partial JSON acknowledgement.

The first prepared launcher timed out before the observer started. A second was
cancelled while waiting to move ring attachment after the handshake. Their
incomplete manifests are retained. The corrected launcher and both subsequent
60-second observer runs completed. No desktop setting, thread priority or engine
behavior was changed.

## Reproduced presentation delay

Two copied native fixtures each recorded 2,880 frames: 720 per phase, 20 warmups
per phase, software renderer, 1,200 starting rows and 16 sources. Both used the
normal chart/widget drawing path at 917×871 physical pixels, scale 1, without a
resize. The monitor was 60 Hz. The longer sample count is diagnostic only;
acceptance remains the original 120 frames per phase.

The first run verified PID 474763/surface 29, retained 32,933 events and
reproduced 18 measured individual presentation misses. Its state sequence joins
all 2,880 frames, plus one explicitly excluded startup state. For frame 749,
work was 15.146755 ms, receipt took 0.008097–0.033106 ms from the client commit
request, state application took 0.004768 ms and state-to-presentation took
31.685221 ms. Total presentation latency was 46.858818 ms.

The second run verified PID 481807/surface 29 and retained 79,536 events. It added
monitor render, render return, output commit/return, frame scheduler, presentation
and GPU-sync callback probes. Exact installed ELF addresses, register values and
getter layouts were checked against the pinned binary and
[renderer source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/render/Renderer.cpp).
The [scheduler source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/output/MonitorFrameScheduler.cpp)
separates frame events from optional explicit-sync completion. No GPU-sync callback
was observed, so no direct GPU-fence completion time is claimed.

The machine has two active displays, at roughly 144 and 60 Hz. Protocol
sync_output events name HDMI-A-1; retained sanitized monitor metadata maps it to
monitor ID 1 at 60 Hz. Numeric monitor IDs on the sampled events separate the
other display. Every selected render/output return is paired in timestamp order;
all output commits succeeded. The SysV bool return is decoded from AL, including
a regression for unrelated upper RAX bits. Unknown/lost records, incomplete
identities and inconsistent event counts fail analysis. There were no lost or
unknown record kinds. Two unrelated feedback identity reads failed; their raw
records remain retained and do not remove chart state events.

This second run reproduced nine measured individual presentation misses. Frame
1462 illustrates the stages:

| Stage | Duration ms |
| --- | ---: |
| Chart work | 11.731258 |
| Client commit request to compositor receipt | 0.587982–0.614605 |
| Receipt to state application | 0.003543 |
| Applied state to next monitor render | 16.003362 |
| Monitor render start to output submission | 0.364490 |
| Output submission call | 0.048759 |
| Submission return to actual presentation | 16.254611 |
| Total chart-start to presentation | 45.005598 |

For all nine late frames, the previous monitor render had **already finished**
when the new chart state was applied: by 0.031267–3.079514 ms. They waited
13.261051–16.271455 ms for the following render, then approximately one refresh
for presentation. This identifies the missed compositor render slot and ensuing
presentation wait in these reproduced frames. It does not establish a slow
surface-commit handler or blocking output-submit call. The submit-to-presentation
interval includes GPU and display work; it is not a direct GPU-duration measure.
The analyzer deliberately reports the first monitor render after state application
as a candidate, rather than claiming access to a DRM buffer/frame identity.

These observer runs perturb the system. They explain the captured frames; they
do not retroactively prove that every historical GPU or software failure had the
same cause. The longer runs' individual misses and percentiles do not replace
unchanged acceptance cohorts. Earlier callback-only pacing was already tested
without a demonstrated benefit and remains rejected; it was not retried here.

## Model updates and page faults

The copied phase probe now records CLOCK_MONOTONIC boundaries. A separate parent
observes its owned child with per-thread minor/major fault events and scheduling
records, retaining raw IPs/timestamps and process mappings. It makes no per-update
counter syscalls in the engine. Fault events follow the
[Linux perf event ABI](https://github.com/torvalds/linux/blob/master/include/uapi/linux/perf_event.h).
This remains an intrusive diagnostic, not acceptance timing.

The first smoke run revealed faults inside the probe's own Vec trace writes.
Capacity reservation alone had not touched those pages. Trace storage is now
initialized and touched before timing, then cleared while preserving capacity.
Three alternating before/after control pairs at 1,200 rows retained 63 model-phase
faults in trace-record writes in **each** original run and zero in **each** corrected
run. ELF PT_LOAD translation and inline symbolization distinguish those writes
from actual TimeScalePoint/TickMark appends. No overall speedup is claimed: code
layout and memory layout also change, and the diagnostic medians vary.

After that correction, the predefined study completed six alternating history
pairs and six consecutive same-history control pairs: 24 executions, CPU 5,
16 Histogram sources, fixed last price, 20 warmups and 2,000 measured updates.
Together with the six probe-control runs, all 30 executions retain 2,020 whole
updates and 6,060 phase records each.

Small-history model phases contain 148–173 measured minor faults per history
execution. Faulting Rust instructions include TimeScale::append_point and
TickMarks::append_point Vec writes; libc vector-store fault IPs are retained
without attributing stripped-library addresses to an unreliable nearest symbol.
Model durations with a fault have medians about 6.895–7.740 microseconds, versus
about 3.627–3.802 microseconds without a fault in these six small-history runs.
Sampling itself adds work, so this is not an estimate of fault cost in production.

The large-history model phases have zero measured minor faults, yet still have
tails. No measured major fault occurred in any history execution's three update
phases. The 100 slowest whole updates in each history execution contain no
recorded off-CPU model interval. Only 9–20 of those 100 small-history samples have
a model fault; all 100 large-history samples have neither a fault nor an off-CPU
model interval. Page faults and scheduling therefore do not explain all tails.
The original roughly 14-microsecond median state was not reproduced. Kernel
interrupt activity and every cache effect were not measured, and no additional
engine update defect is established.

## Decision and validation

The proven measurement defects are fixed. The presentation captures now establish
where the extra wait occurs. A production change to the chart, Iced pacing,
compositor configuration or memory allocation is not justified solely by these
results. No such change was retained.

The unchanged policy says: “investigate and fix the measured operation before
another predefined acceptance set.” The two fixes above affect diagnostic tools,
not the operation measured by the normal acceptance fixture. Consequently the
performance acceptance sets were **not rerun** to seek a pass. Step 4 remains
open, with the prior update INCONCLUSIVE and native 31/32 first-set result retained.
Closing it requires a demonstrated production remedy and the unchanged checks;
explaining a compositor deadline miss does not waive the latency budget.

Passed: 50 Python tests, the copied Rust counter arithmetic test, strict Clippy on
both copied diagnostic examples, root cargo check --all-targets, formatting and
diff checks. A deliberate bool-decoder mutation is rejected by the regression;
the restored suite passes. The normal chart engine and public API are unchanged,
so prior physical mouse passes remain recorded. Touchpad, touchscreen and high-DPI
acceptance remain untested. Conflation remains behind the open timing gates.

Raw runs, observer source snapshot, pinned compositor source, protocols, mappings,
failed starts and analyses are retained under target/timing-resolution/. The
[summary](timing-resolution-summary.json) retains identities, stage measurements
and hashes. It is a diagnostic result, not a milestone acceptance declaration.

## Reproduction

Use fresh paths and build before recording. For example, build the long copied
fixture with `python scripts/profile_frame_wire.py --long-capture --output
target/trace-fresh-build`. In one terminal start the owned launcher (replace the
compositor PID with the current `pgrep -x Hyprland` result):

```sh
mkdir -p target/trace-fresh
WAYLAND_DEBUG=client python scripts/profile_frame_schedule.py --binary target/trace-fresh-build/frame-wire-diagnostic --output target/trace-fresh/run --backend tiny-skia --rows 1200 --compositor-pid 1172 --client-pid-file target/trace-fresh/client.pid --observer-ready target/trace-fresh/observer/ready.json
```

After the launcher reports its waiting PID, in the other terminal:

```sh
sudo python scripts/trace_hyprland_surface.py --pid 1172 --client-pid-file target/trace-fresh/client.pid --output target/trace-fresh/observer
```

The launcher resumes automatically after acknowledgement. The observer requires
the exact hashed compositor/library versions. After both complete, run the wire
analyzer, then the compositor analyzer with a monitor ID verified against protocol
output names and monitor metadata. All such output remains diagnostic only.

For owned update fault recording, build with `scripts/profile_update_ips.py
--rdpmc --phases --output target/fault-fresh-build`, then run
`scripts/profile_update_faults.py --binary
target/fault-fresh-build/update-rdpmc-diagnostic --rows 1200 --output
target/fault-fresh-run` using Python. Do not overlap recordings with builds,
tests, other profiling, or another timing execution.
