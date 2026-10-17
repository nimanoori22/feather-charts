# Last-price animation execution checklist

Plan: [last-price-animation-plan.md](last-price-animation-plan.md).
Starting checkpoint: `cdf80a4`. Implementation and automated/mouse acceptance complete.
Results: [port report](../docs/porting/last-price-animation.md).
Use the source → consumers → contract → design → implementation → test workflow.
Likely file names are provisional; count exports/callers and split slices that
grow beyond roughly five files. No additional approval gate is implied.

## 0. Complete the source contract and integration design

Dependencies: none. Scope: medium investigation/documentation.
Likely files: `docs/porting/last-price-animation.md` and
`docs/porting/last-price-animation-source.json` (new), this plan/checklist.

- [x] Pin the actual TS working-tree revision and hashes; finish source tests,
  imports, real callers and destruction/option-change reads with graph coverage
  checks and direct fallback for changed files.
- [x] Resolve update-flag provenance (including historical/full replacement),
  terminal dot/frame, transparency, source top-layer ordering and canonical
  last-price placement under conflation.
- [x] Confirm owner, explicit clock propagation, frozen payload and fast-path
  eligibility against current Rust consumers; inspect exact Iced/Flowsurface
  scheduling APIs. Compare the plan's alternatives and record chosen signatures.

Verification: every required behavior maps to a source consumer and test case;
no unresolved clock/draw-order/dirty-state decision is silently assumed in code.

## 1. Establish the actual-source oracle and RED cases

Dependencies: 0. Scope: medium, 2–3 files.
Likely files: `scripts/check_last_price_animation_reference.cjs`,
`examples/last_price_animation_reference.rs`, `tests/last_price_animation.rs`.

- [x] Execute actual TS view/renderer/trigger consumers with an injected
  performance clock and recording canvas. Include phase and extension boundaries,
  transparent colors, line widths, DPR rounding and endpoint visibility.
- [x] Add public behavioral RED cases for currently ignored enabled modes and
  retained output. Demonstrate missing behavior rather than only missing types.
- [x] Keep Rust/source comparison fixtures deterministic and check fixture counts;
  freeze triggering sequences, not a second handwritten animation specification.

Verification: source fixture self-checks pass; Rust RED output documents the gap.
Run the source script and focused public tests as the relevant fixtures exist.

## Checkpoint A — Contract

- [x] State, trigger, renderer and scheduling requirements are testable and mapped.

## 2. Implement synchronous model timing state

Dependencies: 1. Scope: medium, about 3 files.
Likely files: `src/model/last_price_animation.rs` (new), `src/model/mod.rs`,
`src/model/series.rs`; keep focused state tests beside the implementation.

- [x] Implement the 2,600 ms cycle, three modes, inclusive end and burst extension
  with explicit Instant inputs. Keep clock acquisition at defined boundaries.
- [x] Store state only for eligible families and consume accepted update metadata
  once through both incremental and full application paths, including clear.
- [x] Cover option transitions and unrelated mutations; invalid/rejected updates
  leave state intact. No repeated trigger from a stored last_update_info flag.

Verification: exact state/sequence oracle and model tests pass; `cargo test --lib`
and `cargo check --all-targets`. No sleep-based timing assertions.

## 3. Implement source-derived circle rendering

Dependencies: 2. Scope: medium, about 4 files including exports.
Likely files: `src/renderers/series_last_price_animation_renderer.rs` and its
module export; `src/views/pane/series_last_price_animation_pane_view.rs` and its
module export. Revise placement if the audited existing abstraction fits better.

- [x] Resolve canonical last index/price/color and first-value conversion; keep
  this independent of conflated row identity and representative extremes.
- [x] Prepare dot/fill/stroke commands with source rounding, line-width/radius,
  alpha multiplication and null/empty/offscreen handling.
- [x] Keep prepared drawing data owned and immutable; renderer output introduces
  no hit target, model borrow, clock read or autoscale contribution.

Verification: pure geometry/color and source renderer-command oracle pass across
DPR 1/1.25/1.5/2; retained command data remains unchanged after state advances.

## Checkpoint B — Pure behavior

- [x] Timing and rendering match source without a window or asynchronous work.

## 4. Deliver Line through public and low-level preparation

Dependencies: 3. Scope: medium, at most about 5 files; split model/frame wiring
from public integration if the audit finds additional owners.
Likely files: `src/ui/line_chart.rs`, `src/ui/multi_pane_frame.rs`,
`src/api/chart.rs`, chosen model/preparation helper, `tests/last_price_animation.rs`.

- [x] Wire Line's existing mode option into frozen prepared frames; disabled
  frames preserve their current output and existing hosts remain supported.
- [x] Match top-pane order and clipping; the inactive OnDataUpdate center dot
  persists while retained old frames do not evolve on later draws.
- [x] Verify canonical endpoint/color under conflation, row overrides, pan/zoom,
  manual/inverted/percentage/log scales and series without a visible last bar.

Verification: public/low-level behavioral tests and fixed-time Line captures pass.
This is a functional checkpoint, not acceptance of a full-prepare-per-Tick loop.

## 5. Integrate native deadlines and animation-only updates

Dependencies: 4. Scope: medium, about 4 files.
Likely files: `src/api/chart.rs`, `src/api/interaction.rs`, `src/api/widget.rs`,
`src/api/last_price_animation.rs` (private helper if needed).

