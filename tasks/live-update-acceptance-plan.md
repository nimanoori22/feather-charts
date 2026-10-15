# Roadmap: finish live-update interaction acceptance

Date: 2026-10-15. Starting checkpoint: `b956460`. Status: planned; acceptance OPEN.
Execution checklist: [live-update-acceptance-todo.md](live-update-acceptance-todo.md).
This extends the unfinished [incremental append milestone](incremental-append-todo.md)
without replacing its history or the older navigation plan/checklist.

## Outcome and scope

Deliver responsive live-update navigation on the available backends, correct
behavior at short chart heights, and an evidence-backed acceptance decision.
Investigate the physical software lag separately from the short-height error and
from the update/GPU timing failures; they are not established as one defect.

Conflation is the next feature milestone only after these gates pass. This
roadmap ends with its source investigation and implementation plan, not speculative
conflation code. No new chart-engine async/threading model, renderer replacement,
dependency upgrade, or performance-budget change is planned.

## Starting evidence

| Item | Current evidence | Decision |
| --- | --- | --- |
| Update history growth | 5/6 complete comparisons pass; one p95 +24.424% against 10% | FAIL |
| Native frames | 63/64 cohorts pass across two sets; GPU narrow/100k/pan presentation p95 43.006836 ms | FAIL; third set not run |
| Software automated frames | 32/32 cohorts pass in those two sets | Partial evidence; not complete three-set acceptance |
| GPU mouse | User passed regular and half-size checks | Preserve observed pass; changes may require focused retest |
| Software mouse | Plot/vertical-axis dragging lags with live feed at roughly one-third window size | FAIL; exact physical/chart dimensions unrecorded |
| Very short chart | Replay at 300×100 and 300×180 fails Histogram preparation with nonfinite coordinates | Reproduced error; cause not yet established |
| Other devices | Mouse only available | Touchpad/touchscreen/high-DPI physical coverage untested |

Sources: [current report](../docs/porting/render-update-follow-up.md),
[mouse record](live-append-mouse-acceptance.md), and
[fixed policy](../docs/porting/frame-acceptance-policy.md).
The software occlusion improvement is already shipped. Rectangle cropping was
measured neutral twice and is not an assumed fix for this work.

## Order and dependencies

```text
1. Capture actual demo input/frame/presentation timeline
   -> 2. Fix the measured software drag bottleneck
3. Reproduce short-height contract and failure boundary
   -> 4. Fix short-height rendering and recovery
5. Attribute update timing failure -> 6. Implement any justified update fix
7. Attribute GPU presentation failure -> 8. Implement any justified frame fix
   -> checkpoint: behavioral and measured fixes verified
   -> 9. Freeze artifacts and run unchanged automated acceptance
   -> 10. Repeat physical checks and record coverage
   -> checkpoint: explicit acceptance decision
   -> 11. Investigate and plan conflation rendering
```

Execute one timing/profile study at a time. Tasks 3, 5 and 7 are independent of
software diagnosis, but builds/tests/captures must never overlap measured studies.
Physical testing follows the final candidate build. Missing hardware does not
prevent code investigation, but cannot be silently marked passed.

## Design and investigation rules

Before changing behavior, follow the repository's source/consumer workflow:
read complete relevant Lightweight Charts components and semantic dependencies,
trace real callers using codebase-memory, verify coverage and read gaps directly.
Inspect existing Rust boundaries and the exact Iced source used by the build.
Consult Flowsurface only for relevant runtime patterns. Record required behavior,
incidental TypeScript patterns, two viable designs, ownership and proposed Rust
signatures before selecting a nontrivial implementation. File targets in the
checklist are investigation leads, not a settled diagnosis or API design.

Keep the synchronous Chart owner, immutable prepared frames, and existing input
router. Prefer removing demonstrated redundant work over adding caches or threads.
If a cache or input batching is justified, specify invalidation/ordering and test
release, cancellation, append, resize and ownership transitions before adopting it.
Do not discard move events unless final positions and intervening gesture
boundaries remain correct under the upstream contract.

### Actual-demo timeline

Extend existing diagnostic facilities rather than building a second engine or
substituting the minimal native fixture for the reported demo. Include the same
two panes, control panel, Histogram data, box primitive and live feed. Record actual
window, available chart and plot bounds, physical/logical sizes, scale, refresh,
backend, source/visible-bar counts and feed cadence. Capture regular, half and
reported problem sizes using recorded dimensions rather than size labels alone.

Correlate app-observed input arrival, handler start/end, chart mutation/preparation,
geometry, software render/GPU submission, and compositor presentation with sample
IDs and compatible clock domains. Distinguish app-observed input latency from
physical device-to-photon latency. Present return/redraw/screenshot is not proof
of presentation. Retain discarded, unresolved and coalesced observations.

Two diagnostic integration choices must be compared against the exact Iced API:
reuse/extend the existing native presentation probe alongside the actual app, or
use a narrowly scoped runtime/compositor diagnostic hook. Select the smallest
option that retains the actual demo path and safely owns its native handles. If
that integration cannot observe a stage, report the gap rather than estimate it.
Keep unsafe/native code example-only with explicit lifetime and teardown rules.

Collect bounded buffered records outside hot logging paths. Separate coarse
stack sampling from latency measurement; debugger-paused runs are diagnostic only.
Measure the probe's own overhead. Enabled probes and output-isolation modes must
be rejected by acceptance readers; default release acceptance remains uninstrumented.
No public chart API is added solely for diagnostics without demonstrated need.

### Short-height behavior

