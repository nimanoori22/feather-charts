# Last-price animation

Source revision and working-tree hashes are in
[last-price-animation-source.json](last-price-animation-source.json).

Series owns a value-only monotonic timing state. Mutations consume the existing
update flag once, including qualifying historical updates. Clear cancels activity;
option switches retain the origin, matching the source. Native removal drops state;
there are no browser timers or new reference cycles.

A Chart-owned timing map was rejected because low-level model/frame consumers
would lose the behavior. Prepared frames instead own endpoint, color, mode and a
copy of timing values, sampled with explicit time. Drawing never reads the model
or a clock. A dedicated payload keeps this decorative marker out of crosshair hit
testing and autoscale. Existing primitives cannot represent both filled and stroked
circles with independent alphas without extra allocations.

Full preparation projects the canonical last price through the settled scale.
Frame owners sample at their supplied Instant; the direct plot API samples at
preparation. Animation-only ticks can resample frozen payloads when no model
invalidation, controller work, conflation warmup or edit/selection work is pending.
Keep the single Canvas geometry/damage boundary. Top source order is crosshair,
then each series animation followed by its top primitives, preserving raised
source order and existing background passes.

The original view, renderer and Series lifecycle/topPaneViews consumers were read.
DataLayer determines triggers; ColorParser multiplies alpha. Browser timeout IDs,
mutable renderer identity and manual destroy are incidental. Timing, canonical
projection, inclusive expiry, draw order and DPR geometry are required.

The historical timing exception remains OPEN/deferred.

## Implementation and API

Rust implementation checkpoint: `4de2876`.

The existing `style.last_price_animation` option now works for Line, Area and
Baseline. Disabled is still the default; Continuous loops; OnDataUpdate pulses
for qualifying mutations, then retains the dot. All four line widths are
supported, including a hidden line with a visible last-price dot. Other series
families do not create animation timing state.

The private timing API is `LastPriceAnimationState::{new, on_update, is_active,
sample}`, using explicit `Instant` arguments. Series creation/mutation acquires
real monotonic time once. No public timed mutation API or clock service was added.
Frame owners resample at their caller's supplied time; the geometry-only settled
plot entry point samples at preparation. `prepare_frame` also honors its supplied
time. `PlotSnapshot::last_price_animations` owns each projected endpoint, timing
snapshot and sampled colors/radii. A retained frame remains frozen after later
Ticks, updates, option changes and series removal.

The fast Tick path checks the displayed frame revision, read-only pending model
invalidation, scrolling controller, conflation warmup and pending edit/selection
work. It never consumes an invalidation just to test eligibility. Initial layout
can leave legitimate settlement work pending: the next Tick handles that normally;
only settled frames take the pulse-only path. Calling `prepare_snapshot` invalidates
the displayed-frame eligibility even if it consumes the model's current mask.

Scheduling uses the existing 16ms native redraw deadline and touch-deadline merge.
The sampled `active` bit stays true until a Tick samples past expiry, guaranteeing
one clearing frame even if the actual deadline was missed. Offscreen/hidden/empty
markers are omitted without changing the series clock. Scroll/kinetic controllers
and background warmup retain priority over the pulse-only path.

The existing single Canvas geometry boundary is preserved. Geometry and
rasterization still run when the pulse changes; there is no claim of a cached
background render or a skipped GPU submission. Only model/frame preparation is
avoided. Exact Iced `widget/src/canvas.rs` forwards the Canvas action's redraw
request to the shell; Flowsurface's toast widget is an example of scheduling
`RedrawRequested` deadlines without an asynchronous engine.

Top rendering follows source order: crosshair/above-series annotations, then
for each source its pulse, top annotations and top primitives, with the existing
background/main and raised-source passes. A new overlapping-marker pixel test
caught and fixed the initial placement of the pulse above a top marker. Native
clipping stays on the plot rectangle. Animation adds no hit geometry or price
range. There are no strong/weak links beyond existing Series ownership, and no
new dependency, thread, subscription, Tokio task or timer handle.

## Verification

`scripts/check_last_price_animation_reference.cjs` verifies the pinned working-tree
hashes, transpiles and executes the actual TS view/renderer/DataLayer/color code,
and compares Rust fixture output. It checks:

- 58 timing/transition samples, including stage boundaries ±epsilon, cycle wrap,
  exact end, expiry, strict extension threshold, repeated updates, mode switches
  and clear;
- 9 accepted DataLayer trigger cases: initial data, full replacements with changed
  bounds, fulfilled historical update, whitespace, latest update, clear/reload;
- 12 canonical Rust endpoint payloads against actual TS renderer arcs at DPR
  1/1.25/1.5/2, plus four independent recording-canvas DPR cases;
- actual source alpha multiplication, transparency, Area/Line overrides and the
  Baseline canonical bar-color rule (which intentionally differs from row stroke
  overrides).

The initial behavioral RED compared software-rendered pixels from otherwise
identical enabled/disabled charts and confirmed that the option had no effect.
The focused Rust integration suite covers dense canonical endpoint projection,
frozen frames, inactive dot/expiry, default-off rendering, offscreen suppression,
left/right scales, Normal/Log/Percentage/Indexed modes and inversion, multi-pane
composition, data clear/reload, style/visibility changes, shared reindexing,
rejected updates, all line widths, short-height recovery, simultaneous animated
scroll/warmup and top-marker ordering/hit independence. No correctness assertion
uses sleeps. The complete existing interaction/marker/primitive/custom-series
suite remains part of final validation.

