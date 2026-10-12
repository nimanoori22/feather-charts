# Implementation roadmap: price lines and last-value labels

Date: 2026-10-12. Status: implemented; automated verification complete, fresh physical acceptance pending.
Execution checklist: [price-lines-todo.md](price-lines-todo.md).
Foundation: [public chart API](../docs/porting/public-chart-api.md).

## Outcome and scope

Render source-compatible last-value axis labels and automatic price lines for
line, candlestick and histogram series. Expose custom price-line creation,
replacement of options, inspection and removal through the reusable Chart API.
Support titles, independent line/label visibility, colors, styles and widths,
multiple series per scale, independent pane scales and live updates.

Preserve current navigation, wheel sensitivity, glyph caching, chart-wide layout
settlement and immutable drawing. Entry/stop/target levels are visual annotations;
they do not execute orders or evaluate alerts.

Defer draggable lines, annotation hit testing/events, last-price animation,
plugins, new series families and area/baseline rendering. Those are separate
milestones. This work must not require a new renderer registry or async machinery.

## Inspected foundation and source findings

- Rust series options already represent last-value visibility, title, price-line
  visibility/source/color/style/width and last-value display mode. Reuse them.
- Rust price scales already provide formatting, projection and `align_labels`.
  Existing owned axis snapshots/renderers support ordinary ticks and crosshair
  labels; extend that pipeline for a collection of series/custom labels.
- Chart owns coordinated data/model operations and prepared frames. Both ordinary
  and composed pane frames must receive decorations through their common path.
- Source `Series.lastValueData(false)` searches nearest-left at the visible right
  edge. The axis label uses that result. Automatic price lines use either that
  result or the global last fulfilled row according to `priceLineSource`.
  Do not assume an axis label and a global-last line always show the same value.
- Source styles come from the selected row, with explicit price-line color taking
  precedence. Formatting/projection uses the associated scale and series base.
- Custom lines belong to a series. Source updates invalidate their pane/axis views;
  series visibility and missing projection suppress output. Line visibility is
  independent of axis-label visibility; a custom title follows the source label
  visibility rules.
- Source axis overlap correction separates actual price coordinate from render
  coordinate, uses ordered sources and a center reference, and draws crosshair
  labels separately. A shifted label must never move its horizontal price line.

Findings use graph discovery, coverage checks and direct reads. Graph inbound
tracing returned no consumers for lastValueData despite direct source call sites;
those views were read directly. Series metadata was stale, so its current relevant
methods were read from disk. These are bounded findings, not an exhaustive audit.

## Source investigation before implementation

Read complete components and relevant imports, trace real consumers across API,
model, pane/axis views and widgets, and inspect original tests. Record the contract,
ownership, alternatives and Rust signatures in `docs/porting/price-lines.md`
before production changes. Finish investigation of defaults, horizontal-line
geometry, contrast colors, pane-title rendering, autoscale participation, width
measurement and overlap ordering; do not infer them from field names.

Starting sources relative to `../lightweight-charts/src/`:

| Area | Sources | Required evidence |
| --- | --- | --- |
| Last value | `model/series.ts`, `model/iseries.ts`, `model/series-options.ts`, series colorer | Visible/global selection, whitespace, close/value selection, colors and base |
| Automatic line/label | `views/pane/series-price-line-pane-view.ts`, `series-horizontal-line-pane-view.ts`, `views/price-axis/series-price-axis-view.ts` | Visibility, title/display modes, clipping, geometry and paint order |
| Custom lifecycle | `api/series-api.ts`, `api/price-line-api.ts`, `model/custom-price-line.ts`, `model/price-line-options.ts`, price-line defaults | Mutation, validation, ordering, invalidation, ownership and cleanup |
| Custom output | Custom price-line pane/axis views, `views/pane/pane-price-axis-view.ts` | Price projection, formatting, title and color fallback |
| Axis layout | `gui/price-axis-widget.ts`, `views/price-axis/price-axis-view.ts`, axis renderer and overlap helper | Width, actual/render coordinates, edge handling, ordering and crosshair priority |

Follow direct dependencies discovered during tracing. Consult exact local Iced
and Flowsurface only where additional drawing/input knowledge is needed.

## Design boundaries and provisional API

Compare two ownership designs before committing:

1. Series-owned typed price-line records, addressed by IDs through Chart. This
   matches source ownership and naturally drops lines when a series is removed.
