# Live-update interaction acceptance checklist

Plan: [live-update-acceptance-plan.md](live-update-acceptance-plan.md).
Starting checkpoint: `b956460`. Software-drag and short-height fixes are implemented.
Timing gates and full physical acceptance remain open; see the implementation report.
Existing measurements and mouse observations remain in their original reports.

## 1. Capture the actual demo timeline

**Dependencies:** None. **Scope:** Medium; split integration from analysis if needed.
**Likely files:** `examples/public_chart.rs`, a focused helper beside it,
`examples/frame_latency_profile/presentation.rs`, a focused diagnostic reader/test.

- [x] Reproduce plot and price-axis drag with live feed and record exact window,
  chart/plot bounds, scale, refresh, backend and visible work at the problem size.
- [x] Correlate app-observed input, update/preparation, geometry, render/submission
  and actual presentation; document clock alignment, probe overhead and any gaps.
- [x] Keep probes optional/buffered and diagnostic-only; reject diagnostic logs
  from acceptance and preserve invalid/discarded/unresolved observations.

**Verification:** Focused clock/record/diagnostic-rejection tests; compare repeated
live-feed gestures with and without probes. Sampling pauses never count as latency.

## 2. Fix the measured software drag cost

**Dependencies:** 1. **Scope:** Small/medium; select files from the measured call path.
**Likely files:** Affected `src/ui/line_chart/` drawing code or actual input path,
its behavioral tests, and the existing diagnostic replay as justified by evidence.

- [x] Identify a reproducible operation explaining the delay and compare the
  smallest viable fixes; do not assume rectangle cropping or async work will help.
- [x] Add a failing behavior/work regression where applicable; implement the chosen
  fix while preserving pixels, hover/hits and gesture release/cancel ordering.
- [x] Demonstrate a matched improvement in the actual reported path; retain failed
  experiments and keep physical acceptance pending until user confirmation.

**Verification:** Focused tests and both-renderer image checks; serial matched
before/after diagnostics at recorded sizes. No improvement means no speculative fix.

## 3. Establish the short-height failure contract

**Dependencies:** None. **Scope:** Medium investigation/test slice.
**Likely files:** `src/ui/multi_pane_layout.rs`,
`src/model/series/histogram_pane_view.rs`, `src/model/price_scale.rs`, focused tests.

- [x] Trace complete upstream scale/Histogram resize behavior and real consumers;
  verify graph coverage and identify the first invalid intermediate value in Rust.
- [x] Add RED tests for the 300×100/300×180 demo shapes and the actual failure
  boundary, including resize down/up and live append rather than only static data.
- [x] Record the required empty/finite behavior, error semantics, ownership and
  viable fixes before modifying the engine.

**Verification:** Regression fails for the demonstrated reason; neighboring valid
sizes and invalid-input controls distinguish layout degeneration from invalid data.

## 4. Fix short-height rendering and recovery

**Dependencies:** 3. **Scope:** Small/medium; fix the earliest violated invariant.
**Likely files:** Only the proven scale/layout/view boundary and its focused tests;
reuse the public demo/reference fixture for integrated recovery coverage.

- [x] Preserve source-compatible finite or empty output at the boundary without
  blanket error suppression, fabricated coordinates or destructive data clearing.
- [x] Test resize recovery with append and manual/automatic price scale, relevant
  price modes, two panes, fractional ratios and narrow widths.
- [x] Check other built-ins sharing the affected conversion; retain invalid-data
  rejection and stable source/hit identities.

**Verification:** RED tests turn GREEN; affected source comparisons/captures pass;
the public demo resizes down and back without an error or stale frame.

## Checkpoint A: Software interaction and short sizes

- [x] Tasks 1–4 have evidence-backed outcomes, focused checks pass and the build is clean.
- [x] Software improvement and short-height correction are reviewed independently;
  neither is claimed to explain the other without evidence.

## 5. Attribute the remaining update comparison

