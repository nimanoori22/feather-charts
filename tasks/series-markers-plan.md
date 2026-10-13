# Implementation roadmap: series markers and pointer identification

Date: 2026-10-13. Status: implemented and automatically verified; fresh physical acceptance pending.
Execution checklist: [series-markers-todo.md](series-markers-todo.md).
Foundation: all six built-in families delivered through `2718f18`.
Existing milestone plans and pending physical checks remain separate.

## Outcome and scope

Expose TradingView-style time-based annotations on all six built-in series,
with replacement/clear operations, source-compatible placement, shapes, text,
autoscale margins, drawing layers and marker identification in click/crosshair
events. Support circle, square, arrow-up and arrow-down; above/below/in-bar and
explicit-price top/bottom/middle positions. Include marker size, optional external
ID/text, marker autoscale enablement and normal/above-series/top layers.

Use the current `Time` behavior and synchronous Chart ownership. Exclude arbitrary
custom primitives, a general plugin framework, marker dragging/editing, trading
actions, alerts, full series/price-line hit testing, hovered-series-on-top,
touch/kinetic gestures and last-price animation. Existing point/crosshair dots
are distinct from these annotations and retain their behavior.

## Preliminary source and Rust findings

The inspected source plugin separates wrapper lifecycle, time reindexing,
autoscale margins, pane placement and shape/text drawing. `primitive.ts` maps
marker times through the shared time scale, then selects the associated series
row using nearest-right before its first row and nearest-left otherwise. Do not
require exact marker timestamps or add markers as chart data/time-scale rows.
`pane-view.ts` stacks above/below markers at the same resolved time, uses high/low
for OHLC (swapped on inverted scales), close for in-bar, and value for scalar
series. Explicit-price positions have distinct offset semantics.

Source autoscale contributes pixel margins with a null price range; it does not
simply include every explicit marker price in the data range. Its margin formula
uses bar spacing and position presence. Preserve the actual algorithm rather
than promising that arbitrarily large text/stacks will always fit.

`renderer.ts` tests text and shapes and returns the first matching visible item;
it reports normal hit order even where drawing uses another layer. Trace the
pane-level selection rules before assigning overlap priority. Text is measured
during source drawing; Rust must prepare text and hit bounds before immutable
drawing, using the existing font/cache path.

Rust already has `InternalHitTestCandidate`, `HitTestPriority`, and
`AutoscaleInfoImpl`/`AutoScaleMargins`. They are reuse candidates, not proof that
marker selection or primitive integration is already wired. `PointerEvent`
currently contains pane/point/logical/time/price/series data; marker identity needs
a deliberate typed extension. Preserve exact-index event rows and raw pointer
coordinates independently of magnet snapping.

These findings come from graph discovery/coverage checks and direct reads of
types/options/wrapper/primitive/pane-view/renderer plus the Rust event/autoscale/hit
types. They are a starting map, not the complete consumer audit. A file-filtered
graph search returned no matches; broad symbol search found the plugin classes.
Complete source/dependency/test reads and traced consumers remain task 0.

## Investigation and design gate

Start at `../lightweight-charts/src/plugins/series-markers/`:
`types.ts`, `options.ts`, `wrapper.ts`, `primitive.ts`, `pane-view.ts`,
`renderer.ts`, `utils.ts`, and all four shape/text helpers. Trace
`createSeriesMarkers`, primitive attachment/adapters, data-change callbacks,
series autoscale aggregation, pane drawing/hit selection and ChartApi event
conversion across the source project. Inspect marker e2e coverage/interactions,
especially text hits, explicit-price text, overlap and hits after time-scale changes.
Pin the local source revision in the port report.

Follow repository AGENTS phases before production edits: complete components,
graph consumers with coverage/source fallback, required versus incidental behavior,
existing Rust architecture, exact Iced rendering/input APIs, two viable designs,
ownership, signatures and mapping back to real consumers. Flowsurface is an example
only. Record ordering requirements, duplicate IDs/times, absent IDs, negative/zero
size, empty/whitespace data, out-of-range times and source failure cases explicitly.
Keep Rust validation transactional; document safer deviations from source failures.

## Architecture and API direction

Compare two designs:

1. Chart/series-owned marker collection with typed replacement/read/options calls
   and prepared marker commands/hit bounds in existing frames.
2. A narrow owned marker attachment identified by a handle, supporting explicit
   detach and multiple collections without a general primitive registry.

Prefer design 1 if real consumers do not require multiple independent attachments.
If they do, use design 2 narrowly; do not silently merge collections or reproduce
JS sharing/subscription identity. Resolve this in task 0, before freezing the API.

