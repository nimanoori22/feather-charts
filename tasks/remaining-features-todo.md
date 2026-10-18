# Remaining public features checklist

Status: milestones A/B/C and combined validation accepted with the available mouse on software/GPU, 2026-10-18.
Plan: [remaining-features-plan.md](remaining-features-plan.md). Baseline `23476f1`.
Do not overwrite earlier plans or close their independent exceptions.

## A — Reusable screenshot/export

- [x] A0: Complete source/consumer/Iced audit; freeze real source flags, layering,
  ownership, freshness, sizing, alpha/limits/errors and baseline provenance.
- [x] A1: Separate plot draw selection with unchanged ordinary screen pixels.
- [x] A2: Apply source-correct selection to axes, labels and full pane composition.
- [x] Checkpoint A-layout: review source semantics and save tested local commit.
- [x] A3: Expose owned prepared/fresh screenshot capture in all three frontends;
  preserve events/gestures/errors and snapshots after owner changes/drop.
- [x] A4: Deliver reusable renderer/export image API with checked dimensions,
  explicit actual backend, verified RGBA contract and bounded allocation.
- [x] A5: Deliver public-API preview/file example and layer/ratio/size controls.
- [x] A6: Pass frozen source/capture/compatibility and both available mouse checks;
  record normal exits, final validation and a local milestone commit.

Verification: current-layout image sizing and fresh/prepared differences are explicit;
all source flag combinations, custom/overlay content and immutable snapshots pass.

## B — Application-defined horizontal behavior

- [x] B0: Audit createChartEx and real consumers; freeze factory, initialization,
  original-item/key/cache, configuration, errors and ownership contracts.
- [x] B1: External non-Clone behavior constructs through public API, with useful
  owned conversion/data errors and unchanged existing constructors.
- [x] B2: Verify conversion, ordering/weights/labels/cache resets, canonical data,
  custom series, original-item events, primitives/navigation and screenshot export.
- [x] Checkpoint B-public: review real external consumer and save tested local commit.
- [x] B3: Deliver a genuine custom-domain demo using no library-private internals.
- [x] B4: Pass new source cases, old source/compatibility gates and both backend
  custom-domain mouse/export checks; save evidence and a local milestone commit.

Verification: behavior-instance equivalence/isolation is documented and tested;
runtime formatting changes do not silently reinterpret populated horizontal keys.

## C — Attribution logo

- [x] C0: Audit/freeze vector, text-color theme, last-pane placement, construction/
  runtime policy, activation priority and source screenshot exclusion.
- [x] C1: Render/cache native logo geometry and add supported configuration/readback;
  preserve existing default-off pixels, layout/autoscale and snapshot ownership.
- [x] C2: Route fixed-destination activation to the host; migrate enum callers and
  test cancellation without plot/edit/axis/divider interference.
- [x] Checkpoint C-input: review lifetime/input policy and save tested local commit.
- [x] C3: Pass source, disabled-baseline, enabled visual/export and both backend
  mouse checks; record final report/evidence and a local milestone commit.

Verification: logo follows the last pane through reorder/removal, uses source theme/
bounds at fractional DPR, and stays absent from source-compatible screenshot output.

## D — Combined completion (D0)

- [x] All-target tests, doctests, all-target check, formatting, strict Clippy and
  diff hygiene pass on the final unchanged source.
- [x] All 34 baseline source comparisons plus the new feature oracles pass;
  exact image scene/count matrices and compatibility checks remain unchanged.
- [x] Source/design/ownership/API/migration/changed-files/tests/evidence/remaining-
  dependency reports and README examples are complete.
- [x] Available software/GPU mouse and export acceptance is recorded for the final
  relevant behavior; normal exits are verified and failed runs retained.
- [x] Final local commits and clean worktree; push only when requested.

## Feature acceptance

- [x] Apps obtain owned chart images with source-correct layer controls, safe output
  sizing and preserved chart/event/gesture ownership across existing modes.
- [x] An external application can construct and use a chart with its own horizontal
  behavior, typed custom data and export without private access or runtime wrapping.
- [x] Attribution renders/activates correctly when enabled, follows last-pane
  lifecycle and preserves source-compatible screenshot exclusion.
- [x] Earlier timing, dedicated drawing-order acceptance and unavailable physical
  device checks retain their independent status; this checklist does not close them.

## Current checkpoint

Core capture/export checkpoint: `801ed3b`. Automated screenshot verification passes:
772 tests/75 suites, 2 doctests, strict Clippy/check/format, all 35 source comparisons,
1158 exports and the unchanged 96-image baseline gate. The user accepted screenshot/export and explicitly requested the next step. Proceed
to B under that instruction. A6 retains its missing backend-specific physical
record; no separate GPU result is inferred from the general acceptance statement.
See [the screenshot report](../docs/porting/screenshot-export.md) and
[evidence](../docs/porting/screenshot-export-evidence.json). Nothing has been pushed.


## Custom-horizontal checkpoint

Public API/data-safety checkpoint: `b60bdf6`. Full regression validation passed
781 tests across 77 suites and two doctests; final demo review passed its focused
control/idle-feed check, all-target check and strict Clippy. All 36 source comparisons,
240 final custom-domain exports and 96 unchanged baseline comparisons pass.
The source and renderer evidence is retained under `target/custom-horizontal`.
The user passed software/GPU mouse/export acceptance; both final-binary processes
exited normally and saved RGBA lengths match their size metadata. B4 is accepted
for the available mouse; independent device/timing exceptions remain open. See the [contract](../docs/porting/custom-horizontal.md)
and [evidence](../docs/porting/custom-horizontal-evidence.json). Nothing was pushed.


## Attribution and combined completion checkpoint

Core `ce8fd95`, four-domain demo `d02a839`, viewport/hover repair `e037a3b`.
Final runtime source passes 796 tests/78 suites, two doctests, format/check/strict
Clippy and all 37 source comparisons. Both actual backends pass 672 logo screens,
2688 exact export-exclusion and 2688 retained-export comparisons. All 1158 accepted
screenshot images, 240 custom-domain images and 96 legacy baseline images remain
identical; the 288 event captures also pass.

The user passed final software and GPU logo/interaction/preview/export checks in
calendar, strike, maturity and sessions. All eight processes exited normally on
the same final binary. These integrated screenshot controls complete A6's formerly
missing backend-specific record; the earlier general statement was not treated as
a GPU result. The final GPU saved image has valid 640×480 RGBA length. The shared
save path retains one image, not a separate image per mode; no software-run saved
file was present for inspection. File serialization also has automated coverage.
See [the attribution report](../docs/porting/attribution.md) and
[evidence](../docs/porting/attribution-evidence.json). A/B/C feature acceptance and
D combined completion apply to the available mouse; independent timing,
drawing-order and unavailable-device exceptions remain open. Nothing was pushed.