Trace layout allocation, model scale height/margins, coordinate conversion,
Histogram view preparation and rectangle generation. Establish whether the bad
value is an input, degenerate usable range, stale scale state, or another proven
condition. Inspect upstream price-scale/Histogram and pane resize consumers before
choosing suppression or clamping semantics.

A valid resize must leave finite drawable geometry or the source-compatible empty
result and recover when space returns. Preserve price/time range, source identities
and pending data according to the source contract. Continue rejecting genuinely
invalid data. Do not blanket-catch errors, replace NaN with zero, clear user data,
or impose an arbitrary demo minimum size to conceal the engine boundary.

### Remaining timing failures

Update diagnostics already reproduce slower execution at identical history with
similar instruction/allocation counts and more cycles/cache misses. This is a lead,
not proof of a specific faulty allocation or historical sample's cause. Profile
the actual timed operation, identify an actionable cost, and add behavioral or
bounded-work regression coverage before changing it. If cause remains unproven,
retain an explicit unresolved outcome; do not implement a speculative optimization.

The GPU miss has a wall-time/thread-CPU gap. Correlate scheduling, buffer waits,
frame deadlines and Wayland feedback before attributing it to computation or the
compositor. Trace actual clock domains and feedback lifetime. Do not substitute
CPU timing for presentation or classify environmental evidence as a pass.

## Verification gates

For each code slice: focused behavioral tests, `cargo fmt --all -- --check`,
`cargo check --offline --all-targets`, and strict Clippy for affected targets.
Run full all-target Rust tests, Python acceptance tests, affected upstream reference
checks and renderer captures at the integrated checkpoint. Reuse existing commands
and fixtures; preserve pixels unless a source-proven behavior correction requires
an explicitly reviewed reference change.

After measured fixes and functional checks, freeze source/binary/policy hashes and
run serially with fresh output directories under the existing policy:

- Update: six alternating 1,200/100,000-row pairs, 16 Histogram sources,
  20 warmups and 2,000 measured updates; every median and p95 growth <=10%.
  Retain complete same-history controls and historical short-window results.
- Native: three complete sets, both backends, both histories and regular/narrow
  sizes; four phases, 20 warmups and 100 measured frames per phase. Work p95
  <=16.666667 ms; presentation p95 <=33.333334 ms. All 96 cohorts must pass.
  Complete each set then stop on FAIL/INCONCLUSIVE, as the policy requires.
- Preserve raw samples, invalid attempts, artifact identities and exact commands.
  No overlapping builds/captures/profilers, sample filtering, budget relaxation,
  or repeat-until-green procedure. Another attempt requires a documented diagnosis
  and measured change; unresolved attribution remains open.

These native phases do not replace physical vertical-axis dragging or the full
control-panel demo checks. Add targeted diagnostic cases for those gaps without
silently changing the frozen acceptance fixture.

## Physical acceptance and responsibility

The agent runs tests, diagnostics, captures and automated gates. The user supplies
physical feel/behavior observations. Extend the existing device record with exact
sizes and one-third/problem-size rows for both backends. After the candidate fix,
exercise zoom, diagonal plot drag, price-axis wheel/drag, box move/resize,
commit/cancel/new gesture, outside release and layout stability with live updates.
Check short-height resize and recovery as well.

Keep mouse, touchpad, touchscreen, high-DPI and externally measured input-to-photon
results separate. Mouse-only success is scoped mouse acceptance. Full hardware
coverage remains open while required devices/scales are unavailable; a narrower
release scope requires an explicit user decision, not automatic waiver or timeout.
Earlier passes stay recorded; shared-path changes require appropriate retesting.

## Completion and follow-on

Publish an acceptance report linking all original failures, fixes, regression
coverage, measurements and physical observations. Close only gates demonstrated
by valid evidence. Any remaining FAIL/INCONCLUSIVE, unresolved software lag or
required hardware gap keeps the corresponding milestone gate open.

Only after the acceptance checkpoint, investigate upstream conflation behavior,
eligible series, pixel bucketing/aggregation, extrema and whitespace, hit/crosshair
mapping and cache invalidation. Preserve raw data, exact update semantics and
source identities. Compare existing Rust preparation versus a minimal derived
render representation, select a first end-to-end series slice from actual source
support, and write a separate implementation roadmap with parity and performance
fixtures. Do not choose aggregation rules or a broad new abstraction in advance.

## Risks and checkpoints

| Risk | Handling |
| --- | --- |
| Instrumentation changes the lag | Quantify overhead; buffered diagnostic-only data; uninstrumented acceptance |
| Scripted fixture misses physical path | Actual demo instrumentation plus user reproduction; retain separate results |
| Short-height fix hides invalid data | Boundary/recovery tests and explicit invalid-input regressions |
| Optimization changes pixels or gestures | Source contract, ordered event tests and both-backend raster comparisons |
| Timing remains variable | Fixed controls and decision rules; keep unresolved evidence open |
| Hardware unavailable | Record untested coverage; never infer a pass |

Checkpoint A (tasks 1–4): diagnosis linked to evidence, software/short-height fixes
reviewable, focused tests and renderer behavior verified. A still-unproven cause
is reported as unresolved and blocks its completion checkbox.
Checkpoint B (tasks 5–8): actionable update/presentation causes addressed with
matched diagnostics, or explicitly retained as unresolved; no acceptance rerun to
hide those outcomes.
Checkpoint C (tasks 9–10): unchanged automated gates and scoped physical results
support an explicit acceptance decision. Conflation remains blocked until the
required acceptance scope is satisfied.
