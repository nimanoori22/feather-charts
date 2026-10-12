# Crosshair and magnet

## Source behavior and consumers

Investigated `model/crosshair.ts`, `model/magnet.ts`, pane crosshair/marker views,
price/time crosshair label views and their renderers. Model pointer updates save
raw coordinates, convert to the nearest logical index, clamp to the strict
visible interval and optionally avoid hidden-series indices. Recalculation
reapplies the raw coordinates. Crosshair X aligns to a time index in normal and
magnet modes; normal mode frees the Y coordinate, not the time coordinate.

Pane views need applied geometry and options; axis views need values formatted
on each scale. Magnet compares screen Y distances over ordered visible,
non-overlay series at the exact index. Normal mode bypasses selection; Magnet
uses close; MagnetOHLC uses open/high/low/close. Equal distances preserve source
order. Whitespace and empty scales yield no candidate. Each series uses its own
first visible value and price scale, avoiding incorrect comparisons of raw
prices on unequal scales.

Price-axis sizing reserves representative extreme labels whenever crosshair
horizontal labels are enabled, independent of pointer visibility. This prevents
pointer movement from repeatedly changing plot width. Floating labels render
above ordinary ticks; disabling a line does not disable its label or marker.
Time label edge alignment moves text/background but retains the tick coordinate.
Price labels clip at axis edges rather than changing their price coordinate.

## Rust design before implementation

Two viable designs are a model-owned typed crosshair with owned snapshots, or
an owner-referencing data source with cached views mirroring TypeScript. Choose
typed state: the existing Rust frame pipeline already materializes owned geometry;
view caches and shared model references add allocations and borrowing hazards.
The crosshair owns options plus optional raw/applied positions, with no parent
reference, closures or Rc cycles. Axis snapshots own labels; frames own paths.
Drop requires no manual cleanup, and drawing only borrows prepared geometry.

```rust
pub enum CrosshairMode { Normal, Magnet, Hidden, MagnetOhlc }
pub struct CrosshairOptions { /* mode and typed line/label options */ }
pub fn set_crosshair_position(&mut self, pane: usize, point: Option<Point>)
    -> Result<(), ChartModelError>;
pub fn set_crosshair_options(&mut self, options: CrosshairOptions)
    -> Result<(), ChartModelError>;
pub fn crosshair(&self) -> &Crosshair;
```

`None` replaces NaN sentinel positions; pane indices replace pane identity.
Canvas pointer messages feed the model. Model recalculation refreshes applied
position. Renderer consumers receive immutable snapshots with exact formatting
and measured metrics. Public API synthetic positioning and hovered-object events
are separate milestones; this implementation does not create placeholder APIs.

## Validation

Implemented 2026-10-12. The one-pane line demo now supports wheel zoom,
horizontal drag/wheel scrolling, normal/magnet/hidden crosshair modes, floating
labels and source-configured line-series crosshair markers. `MagnetOhlc` is
supported in model selection; additional OHLC series rendering remains separate.
No dependencies, async engine mechanisms or shared owner references were added.

Model tests verify raw-position refresh after zoom, hidden/normal modes, invalid
colors/coordinates, removal, resize, exact-index snapping, hidden series,
whitespace without candidates, nearest-index left ties, whitespace-only/cleared
data, unequal scales, overlay exclusion and labels formatted on both scales.
Renderer tests verify independent line visibility, fractional pixel alignment,
invalid ratios, width reservation and label edge placement. Existing complete
frame tests retain their update, layout, clipping and immutability coverage.

Visual inspection caught an existing series formatter setup defect: decimal
values such as 98.1074 could display as `98.10.74`. Series now derives formatter
scale from precision and multiplies minimum movement by that scale, as source
`Series._recreateFormatter` does. Percentage series also derives scale from
precision. A regression test covers decimals, a 0.005 minimum movement,
precision independent of autoscale base, and option replacement with percent.

