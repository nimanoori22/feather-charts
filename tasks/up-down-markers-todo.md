# Up/down update markers execution checklist

Plan: [up-down-markers-plan.md](up-down-markers-plan.md).
Baseline `16f3622`; implementation, automated gates and software/GPU mouse acceptance pass. Original likely files below are
provisional; include callers/exports and split slices exceeding about five files.
Checkpoints are verification gates, not user-permission prompts.

## 0. Freeze behavior and design

Dependencies: none. Scope: 2–4 docs/oracle fixture files.
Likely files: port report/source manifest, actual-source comparison script,
focused reference fixture tests.

- [x] Pin all seven built-in TS files and semantic imports, wrapper/adapter, series
  update and draw-order consumers; check graph coverage and read gaps directly.
- [x] Resolve baseline-relative updates, zero values, appends, duration zero versus
  comments, insertion order, reset/clear, stale option sharing and failed updates.
  Separate source parity from explicit native corrections in fixtures/docs.
- [x] Confirm ownership, ID/error API, supported Line/Area data paths and explicit
  clock/deadline behavior; freeze reference counts and default-off baselines.

Verification: execute actual TS with deterministic timers/series/time-scale stubs
and recording canvas. Required behavior maps to cases before production edits.

## 1a. Establish the public attachment boundary

Dependencies: 0. Scope: 3–5 API/preparation/test files.
Likely files: src/api/up_down_markers.rs, API exports, primitive preparation,
tests/up_down_markers.rs.

- [x] Add validated sign/marker/options and typed supported-series attachment.
- [x] Record a compiling public visible-output RED before implementing rendering.
- [x] Use existing series ownership/normal primitive order; no new hit/autoscale hook.

Verification: typed invalid-input/unsupported-owner tests and meaningful RED.

## 1b. Render manual markers

Dependencies: 1a. Scope: 2–4 renderer/preparation/test files.
Likely files: src/renderers/up_down_markers_renderer.rs, exports and focused tests.

- [x] Draw source circle/open-chevron geometry with exact DPR rounding/strokes,
  neutral series color, clipping and sign colors. Do not reuse annotation arrows.
- [x] Support manual set/query/clear with one marker per key and source order.
- [x] Prove old frames stay frozen and both native backends draw visible output.

Verification: actual-source renderer oracle, public pixel/order checks.

## Checkpoint A

- [x] Public manual markers and source rendering behavior pass.

## 2. Add marker-aware data updates

Dependencies: 1b. Scope: 3–5 controller/data/test files per slice.
Likely files: API/controller, src/api/chart.rs, focused update tests, oracle.

- [x] Implement explicit tracked setData/update commands with existing validation,
  latest/historical routing and canonical data ownership; keep normal APIs usable.
- [x] Test initial baseline, repeated changes, positive/neutral/negative, zero/
  negative baseline, append/update, whitespace and full reset without implicit clear.
- [x] Reject invalid/historical-conflated operations atomically: no changed data,
  marker, baseline or deadline after an error; document direct-API bypass behavior.

Verification: fake-clock actual-source update oracle plus public data/marker checks;
source-native corrections have separate expected results and explanations.

## 3a. Add deterministic expiration state

Dependencies: 2. Scope: 2–3 state/test files.
Likely files: controller/expiry state and clock tests.

- [x] Store optional monotonic deadlines; replace/clear/remove cancel old expiries.
- [x] Cover immediately before/at/after deadline, delayed ticks, zero/overflow,
  manual indefinite markers, options changes and repeated refresh insertion order.
- [x] Limit expiry work/storage to active markers; do not retain stale timer entries
  or scan canonical history. Expose earliest deadline for the existing host/widget.

Verification: deterministic advance(now) tests; no sleeps or wall-clock flakiness.

## 3b. Integrate existing Tick and redraw scheduling

Dependencies: 3a. Scope: 3–5 integration/test files.
Likely files: src/api/interaction.rs, src/api/widget.rs, src/api/chart.rs, widget tests.

- [x] Merge marker expiry with touch/animation deadlines; publish Tick when due
  even with feed/pulse disabled. No 16 ms redraw loop solely for waiting markers.
