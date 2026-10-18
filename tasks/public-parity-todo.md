# Public API and desktop parity checklist

Date: 2026-10-18. Status: implementation in progress; formatting core P0/A0–A3 complete.
Baseline: `55ba7d7`. Plan: [public-parity-plan.md](public-parity-plan.md).
Audit: [public-parity-audit.md](../docs/porting/public-parity-audit.md).
Preserve previous roadmap acceptance and independent exceptions.

## Planning checks

- [x] All ten audit groups mapped to ordered tasks and concrete application outcomes.
- [x] Dependencies, ownership/API direction, verification and local checkpoints specified.
- [x] Existing plans preserved; wide-gamut and locale support limits explicit.
- [x] Each task has acceptance conditions, verification and likely file responsibilities.

## Implementation tasks

### Baseline

- [x] P0: Freeze the implementation baseline and close the scope map. (depends: None).

### A — Formatting correctness

- [x] A0: Freeze formatting precedence, batch and lifecycle contracts. (depends: P0).
- [x] A1: Carry custom batch formatting through series and scale bridges. (depends: A0).
- [x] A2: Apply chart-wide formatting to price scales. (depends: A1).
- [x] A3: Expose atomic runtime localization updates. (depends: A2).
- [ ] A4: Deliver formatting controls and accept milestone A. (depends: A3).
- [ ] Milestone evidence: source regressions, actual software/GPU captures, public consumer/demo, available mouse checks and reviewed local commit.

Formatting core evidence: [formatting-parity.md](../docs/porting/formatting-parity.md).
A4 renderer/mouse acceptance remains open.

### B — Series price-scale assignment

- [ ] B0: Freeze series-to-scale transfer semantics. (depends: A4).
- [ ] B1: Implement one checked scale-transfer operation. (depends: B0).
- [ ] B2: Route public series options through scale assignment. (depends: B1).
- [ ] B3: Demonstrate scale assignment and accept milestone B. (depends: B2).
- [ ] Milestone evidence: source regressions, actual software/GPU captures, public consumer/demo, available mouse checks and reviewed local commit.

### C — Horizontal-axis input and settings

- [ ] C0: Freeze horizontal-axis input and independent-control behavior. (depends: B3).
- [ ] C1: Implement horizontal-axis drag and reset. (depends: C0).
- [ ] C2: Expose and enforce independent axis settings. (depends: C1).
- [ ] C3: Demonstrate axis controls and accept milestone C. (depends: C2).
- [ ] Milestone evidence: source regressions, actual software/GPU captures, public consumer/demo, available mouse checks and reviewed local commit.

### D — Reusable public helpers

- [ ] D0: Add current range queries and horizontal navigation helpers. (depends: C3).
- [ ] D1: Complete series and annotation readbacks. (depends: D0).
- [ ] D2: Expose coordinate and domain-based crosshair helpers. (depends: D1).
- [ ] D3: Validate an external consumer and accept milestone D. (depends: D2).
- [ ] Milestone evidence: source regressions, actual software/GPU captures, public consumer/demo, available mouse checks and reviewed local commit.

### E — Pane pixel sizing

- [ ] E0: Freeze persistent pane-height allocation semantics. (depends: D3).
- [ ] E1: Implement persistent programmatic pane height. (depends: E0).
- [ ] E2: Demonstrate saved pane layouts and accept milestone E. (depends: E1).
- [ ] Milestone evidence: source regressions, actual software/GPU captures, public consumer/demo, available mouse checks and reviewed local commit.

### F — Application metadata

- [ ] F0: Choose and freeze the application metadata API. (depends: E2).
- [ ] F1: Prepare metadata-capable adapter boundaries without changing default behavior. (depends: F0).
- [ ] F2: Retain application metadata in built-in data operations. (depends: F1).
- [ ] F3: Complete metadata event and chart-family adapters. (depends: F2).
- [ ] F4: Demonstrate application metadata and accept milestone F. (depends: F3).
- [ ] Milestone evidence: source regressions, actual software/GPU captures, public consumer/demo, available mouse checks and reviewed local commit.

### G — Native pointer metadata

- [ ] G0: Freeze native pointer metadata semantics. (depends: F4).
- [ ] G1: Populate owned pointer metadata at the widget boundary. (depends: G0).
- [ ] G2: Demonstrate modifier actions and accept milestone G. (depends: G1).
- [ ] Milestone evidence: source regressions, actual software/GPU captures, public consumer/demo, available mouse checks and reviewed local commit.

### H — Locale behavior

- [ ] H0: Freeze the supported default locale contract. (depends: G2).
- [ ] H1: Implement supported localized default ticks and fallback. (depends: H0).
- [ ] H2: Demonstrate localization and accept milestone H. (depends: H1).
- [ ] Milestone evidence: source regressions, actual software/GPU captures, public consumer/demo, available mouse checks and reviewed local commit.

### I — Configured color parsing

- [ ] I0: Freeze configured color parsing and gamut policy. (depends: H2).
- [ ] I1: Route configured parsers through core validation and preparation. (depends: I0).
- [ ] I2: Complete configured extension colors. (depends: I1).
- [ ] I3: Make unsupported target color-space requests explicit. (depends: I2).
- [ ] I4: Demonstrate color policy and accept milestone I. (depends: I3).
- [ ] Milestone evidence: source regressions, actual software/GPU captures, public consumer/demo, available mouse checks and reviewed local commit.

### Z — Combined completion

- [ ] Z0: Run final combined validation and publish the local completion record. (depends: I4).

## Final result

- [ ] Original targeted differences within scope now have passing parity assertions.
- [ ] All final required tests, source comparisons and unchanged relevant image protocols pass.
- [ ] Runtime/public API behavior is demonstrated in calendar, strike, maturity and sessions, including typed custom series.
- [ ] Public migration, source/consumer/ownership/design reports and updated inventory are evidence-backed.
- [ ] Local commits/worktree status recorded; push only if requested.

## Independent and deferred scope — keep open

- [ ] End-to-end Display-P3 rendering and hardware gamut acceptance (separate future milestone; explicit unsupported-target handling is not implementation).
- [ ] Intl behavior outside the support contract frozen at H0 (record exact remaining limitations).
- [ ] Earlier timing acceptance exception.
- [ ] Dedicated series drawing-order mouse acceptance.
- [ ] Physical touchpad, touchscreen and high-DPI acceptance.

Checking off this plan's core milestones does not check off these separate items.
If any is later independently resolved, link its actual evidence rather than
inferring acceptance from synthetic tests or earlier reports.
