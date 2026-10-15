# Presentation and update diagnostic follow-up

## Result

Acceptance remains **OPEN**. These are diagnostic executions, not replacement
acceptance sets. No additional production engine fix is justified by this study.
The unchanged update outcome remains INCONCLUSIVE; the latest native acceptance
set remains 31/32 passing cohorts. Earlier failures remain retained.

The normal-build physical mouse checks already passed both backends at all three
sizes, including short-height recovery. They were not repeated. Touchpad,
touchscreen and high-DPI checks remain untested. Conflation has not started.

## Scope and approach

The normal chart API, update path, drawing, presentation settings and acceptance
budgets are unchanged. Diagnostic builders copy the actual fixture and engine
into separately identified Cargo packages with distinct example targets. Their
metadata identifies them as diagnostics; native scheduling output is rejected by
the existing acceptance reader.

Two observation approaches were compared: interrupt-based cache-miss sampling
with kernel counter reads, and pinned user RDPMC snapshots. The latter avoids
sampling interrupts and per-update syscalls, while retaining raw instruction,
cycle and cache-miss deltas. It requires the supported Linux x86_64 perf ABI,
usable hardware events and active user RDPMC; unsupported configurations fail
instead of silently substituting another measurement.

Each counter owns its perf file and read-only metadata mapping. Drop unmaps the
page before closing its file. Reads follow the kernel metadata sequence and
sign-extension contract, validate active counters and are cross-checked against
a kernel read at construction. No counter handles enter the chart engine.

The optional phase probe adds a temporary method only to the copied Chart. Four
stack-borrowed callback marks separate validation, data-layer mutation and model
application; no callback is stored. The original method remains intact. This
changes code layout and adds measured work, so its results cannot establish the
uninstrumented acceptance outcome.

## Client submission and presentation

A copied presentation probe brackets each real feedback request/flush and saves
480 frame identities. WAYLAND_DEBUG output supplies surface commits and actual
presentation feedback. The analyzer validates count, order, completion and a
common bounded clock offset. Native feedback and Mesa's independent feedback
channel are distinguished, including when they share a surface commit.

A fixed three-repeat study alternated software/1,200-row/regular and
GPU/100,000-row/narrow sessions:

| Repeat | Software measured individual late frames | GPU measured individual late frames |
| --- | ---: | ---: |
| 1 | 60 | 0 |
| 2 | 0 | 0 |
| 3 | 2 | 0 |

Each execution contains 480 frames, including 80 warmups. An individual late
frame exceeds 33.333334 ms; this count does not replace cohort p95 acceptance.
Clock-offset interval widths were 21.974–27.675 microseconds, with another
microsecond allowed for a printed commit timestamp.

For software frame 292 in repeat 3, work took 8.646710 ms and presentation took
39.427392 ms. The feedback request-to-commit interval was 60 microseconds;
commit-to-presentation was bounded at 30.765114–30.793789 ms. No client off-CPU
work or compositor involuntary preemption overlapped the respective intervals.
Frame 343 shows the same pattern. This narrows these delays to after the
**client-side commit request**; it does not establish server receipt, GPU
completion, compositor rendering or display-flip timing. Compositor sleeping
alone is not proof of a cause.

The first GPU parser attempt counted Mesa's extra feedback channel and failed.
Its raw output and failed analysis are retained; corrected results are explicitly
named analysis-v2.json. No execution was discarded to improve a timing result.

## Privileged compositor observation

The observer uses temporary perf uprobes at the installed Hyprland's
scheduleState, commitState and presentFeedback entries. It verifies exact binary
hashes and ELF file offsets before attaching, reads only numeric surface/client
identity fields, and closes every probe automatically after 60 seconds. It
changes no compositor settings and never stops the compositor. The entry and
getter semantics were checked against the
[exact installed Hyprland source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/protocols/core/Compositor.cpp).
Binary upgrades require re-verification of the pinned addresses and layouts.

