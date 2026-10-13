# Interactive primitive editing checklist

Plan: [primitive-editing-plan.md](primitive-editing-plan.md).
Status: planned; no implementation started.

## 0. Contract and design

- [ ] Trace upstream/Rust/Iced consumers with graph coverage and source fallback.
- [ ] Record behavior, alternatives, ownership and public signatures.
- [ ] Resolve synchronous capture, inverse conversion, stable viewport and preview updates.
- [ ] Define handle precedence, minimum extent and every cancellation path.

## 1. Selection and handles (depends on 0)

- [ ] Select/deselect using stable attachment identity; clear stale selection.
- [ ] Render eight handles with matching immutable hit geometry.
- [ ] Verify selection, overlap precedence and narrow-window layout.

## 2. Movement (depends on 1)

- [ ] Add optional edit targets and typed begin/update/commit/cancel routing.
- [ ] Add continuous owner-aware inverse conversion and stable gesture mapping.
- [ ] Move from original bounds with captured drag; preserve ordinary panning/clicks.
- [ ] Verify conversion modes and native input sequences.

## Checkpoint: selection and movement

- [ ] End-to-end selected box moves without moving the viewport.
- [ ] Existing navigation, hit arbitration and lifecycle tests pass.

## 3. Resizing (depends on 2)

- [ ] Implement four edge and four corner constraints and resize cursors.
- [ ] Clamp crossings/minimum extent without swapping handle identity.
- [ ] Verify fractional positions, all handles, inverted axes and repeated edits.

## 4. Cancellation and lifecycle (depends on 2 and 3)

- [ ] Restore on Escape/focus loss; clear capture and cursor on every exit.
- [ ] Handle removal, external replacement, pane shifts and incompatible view changes.
- [ ] Preserve external updates and restore prior navigation/autoscale policy safely.
- [ ] Verify release outside plot, stale frames, live updates and subsequent fresh drags.

## Checkpoint: complete gesture behavior

- [ ] Move/resize/cancel matrix passes without accidental pan or stale rollback.
- [ ] Cleanup runs once per attachment lifecycle; all exit paths resume navigation.

## 5. Rendering and performance (depends on 4)

- [ ] Capture selected/moving/resized/cancelled states on both backends at four ratios.
- [ ] Verify labels, clipping, hit geometry and narrow-window layout stability.
- [ ] Profile preview workloads; record baseline comparisons and limits.
- [ ] Pass formatting, all-target tests/checks, strict Clippy and example builds.
- [ ] Pass existing source-reference checks and recheck actual consumers.

## 6. Documentation and physical acceptance (depends on 5)

- [ ] Document API, ownership, controls, validation evidence and remaining scope.
- [ ] Mouse: selection, movement, eight handles, crossings and release outside plot.
- [ ] Mouse: Escape/focus loss, live updates/removal and fresh gestures afterward.
- [ ] Touchpad: edits plus zoom/pan/price-axis navigation after editing.
- [ ] Half-size/high-DPI: no jitter, viewport movement or stuck cursor.
- [ ] Mark milestone complete only when automated and physical acceptance pass.
