# Bounded Histogram timing and load check

Date: 2026-10-17. The user authorized phase/load checks and asked to prioritize
remaining port features if there is no reproducible engine problem.

- [x] Extend the isolated counter fixture with opt-in per-preparation wall timing.
- [x] Test timer lifecycle/reset and exact same-frame timing attribution.
- [x] Freeze one 36-execution batch: three repetitions, 1,200/100,000 rows,
  reference/off/on, ambient/one CPU contender, 20 warmups and 2,000 measured
  frames each, pinned to CPU 5. Reverse order in the second repetition.
- [x] Keep all results; >5% on/control total median or p95 remains a diagnostic
  calibration flag. No retries to obtain passing timings.
- [x] Report Histogram versus other snapshot time and snapshot wall/thread-CPU
  gaps on the same total-tail frames. Load changes are diagnostic, not acceptance.
- [x] Make an explicit development-priority decision; preserve every open
  acceptance result and its thresholds. No speculative engine optimization.

Reuse the counter helper, isolated builder and existing phase parser. A stack
guard owns its start Instant and adds elapsed time on drop, covering early exits.
It only exists in the isolated library. Timing is opt-in, with reference/off/on
controls for hooks and active collection. This avoids a second instrumentation
framework and new dependencies. Production callers, APIs and rendering stay
unchanged. The analysis compares per-frame Histogram sums with that frame's
snapshot; it must reject negative residuals or incomplete source/sample records.

Ambient means no added load, not a certified idle desktop. Existing applications
stay running. A single owned CPU worker is started with an explicit READY
handshake, pinned to the fixture CPU, checked for continued activity and always
terminated/reaped afterward. It must stop on a parent exit. This tests CPU
contention, not memory/GPU load, and cannot establish historical causality.

Stop after the declared batch and report uncertainty. If no production defect is
established, defer further timing investigation, retain the exception as OPEN,
and resume feature planning. Reopen debugging for a reproducible interaction
problem, evidence of incorrect work or a repeatable same-protocol regression.
Existing acceptance failures are not waived or relabeled as passes.

Result: [report](../docs/porting/histogram-timing-load.md). 0/24 overhead flags;
CPU contention causes elapsed delay while counted work stays fixed. No new
engine defect established. Timing investigation deferred; acceptance OPEN.
