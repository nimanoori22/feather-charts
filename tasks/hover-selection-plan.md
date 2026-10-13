# Implementation roadmap: series and price-line hover selection

Date: 2026-10-13. Status: planned; implementation has not started.
Foundation: `13ac49b` (markers and narrow-window hover-layout fix).
Source revision: `8a7c94ff8ddffde68955db61ef8b58098f239de0`.
Execution checklist: [hover-selection-todo.md](hover-selection-todo.md).
Existing milestone plans and their pending physical checks remain separate.

## Outcome and scope

Identify the source-compatible winning series, price line or marker beneath the
raw pointer, and expose owned identity through public hover/click events. Support
all six built-in families, automatic/custom price lines, independent panes,
source hit tolerances and overlap rules. Add the source's hovered-series-on-top
option without allowing temporary drawing order to change hit selection.

Keep default Time behavior, synchronous preparation and immutable drawing.
Arbitrary custom series/primitives, a plugin registry, dragging/editing annotations,
trading actions, touch/kinetic input, conflation and last-price animation remain
separate. Include existing built-in horizontal base lines when their source
visibility/selection contract applies; do not invent new visible decorations.

## Preliminary findings

These are planning findings, not the completed component/consumer audit required
before production edits.

- Line, Area and Baseline share `line-hit-test-pane-view-base.ts`. Their primary
  hit geometry is a line/point path, not the whole filled region. Preserve simple,
  stepped and curved behavior, point radii and hidden-line source semantics.
  Source curves use twelve approximation steps with the existing control points.
- Bar and Candlestick share `range-hit-test.ts` through `bars-pane-view-base.ts`:
  high/low intervals and time-slot boundaries, not exact painted body/wick pixels.
  Histogram uses the interval between value and its projected base. Adjacent
  logical indexes and gaps affect slot boundaries; nearest-x alone is insufficient.
- `isBetterHit` gives point candidates a special override. Otherwise distance wins;
  equal-distance ties retain stable source/view order. Sorting by enum value would
  change the source behavior.
- `pane-hit-test.ts` also applies primitive-layer and foreground/background rules.
  A single global candidate sort cannot reproduce the complete selection process.
  Marker drawing layer remains distinct from the marker plugin's reported hit layer.
- Horizontal price-line hits use line width plus the source's seven-pixel tolerance;
  they do not use the main series' configurable hit-test tolerance. Custom lines
  expose optional external IDs, which must remain distinct from Rust PriceLineId.
- Hovered rendering uses stable selection order and temporary render order. Area
  and Baseline split normal fill passes from raised outline/point passes; their
  price lines move with the raised pass while normal marker attachments stay in
  the source-defined pass. Fill-only cases need dedicated source fixtures.

Rust already contains `InternalHitTestCandidate`, `HitTestPriority`, `is_better_hit`,
`SeriesOptionsCommon::hit_test_tolerance`, pane-view trait hooks and
`HoveredSourcePaneViews`. Their presence does not imply active built-in selection.
Reuse them where the traced contract fits. Existing Chart events contain exact-index
series rows and marker identity; `PlotSnapshot` has series IDs, grouped geometry
and price-line owner metadata, but line owner metadata alone cannot identify a
particular custom level.

All result pages of the planning symbol queries were read (77 source matches
and 23 Rust matches). The two-way `hitTestPane` trace found pane-widget and
ChartModel pointer consumers and the priority/view/primitive helpers. Coverage
checks found no recorded gaps in the named helper/view paths; `model/series.ts`
was metadata-changed and its relevant view grouping was read directly. Graph
coverage remains best-effort; complete dependency and test reads are task 0.

## Source investigation and design gate

Follow all repository AGENTS porting phases before production edits. Read the
complete components and semantic imports, trace real consumers with graph coverage
and source fallback, separate required behavior from incidental TS structure,
inspect Rust equivalents, consult exact Iced source for input/rendering, compare
viable designs, describe ownership, show signatures and map them back to consumers.
Record the result in `docs/porting/hover-selection.md`.