- [x] Reuse the existing Tick/redraw loop and wake deadlines; preserve kinetic,
  touch and cache-warmup scheduling. Do not add another timer/subscription.
- [x] Guard a stage-only update path with real model/layout dirtiness. Pure pulse
  ticks retain settled endpoint/base data and avoid history/cache/glyph rebuilds;
  simultaneous real work still uses normal preparation.
- [x] Present one final clearing frame when needed, then stop pulse work after
  expiry/disable/removal or when no drawable marker exists. Resume the correct
  elapsed phase after hidden/offscreen periods; do not reset it on wheel input.

Verification: deterministic redraw/deadline/lifecycle tests and work counters;
native Line zoom/pan/crosshair remains responsive. Preserve the current single
damage boundary unless a tested alternative passes both native backends.

## Checkpoint C — Usable native Line

- [x] All modes work end to end; animation-only work is bounded and idle work stops.

## 6. Extend shared behavior to Area and Baseline

Dependencies: 5. Scope: medium, about 3–5 files.
Likely files: shared preparation helper, affected composition points,
`tests/last_price_animation.rs`, source reference fixture.

- [x] Reuse the same timing/rendering state; resolve Area row line colors and
  Baseline above/below-base line colors with the source's last-value color rule.
- [x] Preserve gradients, curved/stepped geometry, fills, markers and raised
  outlines; unsupported families remain unaffected and add no animation options.
- [x] Verify all three families at raw and conflated levels, multiple panes,
  left/right axes and price-scale modes.

Verification: family/color source oracle, public tests and both-backend captures.

## 7. Close lifecycle and mixed-scene regressions

Dependencies: 6. Scope: medium tests, 2–4 files; any discovered root fix is a
separate focused slice with a failing regression first.
Likely files: animation integration tests, model tests and existing capture support.

- [x] Cover initial population, latest/append/bursts, full replacements,
  historical updates where allowed, whitespace, clear/reload and shared reindexing.
- [x] Cover mode/style/visibility switches, offscreen return, removal mid-pulse,
  old frames after removal, multiple clocks/series and coincident kinetic/warmup
  work. Viewport cancellation must not reset an unrelated price-pulse origin.
- [x] Verify crosshair/event canonical data, price lines, markers, top primitives,
  hover ordering and short-height recovery with active animation.

Verification: integration suite, source transition sequences and mixed-scene
captures pass; failures retain their reproduction steps and evidence.

## Checkpoint D — Series and lifecycle coverage

- [x] Three supported families pass; source-independent chart behavior is preserved.

## 8. Add the demo and bounded work/performance verification

Dependencies: 7. Scope: medium, 3–5 files; split the fixture/runner if necessary.
Likely files: `examples/public_chart.rs`, existing profile/capture support,
focused reference/profile fixture and README.

- [x] Add an opt-in demo path for the modes using existing series options and
  live-feed controls; support dense conflation, sparse zoom and multiple series.
- [x] Prove no canonical-history, conflation, tick-selection or glyph rebuild on
  unchanged animation-only ticks, with disabled output/work remaining unchanged.
- [x] Freeze and run the plan's finite matched diagnostic batch once. Retain all
  work counts, build/source identity, preparation/geometry/raster timings and any
  failures; do not tune acceptance thresholds or repeat until green.

Verification: count assertions and fixture self-checks pass; report results as
diagnostic where variation prevents a speed conclusion. No old timing gates rerun.

## 9. Validate, review and document

Dependencies: 8. Scope: medium verification/documentation.
Likely files: `docs/porting/last-price-animation.md`, README, this checklist.

- [x] Pass `cargo fmt --check`, `cargo test --all-targets`,
  `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, affected
  source oracles and both-backend fixed-time captures. Review the final diff.
- [x] Record real mouse zoom/pan/axis checks with active pulse/live feed at
  regular/narrow sizes on both backends. Distinguish those results from simulated
  DPR, touch and hardware coverage that was unavailable.
- [x] Publish source/consumers, required/incidental behavior, API/ownership,
  alternatives, exact files, tests, work/renderer evidence and remaining gaps.

## Completion

- [x] Implementation is complete for Line, Area and Baseline with default-off parity.
- [x] Automated parity, lifecycle, rendering and work-bound gates pass.
- [x] Physical results and unavailable device coverage are explicitly recorded.
- [x] Historical timing acceptance remains separately OPEN with unchanged budgets.

## Execution notes

Completed 2026-10-17: 613 all-target tests, strict Clippy/check/fmt, actual-source
oracles, 48 retained diagnostic runs (3,840 measured frames), 160 unchanged
disabled-output captures, 18 final fractional-DPR captures, and both native mouse
checks. The overlap RED caught a top-marker ordering bug and its regression passes.
Core timing tests use explicit Instants; production time is acquired only at
creation/mutation/preparation boundaries. Native Tick samples frozen payloads.

The finite performance batch had overlapping compilation/tests, so its timing
numbers are diagnostic only. No original timing gates were rerun or relaxed.
The report names the small fixes made after that batch and retains its original
source/binary identity instead of relabeling the measurements. No retries to green.

Mouse acceptance covers Line plus volume and editable primitives; shared family
behavior is tested automatically. Touchpad/touchscreen/physical high-DPI coverage
remains untested. Historical timing acceptance is OPEN/deferred.