- [x] Ensure the price-animation early return cannot swallow expiration; unchanged
  pulse-only ticks keep their fast path before a marker deadline.
- [x] Stop scheduling after expiration/clear/detach/series removal; test early/late
  redraws, empty data, deadline replacement and headless explicit Tick handling.

Verification: state/widget event tests prove idle expiration and no idle redraw loop.

## Checkpoint B

- [x] Managed updates and deadline behavior pass, including idle and pulse coexistence.

## 4. Close lifecycle and work bounds

Dependencies: 3b. Scope: 2–4 tests/preparation files.
Likely files: public lifecycle tests, primitive lifecycle hooks, software damage test.

- [x] Cover options/neutral color updates, manual markers, clear/full reset,
  multiple controllers/series, move/remove/hidden series and generic primitive removal.
- [x] Check frozen old frames and coexistence with regular annotations, conflation,
  watermarks, price lines, pulse and editable primitives; no hit/autoscale changes.
- [x] Prove no history-sized work per managed update/tick, bounded active expiry
  storage, no unnecessary preparation before expiry, and one software damage region.

Verification: focused lifecycle/work tests, existing primitive/marker/animation suites.

## 5. Provide the public demo

Dependencies: 4. Scope: 2–3 demo/README files.
Likely files: examples/public_chart.rs, examples/public_chart/up_down_markers.rs, README.

- [x] Add opt-in latest/historical updates and manual/clear/lifetime/color controls
  for Line and Area, using deterministic managed baseline data.
- [x] Permit pausing live feed/disabling pulse so idle expiration is observable.
- [x] Preserve default-off UI and all existing navigation/box interactions;
  include narrow/short-height recovery. No numeric yield-curve feature added.

Verification: demo controls/state tests and software/GPU smoke runs.

## Checkpoint C

- [x] Integrated feature and work/lifecycle gates pass.

## 6. Capture and native acceptance

Dependencies: 5. Scope: 2–4 fixture/runner/evidence files.
Likely files: demo capture helper, finite capture runner, port evidence directory.

- [x] Run the plan's 120 feature captures and 24 default-off comparisons with frozen
  clock/state, source/binary/fixture identity and repeated-frame pixel assertions.
- [x] Inspect representative DPR, boundary, historical and mixed scenes; retain
  failures and distinguish degenerate clipping from expected visible cases.
- [x] Record mouse wheel zoom, diagonal pan, price-axis wheel/drag and box edits
  on both renderers at regular/narrow/problem sizes, with short-height recovery.
  Verify positive/neutral/negative and idle expiry; record other devices separately.

Verification: unchanged capture assertions and recorded native responses pass.

## 7. Validate and document

Dependencies: 6. Scope: checks/docs, about 3 files.
Likely files: README, source/port report/evidence, checklist.

- [x] Pass cargo fmt --check, cargo test --all-targets, cargo check --all-targets,
  cargo clippy --all-targets -- -D warnings and affected source oracles.
- [x] Review source contract/corrections, API ownership, mutation atomicity, deadline
  cancellation, no-polling idle behavior, bounds, drawing order and compatibility.
- [x] Publish exact files, alternatives, clock/data API usage, capture/work/native
  results, date/time-only scope and device limitations.

## Checkpoint D

- [x] Feature implementation and automated/source/native gates pass.
- [x] Physical device gaps and historical timing acceptance remain explicitly separate.

## Execution notes

Core commit `e53198f`; integrated demo/tests commit `1d3b361`. Actual-source
oracle: 54 renderer cases and 14 timeline states. All-target suites: 654 passing
tests, zero failures/ignored. Final software/GPU matrix: 120 feature captures and
24 unchanged default-off images. Report and evidence:
[up-down-markers.md](../docs/porting/up-down-markers.md).

Source guards historical edits only while effective conflation is active; sparse
historical updates remain valid. There is no current public series-move API;
pane-index shifts and removal are tested without adding another feature.
Software/GPU mouse acceptance passed on explicitly selected renderers, including
idle expiry and manual persistence. Other devices remain untested; historical
timing acceptance remains OPEN/deferred.