| Source group | Starting paths in lightweight-charts/src | Contract to establish |
| --- | --- | --- |
| Geometry | `renderers/{hit-test-common,line-hit-test,range-hit-test,walk-line}.ts` | Bounds, distances, degenerate points, curve approximation, visible ranges and logical gaps |
| Series views | `model/series/{series-pane-view-base,line-hit-test-pane-view-base,line-pane-view,area-pane-view,baseline-pane-view,bars-pane-view-base,histogram-pane-view,fill-series-hovered-pane-view}.ts` | Projection, options, invalidation, primary hits and split drawing |
| Selection | `model/{internal-hit-test,pane-hit-test,hovered-source-order,pane,series,chart-model}.ts` | Stable ordering, primitive precedence, hover changes and temporary rendering order |
| Price lines | `renderers/horizontal-line-renderer.ts`, `views/pane/{series-horizontal-line-pane-view,series-price-line-pane-view,series-horizontal-base-line-pane-view,custom-price-line-pane-view}.ts`, `model/{custom-price-line,price-line-options}.ts` | Visibility, hit width, built-in versus custom identity and option lifecycle |
| Events/drawing | `gui/{pane-widget,chart-widget}.ts`, `api/{chart-api,ichart-api,series-api}.ts`, `views/pane/hovered-source-pane-views.ts` | Raw versus magnet coordinates, owned event conversion, cursor and drawing passes |
| Markers | Existing marker plugin, primitive wrappers and series attachment consumers | Integrate current marker selection without changing plugin hit semantics |

Inspect source unit/e2e cases for hovered-series line/overlap, marker versus price
line overlap, price-line hit/miss, hit-testing coverage, and hovered Area/Baseline/
fill-only/point-marker/price-line graphics. Trace relevant consumers before choosing
a Rust design; implementation must not infer behavior from file names alone.

## Architecture and public API direction

Compare at least these two viable designs in task 0:

1. Store typed hit geometry and stable source/view identity in owned prepared pane
   frames. Resolve selection from that settled frame; derive temporary render passes
   from the winning series without changing the selection metadata.
2. Resolve hits directly through synchronous model pane views, sharing their
   projected caches with the frame pipeline. This avoids some copied metadata but
   needs a proven guarantee that hits and displayed geometry share the same layout.

Prefer the first design if it fits existing snapshots without excessive copies.
Avoid independent reprojection on every pointer event and backend Canvas-path
inspection as the behavioral contract. Source mathematical hit geometry can differ
from rasterized pixels. Extract focused modules instead of growing the already
large `src/ui/line_chart.rs` or introducing a parallel frame owner.

Provisional API, finalized only after the consumer audit:

```rust
pub struct ChartHit {
    pub series: SeriesId,
    pub target: HitTarget,
}
pub enum HitTarget {
    SeriesPoint,
    SeriesLine,
    SeriesRange,
    PriceLine(PriceLineHit),
    Marker(MarkerHit),
}
// PriceLineHit distinguishes Custom(PriceLineId) from built-in horizontal lines.
// PointerEvent gains an optional owned winning hit.
// Chart offers a settled-frame hit lookup and the hovered-series-on-top option.
```

Do not promise a bar/item index where the actual source helper returns only a
series and semantic hit kind. Keep event series_data's existing exact-index rules.
External strings remain optional and may be duplicated; internal handles stay
unique and never become array indexes. If price-line external IDs are needed,
extend existing options/validation transactionally, without changing handle identity.

Resolve compatibility explicitly: existing `PointerEvent::marker` should agree
with the winning marker when the unified event reports a marker. Keep
`Chart::marker_hit` as a documented marker-specific lookup, or document a justified
migration before changing it. Do not silently return contradictory identities.
Prefer additive event fields over discarding the existing marker API.

```text
Chart -> model -> owned series, price lines and marker attachments
Preparation -> settled frame -> owned hit geometry + stable source/view order
Pointer update -> owned winning hit -> event + temporary drawing selection
Canvas -> borrows immutable frame -> publishes existing input messages
```

No borrowed model reference escapes in events. IDs are validated against their
owner; series/pane/line/attachment removal invalidates subsequent hits. Retained
old frames remain immutable snapshots. No new Rc cycle, lock, Tokio task or async
channel is needed. Rendering must not mutate selected state or reorder the model.

## Delivery sequence and dependencies