**Dependencies:** None; measured runs must be isolated from tasks 1–4.
**Scope:** Medium diagnostic slice.
**Likely files:** Existing `examples/preparation_profile.rs`,
`examples/support/preparation_diagnostics.rs`, `examples/support/update_counters.rs`,
a focused analysis artifact/report.

- [x] Reuse the actual timed fixture and matched same-history controls to localize
  the cost behind the unresolved p95 comparison; preserve allocation/cache evidence.
- [x] Distinguish useful cache/memory/scheduling evidence from an identified engine
  defect; document any measurement limitations or permissions needed.
- [x] Record a causal, actionable operation before selecting an implementation;
  otherwise leave the failure unresolved rather than declare it noise or passed.

**Verification:** Raw samples and counters join correctly; enabled instrumentation
is excluded from acceptance; reproducible diagnostic and interpretation recorded.

## 6. Implement the justified update fix

**Dependencies:** 5 identifies an actionable cost. **Scope:** Small/medium.
**Likely files:** The identified update/data/scale operation and focused tests;
existing bounded-work/reference fixtures if that operation affects them.

- [ ] Add a behavioral or bounded-work regression that would catch the established defect.
- [ ] Apply the smallest fix while preserving append, replacement, middle insertion,
  whitespace, ownership and custom-series semantics relevant to the changed path.
- [ ] Demonstrate improvement with matched diagnostics before preparing acceptance.

**Verification:** Focused regressions, relevant upstream comparisons and update
operation guards pass. An unproven cause does not complete this task.

## 7. Attribute the GPU presentation miss

**Dependencies:** None; timing studies run serially. **Scope:** Medium.
**Likely files:** `examples/frame_latency_profile/native.rs`, `presentation.rs`,
existing diagnostic reader and report; inspect exact Iced runtime before changing it.

- [x] Reproduce/analyze narrow/100k/pan with correlated wall/CPU times, presentation
  feedback, frame deadlines, buffer waits and scheduling evidence where available.
- [x] Separate rendering cost from waiting/descheduling and validate clock/feedback
  semantics; report gaps without substituting submission return for presentation.
- [x] Identify an actionable frame/runtime defect or retain explicit unresolved
  attribution; do not erase the 43.006836 ms failed result.

**Verification:** Diagnostic trace accounts for frame identities and discarded/
unresolved feedback; reader tests and native teardown behavior remain correct.

## 8. Implement the justified frame fix

**Dependencies:** 7 identifies an actionable defect. **Scope:** Small/medium.
**Likely files:** Only the proven frame scheduling/drawing/feedback boundary and
focused tests; application/runtime integration only if evidence requires it.

- [ ] Add a regression for the established defect; preserve gesture, resize and
  feedback lifetime semantics with explicit ownership for native resources.
- [ ] Measure the corrected operation without new spinning, missed damage, dropped
  input or a chart-engine threading change.
- [ ] Retain the distinction between an engine fix, a measurement correction and
  an external environmental limitation in the report.

**Verification:** Focused tests and serial before/after traces pass; environmental
attribution alone never changes a failed acceptance result into PASS.

## Checkpoint B: Candidate ready for acceptance

- [ ] Actionable software, short-height, update and presentation fixes have tests,
  measured evidence where relevant, and review; unresolved items remain unchecked.
- [x] Fmt, all-target Rust tests/check/strict Clippy, Python acceptance tests,
  affected source-reference checks and both-backend capture comparisons pass.

## 9. Run unchanged automated acceptance

**Dependencies:** Checkpoints A and B. **Scope:** Medium verification/report slice.
**Likely files:** Fresh local artifact directories; new report/measurement summaries;
existing acceptance readers remain unchanged unless a separately proven reader bug exists.

- [x] Freeze source/binary/policy hashes; run the six fixed 2,000-sample update pairs
  with full same-history controls and unchanged <=10% median/p95 growth rule.