Provisional surface (names/signatures finalized by that gate):

```rust
pub enum MarkerPosition {
    AboveBar, BelowBar, InBar,
    AtPriceTop(f64), AtPriceBottom(f64), AtPriceMiddle(f64),
}
pub enum MarkerShape { Circle, Square, ArrowUp, ArrowDown }
// SeriesMarker owns Time, position, shape, color, size and optional ID/text.
// Chart: set_series_markers, series_markers, replace_series_marker_options.
// PointerEvent: optional typed marker hit containing SeriesId and marker identity.
```

Distinguish optional user-facing IDs from internal collection identity; do not
assume unique external strings. Explicit-price enum variants require a price by
construction. Reuse existing validation/error conventions and finite bounds.

```text
Application -> Chart -> series/marker data and options
Preparation -> indexed markers -> projected commands + measured text + hit bounds
Frame owner -> immutable pane commands and hit metadata
Canvas -> borrows frame; publishes owned input messages
Chart update -> resolves marker hit; returns owned pointer events
```

No new back-reference, Rc cycle, callback owner, lock, async channel or dependency
is expected. Series removal releases its marker collection; dropping Chart releases
everything. Clearing series data temporarily removes prepared markers while keeping
annotations for reload, subject to the source lifecycle audit. All shared time-index
changes must invalidate affected collections, including changes in other series.
Hit bounds must correspond to the settled frame being displayed. Preparation owns
projection/text work; drawing never changes marker data or text bounds.

## Delivery order

0. Establish contract, lifecycle/API/ownership and source oracle approach.
1. Deliver a minimal vertical path: validated circle above a public Candlestick
   series, replacement/clear and owned-frame drawing.
2. Complete time mapping, all positions, stacking and autoscale on six families.
3. Complete remaining shapes, optional text, size and all drawing layers.
4. Add shape/text marker selection against prepared frames.
5. Extend public pointer events and reconcile hover through lifecycle/navigation.
6. Deliver public demonstration, source/raster/performance verification and report.
7. Complete physical acceptance separately from automated/native checks.

Each numbered phase is subdivided in the checklist into buildable slices of roughly
five production files or fewer. Do not expose unsupported behavior as working.
Checkpoints verify progress without introducing approval pauses into authorized
implementation. Use logical local commits; pushing requires the user's request.

## Acceptance and risks

| Area | Required evidence |
| --- | --- |
| Time/lifecycle | Duplicate times, missing timestamps, whitespace, before/after series range, other-series reindexing, latest/history changes, clear/reload, hidden/removal |
| Placement | Six positions, OHLC/scalar, same-bar stacking, independent panes/scales, normal/log/percentage/indexed/inverted/manual scales |
| Rendering | Four shapes, size zero, text-only hits, empty/long text, alpha, edge clipping, independent collections if supported, three layers |
| Pointer events | Shape/text boundaries, overlap selection, missing/duplicate IDs, raw versus magnet position, click versus drag, exit/clear/remove, hit after zoom/resize/DPI change |
| Autoscale | Source margin formulas, enabled/disabled, bar-spacing changes, mixed-series margin aggregation and bounded frame settlement |
| Performance | No per-draw projection/shaping; unchanged empty-marker path; large mostly-offscreen collection and zoom/update/hover median/p95 |

High risks: assuming chronological input semantics, confusing external/internal IDs,
stale shared indexes or hit bounds, repeated text shaping on mouse movement, treating
marker margins as price ranges, and adopting drawing layer order as hit priority.
Resolve these with source fixtures and focused regressions before broad integration.
Document Canvas/Iced text/raster differences; do not require identical font pixels.

## Final verification

Run focused tests and actual-source oracles at each slice. At completion:

```sh
cargo fmt --check
cargo test --all-targets
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --examples
git diff --check
```

Run all existing source comparison scripts plus the new marker oracle. Capture
TinySkia and WGPU at ratios 1, 1.25, 1.5 and 2 for mixed series, stacking/text,
layers, inverted/explicit prices and hit-after-navigation fixtures. Inspect results
and run bounded native startup. Benchmark the same pre/post workloads against
`2718f18`, separately measuring empty-marker overhead and populated collections.

`docs/porting/series-markers.md` must report source components/revision, consumers,
contract, discarded TS conventions, alternatives, chosen ownership/API, exact files,
tests, results, limits and remaining dependencies. README states the verified scope.
Fresh physical zoom/pan/axis-scale/hover/click checks remain a separate acceptance
gate; synthetic input and prior hardware checks cannot close it.
