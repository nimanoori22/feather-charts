# Implementation roadmap: Area, Bar and Baseline

Date: 2026-10-12. Status: in progress; Area delivered, Bar next.
Execution checklist: [built-in-series-todo.md](built-in-series-todo.md).
Foundation: [public Chart API](../docs/porting/public-chart-api.md) and
[price lines/labels](../docs/porting/price-lines.md), delivered through `706a695`.
Existing roadmaps and their pending hardware checks are preserved.

## Outcome and scope

Make Area, Bar and Baseline usable through the existing public Chart component,
in that order. Each series must support data replacement, append/latest/historical
updates, whitespace, options replacement, visibility, removal, shared/independent
price scales, multiple panes, navigation, crosshair/magnet and price lines/labels.
Finish and verify each series before exposing the next through `add_series`.

Area includes its outline, gradient fill, relative gradient and inverted fill.
Bar includes high/low stems, open/close ticks, open visibility and thin-bar options.
Baseline includes a projected configurable base value and distinct gradients and
line colors above/below it. Preserve source per-row overrides and supported
simple/stepped/curved line behavior and point/crosshair markers for line-like types.

Last-price animation, general hover/hit testing and hovered-series-on-top behavior,
series-marker plugins, custom-series rendering, new gestures and trading actions
remain separate milestones. Document these limits even where options exist.
Do not add async machinery, a renderer registry or a second text/cache pipeline.

## Inspected foundation

- `SeriesOptionsMap` and built-in data types already represent all three families.
  Extend their current consumers rather than creating parallel public types.
- `Chart::add_series` currently accepts Line, Candlestick and Histogram; its
  `SeriesPaneView` selection has the same three branches. Add new branches only
  after their complete preparation/rendering paths exist.
- Public data validation already delegates row creation and validates ordering
  and finite bounded values. Audit new family-specific options and row overrides
  before mutation; retain existing error/non-destructive replacement conventions.
- `walk_line` currently emits solid-colored strokes with source-compatible
  interpolation and dash-phase handling. It is a reuse candidate, not an existing
  gradient-fill implementation. Fill styles can change independently of strokes.
- The current last-value color selection handles the three rendered families.
  New renderers alone will not give new families correct axis labels: audit all
  series-type matches in last values, crosshair markers, magnet and event data.
- Source Area and Baseline share `PaneRendererAreaBase`, which closes paths to a
  projected base or pane edge and uses the source line walker for style segments.
  Baseline stroke color uses a gradient with a discontinuity at the base; assigning
  an entire segment the endpoint color would lose behavior at crossings.
- Source Bar has its own width/parity/tick rules. Reuse OHLC projection from the
  candlestick path where appropriate, but preserve Bar's distinct geometry.

These are planning findings from graph discovery/coverage and direct source reads,
not a completed consumer audit. The class-level Area-base inbound graph trace
returned no callers; direct source establishes its Area/Baseline subclasses.
Full component and consumer investigation is required before implementation.

## Source investigation and design gates

For each family, read complete components plus semantic dependencies, trace real
API/model/view/renderer consumers with codebase-memory, check coverage and inspect
original tests. Separate required behavior from incidental TS sharing/inheritance.
Record ownership, at least two viable Rust designs, important API signatures and
consumer mappings before production edits, following the repository AGENTS workflow.

Starting map, relative to `../lightweight-charts/src/`:

| Slice | Sources and questions |
| --- | --- |
| Area | `model/series/area-series.ts`, `area-pane-view.ts`, `renderers/area-renderer.ts`, `area-renderer-base.ts`: fill closure, visible range, inversion, relative gradient, outline/markers |
| Bar | `model/series/bars-pane-view.ts`, `bars-pane-view-base.ts`, `renderers/bars-renderer.ts`, `optimal-bar-width.ts`: OHLC projection, colors, parity, compression and tick visibility |
| Baseline | `model/series/baseline-series.ts`, `baseline-pane-view.ts`, `renderers/baseline-renderer-area.ts`, `baseline-renderer-line.ts`: base projection, crossings, relative gradient, shared fill behavior |
| Shared styles/path | `renderers/gradient-style-cache.ts`, `walk-line.ts`, line-renderer base, `model/series-bar-colorer.ts`: gradient stops, independent fill/stroke row styles, stepped boundaries, curves, dash phase |
| Engine/public behavior | `model/series.ts`, data/plot-row creation, series options/defaults, public series API, magnet/crosshair views: autoscale/base participation, last-value colors, validation, lifecycle |

Follow discovered imports and consumers; inspect exact local Iced Canvas path,
gradient fill/stroke and clipping APIs before choosing bitmap/logical conversion.
Use Flowsurface only as an implementation example when relevant. Pin source revision
in each port report; local TypeScript is the behavioral reference, not newer docs.

## Architecture and ownership

Preserve the existing synchronous model and owned-frame pipeline:

```text
Application -> Chart -> data/model/series and typed pane views
Preparation -> projected items -> owned fill/stroke/rectangle commands
Frame owner -> settled pane/axis commands -> Iced geometry
Canvas -> immutable frame borrowing
```

No new owner back-reference, callbacks, locks or Rc cycle is required. Model effects
invalidate preparation; the draw path must not project data, shape text or mutate
series state. Each series draws once in source order; fill then outline/markers,
with price decorations and crosshair retaining their established layers.

Compare these viable fill designs during the first design gate:

1. A small owned fill-path/gradient command alongside existing stroke/rectangle
   commands, preserving model/UI separation and straightforward oracle testing.
2. Family-specific prepared Iced geometry, reducing adapter work but coupling
   preparation to Iced and making shared path/source comparisons harder.

Prefer design 1 if the actual Iced gradient APIs support the required source
semantics. Reuse concrete shared fill/path code where Area and Baseline need it;
avoid a generic plugin framework. The existing walker is specialized to one stroke
color: compare a narrow shared path/style traversal with separate fill traversal,
and choose based on source segment semantics and measured cost. Do not mechanically
close existing stroke segments when fill styles differ from line styles.

Public API continues to use `add_series(SeriesOptionsMap::Area/Bar/Baseline(..),
pane, position)`, existing data items and existing lifecycle/options methods.
Derive internal prepared-view signatures after consumer analysis. Validate new
color/marker/base options before mutation and prove invalid replacements preserve
working data/options. No new public wrapper or patch type is planned.

## Delivery order and dependencies

1. Establish the Area contract and resolve fill/gradient representation early.
2. Implement/test Area render commands, model preparation, frame adaptation and
   public integration in buildable slices; then verify its complete behavior.
3. Establish Bar's contract, implement distinct geometry and OHLC preparation,
   integrate the API and verify the complete path.
4. Establish Baseline's contract using the proven Area infrastructure; implement
   gradient discontinuities/base closure, preparation and complete integration.
5. Deliver a public demonstration of all six built-in series and consolidated
   source, raster, performance, lifecycle and hardware evidence.

Bar's geometry is independent of Area fills, but delivery remains sequential to
keep shared API/frame edits small. Baseline depends on Area's proven fill path.
Each task has dependencies, acceptance and verification in the checklist. Keep
production changes near five files or fewer per slice; include necessary exports
and fixtures, and subdivide an integration task if actual scope exceeds that.
Checkpoints verify progress; they do not add approval pauses to authorized work.
Use local logical commits during implementation. Pushing is a separate user action.

## Acceptance matrix and risks

| Area | Required evidence |
| --- | --- |
| Family styles | Area inversion/relative gradient/outline visibility; Bar open/thin/compressed ticks; Baseline above/below/equal/offscreen base, relative gradient and crossings |
| Paths/styles | Single point, empty/whitespace, row overrides, simple/steps/curves, independent fill/stroke changes, alpha and dash continuity |
| Projection | Normal/log/percentage/indexed modes, inverted/manual scales, negative and zero data where valid, first-visible base and source autoscale behavior |
| Shared engine | Latest/historical updates, clear/reload, hidden/removal, visible/global last values, custom levels, crosshair/magnet and exact event rows |
| Layout/drawing | Mixed types on a pane, independent panes/scales, both axes, narrow bounds, fractional ratios, bounded settlement, immutable/repeatable retained frames |
| Performance | Same data and navigation/update fixtures before/after; median/p95 preparation and glyph work, with no concurrent builds |

Highest risks are gradient stop discontinuities on both backends, coordinates
mixing bitmap/logical units, style-boundary fill gaps, cubic/stepped baseline
crossings, Bar width parity under compression and unhandled series-type branches.
Resolve gradient support early, compare actual source commands, capture boundary
cases and search all relevant type matches before exposing each family.
Baseline autoscale participation must be traced/tested; do not assume it matches
custom price lines. Whitespace behavior must follow source rather than an invented
rule about breaking or connecting paths. Record unsupported source options honestly.

## Verification and completion

At each family checkpoint: focused behavioral tests/source comparisons,
`cargo fmt --check`, `cargo check --all-targets`, bounded native startup and
TinySkia/WGPU captures at ratios 1, 1.25, 1.5 and 2. Rerun existing affected oracles;
preserve prior Line/Candlestick/Histogram and navigation assertions.

Before final delivery:

```sh
cargo fmt --check
cargo test --all-targets
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --examples
git diff --check
```

New source oracles must execute actual TypeScript renderers/path/gradient methods
with equivalent normalized input. Do not require browser/Iced font pixels to match.
Raster captures verify colors, gradients, clipping, layers and deterministic
drawing on both backends. Investigate measured regressions before completion.

Each family report records exact source revision/components, consumer contract,
discarded TS conventions, alternatives, ownership/API, exact changed files,
tests/checks, results and remaining dependencies. README describes supported
series and remaining limits. Keep automated/native and physical acceptance separate;
do not mark hardware boxes complete from synthetic events or old acceptance.

No calendar estimate or completion percentage is assigned before investigation.
