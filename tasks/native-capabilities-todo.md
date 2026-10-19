# Native capability checklist

Plan: [native-capabilities-plan.md](native-capabilities-plan.md). Baseline: `fb096eb`.

## A — Display-P3: NOT IMPLEMENTED (planning only)

- [ ] A0: Freeze source/native capability matrix.
- [ ] A1: Define managed color preparation and reference vectors.
- [ ] A2: Implement and verify each supported native screen backend.
- [ ] A3: Implement color-space-aware owned export.
- [ ] A4: Complete normal and wide-gamut hardware acceptance.

Existing sRGB rendering and configured parsing are implemented; they do not close A.

## B — Broader Intl expansion: NOT IMPLEMENTED (planning only)

- [ ] B0: Freeze the chart-used locale/calendar contract and source fixtures.
- [ ] B1: Select formatter/data approach and dependency policy.
- [ ] B2: Implement the expanded calendar/locale formatting paths.
- [ ] B3: Complete source, renderer/export and normal mouse acceptance.

Existing H verified Gregorian profiles remain implemented; they do not close B.

## C — Runtime default price axis: IMPLEMENTED AND ACCEPTED

- [x] C0: Freeze source contract and reproduce missing public behavior.
- [x] C1: Implement the typed atomic patch/readback and propagation.
- [x] C2: Verify downstream behavior and add existing-demo controls.
- [x] C3: Complete gates, renderer/mouse checks, inventory and local checkpoint.

Do not infer a pass for deferred hardware/timing/drawing-order work.

Evidence: [runtime-primary-axis.md](../docs/porting/runtime-primary-axis.md). Software and GPU mouse acceptance passed in all four modes; GPU confirmation was supplied after Codex was accidentally closed.