The source oracle compares 89 new cases against original TypeScript methods:
32 navigation, 16 crosshair-line, 24 price-label background/tick, 12 time-label
background and 5 stable magnet selection cases. Existing source scripts pass
756 cases: line 180, price-axis 128, time-axis 192, layout 192, corners 64.
These are deterministic geometry/selection checks, not a claim of complete
browser pixel equality or full library parity.

Final commands:

```sh
cargo fmt --check
cargo test --all-targets
cargo check --all-targets
cargo clippy --lib --examples -- -D warnings
cargo build --examples
git diff --check
node scripts/check_interaction_reference.cjs
node scripts/check_line_reference.cjs
node scripts/check_price_axis_reference.cjs
node scripts/check_time_axis_reference.cjs
node scripts/check_chart_layout_reference.cjs
```

All pass: 292 library tests and 3 demo tests; 845 source comparison cases.
Interactive captures render twice and assert unchanged frame/model/animation
state, identical output, visible series/crosshair markers and no exposed seams.
28 TinySkia and 28 WGPU scenes cover seven existing scenarios at ratios 1,
1.25, 1.5, 2. Output is in `/tmp/feather-interactive-tinyskia` and
`/tmp/feather-interactive-wgpu`. Representative default and narrow/gradient
captures were inspected visually. WGPU required graphics-device access outside
the sandbox. Native font rasterization can differ from browser text.

## Exact files changed

State and model: `src/model/crosshair.rs`, `src/model/magnet.rs`,
`src/model/mod.rs`, `src/model/chart_model.rs`, `src/model/axis_snapshots.rs`,
`src/model/time_scale.rs`, `src/model/series.rs`,
`src/model/chart_model/tests.rs`, `src/model/chart_model/tests/navigation.rs`.

Geometry and tests: `src/renderers/crosshair_renderer.rs`,
`src/renderers/mod.rs`, `src/renderers/price_axis_renderer.rs`,
`src/renderers/price_axis_renderer/tests.rs`,
`src/renderers/time_axis_renderer.rs`,
`src/renderers/time_axis_renderer/tests.rs`.

Iced adapters and demo: `src/ui/chart_input.rs`, `src/ui/mod.rs`,
`src/ui/line_chart.rs`, `src/ui/price_axis.rs`, `src/ui/time_axis.rs`,
`examples/line_chart.rs`.

Reference fixtures: `examples/price_axis_reference.rs`,
`examples/time_axis_reference.rs`, `examples/interaction_reference.rs`,
`scripts/check_interaction_reference.cjs`.

Reports/checklists: `docs/porting/mouse-navigation.md`, this report,
`tasks/plan.md`, `tasks/todo.md`.

## Remaining dependencies and limits

Physical mouse/touchpad input has not been manually tested in a desktop window.
Touch/kinetic/axis gestures, multi-pane UI, additional series renderers, custom
series magnet candidates, synthetic public API positioning and hover/click
object events remain separate work. General hit testing is not introduced.
The nearest candidate operation scans eligible series synchronously and avoids
sorting/allocation inside selection; the current model still prepares owned
candidate coordinates. Large interactive workloads have not been benchmarked.

Graph coverage was checked for relied-on paths. Metadata was best-effort; stale
TimeScale/draw-line/Series/price-formatter records were supplemented with current
source reads. Canvas Action lives in Iced `widget/src/action.rs`, and crosshair
markers use source `renderers/marks-renderer.ts` (there is no separate
crosshair-marks renderer file).

## Desktop freeze regression (2026-10-12)

The first physical run exposed repeated identical cursor-position messages.
Every message rebuilt prepared geometry and the UI, keeping the native input
loop busy. The earlier raster captures never exercised that event cycle.
`ChartInput` now retains the last processed logical cursor coordinate and
suppresses unchanged movement. It retains outside-plot coordinates for drag
continuation and resets the cursor on focus loss/window leave. Wheel, button
and layout changes still dispatch independently.