2. A Chart-owned map keyed by series/line IDs. This simplifies lookup but requires
   explicit cleanup and coordination with the model's series lifecycle.

Prefer design 1 if it fits the existing model without duplicate ownership.
Expose IDs and synchronous mutations rather than JS wrapper identity, shared
mutable callback objects or manual destroy. Keep any user-facing string tag
distinct from the internal lifecycle ID. Validate all new options before mutation;
invalid updates preserve existing options and frames. Determine removed/foreign
handle behavior explicitly and test it; match the existing typed-error style.

Implemented API after consumer analysis (inspection returns an owned clone):

```rust
// PriceLineOptions includes a required finite price and source-backed defaults.
pub struct PriceLineId(/* opaque, non-recycled identity */);
impl Chart {
    pub fn add_price_line(&mut self, series: SeriesId, options: PriceLineOptions)
        -> Result<PriceLineId, ChartError>;
    pub fn replace_price_line_options(&mut self, line: PriceLineId,
        options: PriceLineOptions) -> Result<(), ChartError>;
    pub fn price_line_options(&self, line: PriceLineId)
        -> Result<PriceLineOptions, ChartError>;
    pub fn remove_price_line(&mut self, line: PriceLineId)
        -> Result<(), ChartError>;
}
```

Reuse existing full-options replacement conventions; introduce a patch type only
if real consumers need it. Last-value labels require no separate public owner.

```text
Application -> Chart -> model -> series -> custom price-line records
Frame preparation -> owned line/axis/pane-label snapshots
Canvas -> immutable borrowed frame
```

All relevant labels must participate in shared axis-width settlement before glyph
geometry is finalized. Prepare decorations once per pane/scale, avoiding duplicate
lines or labels when several series are composed. Use measured metrics and source
contrast rules; do not invent a second text cache or recolor labels arbitrarily.

## Delivery order

1. Establish behavioral contract and ownership/API design.
2. Implement/test visible versus global last-value selection and formatting.
3. Render automatic price lines through the shared frame path.
4. Prepare/render owned last-value axis and pane-title labels.
5. Integrate label measurement into single/multi-pane frame settlement.
6. Add custom price-line lifecycle/API with validation and cleanup tests.
7. Render custom lines, axis labels and titles using the same primitives.
8. Implement source overlap alignment without moving true price coordinates.
9. Demonstrate live updates and entry/stop/target lifecycle through the public API.
10. Complete source comparisons, raster/performance checks and final report.

The checklist breaks these into bounded slices. Keep each slice buildable, include
exports/callers/fixtures, and split a slice if it grows past roughly five production
files. Tests/source comparisons accompany behavior; final verification consolidates
evidence rather than adding all tests at the end. No automatic publishing/push.

## Acceptance and risks

| Risk | Acceptance/mitigation |
| --- | --- |
| Wrong value after scrolling away from the latest bar | Test global-last and visible-last independently, including sparse data |
| Incorrect base/scale/format | Cover left/right/overlay placement, log/percentage/indexed modes, volume and precision |
| Labels change plot width or cause redraw loops | Include label metrics in chart-wide bounded settlement; test repeated preparation |
| Collisions move the price line | Keep true coordinate separate; compare overlap algorithm and disabled alignment with source |
| Custom level unexpectedly expands autoscale | Trace actual source autoscale consumers and test the resulting contract |
| Stale annotations after removal/clear | Test live replacement, whitespace, hidden series, series/pane removal and stale IDs |
| Default-enabled features alter existing captures | Update expected output deliberately; retain prior geometry/navigation assertions |
| Hot-path regressions | Measure unchanged workloads with identical data, build profile, median/p95 and glyph-cache reuse |

At checkpoints run formatting, focused tests and `cargo check --all-targets`.
Before completion run `cargo test --all-targets`, strict all-target Clippy and
`cargo build --examples`. Add source oracles for selection/geometry/alignment;
rerun affected existing oracles. Capture both TinySkia and WGPU at scales 1, 1.25,
1.5 and 2, with clusters, long titles, both sides, hidden/empty states and live input.
Verify retained-frame drawing is deterministic and does not mutate the engine.

Native/synthetic checks and physical mouse/touchpad acceptance are recorded
separately. The final report must cover investigated sources, consumers, contract,
discarded TS patterns, chosen ownership/API, alternatives, exact changed files,
tests, commands/results and remaining dependencies. No completion percentage or
delivery estimate is claimed before implementation evidence exists.