- [x] Run the fixed three native sets (96 phase cohorts) with unchanged work p95
  <=16.666667 ms and presentation p95 <=33.333334 ms; stop per existing policy.
- [x] Retain every raw failure/invalid attempt and exact command; publish PASS,
  FAIL or INCONCLUSIVE without filtering, overlapping jobs or rerunning to green.

**Verification:** Existing readers/tests agree with complete raw artifacts;
missing data/backend fallback/discards are handled by the fixed policy.

## 10. Complete physical checks and decide acceptance

**Dependencies:** 9 passes for final acceptance; physical diagnosis can happen earlier.
**Scope:** Small/medium hardware/report slice.
**Likely files:** `tasks/live-append-mouse-acceptance.md`, new acceptance report,
`tasks/incremental-append-todo.md` and README links.

- [x] User tests both backends at recorded regular/half/problem sizes with live
  feed: zoom, diagonal pan, price-axis wheel/drag, box edit/cancel, release/capture
  and stable layout; short-height recovery included. Record concrete results.
- [x] Record mouse, touchpad, touchscreen and high-DPI coverage separately;
  unavailable hardware remains untested and external input-to-photon is not inferred.
- [x] Close only demonstrated gates and publish the remaining scope. Full hardware
  acceptance requires its devices, or an explicit user decision narrowing scope.

**Verification:** Every accepted device/backend/size row has observed evidence;
automated and physical outcomes remain separate and historical failures linked.

## Checkpoint C: Live-update milestone accepted

- [ ] Unchanged automated gates PASS and required physical acceptance scope is met.
- [ ] Final report states exact scope and residual limitations; no unresolved
  software drag failure or short-height error is silently waived.

## 11. Plan conflation rendering

**Dependencies:** Checkpoint C. **Scope:** Medium source/design planning slice only.
**Likely files:** New conflation plan/checklist and source-contract report.

- [ ] Trace upstream eligible series, pixel grouping, aggregation/extrema, whitespace,
  hit/crosshair mapping and invalidation; inspect existing Rust preparation consumers.
- [ ] Compare two minimal Rust designs and select the first end-to-end series path
  from actual source behavior, preserving raw data and update/index identities.
- [ ] Write the next implementation roadmap with deterministic upstream fixtures,
  rendering/hit regressions and performance goals established before measurements.

**Verification:** Each design requirement maps to real source consumers and a
planned behavioral check; no conflation implementation is started in this roadmap.

## Current evidence

- Actual-demo software render/present CPU p95 fell from 117.512 to 11.179 ms;
  the user confirmed smooth plot and price-axis dragging at the problem size.
- Short-height finite projection and resize/live-update recovery regressions pass.
- Exact cached/direct pixel checks pass all 768 comparisons across both renderers.
- Frozen-candidate update result: INCONCLUSIVE (two history and three control flags).
- Native acceptance stopped after set 1: 31/32 cohorts passed; software regular
  1,200-row pan presentation p95 was 43.722392 ms. All 16 GPU cohorts passed.
- No actionable remaining update/presentation defect has been established.
  Tasks 6/8, full physical acceptance and conflation remain blocked by their gates.

## Timing attribution continuation and final mouse result

The [follow-up report](../docs/porting/live-update-timing-attribution.md) records
cache-miss IP sampling, correlated native scheduling and their limitations.
Copied diagnostic builds now have distinct package identities; a real executable
regression catches stale Cargo reuse. One software delay contains 4.47 ms of
involuntary preemption, while two other delays have almost no off-CPU frame work.
There is still no justified additional engine/presentation fix, so tasks 6/8 and
unchanged automated acceptance remain open.

The user passed all normal-build mouse checks on both backends at regular, half
and problem sizes, including short-height recovery. Exact sizes were not logged
by the normal build. Touchpad, touchscreen and high-DPI coverage remain untested;
Checkpoint C and conflation remain open.
