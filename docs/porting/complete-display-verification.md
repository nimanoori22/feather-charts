# Complete-display regression and visual verification

## Contract and scope

This step verifies the existing one-pane line/grid/price/time display rather
than introducing another rendering architecture. Model mutations, layout,
measurements, command replay, and animation sampling remain synchronous frame
preparation. Drawing borrows only owned prepared geometry.

The required invariants are:

1. TimeScale width, pane height, plot snapshot size, and allocated plot agree.
2. Price/time axes and corners meet the plot at exact logical boundaries.
3. Scale marks, prepared tick rectangles, and grid strokes share coordinates.
   Bitmap snapping is checked separately from unsnapped logical positions.
4. Prepared line points match time/price coordinate conversion using the series'
   first visible value, including percentage/indexed modes.
5. Published geometry is finite and previously returned frames remain unchanged.
6. Clearing, whitespace-only data, removal, and reload cannot retain old lines.
7. Drawing cannot measure, change model state, or sample animation; repeated
   rasterization of a prepared frame must be identical on a given backend.
8. Fractional clipping cannot expose the parent background between regions.

Clipping in undersized containers is intentional. The source minimum pane and
requested axes may exceed the container. Neighboring visible points can lie
outside the plot, and edge alignment can move label text without moving ticks.
These are not failures, nor are source-suppressed time labels required to appear.

## Source and consumer analysis

Previous contracts remain in `axis-aware-layout.md`, `price-axis-rendering.md`,
`time-axis-rendering.md`, and `line-rendering.md`.

Material references for this pass:

- Original `model/price-scale.ts`: `_logicalToCoordinate`,
  `_coordinateToLogical`, `priceToCoordinate`, and tick-builder construction.
- Original `model/price-tick-mark-builder.ts`: tick values passed to scale
  coordinate conversion and consumed by axis widgets/grid.
- Original `gui/price-axis-stub.ts`: bottom-color fill and bitmap border sizes.
- Iced 0.14 `core/src/renderer.rs`: `Headless::new` and `screenshot`.
- Iced `renderer/src/fallback.rs`, `wgpu/src/lib.rs`, and
  `tiny_skia/src/lib.rs`: explicit headless backend selection and physical-size /
  logical-scale screenshot contract.
- Iced `wgpu/src/geometry.rs`: `draft`, `paste`, and `into_geometry` ordering.

Codebase-memory was used for structural discovery and source call tracing,
with direct source verification. Coverage metadata is best-effort, not a proof
of complete call resolution; renderer methods were read directly where necessary.

No TypeScript inheritance, DOM sharing, callbacks, or asynchronous chart-engine
machinery was added. Iced renderer initialization uses its existing async API
only in the offscreen verification harness; the engine remains synchronous.

## Coverage audit

| Requirement | Existing evidence | New evidence |
|---|---|---|
| Allocation, minima, undersized regions | `chart_layout` tests; 192 source cases | Full-frame matrix across restoration and scale modes |
| Fit/range replay and pending work | `fit_and_range_commands_replay_at_final_measured_width_without_draining_new_work`, command-order tests | Final grid/line/tick assertions across mutations |
| Animation replay/cancellation/budget | `animation_samples_once_per_frame_survives_resize_and_navigation_cancels_continuation`, pass-limit tests | Completion cannot resurrect after redraw/resize; resized sampled scene |
| Axis mark formatting/visibility | Existing `axis_snapshots`, `price_axis`, `time_axis` tests | Integrated mode/visibility/font matrix |
| Magnitudes, history, append options | Existing model/coordinator tests | Width requests, settled frames, enabled/disabled shifting in one owner |
| Clearing and ownership | Existing snapshot/removal tests | Clear → whitespace → reload → removal → registration sequence |
| Fractional glyphs and clipping | Existing adapter/measurement tests | TinySkia and WGPU raster checks at four actual screenshot scale factors |
| Corners | Step-4 deterministic unit test | 64 comparisons against actual original PriceAxisStub methods |
| Native display and source appearance | Previous default GUI capture | Native gradient/two-axis window; browser reference captures |

New named tests:

- `complete_display_targeted_matrix_and_owned_frames`: seven explicit configs,
  four ratios, four price modes, repeated wide/narrow/short/undersized/zero/restore
  layouts, independent left/right/hidden axes, and border/tick combinations.
- `complete_display_updates_clear_whitespace_remove_and_reload`: load/fit,
  resize, append, replace, view history, append, historical replacement, clear,
  whitespace, reload, removal, and registration again.
- `complete_display_magnitude_typography_visibility_and_new_bar_policies`:
  small decimals → large prices, font size/family refresh, hidden axes, and both
  new-bar viewport policies.
- `completed_animation_does_not_return_after_resize_or_unrelated_redraw`:
  explicit timestamps, append/resize during animation, completion and redraw.
- `explicit_display_scenarios_prepare_without_global_argument_state`: actual
  demo scenarios with real Iced measurements, including empty → reload.

`assert_frame` checks model/plot dimensions, shared region edges, source marks,
prepared text/ticks, snapped grid strokes, finite paths, and line/model conversion.
Deterministic metrics depend on explicit typography and label lengths, not on
platform-specific font widths. Existing focused tests supply exact sequencing
coverage instead of duplicating those algorithms here.

## Defects demonstrated and fixed

### Logarithmic ordinary tick conversion

The matrix failed for large negative prices when switching to logarithmic mode:
`Price(InvalidSnapshot)`. Tick values supplied by the builder are absolute prices
in normal/log modes. Rust previously applied `from_log` to them before
`price_to_coordinate`, which could overflow or produce invalid positions.

`PriceScale::logical_to_coordinate` now converts percentage/indexed tick values
back to absolute prices, but passes normal/log tick prices directly to the normal
coordinate operation. This matches the original converter boundary. The matrix
also checks each normal/log mark against direct price coordinate conversion.

### Fractional backend clip seams

WGPU exposed the magenta offscreen clear color at fractional price-axis/plot
boundaries. For example, the 1.25× gradient scene had a purple pixel at physical
x=92 along the left plot edge. TinySkia and geometry assertions alone did not
show the same failure.

`IcedChartFrame::draw` now paints a continuous prepared chart backdrop before
independently clipped regions, preserving the plot-height gradient and corner
bottom color. The backdrop uses its own clipped draft: WGPU otherwise flushes
the root frame's ungrouped meshes after pasted clips, obscuring the chart.
The raster harness checks both absence of clear-color leakage and presence of
the prepared blue line, preventing a blank backdrop from falsely passing.

This does not shift ticks, enlarge label clips, resize the model, or advance
animation. Region translation and clipping remain explicit.

## Design, ownership, and alternatives

Command/geometry assertions are portable and localize defects but cannot prove
raster coverage. Offscreen backend captures exercise real rendering but depend
on fonts/drivers. Both are used; byte-perfect browser/Iced image equality is not
an acceptance criterion because their font rasterization differs.

The shared `DisplayConfig` contains explicit data transforms, dimensions,
typography, and scene options. Process arguments are parsed only at demo startup.
Tests and per-bar generation do not inspect global arguments.

```text
Application/test fixture owns model, data, view, controller, frame owner
  preparation returns owned frame snapshots
  drawing borrows only the prepared Iced frame

Capture harness owns renderer and emitted image/metadata bytes
Browser harness owns local source chart for the same data/configuration
```

There are no new model handles, ownership cycles, caches, threads, or alternate
invalidation queues. No public renderer API redesign was necessary.

## Reproducible scenarios and captures

The demo accepts `--scenario default|large-negative|narrow|gradient|historical|animation|empty`.
The animation capture uses one fixed halfway sample while resizing; the browser
uses the same sampled offset rather than an independently advancing clock.
The native demo retains its existing interactive controls.

Diagnostics show actual available chart bounds, plot dimensions, allocated
axis sizes, requested axis sizes, device ratio, and settlement pass count.

Run from the repository root:

```sh
cargo run --example line_chart -- --scenario gradient
cargo run --example line_chart -- --capture-dir /tmp/feather-display-captures
cargo run --example line_chart -- --capture-dir /tmp/feather-display-wgpu --wgpu
node scripts/build_display_reference.cjs
node scripts/capture_display_reference.cjs /tmp/feather-display-captures /tmp/feather-lwc-reference.js
```

