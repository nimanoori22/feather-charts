# Text watermark execution checklist

Plan: [text-watermark-plan.md](text-watermark-plan.md).
Starting checkpoint: `61bb52b`. Implementation and software/GPU mouse acceptance
completed on 2026-10-17. See the [port report](../docs/porting/text-watermark.md).
Original slice descriptions follow; final file names and results are in the report.
Existing plans/checklists are unchanged. No additional approval gate is implied.

## 0. Confirm the source contract and font integration

Dependencies: none. Scope: investigation, 2 documentation files.
Likely output: `docs/porting/text-watermark.md` and source hash manifest.

- [x] Pin actual TS working-tree hashes and finish imports, public factory/wrapper,
  pane draw order, test consumers and removal/lifecycle traces with coverage checks.
- [x] Compare primitive-owned versus Chart-owned designs and local/shared font
  cache approaches. Resolve native top-anchor metrics, font selection, no-wrap
  behavior, empty strings, embedded line separators and zero/invalid numeric policy.
- [x] Confirm the public types, validation/error boundary, owned options updates,
  frozen renderers and pane identity against real consumers before production edits.

Verification: every required behavior maps to source evidence and a test case.
Keep a bounded decision log; no speculative text/plugin framework.

## 1. Establish actual-source comparison and RED cases

Dependencies: 0. Scope: 2–3 fixture/test files.
Likely files: `scripts/check_text_watermark_reference.cjs`, focused Rust layout
fixtures/tests; source manifest.

- [x] Execute actual TS defaults/view/renderer with injected metrics and a recording
  canvas. Cover nine alignments, long/exact-fit/short widths, multiline heights,
  empty/hidden states, custom spacing, replacement defaults and degenerate bounds.
- [x] Freeze deterministic fixture counts. Separate layout parity from native font
  measurement/shaping and platform rasterization.
- [x] Define public pixel/geometry assertions and reference fixtures. At the first
  compiling public API boundary in Task 2, record a failing visible-output check
  before implementing drawing; unknown-type compile failures do not count as RED.

Verification: source fixture self-checks pass; public behavioral RED is required
at the first compiling slice in Task 2.

## Checkpoint A — Contract

- [x] Source/ownership/font decisions are settled and independently testable.

## 2. Deliver a single centered line

Dependencies: 1. Scope: split into 2–5-file slices if needed.
Likely files: `src/api/text_watermark.rs`, `src/api/mod.rs`,
`src/ui/text_watermark.rs`, UI module export, `tests/text_watermark.rs`.

- [x] Add validated default options and the TextWatermark DrawingPrimitive. Attach
  to an existing pane using its stable owner and existing primitive ID/lifecycle.
- [x] After minimal types compile, demonstrate the public visible-output RED, then
  prepare native text into owned paths using existing bounded helpers. Resolve
  any minimal shared text-helper change in a separate verified slice.
- [x] Draw one centered line on the source normal layer, clipped to the plot, with
  no new hit target, axis sizing, autoscale effect or redraw loop.

Verification: visible one-line public test and both native renderer smoke checks;
retain an old prepared frame through replacement/removal and compare its output.

## 3. Complete source layout and styles

Dependencies: 2. Scope: 3–5 files; separate options from glyph adaptation if larger.
Likely files: watermark API/preparation/layout modules, focused tests and oracle.

- [x] Add multiple lines, all alignments, independent horizontal shrink-to-fit,
  exact source padding/block offsets, default/custom spacing and empty-line rules.
- [x] Support per-line native family/style/weight, size and parsed alpha colors;
  preserve no-wrap, fallback and top-line-box semantics.
- [x] Validate external values before mutation. Exercise zero/short dimensions,
  long/Unicode/whitespace text and failed replacements without stale output/panics.

Verification: actual-source oracle, native metric/path tests and focused public
suite pass; existing axis text anchors/cache tests remain unchanged.

## Checkpoint B — Visible feature

- [x] Multiline layout and styles work through the public API on both backends.

## 4. Verify lifecycle and mixed scenes

Dependencies: 3. Scope: 2–4 test/preparation files.
Likely files: `tests/text_watermark.rs`, existing primitive/pane tests and capture
support; any root fix gets a focused failing regression first.

