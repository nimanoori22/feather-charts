# Persistent pane sizing (milestone E)

Baseline: `917c28b`. E0–E2 complete; automated gates and four-mode software/GPU mouse acceptance pass.

## Source and consumers

The complete `src/api/pane-api.ts` delegates `setHeight` to
`ChartModel.changePanesHeight`. The source model converts the requested height to
stretch allocation using the current total pane height and total stretch. It
clamps the target between 30px and the total minus 30px per other pane. Other
panes share the difference sequentially in pane order, with the same clamps.
This order matters when a neighbor reaches its minimum; it is not proportional
redistribution. `ChartWidget._adjustSizeImpl` subsequently applies those factors,
rounding to device pixels and giving the last pane the remainder, with a 2px
layout floor. The source graphics test `panes/change-height.js` and pane tutorial
exercise the public setter after initial layout. Pane move/swap retains each
pane's allocation; removal redistributes space according to remaining factors.

Graph discovery, model traces and exact snippets were supplemented with actual
source consumers because caller edges for `changePanesHeight` are sparse.
Coverage checks for all relied-on source and native paths reported no recorded
issues (best-effort evidence, not a completeness claim).

## Design, alternatives and ownership

Two viable implementations were considered: retaining explicit pixel constraints
in the layout owner, or converting each request into the existing stretch factors.
The latter preserves source behavior on window/divider resizing and pane
reorganization. The former would require conflict resolution and would pin heights
on resize instead of scaling them. Calling the engine's existing `set_pane_height`
alone is insufficient: `MultiPaneChartFrameOwner` overwrites it with weighted
allocation during prepare.

The public API is `Chart<B>::set_pane_height(&mut self, pane: usize, height: f64)
-> Result<(), ChartError>`, with a mutable forward on `YieldCurveChart`. Calendar,
strike and custom domains use the same generic owner. The chart owns panes and
their existing stretch factors; temporary calculated factors are validated before
mutation. No references, callbacks, retained wrappers, threads or async machinery
are added. Source wrapper identity and manual lifecycle management are unnecessary.

## Contract and native limits

Requests use the last settled layout; applications prepare once before requesting
pixel sizing. The next prepare applies the new allocation; subsequent window
resizing scales its proportions. Finite negative heights clamp like source.
Invalid pane indices or non-finite inputs return errors before mutation or gesture
cancellation. A single pane is unchanged. Exact factor no-ops are unchanged.
Changed allocation cancels active viewport interactions, animation and box previews
through the existing owner boundary. Ordinary divider behavior is untouched.

Before first layout, or when total height cannot leave positive allocation after
neighbor minimums, sizing is inert. This avoids the source's zero-division or
negative-factor state at collapsed heights, and preserves allocation for recovery.
The source's ordinary 30px clamp and sequential redistribution remain unchanged
for positive feasible allocations. This native safety adaptation is explicit;
short-window results are not claimed to be exact source parity.

## Verification

The source probe bundles the actual local 5.2.1 implementation, freezes watched
source hashes and records actual heights through change, resize, clamp, move and
removal. Native tests compare the same states, with an explicit initial 2:1:1
allocation matching the source fixture. Red API compilation evidence is retained
under `target/pane-sizing/red.log` before implementation. Behavioral tests cover
persistence through repeated prepare, resize, structure changes, finite validation,
negative/oversized clamps, unprepared/collapsed recovery and navigation cancellation.

Files: `src/api/pane_management.rs`, `src/api/yield_curve.rs`,
`tests/pane_sizing.rs`, `scripts/check_pane_sizing_reference.cjs`, and source
fixtures `pane-sizing-{source,states}.json`. Demo and renderer evidence follow.

Previous timing, formatting-GPU, dedicated drawing-order and untested physical
device exceptions remain independent. E does not implement metadata, locales,
configured parsers or Display-P3 rendering.

## Public consumer and renderer protocol

