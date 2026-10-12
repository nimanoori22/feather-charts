# Price lines and last-value labels: implementation checklist

Status: implemented; automated checks complete, physical acceptance pending. Design and scope: [price-lines-plan.md](price-lines-plan.md).
Use local commits for buildable slices; pushing remains a separate user action.

## 0. Source contract and design (medium; dependency: none)

- [x] Read full source components/imports and trace real API/model/view/widget consumers.
- [x] Record visible/global semantics, autoscale, ordering, validation and cleanup contracts.
- [x] Compare ownership designs; define Rust types/signatures and map real consumers.

Likely file: new `docs/porting/price-lines.md`. Verify source citations, coverage
fallbacks and consumer mappings before implementation.

## 1. Last-value selection (small; dependency: 0)

- [x] Provide a typed no-data/value result using existing rows, price scales and color selection.
- [x] Distinguish global latest from visible-right nearest-left, including whitespace and empty scales.
- [x] Test formatting/base/color behavior for supported series and scale modes against source.

Likely files: `src/model/series.rs`, focused series tests, reference fixture/script.
Verify focused tests and selection oracle; keep public API exposure minimal.

## 2. Automatic price lines (medium; dependency: 1)

- [x] Honor visibility, source, row/explicit color, width/style and selected scale.
- [x] Integrate clipped line geometry once per series into the shared pane frame path.
- [x] Test clear/hidden/remove and fractional pixel geometry against source.

Likely files: pane-view module, existing line renderer, `src/ui/line_chart.rs`,
focused tests/reference fixture. Verify focused tests, source geometry and a demo frame.

## Checkpoint A: automatic lines

- [x] Formatting, focused tests and all-target check pass.
- [x] Global/visible behavior is demonstrated; prior navigation tests still pass.

## 3. Last-value label primitives (medium; dependency: 1)

- [x] Extend owned axis/pane-label snapshots with true coordinate, text, colors and display modes.
- [x] Reuse current text measurement/cache and source contrast rules.
- [x] Test visibility/title rules, formatting and label rectangle/tick geometry on both sides.

Likely files: `src/model/axis_snapshots.rs`, series axis-view module,
`src/renderers/price_axis_renderer.rs`, `src/ui/price_axis.rs`, focused tests.
Verify label tests and source comparison; do not yet claim collision acceptance.

## 4. Shared label layout (medium; dependencies: 2, 3)

- [x] Include last-value and pane-title metrics in settled frames for both frame entry points.
- [x] Preserve shared axis widths, clipping and source paint order with multiple series/panes.
- [x] Test long labels, precision changes, narrow bounds and bounded repeated settlement.

Likely files: `src/model/chart_model.rs`, `src/ui/chart_frame.rs`,
`src/ui/multi_pane_frame.rs`, frame/layout tests. Verify all-target check and captures.

## Checkpoint B: automatic line and label path

- [x] Line/candlestick/histogram labels work through the public component.
- [x] Data updates and manual price/time navigation retain correct projection.
- [x] Formatting, focused tests and all-target check pass.

## 5. Custom line lifecycle/API (medium; dependency: 0)

- [x] Implement source-backed options and typed, non-recycled line IDs under chosen ownership.
- [x] Expose add/replace/inspect/remove; invalid options preserve state.
- [x] Test foreign/removed IDs, series cleanup and data clear/reload without lost line ownership.

Likely files: new model options/line module, `src/model/series.rs`,
`src/api/chart.rs`, exports and `tests/chart_api.rs`; split storage/API commits if needed.
Verify focused lifecycle tests and all-target check. No special input gesture is added.

## 6. Custom line/label output (medium; dependencies: 4, 5)

- [x] Render each level using series projection, source styles/color fallback and independent visibility.
- [x] Render axis values and pane titles via existing label primitives; avoid duplicate composed output.
- [x] Test updates/removal, hidden/empty states, pane isolation and verified autoscale behavior.

Likely files: custom pane/axis views, model snapshot assembly, focused frame/API tests.
Verify source comparison and both-side multi-pane captures.

## 7. Label overlap alignment (medium; dependency: 6)

- [x] Preserve ordered-source center/tie behavior and source edge/height corrections.
- [x] Honor align_labels=false; keep crosshair rendering separate and prioritized.
- [x] Test clusters, equal coordinates, edges and crowded short axes without moving line coordinates.

Likely files: focused alignment helper, axis preparation/renderer and tests,
new/extended source oracle. Verify oracle and deterministic multi-label captures.

## Checkpoint C: custom levels and alignment

- [x] Public API lifecycle and clustered labels behave correctly across panes/scales.
- [x] Formatting, focused tests and all-target check pass.
- [x] No stale output or unbounded layout cycling remains.

## 8. Public example/live update demonstration (small; dependency: 7)

- [x] Extend `examples/public_chart.rs` with entry/stop/target add/update/remove and live series updates.
- [x] Provide a reproducible way to inspect last-value visibility, historical scrolling and label clusters.
- [x] Document the new public API and supported scope in `README.md`.

Verify headless smoke and bounded native startup; inspect public API usability.
Keep host/demo controls out of the reusable engine.

## 9. Integrated verification and report (medium; dependency: 8)

- [x] Run fmt, full all-target tests/check, strict Clippy and example builds.
- [x] Run new/affected source oracles and TinySkia/WGPU captures at fractional scales.
- [x] Compare equivalent navigation/data-update timings and glyph reuse; investigate regressions.
- [x] Record final contract, design, exact changed files, results and deferred dependencies.
- [ ] Complete fresh physical mouse/touchpad and crowded-label visual acceptance. Automated captures were inspected; hardware acceptance remains pending.

Likely files: capture fixtures/tests and `docs/porting/price-lines.md`.
Completion requires working public API behavior and recorded automated evidence;
pending physical checks remain explicitly unchecked.

Evidence: [final report](../docs/porting/price-lines.md). Automated checks cover
406 Rust tests, 19,515 source cases, both render backends and unchanged-workload
profiling. Local implementation commits are not pushed automatically.