The new input regression failed before this fix and passes afterward. It covers
stationary replay, dragging outside the plot and re-entry. A connected demo
test applies repeated pointer and redraw events at ordinary and fractional
canvas sizes and verifies that frame updates settle. Full validation passes:
293 library tests plus 4 demo tests, all-target check, Clippy and formatting.
The fixed native application ran for 25 seconds without exiting or producing
errors, then the test timeout closed it intentionally. Automated compositor
pointer movement was unavailable; physical wheel/drag responsiveness still
needs user confirmation. Diagnostic logging was removed.

## Wheel zoom performance (2026-10-12)

Profiling the debug build identified repeated axis glyph extraction as the
main latency, not time-scale updates. Iced `Text::draw_with` creates a fresh
Swash outline cache per label. Our adapters had called it for every label in
every newly prepared frame, even when only the position changed.

The UI-owned `AxisFontResolver` now keeps at most 256 origin-relative label
path entries, keyed by exact content, resolved font (including weight), size,
horizontal alignment and color. Current axis text uses fixed advanced shaping,
vertical centering, infinite width and a 1.2 line-height. Preparation translates
cached paths into newly owned frame geometry; drawing still only reads paths.
Font registration clears the cache. FIFO eviction bounds retained labels, and
previous frames remain valid after eviction. Colored bitmap-glyph paths retain
the colors returned by Iced. Exact backend text measurement is unchanged.

The alternative of using ordinary Canvas text would reduce extraction work,
but reintroduce the previously verified TinySkia clipping limitation. Cache
origin-relative paths instead: reuse existing font ownership and clipping, add
no dependency and leave navigation semantics unchanged.

Reproduce the benchmark with:

```sh
cargo test --example line_chart profile_zoom_frame_stages -- --nocapture
```

It uses 120 zoom frames, discards 20 warmup samples, and prints median/p95 stage
times. Both alternating directions and sustained zoom-in/out are exercised at
917 × 870.80005 logical pixels. Times describe preparation, not GPU presentation
or physical wheel-to-screen latency.

| Attempt/workload | Before median/p95 | After median/p95 | Decision |
| --- | --- | --- | --- |
| Cache axis glyph paths, alternating zoom | 124.683 / 132.945 ms | 0.599 / 0.816 ms | Kept; dominant cost removed |
| Engine/measurement, alternating zoom | 9.239 / 9.626 ms | 9.170 / 10.531 ms | Unchanged; no speculative measurement cache |
| Cached glyph paths, sustained zoom | Not measured before | 0.522 / 0.610 ms | Additional workload check |
| Engine/measurement, sustained zoom | Not measured before | 8.100 / 8.810 ms | Additional workload check |

Repeated alternating measurements yielded 0.569–0.608 ms median for cached
glyph extraction versus 124.683 ms before; the improvement exceeds sample
variance. Summed median preparation stages fall from about 134 ms to 10 ms.
No smoothing animation, sensitivity change or wheel-event dropping was added.

Tests compare cached paths/colors with direct Iced extraction, translated
positions, distinct fonts/sizes/alignments/colors, font registration, FIFO
eviction and previously prepared geometry ownership. All 295 library tests
and 5 demo tests pass, together with all-target check, build, Clippy and fmt.
28 TinySkia and 28 WGPU interactive captures pass repeatability, clipping and
state-immutability assertions. Compared with pre-cache images, 22/28 captures
per backend are byte-identical. Worst changed color-channel fractions are
0.003500% (TinySkia) and 0.000448% (WGPU); translation rounding can alter a few
edge antialiasing pixels. Geometry/layout is unchanged. New captures live in
`/tmp/feather-zoom-cache-tinyskia` and `/tmp/feather-zoom-cache-wgpu`.

Files for this follow-up: `src/ui/price_axis.rs`, `src/ui/time_axis.rs`,
`examples/line_chart.rs`, this report. First encounters of new labels still
extract glyphs; the bounded cache may evict uncommon labels. Physical
touchpad smoothness remains unverified. After this fix, the user ran the native
application and confirmed that zoom was "super smooth". Earlier they confirmed
that the desktop freeze was resolved. Full drag/cancellation hardware testing
and touchpad-specific behavior remain unverified.
