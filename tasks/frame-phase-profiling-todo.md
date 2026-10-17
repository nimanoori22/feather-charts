# Slow-frame investigation checklist

Plan: [frame-phase-profiling-plan.md](frame-phase-profiling-plan.md).
Execution started 2026-10-17. Selected CPU investigation is complete; see
[results and limitations](../docs/porting/frame-phase-profiling.md).
These tasks cover locating/profiling; no engine fix
or acceptance pass is implied. Execute timing tasks serially.

## Task 1 — Attribute retained slow samples

Dependencies: none. Scope: small (analysis script/test and report).

- [x] Parse ordered arrays, validate exact counts/finite values and binary identity.
- [x] Produce per-run total-tail sample attribution and normal/slow distributions.
- [x] Preserve pair 4 and all other runs; test missing data and non-additive p95s.

Verification: analysis reproduces recorded total p95 and reports the same-sample
phase breakdown. Files: new scoped analysis/test scripts; artifact CSV/plots.

## Task 2 — Freeze diagnostic reproduction

Dependencies: task 1. Scope: small (runner/protocol and environment manifest).

- [x] Record build/system/device/clock identity and tool access/availability.
- [x] Freeze optimized original and symbolized diagnostic artifacts separately.
- [x] Freeze the finite matched batch, diagnostic cohort and overhead protocol.

Verification: hashes resolve and reproduction has explicit stop/inconclusive
rules. Files: scoped runner/manifest; existing fixture CLI reused.

## Checkpoint — Phase evidence and reproducible protocol

- [x] A broad phase is supported by matching tail samples, or uncertainty is stated.
- [x] Existing budgets, failed records and original executables are preserved.

## Task 3 — Fill missing phase observations

Dependencies: tasks 1–2. Scope: medium, at most 3–5 files.

- [x] Reuse recorder/counters for only selected phases, with sample/frame IDs.
- [x] Record wall/CPU time, warmup boundaries, scope, overflow/loss and clock mapping.
- [x] Test record pairing/overflow and quantify off/on instrumentation overhead.

Result: phase pairing and trace completeness pass. Instrumentation was reduced
after a guard failure, but the reduced batch still has one >5% flag; overhead
qualification remains INCONCLUSIVE. Likely files: isolated copies
of preparation_profile.rs and support/preparation_diagnostics.rs; alternatively
native.rs and existing runtime recorder for the chosen native branch.

## Task 4 — Collect the selected profile

Dependencies: task 3. Scope: small per branch; do only evidence-relevant branches.

- [x] CPU branch: obtain symbolized slow/normal stacks inside measurement windows.
- [ ] Privileged wait-state accounting remains unperformed: scheduler tracepoints
  were denied; a bounded capture is a conditional follow-up if needed.
- [ ] Native post-submit branch was outside the selected headless CPU capture.
  Existing presentation failures remain open.

Verification: packet reports event availability, symbol quality, clock identity,
perturbation and lost records. A missing required signal is explicitly unresolved.
Files: branch-specific capture configuration and analysis/report, no engine edits.

## Checkpoint — Evidence quality

- [x] Profile describes the same workload/phase that reproduced the symptom.
- [x] Setup/normal frames do not masquerade as slow-frame evidence.
- [x] Lost samples or denied permissions are not silently treated as zero events.

## Task 5 — Establish the next actionable cause

Dependencies: task 4. Scope: small (one controlled experiment and evidence report).

- [x] State competing hypotheses and one discriminating diagnostic intervention.
- [x] Reconcile measured extra time to specific execution/waiting intervals.
- [x] Publish supported cause or INCONCLUSIVE result, raw artifact hashes and next
  experiment; leave acceptance open until subsequent fix/verification succeeds.

Verification: another developer can reproduce the evidence packet and understand
its limits. Files: scoped report/summary and existing acceptance task ledger.


## Investigation outcome

The selected CPU branch is delivered with 48 fixed overhead-control executions
and two 12-execution stack batches. Frame-pointer capture restores measured
snapshot call chains. The removal of resource-usage queries did not eliminate
all control flags. Snapshot preparation is located; the historical regression's
cause is not proven. No engine remedy or unchanged acceptance retry is claimed.
The conditional privileged/native branches above remain unperformed.
