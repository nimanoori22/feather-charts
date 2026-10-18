# Series price-scale assignment (public parity milestone B)

Status: B0 investigation and regression work. Baseline `0c27d0c`.
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
focused Clippy pass. Full renderer/source/compatibility and mouse gates remain B3.
