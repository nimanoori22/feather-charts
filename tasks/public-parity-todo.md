# Public API and desktop parity checklist

Date: 2026-10-19. Status: implementation in progress; formatting core P0/A0–A3 complete; B0–B2 implemented; milestone B accepted; milestone C accepted; milestone D accepted; milestone E accepted; milestone F accepted; milestone G accepted; milestone H accepted; milestone I next.
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
A4 automated gates pass: 807 tests, 38 source checks, 576 screen replays and retained
exports; 2982 accepted compatibility images unchanged. The user accepted formatting and explicitly requested the next feature. Proceed to B
under that instruction. A4 retains its missing separate GPU physical record; no
GPU result is inferred. See formatting-evidence.json.

### B — Series price-scale assignment

- [x] B0: Freeze series-to-scale transfer semantics. (depends: A4).
- [x] B1: Implement one checked scale-transfer operation. (depends: B0).
- [x] B2: Route public series options through scale assignment. (depends: B1).
- [x] B3: Demonstrate scale assignment and accept milestone B. (depends: B2).
- [x] Milestone evidence: source regressions, actual software/GPU captures, public consumer/demo, available mouse checks and reviewed local commit.

B3 automated evidence: 816 tests, 39 source comparisons, 768 screen replays and
retained exports; 3558 accepted compatibility images unchanged. Four-mode software/GPU
physical checks, previews/exports and normal window exits pass. See [scale-assignment.md](../docs/porting/scale-assignment.md)
and its pinned evidence record.

### C — Horizontal-axis input and settings

- [x] C0: Freeze horizontal-axis input and independent-control behavior. (depends: B3).
- [x] C1: Implement horizontal-axis drag and reset. (depends: C0).
- [x] C2: Expose and enforce independent axis settings. (depends: C1).
- [x] C3: Demonstrate axis controls and accept milestone C. (depends: C2).
- [x] Milestone evidence: source regressions, actual software/GPU captures, public consumer/demo, available mouse checks and reviewed local commit.

C3 demo and automated evidence: 832 tests, 40 source checks, 864 screen replays and
retained exports; 4326 accepted compatibility images unchanged. Four-mode software
and GPU mouse acceptance, previews/exports and all eight normal window exits pass. See [axis-interaction.md](../docs/porting/axis-interaction.md)
and its pinned evidence record.

### D — Reusable public helpers

- [x] D0: Add current range queries and horizontal navigation helpers. (depends: C3).
- [x] D1: Complete series and annotation readbacks. (depends: D0).
- [x] D2: Expose coordinate and domain-based crosshair helpers. (depends: D1).
- [x] D3: Validate an external consumer and accept milestone D. (depends: D2).
- [x] Milestone evidence: source regressions, actual software/GPU captures, public consumer/demo, available mouse checks and reviewed local commit.

D3 automated gates pass: 844 tests, 41 source comparisons, 576 screen replays
(including 72 linked charts) and 504 retained exports. All 5190 accepted
compatibility images are unchanged. Four-mode software/GPU mouse acceptance,
idle counters, previews/exports and all eight normal window exits pass. See [public-helpers.md](../docs/porting/public-helpers.md) and its pinned evidence record.

### E — Pane pixel sizing

- [x] E0: Freeze persistent pane-height allocation semantics. (depends: D3).
- [x] E1: Implement persistent programmatic pane height. (depends: E0).
- [x] E2: Demonstrate saved pane layouts and accept milestone E. (depends: E1).
- [x] Milestone evidence: source regressions, actual software/GPU captures, public consumer/demo, available mouse checks and reviewed local commit.

E2 automated gates pass: 849 tests, 42 source comparisons, 672 screen replays and
672 retained exports. All 5766 accepted compatibility images are unchanged.
Four-mode software/GPU mouse acceptance, previews/exports and all eight normal window exits pass. See [pane-sizing.md](../docs/porting/pane-sizing.md)
and its pinned evidence record.

### F — Application metadata

- [x] F0: Choose and freeze the application metadata API. (depends: E2).
- [x] F1: Prepare metadata-capable adapter boundaries without changing default behavior. (depends: F0).
- [x] F2: Retain application metadata in built-in data operations. (depends: F1).
- [x] F3: Complete metadata event and chart-family adapters. (depends: F2).
- [x] F4: Demonstrate application metadata and accept milestone F. (depends: F3).
- [x] Milestone evidence: source regressions, actual software/GPU captures, public consumer/demo, available mouse checks and reviewed local commit.

Source/contract, API and data checkpoints: `69350b1`, `020bdda`, `4a72e9b`;
consumer checkpoint: `af993e7`. Final validation: 860 tests in 87 suites, two
compile-fail doctests, 43 source comparisons, 864 metadata screen replays and
864 retained exports, 384 metadata-only pixel comparisons and 6,438 unchanged
compatibility images. The user passed software/GPU mouse acceptance in all four
domains, with all eight normal sessions exiting successfully. See
[application-metadata.md](../docs/porting/application-metadata.md) and its pinned
evidence. Independent exceptions remain unchanged.

### G — Native pointer metadata

- [x] G0: Freeze native pointer metadata semantics. (depends: F4).
- [x] G1: Populate owned pointer metadata at the widget boundary. (depends: G0).
- [x] G2: Demonstrate modifier actions and accept milestone G. (depends: G1).
- [x] Milestone evidence: source regressions, actual software/GPU captures, public consumer/demo, available mouse checks and reviewed local commit.

Contract/API/consumer checkpoints: `9110d08`, `f356c76`, `7a139df`.
866 tests in 88 suites, two compile-fail doctests and 44 source comparisons pass.
Both renderer matrices pass 576 screen replays, 576 retained exports and 192
modifier-only pixel comparisons. Affected unchanged event/axis protocols match
1,776 accepted images. The user passed all four modes on software/GPU, including
focus and previews/exports; eight normal sessions exited successfully using the
validated binary. See [native-pointer.md](../docs/porting/native-pointer.md) and
its pinned evidence. Combined remaining captures are Z0; independent exceptions
remain unchanged.

### H — Locale behavior

- [x] H0: Freeze the supported default locale contract. (depends: G2).
- [x] H1: Implement supported localized default ticks and fallback. (depends: H0).
- [x] H2: Demonstrate localization and accept milestone H. (depends: H1).
- [x] Milestone evidence: source regressions, actual software/GPU captures, public consumer/demo, available mouse checks and reviewed local commit.

Formatter/consumer checkpoints: `b083cec`, `4daa116`. All common gates pass:
872 tests in 89 suites, two compile-fail doctests and 45 source comparisons.
The new matrix matches 560 ticks and 192 date/month labels across 16 locale cases.
Both native renderers pass 960 screen replays and 960 retained exports; the
unchanged formatting protocol matches all 576 accepted images. Four-mode software
and GPU mouse acceptance, previews/exports and all eight normal exits pass.
See [time-locale.md](../docs/porting/time-locale.md) and its pinned evidence.
Full Intl outside the supported Gregorian profiles remains explicitly partial;
combined remaining captures are Z0 and independent exceptions are unchanged.

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