The build script uses dependencies already installed in the neighboring source
checkout and emits temporary compiled source/bundle under `/tmp`, without
modifying that checkout or fetching packages. Chromium/Puppeteer must be available
for browser capture. An unavailable requested renderer is an error, not a skip.

Each Iced capture includes PPM pixels plus JSON recording backend, configured
font/size, logical and physical dimensions, scale factor, plot/axis dimensions,
and settlement passes. Browser PNG/JSON records Chromium, device pixel ratio,
configured font and source axis sizes. Generic sans-serif is deliberately not
claimed to resolve to the same concrete font in Chromium and Iced.

The following were performed on this machine:

- 28 TinySkia and 28 WGPU captures: seven scenes at 1, 1.25, 1.5, and 2×.
  Each frame was drawn twice; RGBA output matched, frame data/offset/controller
  state were unchanged, no clear-color seams remained, and nonempty blue lines
  remained visible.
- 28 matching Chromium-source captures using the local TypeScript build and
  actual browser device pixel ratios.
- Visual inspection of gradient/two-axis, narrow, large-negative, and historical
  scenes against browser output, plus corrected WGPU gradient output.
- Native gradient/two-axis GUI launch and window-only screenshot inspection:
  `/tmp/feather-display-native.png`; reported native ratio 1 and chart bounds
  917×871, with allocated plot 782×842 after source size rounding.

All generated captures remain local under `/tmp`; they are reproducible artifacts,
not committed platform-dependent golden images. The Omarchy capture guidance was
used for the window-only screenshot; no desktop configuration was changed.

## Source comparisons and validation

After building examples, all source differential checks passed:

- 180 line path/color/dash/marker cases.
- 128 price-axis sizing/background/tick/text cases.
- 192 time-axis sizing/order/emphasis/edge-alignment cases.
- 192 layout allocation cases.
- 64 new corner gradient/shared-border/fractional-snapping cases.

Total: **756 source differential cases**. Existing price/time cases already cover
narrow plots, fractional ratios, border/tick combinations, and gradients; the
corner gap was filled rather than duplicating those cases.

Formatting, all-target tests/checks, Clippy, default/large-negative smoke runs,
source-build/capture tools, and native demo launch were validated. All-target tests
pass: **276 library tests and two demo tests**. Clippy retains only the five
pre-existing test warnings; `cargo clippy --lib --examples -- -D warnings` passes.

## Files changed

- `examples/support/display_scenario.rs`: shared explicit scene inputs.
- `examples/line_chart.rs`: config-driven data, scenarios, diagnostics, headless
  raster/immutability/seam checks, and demo scenario test.
- `src/model/chart_model/tests/complete_display.rs` and `tests.rs`: connected
  display fixtures, assertions, regression matrix and sequences.
- `src/model/price_scale.rs`: demonstrated log-tick conversion fix.
- `src/ui/chart_frame.rs`: continuous background with correct draft ordering.
- `examples/chart_layout_reference.rs` and `scripts/check_chart_layout_reference.cjs`:
  source corner comparisons.
- `scripts/build_display_reference.cjs` and `scripts/capture_display_reference.cjs`:
  reproducible local source build and browser captures.
- `docs/porting/complete-display-verification.md`: contract, audit and this report.

## Remaining limits

Fractional scaling was verified through real TinySkia/WGPU screenshot viewports
and Chromium device pixel ratios, not merely by setting the command ratio. Native
monitor-scale switching, moving between differently scaled monitors, other OSes,
other GPU drivers, and every font family remain unverified. Native animation
resize was covered programmatically and offscreen, not by a manual video capture.

Visual comparisons are qualitative, not pixel-identical goldens. Backend font
metrics can produce different requested widths, and ordinary-tick sizing still
excludes deferred floating-label/crosshair contributions. Browser-scale mark
selection is not claimed to match exactly in every price-mode case. Multi-pane
layout, conflation, additional series renderers, gestures, and crosshair remain
outside this milestone. The next implementation milestone is mouse zoom/scroll
and crosshair interaction.
