# Implementation roadmap: histogram and synchronized price/volume panes

Date: 2026-10-12. Status: implemented; physical mouse/touchpad acceptance pending.
Results: [histogram report](../docs/porting/histogram-rendering.md) and
[multi-pane report](../docs/porting/multi-pane-rendering.md).
Execution checklist: [histogram-volume-todo.md](histogram-volume-todo.md).
Previous milestone: [candlestick-plan.md](candlestick-plan.md). Its pending
physical mouse/touchpad checks remain in [candlestick-todo.md](candlestick-todo.md).

## Outcome and scope

Deliver histogram rendering, then a runnable chart with candlesticks above a
volume histogram. Both panes share time coordinates, zoom and horizontal scroll;
each has its own price scale. Crosshair time stays aligned across panes, while
price interaction belongs to the active pane. Window resizing, pane-height
changes and live data updates must preserve that alignment and smooth navigation.

Include basic pane-divider resizing after establishing the source contract.
Defer arbitrary pane reordering, the general public chart/widget facade, new
series families, touch/kinetic gestures, plugins and unrelated hit testing.
The reusable public API is the next milestone after this rendering path is proven.

## Inspected foundation

- Histogram input, plot rows and style options already exist. Series autoscale
  includes the configured histogram baseline and has a behavioral test.
- Renderer exports currently have no histogram renderer. Reuse the existing
  backend-neutral commands, selected scales and owned pane-view approach.
- `ChartModel` already owns multiple panes and one time scale, and exposes
  `add_pane` and `set_pane_height`. This does not establish complete UI support.
- `ChartFrameOwner` shares line/candle preparation but explicitly rejects more
  than one pane. `ChartLayout` and `ChartInput` currently describe one plot.
- Reuse the existing shared demo support, glyph cache, navigation coalescing,
  source-comparison scripts and capture/benchmark mechanisms.

These findings use graph discovery, coverage checks and direct source reads.
Coverage reported no recorded issues for the relied-on Rust paths; this is a
best-effort signal. Full source consumer analysis remains an implementation task.

## Source investigation map

Source paths below are relative to `../lightweight-charts/src/`.

| Area | Starting sources | Contract to establish |
| --- | --- | --- |
| Histogram geometry | `renderers/histogram-renderer.ts` | Baseline thickness, positive/negative bars, width rounding, adjacency correction, gaps and paint order |
| Histogram preparation | `model/series/histogram-pane-view.ts`, `line-pane-view-base.ts`, their imports | Visible selection, colors, baseline projection, cache invalidation and empty output |
| Data and scales | Histogram series, series colorer, series/data-layer and price-scale consumers | Row overrides, autoscale, first value, updates, whitespace and supported scale modes |
| Pane layout/lifecycle | `gui/chart-widget.ts`, `gui/pane-widget.ts`, pane/model and separator consumers | Shared axis widths, pane heights, clipping, time-axis placement, insertion/removal and resize rules |
| Input and crosshair | Chart/pane widgets, magnet, crosshair pane/axis views | Active pane, gesture ownership, cross-pane vertical projection, labels and cancellation |
| Iced integration | Exact local Iced widget/canvas/runtime sources; relevant Flowsurface examples | Event capture, layout, divider dragging, clipping and fractional device scaling |

Locate dependencies rather than guessing their filenames. Read complete source
components, trace their actual consumers with codebase-memory, check coverage
and read missed source directly. Separate required behavior from incidental
TypeScript inheritance, identity, callbacks and manual destruction.

Before engine changes, record consumer groups, behavioral contracts, two viable
designs, ownership, important Rust signatures and consumer mappings in
`docs/porting/histogram-rendering.md` and `docs/porting/multi-pane-rendering.md`.
The first report precedes histogram work; the second precedes multi-pane work.
The roadmap names likely files, not a finalized API.

## Architecture and design boundaries

Keep preparation and rendering synchronous. Reuse typed histogram rows/options;
do not add another volume data model or async transport inside the engine.
Prepare owned geometry and snapshots; drawing borrows them without measuring
text, draining invalidation or advancing animation.

