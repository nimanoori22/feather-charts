# Implementation roadmap: conflation rendering

Date: 2026-10-17. Status: planned; no implementation delivered by this document.
Starting checkpoint: `92d386d`. Execution: [conflation-todo.md](conflation-todo.md).
This expands [conflation-next.md](conflation-next.md). Timing investigation stays
deferred; historical acceptance failures, device gaps and budgets remain unchanged.

## Outcome and scope

Render dense data using the local Lightweight Charts 5.2.1 conflation contract:
fewer derived bars at small horizontal spacing, without replacing canonical data.
Support all six built-in series and typed custom series, global/per-series
thresholds, cache invalidation, live updates and optional precomputation. Deliver
Line first as a complete public-chart slice, then the other families.

Conflation is a drawing representation, not a destructive downsample, an arbitrary
pixel-bucket algorithm or a fix for the unresolved historical timing exception.
Do not add other missing plugins, chart types, public pane-management features,
Tokio, new thread-safe engine infrastructure or speculative rendering caches.

## Starting evidence and source contract

Planning inspected the following local source paths. Complete consumer/lifecycle
verification remains task 0 before production edits, following AGENTS.md.

| Source | Required investigation/behavior |
| --- | --- |
| `model/data-conflater.ts`, `model/conflation/constants.ts` | Pairwise recursive aggregation, power-of-two levels 2..512, factor-1 passthrough, original counts, remainder folding, custom reduction and last-chunk repair |
| `model/series.ts` | Render rows versus canonical bars; per-series threshold overrides global threshold; effective active state; invalidation and warmup |
| `model/time-scale.ts` | DPR-aware thresholds, spacing/option changes and possible levels |
| `model/data-layer.ts` | Historical-update rejection while the effective factor is >1; latest-bar update hook before canonical replacement |
| `model/series/{line-pane-view-base,bars-pane-view-base,custom-pane-view,series-pane-view-base}.ts` | Line-family representative values, OHLC projection, style lookup, visibility, typed custom renderer data and level changes |
| `tests/unittests/conflation{,-integration}.spec.ts` | Boundary, remainder, recursive, custom-reducer and streaming parity intent |
| `tests/e2e/{graphics,coverage}/test-cases/` conflation cases | Public configuration and representative dense/custom rendering fixtures |

Important details already observed:

- The effective threshold factor is series override, then global override, then
  1. Factor selection uses spacing and DPR, floors to a power of two and caps at
  512. Enabling the option can still yield factor 1; test both sides of every
  boundary, including DPR changes and per-series overrides.
- Groups are derived from the series' ordered fulfilled rows, not from whichever
  visible window happens to be selected. Verify index gaps/whitespace against the
  source; do not invent timestamp buckets or regroup on every pan.
- Built-in aggregates retain first open, maximum high, minimum low, final close,
  first logical index and summed original count. Odd remainders fold into the
  preceding group; seven rows at factor 2 produce counts 2,2,3.
- For Line/Area/Baseline/Histogram, merged drawing values use the high or low
  farther from the close, choosing low on a tie. OHLC families draw aggregated
  OHLC values. Row style comes through source color lookup; it is not inferred
  merely from the final close or copied blindly from an aggregate.
- Actual `Series.isConflationEnabled()` tests effective factor >1. The upstream
  integration tests mock that method, so their labels alone do not establish
  option-enabled behavior. Reject prohibited historical updates before mutation;
  allow normal historical updates when the effective factor is 1.
- Custom series without a reducer retain raw drawing data. Custom reducers are
  pairwise and potentially non-associative: preserve invocation order, context,
  original counts and price-value-builder semantics, including singleton cases.

Rust already has time-scale factor calculation, threshold/precompute options,
`PlotRow::original_data_count`, custom contexts and `CustomSeriesPaneView::conflate`.
Built-in preparers still return `UnsupportedConflation`; the public custom adapter
uses raw visible rows and supplies factor 1. Extend these paths instead of creating
a parallel public chart API. Existing rejection tests must gain positive behavior
coverage as each supported path becomes available; retain unrelated error checks.

