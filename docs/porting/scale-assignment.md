# Series price-scale assignment (public parity milestone B)

Status: B0–B2 implemented in `1bdbc68`; B3 accepted: demo, automated gates and four-mode software/GPU mouse acceptance pass. Baseline `0c27d0c`.
Formatting was accepted for progression by the user; its separate GPU mouse record
remains pending and is not inferred from that statement.

## Source and consumers

Local 5.2.1 Series.applyOptions requests ChartModel.moveSeriesToScale before merging
a changed priceScaleId. ChartModel removes/adds the same source on its current Pane
with keepSourcesOrder. Pane creates missing overlays, deletes empty overlay
registrations, preserves shared overlays, and recalculates affected scales.
PriceScale.removeDataSource resets an emptied scale to automatic with no range.
SeriesApi delegates options and queries the actual current scale and pane.

The source graphics consumer api/change-series-order.js explicitly checks unchanged
series order after moving a reordered series to the left axis. API scale queries,
overlay-width cases, last-value labels/crosshair and pane transfer consumers require
membership and stored options to agree. Graph edges for Series.applyOptions are
sparse/stale; current method bodies and usage searches supplement the graph.

## Designs and ownership

1. Reuse Pane.move_source_to_scale, with ChartModel as the sole scale/membership
   mutation owner. Its existing type-erased projection updates the common option
   on either built-in or custom series. Chart preflights, cancels stale interaction
   once, and delegates. Chosen: preserves typed data, views, handles and annotations.
2. Remove/recreate a public series on another scale, or separately mutate typed
   owners in each frontend. Rejected: loses identity/lifecycle and duplicates cache,
   error and annotation handling. No new retained scale wrapper is needed.

Chart owns model/typed views; model and pane adapters share existing series Rc
handles. Transferring reuses those handles, never removes a data-layer series or
reconstructs a custom view. Empty overlay scale ownership is released; the source
and its annotations remain owned. No new backreference, thread or asynchronous path.

## Native contract and proposed API

```rust
impl<B: ChartBehavior> Chart<B> {
    pub fn series_price_scale(&self, id: SeriesId) -> Result<PriceScalePosition, ChartError>;
    pub fn set_series_price_scale(&mut self, id: SeriesId, position: PriceScalePosition)
        -> Result<(), ChartError>;
}
```

Source strings map to Left, Right or Overlay(String), including the empty overlay
name. Typed Overlay("left")/Overlay("right") are invalid aliases: use the dedicated
variants. A missing named overlay is created in the source's current pane, even if
another pane has an identically named overlay. Selectors are values, not retained
references; unknown/foreign series IDs fail before interaction cancellation.

Creation's explicit native position argument wins over any conflicting option;
stored price_scale_id is normalized to the actual ID. Replacement options with
Some(id) request relocation; None retains current membership (source undefined does
not relocate). Explicit setter also normalizes the stored option. Existing series
pane moves preserve assignment. No-op assignments preserve marks/events/gestures.

Preflight validates ID/type/options/selector before any mutation or cancellation.
Invalid mixed replacements leave options, membership, data and interaction intact.
Successful relocation cancels stale edits/navigation once, preserves series order,
handles, data and annotations, then settles old/new scale autoscaling and crosshair.
Shared manual scale state is retained; empty overlays are removed, so recreating
one uses fresh configured defaults. Side axes persist and reset when emptied.

Rust does not retain JS source-object identity conventions or mutable option sharing.
Ownership and fresh typed selectors replace retained scale API wrappers; retained
screenshots keep their resolved geometry and strings after transfers.

## Proven overlay interaction defect

A compiling regression reproduced unchanged overlay range after diagonal plot drag.
The native PlotPan reduced every non-left scale to Right; source PaneWidget passes
its actual default scale to ChartModel scrolling. Keep the real typed position in
PlotPan and reuse the existing synchronous model/pane scroll operations for named
overlays. Existing public axis-side methods remain compatibility wrappers. No Iced
event decoding changes are needed; mouse/touch use the same established pan path.

## Core implementation and verification

Implemented one checked ChartModel assignment owner over the existing Pane transfer,
plus public Chart/YieldCurveChart setter and fresh query. Explicit creation selectors
normalize stored IDs; option replacement preflights type/visual validity before
interaction cancellation and routes both built-in/custom updates through that owner.
CustomSeriesHandle.price_scale_id returns an owned retained stored-ID readback; use
the checked chart query for current membership. No custom view/renderer Clone bound
was added. Existing axis-side scrolling methods remain public compatibility wrappers.

The initial public replacement regression compiled and failed on the empty left
scale; it now passes. All 20 frozen browser/native states agree. Eight focused tests
cover option/direct transfers, stored/actual assignment, order/data/price-line
identity, missing/empty-name overlays, manual/shared-overlay lifetime, creation
precedence, retained custom handles, invalid mixed replacements, foreign IDs,
no-op/failed active-pan preservation and successful cancellation. A custom-view
counter proves transfers do not destroy the view or clone canonical payloads.
The separately reproduced overlay plot-pan bug also passes after its fix.
Library, existing pane-management, formatting and localization suites plus strict
focused Clippy pass. B3 renderer/source checks pass; final compatibility and mouse gates are recorded below.

