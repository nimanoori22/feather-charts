# Implementation roadmap: interactive candlestick charts

Date: 2026-10-12. Status: implemented and verified automatically; native startup checked, physical mouse/touchpad acceptance pending.
Execution checklist: [candlestick-todo.md](candlestick-todo.md).
Previous milestone: [mouse navigation and crosshair](plan.md). Its remaining
hardware checks are preserved in [todo.md](todo.md).

## Outcome and scope

Deliver a one-pane candlestick demo using the existing time/price axes, wheel
zoom, horizontal scrolling, crosshair labels and magnet modes. Candle bodies,
wicks, borders, colors and pixel alignment must preserve demonstrated
Lightweight Charts behavior. Existing line charts must remain correct and smooth.

This milestone does not include histogram/volume rendering, multiple panes,
the public chart/widget facade, touch/kinetic gestures, price lines or general
hit testing. Histogram plus a volume pane is the subsequent candidate milestone;
the reusable public API follows once these rendering paths are proven.

## Inspected foundation

- Typed OHLC input, candlestick plot rows and style options already exist.
  Reuse them rather than introducing another data model.
- `ChartModel::prepare_line` and `LinePaneView` demonstrate synchronous
  preparation with owned output and stale-state clearing.
- `ChartFrameOwner::prepare` currently accepts `LinePaneView`; the complete
  frame path needs a deliberate extension for candlesticks.
- Crosshair refresh already considers close values in Magnet mode and OHLC
  values in MagnetOhlc mode. Verify real candle inputs before changing it.
- Keep the existing bounded axis-glyph cache and stationary-pointer suppression.

These findings use codebase-memory searches, snippets, coverage checks and
direct source inspection. Coverage reported no recorded issues for the relied-on
Rust paths; that signal is best-effort. Full TypeScript consumer tracing is Task 0,
not a completed design investigation.

## Source investigation map

Paths below are relative to `../lightweight-charts/src/`.

| Area | Primary source | Contract to establish |
| --- | --- | --- |
| Candle preparation | `model/series/candlesticks-pane-view.ts`, `bars-pane-view-base.ts`, `series-pane-view-base.ts` | Visible selection, OHLC projection, baseline, invalidation and empty output |
| Geometry | `renderers/candlesticks-renderer.ts`, `optimal-bar-width.ts` and their imports | Width rounding, wick/body/border ordering, dense spacing, doji and fractional ratios |
| Colors/options | `model/series-bar-colorer.ts`, `model/series/candlestick-series.ts` | Up/down rule, per-row overrides, option precedence and replacement |
| Crosshair | `model/magnet.ts`, `model/chart-model.ts`, crosshair pane/axis views | Close versus OHLC snapping, tie order, labels, candle marker eligibility |
| Data/lifecycle | Series/data-layer consumers and source tests | Update/replacement, whitespace, removal, autoscale and retained frames |
| UI integration | Local Iced canvas/runtime sources; relevant Flowsurface examples | Event routing, read-only drawing, clipping and device scaling |

For each component, read the complete source and relevant dependencies, trace
actual consumers across the original project, check graph coverage, and inspect
missed source directly. Separate required behavior from incidental TypeScript
inheritance, callbacks, shared references and manual destruction.

Record the contract, consumer groups, alternatives, ownership, important Rust
signatures and consumer mappings in `docs/porting/candlestick-rendering.md`
before modifying engine code. The roadmap specifies outcomes, not a final API.

## Architecture and ownership

Retain backend-neutral logical-coordinate preparation followed by bitmap draw
commands and an Iced adapter. Apply device ratios exactly once. Drawing borrows
owned prepared geometry; it does not mutate the model, measure text or sample
animations.

```text
Application owns DataLayer, ChartModel, animation controller and frame owner
Model preparation borrows candle rows and selected time/price scales
Frame owns candle geometry, axes and crosshair geometry
Canvas borrows frame -> publishes existing typed input messages
Application applies synchronous model operations -> prepares next frame
```