Unprivileged observation was denied by the OS. The user ran the bounded observer
with sudo in their terminal. The first 60-second recording expired before the
chart launcher started because of an overly conservative timing guard. A second
recording used an automatic waiter and included all three software sessions.
Both recordings are retained; neither is acceptance evidence.

The second recording completed with 23,519 sample records and no lost/unknown
record kinds. However, no client PID/surface identity was captured: the observer
did not observe the expected PID-file contents. The reason for that visibility
gap is unresolved. Each session has one temporal candidate with exactly 480
scheduled states matching its frame sequence, but timing agreement alone does
not prove surface ownership. None of the three chart sessions reproduced a
measured late frame. Consequently this trace cannot identify the earlier delay's
cause. Later observer output now records its resolved PID-file path, observed
client PIDs and whether any chart identity was observed, making this failure
explicit.

## Update cost study

The whole-update RDPMC study used six alternating 1,200/100,000-row pairs, 16
Histogram sources, append updates, fixed last price, CPU 5, 20 warmups and 2,000
measured samples per execution. All raw samples and three hardware-event deltas
are retained. Calibrated three-counter snapshot cost was 76–78 ns; two snapshots
bound each update. Update medians ranged from 6.288 to 6.567 microseconds and
p95 values from 7.435 to 13.412 microseconds. The earlier roughly 14-microsecond
median state was not reproduced.

A separate fixed phase study ran six alternating history pairs and six
same-history control pairs, with the same fixture/sample counts. All 24
executions retain 2,020 whole-update records and 6,060 phase records each.
Snapshot calibration was 76–83 ns. Whole-update medians ranged from 7.126 to
7.504 microseconds; p95 ranged from 7.870 to 18.815 microseconds.

The model phase contains substantial tail variation at both history sizes. For
example, the median model duration among the slowest 5% of whole updates was
11.6825 microseconds in history pair 5's large case and 10.9445 microseconds in
pair 6's small case. Their corresponding model cache-miss medians were 1,253
and 1,169. Other tails have fewer misses and little additional user-cycle cost.
These are matched per-sample observations, not subtraction of unrelated
percentiles. They are leads for cache/kernel/scheduling investigation, not proof
of an allocator defect, linear history traversal, or one actionable engine bug.

## Validation and artifacts

Passed: 43 Python tests; the copied Rust counter arithmetic regression; strict
Clippy on both copied diagnostic examples; root cargo check --all-targets;
formatting and diff checks. The counter test covers sign extension, signed kernel
offsets, full-width counters and invalid widths. Parser tests cover mixed native
and Mesa channels, wrong identities, missing commits/feedback, invalid clock
alignment and duplicate/reused feedback identities. Existing engine behavior
was not changed; the earlier engine acceptance results are preserved.

The curated JSON summary records protocol, executable and raw-artifact hashes.
Local raw files are under target/presentation-follow-up/, including wire-study,
privileged-study-retry, rdpmc-study and rdpmc-phase-study. The first privileged
recording remains under /tmp/feather-compositor-trace-20261015/ and is copied into
the artifact tree. Privileged root-created files were read without altering them.

## Next unresolved work

Before another privileged attempt, establish a PID-file handshake visible from
the observer's own environment and fail early if it cannot see the launcher.
Only a trace with verified chart identity and a reproduced late frame can advance
the compositor attribution. Preserve unsuccessful sessions as well.

For update timing, reproduce the original slow state and correlate the model
phase's wall/user-cycle gaps with faults or kernel activity. A diagnostic phase
tail by itself does not justify changing the normal update path. Implement a
production fix only when the cause and behavioral contract are established;
then rerun the unchanged acceptance protocols once under their stop rules.

## Verified continuation

The [verified timing report](timing-resolution.md) records the corrected PID
handshake, two captures with verified chart identity and reproduced late frames,
monitor-render/output timing, and a 30-execution page-fault study. The trace-buffer
fault artifact is fixed. Production timing acceptance remains open; no engine
remedy or repeated acceptance pass is claimed.
