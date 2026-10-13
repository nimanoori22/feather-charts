# Interactive primitive editing checklist

Plan: [primitive-editing-plan.md](primitive-editing-plan.md).
Status: implementation and automated checks complete; physical acceptance pending.
Performance limitation: overall preparation remains history-dependent; see the port report.

## 0. Contract and design

- [x] Trace upstream/Rust/Iced consumers with graph coverage and source fallback.
- [x] Record behavior, alternatives, ownership and public signatures.
- [x] Resolve synchronous capture, inverse conversion, stable viewport and preview updates.
- [x] Define handle precedence, minimum extent and every cancellation path.

## 1. Selection and handles (depends on 0)

- [x] Select/deselect using stable attachment identity; clear stale selection.
- [x] Render eight handles with matching immutable hit geometry.
- [x] Verify selection, overlap precedence and narrow-window layout.

## 2. Movement (depends on 1)

- [x] Add optional edit targets and typed begin/update/commit/cancel routing.
- [x] Add continuous owner-aware inverse conversion and stable gesture mapping.
- [x] Move from original bounds with captured drag; preserve ordinary panning/clicks.
- [x] Verify conversion modes and native input sequences.

## Checkpoint: selection and movement

- [x] End-to-end selected box moves without moving the viewport.
- [x] Existing navigation, hit arbitration and lifecycle tests pass.

## 3. Resizing (depends on 2)

- [x] Implement four edge and four corner constraints and resize cursors.
- [x] Clamp crossings/minimum extent without swapping handle identity.
- [x] Verify fractional positions, all handles, inverted axes and repeated edits.

## 4. Cancellation and lifecycle (depends on 2 and 3)

- [x] Restore on Escape/focus loss; clear capture and cursor on every exit.
- [x] Handle removal, external replacement, pane shifts and incompatible view changes.
- [x] Preserve external updates and restore prior navigation/autoscale policy safely.
- [x] Verify release outside plot, stale frames, live updates and subsequent fresh drags.

## Checkpoint: complete gesture behavior

- [x] Move/resize/cancel matrix passes without accidental pan or stale rollback.
- [x] Cleanup runs once per attachment lifecycle; all exit paths resume navigation.

## 5. Rendering and performance (depends on 4)

- [x] Capture selected/moving/resized/cancelled states on both backends at four ratios.
- [x] Verify labels, clipping, hit geometry and narrow-window layout stability.
- [x] Profile preview workloads; record baseline comparisons and limits.
- [x] Pass formatting, all-target tests/checks, strict Clippy and example builds.
- [x] Pass existing source-reference checks and recheck actual consumers.

## 6. Documentation and physical acceptance (depends on 5)

- [x] Document API, ownership, controls, validation evidence and remaining scope.
- [ ] Mouse: selection, movement, eight handles, crossings and release outside plot.
- [ ] Mouse: Escape/focus loss, live updates/removal and fresh gestures afterward.
- [ ] Touchpad: edits plus zoom/pan/price-axis navigation after editing.
- [ ] Half-size/high-DPI: no jitter, viewport movement or stuck cursor.
- [ ] Mark milestone complete only when automated and physical acceptance pass.

## Recorded limit

- [ ] Meet the original strict history-independent per-event performance target.
  Editing math and preview notifications are bounded; full chart preparation and
  live data updates still grow with history/source count. Measured debug limits
  and baseline comparisons are recorded in `docs/porting/primitive-editing.md`.