## Public demo and capture protocol (B3)

The `--scale-assignment` toolbar supports selected blue/custom series, setter versus
option replacement, left/right/shared/named/empty overlays, manual/automatic range
and an invalid-alias probe. Actual/stored IDs, pane and order are displayed together.
Custom→line shares a pane; assigning both to Shared A demonstrates shared manual
range and cleanup only after the last source leaves. Existing pane movement, live
feed, box editing and screenshot preview/export controls remain available.

`capture_scale_assignment.py` exercises four domains × eight scenes × three sizes
× four pixel ratios on both strict renderers: 768 screen replays and retained
exports. Each retained screenshot is rerendered after moving its owner to another
scale, resizing and dropping the chart. Existing protocols, counts, thresholds and
accepted image hashes remain independent compatibility gates.

B3 complete. Preserve prior formatting GPU, timing, drawing-order and unavailable-device records.

## Files and review

Core Rust changes: `src/api/chart.rs`, `custom_series.rs`, `interaction.rs`,
`price_scale.rs`, `yield_curve.rs`; `src/model/chart_model.rs`,
`chart_model/scale_assignment.rs`, `pane.rs`, `series.rs`, `series/projection.rs`,
and `series_options.rs`. Behavioral regressions live in `tests/scale_assignment.rs`.
The existing pane-management consumer adds `scale_assignment.rs` and
`scale_assignment_captures.rs`, with small dispatch/host changes in
`examples/pane_management.rs` and `examples/pane_management/host.rs`.

Self-review checked option/membership atomicity, custom view ownership, shared-scale
lifetime and existing pane source ordering. Demo actions reuse public APIs; optional
controls stay in their own module. No dependencies or background engine work were
added. Full replacement via the demo's options button intentionally reconstructs its
demonstration styling; applications needing assignment alone should use the setter.

## Remaining dependencies

B3 physical mouse acceptance passed on both renderers. Horizontal
axis input/settings (C), broader public helper/readback APIs (D), pane pixel sizing
(E), metadata (F/G), locale defaults (H) and color parsing/gamut policy (I) remain
separate milestones. No automatic closure of older timing or device exceptions.

## Mouse acceptance procedure

Use the normal live-feed demo in calendar, strike, maturity and sessions modes,
first with `ICED_BACKEND=tiny-skia`, then `ICED_BACKEND=wgpu`.

1. Select blue line and custom in turn. Try setter/options, Left, Right, Shared A,
   Other B and Empty-name overlay. Actual and stored IDs must agree; order and data
   must survive. Overlays share plot space without adding a dedicated axis.
2. Reset; use Custom→line to share a pane. Assign both series to Shared A; Freeze
   sets that shared scale to 0–10. Move one to Other B; the remaining series keeps
   the frozen scale. Leave/recreate the overlay and check it returns to automatic.
   Some values intentionally fall outside the demonstration manual range.
3. Try Invalid alias; it must report rejection and preserve assignment. Move/swap
   panes, move the line to a new pane, then Reset. The box follows the blue series.
4. With feed running at regular, narrow and the earlier problem size, test wheel
   zoom, diagonal plot drag, price-axis wheel/drag and box move/resize/Escape.
   Plot drag follows the pane's default scale, which may differ from the selected
   series' independent scale. Shrink to short height, enlarge and check recovery.
5. Capture + save, inspect the preview, then Hide preview. Close all four windows
   and report the renderer, passed checks or concrete failing gesture/size.

An automated ratio capture is not physical high-DPI acceptance. A backend's physical
result is recorded only after an explicit user report and normal session exit.

## Validation record

`cargo fmt -- --check`, `cargo test --offline --all-targets` (816 tests across 82
suites), `cargo test --offline --doc` (2 tests), `cargo check --offline --all-targets`,
`cargo clippy --offline --all-targets -- -D warnings`, and example builds pass.
All 39 source comparisons pass, including the 20 frozen assignment states.
Both strict renderers pass 768 exact screen replays and 768 retained exports.
Representative left-axis, overlay, shared/manual and moved-pane captures were
visually inspected across all four domains. Source and demo hashes match the tested
binary's capture manifest. No timing or image threshold was relaxed.

The machine-readable [evidence record](scale-assignment-evidence.json) pins source
comparisons, binary, protocol/results and log hashes. Mouse acceptance is recorded
separately; synthetic captures do not establish physical smoothness.

All five unchanged compatibility protocols pass: events (912), custom horizontal
(240), screenshot/export (1158), attribution (672), and formatting (576): **3558
previously accepted images unchanged**. B3 physical acceptance is recorded below.

Software physical acceptance: the user reported all checks, previews and exports
passed in calendar, strike, maturity and sessions. All four pinned-binary sessions
exited 0. GPU physical acceptance also passed all four modes, previews and exports, with
all four sessions exiting 0. Older formatting/device exceptions
are not inferred from these scale-assignment checks.

Reviewed local checkpoints: core `1bdbc68`, demo/evidence `618dc49`; no push requested.