Graph verification is best effort. The graph misses incoming `conflatedBars`
calls found directly in the three pane-view bases. Coverage flagged changed
upstream Series/TimeScale/DataLayer files and an excluded custom coverage test;
direct current-source reads informed this outline. Task 0 must freeze source
revision plus working-tree file hashes and finish full reads/traces, rather than
relying on stale graph snippets or assuming a clean upstream checkout.

## Rust design and ownership

Compare two viable designs at task 0, then record final signatures and call-site
mapping before implementation:

| Design | Benefit | Cost |
| --- | --- | --- |
| Series-owned aggregate cache, analogous to upstream | One shared cache close to canonical mutation and all model consumers | Requires careful mutable/cache access through existing series handles; must avoid holding a borrow across custom callbacks |
| Pane-view-owned derived cache using source generations | Fits current owned pane views and immutable model reads; no new shared ownership | Must wire generations and low-level stateless preparation consistently; duplicate views can duplicate derived storage |

Prefer the pane-view-owned cache for the first built-in slice: current Rust
preparers read `&Series`, and the new state is derived drawing data. Use the same
pure conflation core for low-level model preparation and the cached public path;
do not implement two algorithms. Finalize this choice only after task 0 confirms
all actual consumers; document any evidence-driven change.

```text
Chart -> model -> existing series handle -> canonical rows and mutation generation
      -> series pane view -> owned derived rows/cache keyed by generation + level
      -> prepared frame -> owned projected geometry (retained snapshots stay valid)
Typed custom adapter -> typed reducer/view and typed derived payload cache
```

The cache stores source logical identity/counts and the aggregate values required
for rendering. Canonical time/style/data queries still use the source. Reuse the
existing pane-data generation only if tests show it covers every relevant mutation;
otherwise add a focused data revision. Changes in effective level, source identity,
data, reducer/options and relevant style must not reuse stale results.

Keep new core/cache types private or crate-private. Reuse PlotRow, PlotRowValue,
TimePointIndex and existing option/trait types. Factor selection and aggregation
should be testable without Iced. Do not retain renderer references to mutable
cache storage or hold a model RefCell borrow while invoking application reducers.
Removal drops owned caches; no new strong owner cycles or background engine locks.

## Ordered delivery

```text
0 contract -> 1 source oracle -> 2 shared core -> 3 Line end-to-end
                                             -> 4 live updates/cache invalidation
                                             -> checkpoint: usable dense Line
4 -> 5 Candlestick/Bar -> 6 Histogram -> 7 Area -> 8 Baseline
4 -> 9 typed custom series
5..9 -> 10 interaction/retained-frame integration -> 11 optional precomputation
     -> 12 bounded performance verification -> 13 final validation/report
```

Keep execution serial where files or mutation contracts overlap. Each numbered
task has a focused checklist, likely files, dependencies and verification in
[conflation-todo.md](conflation-todo.md). Split a task further if it grows beyond
roughly five implementation/test files. Every slice leaves nonconflated charts
working; remove a family's rejection guard only when its complete path is tested.

Task 4 covers latest replacement, append, chunk-boundary/remainder repair, full
replacement/clear, shared time reindexing, toggles and series removal. Cached
results must equal a fresh rebuild after every operation. Repair only the affected
tail where that equivalence is proven; invalidate other levels deliberately.
Do not claim all appends are O(1), reproduce upstream's weak array hash, or replay
the full history on every unchanged warm frame. Unchanged peer series remain intact.

Task 10 traces raw-data consumers independently of drawing: autoscale/first/last
values, crosshair magnet and payloads, hits/hover order, markers, price lines,
primitives and editing. Compare their actual source behavior with aggregated
geometry. Do not return an aggregate in place of an exact-index raw event merely
because it was drawn. Test both public Chart and existing low-level entry points.