```text
Application owns DataLayer, ChartModel, animation controller and frame owner
ChartModel owns one time scale and pane-local price scales/series relationships
Frame owner coordinates one chart-wide animation/invalidation/layout cycle
Prepared chart owns pane snapshots, axes and crosshair geometry
Canvas borrows snapshots and emits typed pane-aware interaction
Application applies synchronous operations and prepares the next chart frame
```

For histogram integration, compare extending the existing private typed dispatch
with a histogram variant against separate typed entry points using the shared
settlement path. Prefer the smallest design that preserves existing callers.

For multiple panes, compare one frame owner containing pane preparation state
against a chart coordinator with subordinate pane preparers. Both must sample
animation once and settle shared dimensions globally. Independent owners that
each consume chart invalidation are not a valid design. Avoid a generic renderer
registry, thread-safe shared state or speculative public facade.

Source investigation must settle pane index/lifecycle rules, maximum shared
left/right axis widths, height allocation/minima, divider ownership, crosshair
projection and visibility behavior before new UI contracts are frozen.

## Delivery order

```text
0. Histogram source contract and design
 -> 1. Histogram geometry and source oracle
 -> 2. Model preparation
 -> checkpoint: backend-neutral histogram
 -> 3. Shared frame integration and histogram demo
 -> 4. Multi-pane source contract and design
 -> checkpoint: working histogram, reviewed multi-pane design
 -> 5. Multi-pane layout
 -> 6. Chart-wide frame settlement and drawing
 -> checkpoint: aligned static price/volume frame
 -> 7. Pane-aware navigation and crosshair
 -> 8. Pane-divider resizing
 -> checkpoint: synchronized interactive panes
 -> 9. Price/volume demo and live data lifecycle
 -> 10. Integrated parity, raster and performance validation
 -> checkpoint: verified price/volume milestone
```

Keep each slice buildable. Split tasks exceeding roughly five production files;
include affected callers, exports, fixtures and tests. Add source parity tests
with each behavior rather than postponing them to final verification.

## Acceptance and verification

- Histogram geometry matches the actual source at narrow/wide spacing and
  fractional independent device ratios, including equal-baseline values,
  negative values, gaps, adjacency correction and per-row color precedence.
- Preparation respects source visible ranges and selected price scales. Empty,
  hidden, whitespace-only, removed and failed preparations clear stale output.
- Every pane uses the same settled time coordinates and plot width. Local axes,
  clipping and heights remain correct with unequal label widths and small bounds.
- Navigation from either plot updates the common time scale. One event has one
  owner; gestures, dividers, leaving the chart and loss of focus follow the source
  contract. Crosshair vertical projection/active-pane price labels match source.
- Append, same-time update, replacement/prepend, clear/reload, series removal,
  option changes and pane lifecycle preserve coherent frames and time alignment.
  Missing volume stays whitespace; demo code does not fabricate zero observations.
- Existing line/candle source comparisons, raster captures and behavioral tests
  remain green. Retained frames remain immutable and layout settlement bounded.
- Measure equivalent single-pane workloads before/after to detect regressions;
  measure two-pane navigation separately with data size, build profile and
  median/p95. Investigate hot-path allocation or duplicated work before caching.
- Record native startup, synthetic input and physical mouse/touchpad acceptance
  separately. Automated tests do not mark the physical checks complete.

At checkpoints run `cargo fmt --check`, relevant tests and
`cargo check --all-targets`. Before completion run `cargo test --all-targets`,
`cargo clippy --all-targets -- -D warnings`, `cargo build --examples`, affected
source-comparison scripts, both supported raster backends and navigation
benchmarks. Record exact commands/results in the porting reports.

Completion requires a runnable price/volume demo, verified source behavior,
preserved single-pane performance and honest recording of any hardware checks
still pending. No completion percentage or delivery estimate is asserted before
the contracts and connected implementation are verified.