## Bounded work diagnostic

The fixed batch is **24 matched pairs / 48 serial processes**, using Line, Area
and Baseline, 8,192/100,000 rows, 800×500/300×300, TinySkia/WGPU, DPR 1,
conflation enabled, alternating Disabled/Continuous order. Each process uses
20 warmups and 80 measured frames at explicit 32.5ms steps. The manifest freezes
commands, binary/source hashes and counter definitions before execution. All 48
processes and **3,840 measured frames passed**. Each measured Tick made zero
formatter calls and retained the original base stroke/fill allocations. The
focused test independently verifies 100 settled Ticks at both history sizes.
Source inspection confirms this path samples only owned marker payloads; it
cannot build history, conflation levels, autoscale, tick marks or glyph paths.

Raw CSVs, logs, RGBA captures and the measured executable are retained under
`target/last-price-animation/batch/`. Compact hashes/results are committed in
[evidence/last-price-animation/batch-results.json](evidence/last-price-animation/batch-results.json)
and the original protocol in
[batch-manifest.json](evidence/last-price-animation/batch-manifest.json).
Per-process preparation medians ranged from 129–3,419.5ns for Disabled and
272.5–4,816ns for Continuous. These are diagnostics, **not a speed comparison or
latency acceptance result**: compilation/tests overlapped portions of the batch,
the desktop load was uncontrolled, and screenshot time includes readback and
conversion rather than native presentation alone. Every run was retained; none
was repeated to obtain a passing number.

The frozen batch preceded the top-marker ordering regression fix, the low-level
`prepare_frame` clock-handoff correction and a Clippy syntax fix. Those changes
do not affect its annotation-free Chart/Tick workload. Final-source captures and
the full test suite validate the resulting implementation separately; the original
manifest is not rewritten to pretend it measured a later source hash.

## Physical mouse acceptance

On 2026-10-17 the user tested the dev-built dense Line + volume demo with
Continuous animation, live feed and the editable primitive enabled. They reported
**“All checks passed; window closed”** separately for TinySkia and WGPU, at
regular and narrow sizes. Checks included wheel zoom, diagonal dragging,
price-axis wheel/drag, box movement/resizing, pulse tracking and scrolling the
canonical last bar out of view. This is mouse acceptance for the shared native
integration; Area/Baseline parity is covered automatically.

Touchpad, touchscreen and physical high-DPI acceptance remain untested. Simulated
fractional-DPR rendering does not establish physical monitor/input acceptance.
The historical timing exception remains OPEN/deferred with its original budgets.

## Files and remaining work

Core: `src/model/last_price_animation.rs` and its tests, `src/model/series.rs`,
`src/model/mod.rs`, `src/model/chart_model.rs`;
projection/rendering: `src/views/pane/series_last_price_animation_pane_view.rs`,
`src/renderers/series_last_price_animation_renderer.rs` and module exports;
frame/native integration: `src/ui/{line_chart,chart_frame,multi_pane_frame}.rs`,
`src/api/{chart,interaction,primitives}.rs`.
Fixtures/tools: `tests/last_price_animation.rs`, `examples/public_chart.rs`,
`examples/last_price_animation_profile.rs`, source oracle and fixed-batch runner.
Documentation: README, this report/source hash manifest, committed compact evidence
and the plan/checklist.

No related source component blocks this feature. Candlestick, Bar, Histogram and
custom-series pulses are outside upstream's last-price animation feature.

## Final result

- `cargo fmt --check`, `cargo check --all-targets` and
  `cargo clippy --all-targets -- -D warnings` pass.
- `cargo test --all-targets`: **613 tests pass**.
- The animation source oracle and the existing conflation oracle pass.
- **160 disabled-output images are byte-identical** to retained pre-feature
  captures across all six built-in families and mixed Line/volume scenes, both
  renderers, DPR 1/1.25/1.5/2.
- **18 final-source fractional-DPR captures** cover all three animated families
  on both backends at 1.25/1.5/2. They render the source's 325ms stage by setting
  the owned frozen sample to radius 7, fill alpha .125 and stroke alpha .6;
  repeated drawing of each retained frame produces identical pixels. DPR 1
  fixed-stage output is also retained for all 48 diagnostic runs. One final Area
  software capture at DPR 1.5 was visually inspected.
- Both mouse acceptance runs pass. Other device coverage remains open as above.

Capture protocols/results and final check status are in
[evidence/last-price-animation/](evidence/last-price-animation/).
Raw captures remain under `target/last-price-animation/`; screenshots/timing
readbacks are not compositor presentation measurements.

Final review checked trigger consumption at both mutation paths, ownership/drop,
retained output, top ordering, dirty-state eligibility, default-off parity and
scheduler coexistence. The scope adds no untrusted-input path or unsafe code.
The public prepared `PlotSnapshot` gains its animation payload field; external
hosts constructing it should continue using its `Default`/preparation helpers.