Compare at least two integration designs in Task 0:

1. A small enum for supported line/candle pane views and prepared series output,
   retaining the shared layout/animation/frame owner.
2. Separate typed line/candle preparation entry points with the shared frame
   settlement extracted only where duplication would otherwise occur.

Choose based on actual callers, ownership clarity, dispatch cost, allocation and
API ergonomics. Avoid a generic renderer registry or trait hierarchy unless
current consumers require it. No new asynchronous engine machinery or shared
thread-safe state is justified by this scope. Preserve existing line call sites
where practical; document necessary API changes and update every caller.

## Delivery order

```text
0. Source contract and integration design
 -> 1. Candle geometry and source oracle
 -> 2. Model-owned candle preparation
 -> checkpoint: backend-neutral candles
 -> 3. Iced/frame integration
 -> 4. Crosshair and OHLC interaction verification
 -> checkpoint: interactive candle frame
 -> 5. Runnable candlestick demo
 -> 6. Integrated parity, raster and performance validation
 -> checkpoint: verified candlestick milestone
```

Each task should remain a focused slice, with existing targets building and
tests passing. Split tasks further if implementation exceeds roughly five
production files; exports and fixtures must still be updated. Source parity
tests accompany the behavior they validate rather than being deferred to Task 6.

## Required verification

- Rising/falling/equal open-close candles; body/wick/border visibility; default
  and per-row colors; narrow and wide spacing; shared/overlapping pixel edges.
- Correct source visible-range rules without assuming line neighbor extension.
  Empty, hidden, whitespace-only, disjoint and zero-size states clear output.
- Selected-scale OHLC coordinates and autoscale under normal, logarithmic,
  percentage and indexed modes, including supported scale edge cases.
- Fractional device ratios, clipped drawing, finite geometry, immutable retained
  frames and bounded layout settlement with floating labels.
- Normal, Magnet and MagnetOhlc behavior, source tie ordering, unequal scales,
  hidden/overlay candidates, whitespace and refresh after updates/navigation.
- Append, same-time update, history replacement/prepend, option replacement,
  clear/reload, removal and resize without stale candle/crosshair output.
- Line regression suite remains green. Reuse the zoom benchmark to compare
  equivalent line workloads before/after integration; measure candle workloads
  separately and investigate material regressions rather than adding caches by
  assumption. Record build profile, data size, median/p95 and hardware limits.

At checkpoints run:

```sh
cargo fmt --check
cargo test --all-targets
cargo check --all-targets
cargo clippy --lib --examples -- -D warnings
git diff --check
```

Add a focused candle reference fixture/oracle using the actual original
TypeScript implementation and the existing harness pattern. Compare geometry
before raster checks. Run affected existing reference scripts and capture
representative TinySkia/WGPU scenes; backend antialiasing need not be identical
to Canvas2D. Keep generated captures outside the repository.

Manually exercise native zoom, drag, resize, mode changes and data updates.
Record unavailable touchpad/platform checks explicitly. User confirmation of
smooth line zoom is a baseline, not proof of candle performance.

## Risks and investigation decisions

| Risk | Resolution |
| --- | --- |
| Line-specific frame types encourage duplicated layout logic | Compare the two bounded integration designs before coding |
| Pixel rounding changes candle widths or border/wick placement | Execute source geometry oracle across spacing/ratio boundaries |
| Existing OHLC plumbing hides color or autoscale gaps | Connected tests with actual candle rows/options before UI integration |
| MagnetOhlc exists but candle behavior is unproven | Verify candidates and ties against original consumers |
| New frame dispatch regresses line zoom | Retain glyph cache; run equivalent line benchmark and regressions |
| Unsupported source features appear during tracing | Document them explicitly; implement required candle behavior or return a typed unsupported result where appropriate |

The final report must name exact source/Rust files, consumers, contract,
discarded TypeScript patterns, chosen ownership/API, alternatives, behavioral
tests, validation results and remaining dependencies. No calendar or completion
percentage is assigned before Task 0 resolves the source contract.
