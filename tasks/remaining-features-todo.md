# Remaining public features checklist

Status: screenshot/export implementation in verification, 2026-10-18. Later milestones have not started.
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
- [ ] A6: Pass frozen source/capture/compatibility and both available mouse checks;
  record normal exits, final validation and a local milestone commit.

Verification: current-layout image sizing and fresh/prepared differences are explicit;
all source flag combinations, custom/overlay content and immutable snapshots pass.

## B — Application-defined horizontal behavior

- [ ] B0: Audit createChartEx and real consumers; freeze factory, initialization,
  original-item/key/cache, configuration, errors and ownership contracts.
- [ ] B1: External non-Clone behavior constructs through public API, with useful
  owned conversion/data errors and unchanged existing constructors.
- [ ] B2: Verify conversion, ordering/weights/labels/cache resets, canonical data,
  custom series, original-item events, primitives/navigation and screenshot export.
- [ ] Checkpoint B-public: review real external consumer and save tested local commit.
- [ ] B3: Deliver a genuine custom-domain demo using no library-private internals.
- [ ] B4: Pass new source cases, old source/compatibility gates and both backend
  custom-domain mouse/export checks; save evidence and a local milestone commit.

Verification: behavior-instance equivalence/isolation is documented and tested;
runtime formatting changes do not silently reinterpret populated horizontal keys.

## C — Attribution logo

- [ ] C0: Audit/freeze vector, text-color theme, last-pane placement, construction/
  runtime policy, activation priority and source screenshot exclusion.
- [ ] C1: Render/cache native logo geometry and add supported configuration/readback;
  preserve existing default-off pixels, layout/autoscale and snapshot ownership.
- [ ] C2: Route fixed-destination activation to the host; migrate enum callers and
  test cancellation without plot/edit/axis/divider interference.
- [ ] Checkpoint C-input: review lifetime/input policy and save tested local commit.
- [ ] C3: Pass source, disabled-baseline, enabled visual/export and both backend
  mouse checks; record final report/evidence and a local milestone commit.

Verification: logo follows the last pane through reorder/removal, uses source theme/
bounds at fractional DPR, and stays absent from source-compatible screenshot output.

## D — Combined completion (D0)

- [ ] All-target tests, doctests, all-target check, formatting, strict Clippy and
  diff hygiene pass on the final unchanged source.
- [ ] All 34 baseline source comparisons plus the new feature oracles pass;
  exact image scene/count matrices and compatibility checks remain unchanged.
- [ ] Source/design/ownership/API/migration/changed-files/tests/evidence/remaining-
  dependency reports and README examples are complete.
- [ ] Available software/GPU mouse and export acceptance is recorded for the final
  relevant behavior; normal exits are verified and failed runs retained.
- [ ] Final local commits and clean worktree; push only when requested.

## Feature acceptance

- [ ] Apps obtain owned chart images with source-correct layer controls, safe output
  sizing and preserved chart/event/gesture ownership across existing modes.
- [ ] An external application can construct and use a chart with its own horizontal
  behavior, typed custom data and export without private access or runtime wrapping.
- [ ] Attribution renders/activates correctly when enabled, follows last-pane
  lifecycle and preserves source-compatible screenshot exclusion.
- [ ] Earlier timing, dedicated drawing-order acceptance and unavailable physical
  device checks retain their independent status; this checklist does not close them.

## Current checkpoint

Core capture/export checkpoint: `801ed3b`. Automated screenshot verification passes:
772 tests/75 suites, 2 doctests, strict Clippy/check/format, all 35 source comparisons,
1158 exports and the unchanged 96-image baseline gate. A6 remains open for actual
mouse/preview/file checks on both backends; B/C must follow that acceptance gate.
See [the screenshot report](../docs/porting/screenshot-export.md) and
[evidence](../docs/porting/screenshot-export-evidence.json). Nothing has been pushed.