Task 11 treats browser scheduling as an adapter concern. Upstream uses scheduler
tasks or microtasks for optional warmup. Choose a bounded cooperative host step
using existing application scheduling after inspecting the exact Iced source and
Flowsurface examples. The engine remains synchronous. Cancel obsolete work by
source generation, bound work/storage, and exercise removal/replacement mid-warmup.
Document the mapping of requested priorities. Do not silently ignore represented
precompute options or block initial interaction with an unbounded eager build.

## Verification and completion criteria

- A source-derived oracle imports/executes actual local TS components; do not use
  a second handwritten aggregation algorithm as the specification. Cover all
  levels, thresholds, odd counts, gaps, original counts/styles, custom reduction
  order, mutation sequences and active-factor historical errors.
- At factor 1/disabled, existing behavior, pixels and error handling remain intact.
  At factor >1, compare against upstream conflated output, not against an
  unconflated image that intentionally has different geometry. Exercise simple,
  stepped and curved lines, OHLC colors/wicks, filled gradients and custom views.
- Test regular, narrow and degenerate/short sizes; ratios 1,1.25,1.5,2; manual and
  auto price scales, inverted/log/percentage/indexed modes; mixed series/panes,
  zoom across level boundaries, live feed and immutable retained frames.
- Count work before timing. An 8,192-row exact-multiple factor-8 core fixture must
  yield 1,024 aggregates. Projection counts must match the source-selected visible
  aggregates plus required join/control neighbors. A warm unchanged frame must
  not rebuild history. Cache storage must remain bounded by source size/levels.
- Run one frozen finite timing batch after correctness passes: Line and Histogram,
  8,192/100,000 rows, matched dense viewport/DPR, three alternating enabled/disabled
  pairs (24 executions). Record effective levels, actual work, cold build, warm
  preparation, latest/append cost and memory. Keep all failures and controls;
  freeze exact commands/sample counts before timing and run serially with no
  concurrent builds/tests. No extra repetitions to obtain a pass.
- Exact work reduction and source parity establish the feature. Timing inside
  run-to-run variation is inconclusive and cannot support a speed claim. Investigate
  a repeatable disabled-path regression; do not weaken existing budgets or reopen
  the old exception merely because new diagnostic numbers vary.
- Run `cargo fmt --check`, focused tests, `cargo test --all-targets`,
  `cargo check --all-targets` and `cargo clippy --all-targets -- -D warnings`.
  Run source oracles, Python analysis tests if touched, and TinySkia/WGPU captures.
  Review the final changes before marking implementation complete.
- Repeat physical mouse zoom/pan/price-axis gestures during live feed across level
  boundaries on both backends at regular/narrow sizes. Record unavailable touchpad,
  touchscreen and high-DPI hardware as pending; never infer a pass from automation.

Publish `docs/porting/conflation-rendering.md` with investigated source/consumers,
required versus discarded TS mechanisms, chosen Rust API/ownership/alternatives,
changed files, behavioral/reference/raster results, work/timing evidence, supported
limits and remaining dependencies. Label implementation, automated verification,
physical acceptance and the deferred historical timing exception separately.

## Risks and decisions to resolve early

| Risk | Required handling |
| --- | --- |
| Cache invalidation misses same-length updates or reindexing | Revision-based keys and warm-versus-fresh mutation tests |
| Per-series level differs from the global time-scale level | Use effective per-series factors for rows/invalidation; audit renderer factor payloads |
| Grouping changes on pan or loses whitespace/style identity | Full-series anchor and source-derived index/style/gap fixtures |
| Custom reducer reentrancy or non-associativity | Release borrows before callbacks; preserve reduction order and typed payload lifetime |
| Optimized latest repair differs from recursive rebuild | Oracle comparisons; keep correct rebuild fallback at proven structural boundaries |
| Cold cache/warmup stalls interaction or grows without bound | Separate cold/warm metrics, bounded jobs and generation cancellation |
| Source fixtures mock away real behavior or source quirks | Verify actual call paths, pin current source hashes, document discrepancies before coding |

No user preference is needed to start task 0. Any unresolved API/ownership/scheduling
choice above is an implementation investigation gate, not a claim already verified.