`pane_management --pane-sizing --screenshot-export --live-feed` exposes pane
selection, 80/150/300px requests, minimum/maximum clamps, invalid inputs and
Save/Restore layout. Existing pane moves/removal and divider controls remain.
Saved layouts are application-owned stretch snapshots by current pane slot;
restoring requires the same count and reapplies those proportions at the current
window size. It does not retain removed pane owners. The readout reports actual
settled logical pixel heights, including device rounding. Reset clears the save.

The four-domain consumer test uses only public mutations, checks restoration,
short-height recovery, count mismatch after removal and Reset. New captures cover
seven scenes (set, minimum, maximum, restore, moved, removed, recovered), all four
domains, 800×500 / 300×300 / 300×90 and DPR 1 / 1.25 / 1.5 / 2, on both asserted
renderers: 672 replayed screens and 672 retained exports. Snapshots are checked
again after restoring allocation, resizing and dropping their chart. No existing
capture protocol is weakened; previous protocols run unchanged for compatibility.

Additional files: `examples/pane_management{.rs,/host.rs,/sizing.rs,/sizing_captures.rs}`,
`scripts/capture_pane_sizing.py`, README, API inventory generator/readout and checklist.

Review reproduced an active-divider regression: a held resize used its old stretch
pair after a programmatic height change and overwrote the requested layout.
The existing real-canvas structural-cancellation test was extended with height
changes and failed on the stale factors (`red-divider.log`). The setter now uses
the full interaction-cancellation boundary, which advances the widget revision
and releases divider captures as well as plot/axis/box gestures. No-op and rejected
requests still return before cancellation. Divider behavior without an owner
mutation is unchanged.

## Corrected-build validation

The corrected code passes 849 tests in 85 suites, 2 doctests, example builds,
all-target check, strict all-target Clippy, formatting and inventory checks.
All 42 source comparisons pass. The new protocol passes all 672 screen replays
and 672 retained-export checks on actual tiny-skia and wgpu renderers. Current
calendar sizing, maturity restore and session recovery images were visually
inspected. The earlier all-target attempt could not execute a test binary
(`No such file or directory`) while a focused rebuild was running; that log is
retained separately. The corrected full run completes successfully.

Core checkpoints: `b3c478f` (source-compatible allocation) and `5522703`
(active divider cancellation). Earlier failed API compilation and the reproduced
divider regression are retained as red evidence. No prior comparison script,
capture assertion or timing threshold was weakened. Compatibility results and
physical acceptance below refer to the same current normal example binary.

All eight unchanged compatibility protocols pass with **5766 accepted images
unchanged**: events 912, custom horizontal 240, screenshot/export 1158, attribution
672, formatting 576, scale assignment 768, axis interaction 864 and public helpers
576. The previous 41 source comparison scripts and all eight capture scripts are
byte-for-byte unchanged from `917c28b`. Current production/demo files and normal
binary hashes match the capture protocol. The [pinned evidence record](pane-sizing-evidence.json)
records gate results, source states, protocol and log hashes, retained failures and
independent exceptions. Four-mode software and GPU mouse acceptance, previews/exports and all eight normal window exits pass.

## Physical acceptance

The user accepted the final normal tiny-skia and wgpu demos in calendar, strike,
maturity and sessions. Checks covered pane selection and 80/150/300px requests,
minimum/maximum and invalid inputs, Save/change/Restore and Restore after window
resize with unchanged pane count, divider dragging, pane move/swap/removal→Reset,
live-feed zoom/pan, time/price-axis drag/reset, box move/resize/Escape and release
outside at regular, narrow and the earlier problem size, short-height recovery
and preview/export. All eight sessions exited 0 and match the binary hash used
by the accepted captures. These checks do not close separate formatting-GPU,
timing, dedicated drawing-order or physical touchpad/touchscreen/high-DPI records.

No further source component blocks E. Application metadata is milestone F; native
pointer metadata, locale behavior and configured color parsing remain G–I.