0. Complete source/consumer contract, alternatives, ownership and API design.
1. Deliver a minimal Line hit through settled frames and public events.
2. Extend the same path to Area/Baseline, OHLC ranges and Histogram ranges.
3. Add price-line identity metadata and public selection.
4. Integrate complete source overlap arbitration and hover lifecycle.
5. Add hovered rendering, including Area/Baseline split passes.
6. Demonstrate and verify the full interaction, then record physical acceptance.

The checklist subdivides each phase into small buildable slices. Helpers remain
private until a complete public path supports them. During intermediate slices,
state the supported families/targets rather than exposing inactive API branches.
Checkpoints verify progress without inserting permission pauses into authorized
implementation. Save logical local commits; pushing requires the user's request.
Execute sequentially: these tasks share event identity, frame geometry and ordering.

## Acceptance and regression matrix

| Area | Required cases |
| --- | --- |
| Line families | Simple/step/curve, endpoints, degenerate/single row, point priority/radius, lineVisible options, empty/whitespace and clipped output |
| Range families | OHLC high/low, inverted ranges, Histogram positive/negative/base, adjacent time slots and logical gaps, tolerance boundaries |
| Price lines | Automatic/custom/base source rules, visible versus hidden line, label-only, widths/styles, duplicate/missing IDs, replace/remove/clear/reload |
| Overlaps | Distance, point override, equal ties, reverse stable source order, same-series view order, marker foreground precedence, reported hit layer versus draw layer |
| Hovered rendering | Enabled/disabled/default option, all six families, fill/outline split, price lines and marker layers, non-sticky crossover, immutable retained frames |
| Lifecycle/input | Raw versus magnet pointer, click versus drag, axes, pane transitions/exit, stationary mutations, hide/removal, append/history, zoom/pan/scale/resize/DPI |
| Host layout | Narrow-window status changes must retain chart bounds; keep the existing regression green |
| Performance | Nearby/visible candidate search, no full-history projection or text shaping for hover, bounded reconciliation with no redraw loop |

Use actual-source differential fixtures for geometry and arbitration; do not
replace them with self-consistency tests. Freeze coordinate/radius boundary cases
at ratios 1/1.25/1.5/2. Selection uses logical coordinates and must not acquire a
DPI-dependent tolerance unless the source does so.

## Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Raster hit shapes replace source mathematical geometry | Differential Line/range fixtures include hidden-line, curve and time-gap cases |
| A raised series keeps winning overlaps | Separate stable selection order from temporary render passes; replay the source crossover sequence |
| Filled series cover later strokes or blend twice | Port split-view grouping; capture translucent and fill-only Area/Baseline cases |
| Custom price-line identity is lost | Carry typed internal identity through preparation; test absent/duplicate external IDs and mutation |
| Stationary pointer retains removed targets | Reconcile revision changes and assert redraw settles after emitting the changed hit |
| Hover preparation scans all data or reshapes text | Preserve x-bound search/prepared caches and measure against the existing revision |

## Verification and completion

At every slice run focused behavioral tests and the relevant source fixture.
At final verification run:

```sh
cargo fmt --check
cargo test --all-targets
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --examples
git diff --check
```

Run all existing source comparison scripts plus the new hover-selection oracle.
Capture mixed-family overlaps, marker/price-line conflicts, moved hover selection,
Area/Baseline fill-only and split-pass scenes on TinySkia/WGPU at four ratios.
Exercise actual native widget input and a bounded native startup, not just manually
constructed pointer payloads. Keep the narrow-window regression in the host test.

Compare identical preparation/hover/zoom/append workloads against `13ac49b`, with
empty scenes, many visible overlapping series, and large mostly-offscreen data.
Record median/p95, candidate work and text-shaping/cache behavior. Source helpers
use x-bound searches; avoid an unconditional full-history scan. Reuse baseline
fixtures where possible and isolate build artifacts to avoid benchmark contamination.
No claim of 60 fps follows solely from debug timing or successful captures.

Finish README, the port report and this checklist with source contracts, exact
files, ownership/design alternatives, tests/results, supported limits and remaining
dependencies. No skipped assertions, weakened gates or claims of full plugin parity.

Fresh physical mouse/touchpad acceptance checks overlap switching, line/marker/
price-line hover, clicks versus drag, zoom/pan/axis scaling, narrow-window resize,
live updates, removal and crowded panes. Record results separately. Existing marker
and earlier built-in-series hardware gates remain pending until independently tested.