- [x] Test replace/hide/show/empty/remove, invalid IDs and rejected options;
  retained frames remain frozen and attachment order/identity stays stable.
- [x] Test pane-owner identity, surviving empty panes, series removal, actual
  pane removal and pane-index shifts. Reuse generic primitive ordering/lifecycle
  for multiple attachments; no time/price domain dependency.
- [x] Check source order with filled/raised series, grid, markers, price lines,
  crosshair, animation and normal/top primitives; pointer winners, autoscale and
  axis bounds remain unchanged. Include resize/short-height recovery.

Verification: focused lifecycle/hit/order tests and original primitive suite pass.

## 5. Add work bounds and the demo

Dependencies: 4. Scope: 3–5 files; split counters/tests from demo if larger.
Likely files: watermark/text-helper tests, `examples/public_chart.rs`, optional
`examples/public_chart/watermark.rs`, README.

- [x] Prove bounded width/glyph caching and zero unnecessary reshaping during
  unchanged preparation, pan/live updates and pulse-only Tick. Resize changes
  fit/position without scanning history; style/text changes invalidate needed keys.
- [x] Add `--text-watermark` to the public demo with visibility, alignment/style
  and text replacement controls using existing host-owned options/primitive IDs.
- [x] Support line/filled series, dense conflation, multiple panes, live feed and
  the existing editable primitive without introducing another animation timer.

Verification: meaningful shape/cache counters and demo smoke tests. Keep the
existing one-geometry software damage behavior. No unrelated timing benchmark.

## Checkpoint C — Lifecycle and work

- [x] Lifecycle/coexistence tests pass; work depends on text, not chart history.

## 6. Run renderer and mouse acceptance

Dependencies: 5. Scope: existing capture support, compact protocol/results.
Likely files: public demo capture helper and `docs/porting/evidence/text-watermark/`.

- [x] Freeze commands, fonts and binary/source identity. Run the plan's 120-image
  watermark matrix and 24 default-off before/after comparisons. Retain failures;
  check frozen-frame repeatability and inspect representative images.
- [x] Test font fallback, per-line fitting, clipping and layer order at DPR
  1/1.25/1.5/2 on TinySkia/WGPU; distinguish synthetic metrics from real text output.
- [x] Record mouse wheel zoom, diagonal pan, price-axis wheel/drag and box editing
  with watermarks/live feed at regular/narrow sizes on both backends. Record
  unavailable touchpad/touchscreen/physical high-DPI coverage separately.

Verification: finite captures and native checks pass; no historical timing budgets
or gate statuses are changed by these results.

## 7. Validate, review and document

Dependencies: 6. Scope: verification/docs, about 3 files.
Likely files: README, `docs/porting/text-watermark.md`, this checklist.

- [x] Pass `cargo fmt --check`, `cargo test --all-targets`,
  `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, the
  watermark oracle and affected existing source comparisons.
- [x] Review final ownership, validation, paint order, cache bounds, public API
  impact and absence/default-off parity against source consumers.
- [x] Publish source hashes, required/incidental semantics, alternatives, exact
  files, test/capture/work results, demo usage and font/platform limitations.

## Checkpoint D — Complete

- [x] Text watermark implementation and automated/source parity gates pass.
- [x] Native mouse results and unavailable device coverage are recorded.
- [x] Image watermarks remain a separate follow-up feature.
- [x] Historical timing acceptance remains separately OPEN/deferred.

## Completion evidence and limits

626 all-target tests, check, strict Clippy, formatting and source comparisons pass.
The 120 watermark captures and 24 default-off comparisons pass on both renderers.
Both mouse responses: “All checks passed; window closed.”

Native top anchors and Arabic/CJK/fallback shaping are tested. ASCII line controls
normalize to spaces; other Unicode paragraph controls use native Iced semantics,
without a claim of exact browser equivalence. Browser/native glyph pixel equality
is outside scope. Physical touchpad/touchscreen/high-DPI checks remain untested.
The historical timing exception is OPEN/deferred. Failed RTL and WGPU runs are
retained alongside corrected results in the compact evidence directory.
