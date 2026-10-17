# Conflation rendering implementation

Status: implementation in progress. Plan: `tasks/conflation-plan.md`.
The historical timing exception remains open and deferred.

## Source component and consumers

The complete `model/data-conflater.ts` defines recursive pair aggregation and
remainder folding, derived-row counts/identity, custom reducer contexts and
last-chunk updates. Constants define powers 2..512. Source hashes, including
current local edits to Series/DataLayer/TimeScale, are frozen in
[conflation-source.json](conflation-source.json).

Direct current-source reads supplement graph traces: generic/interface calls
from LinePaneViewBase, BarsPaneViewBase and CustomPaneView to conflatedBars are
missing graph edges. These three consumers cover the six built-ins and custom
rendering; their shared base selects visible derived rows and projects coordinates.
Color lookup still reads the canonical row at an aggregate's initial index.
Autoscale, first/last prices, magnet and API event data read canonical rows.
Series owns per-factor caches; DataLayer invokes latest repair and prohibits
historical changes when the effective per-series factor exceeds one. Custom
series without a reducer draw raw rows, even though the time-scale factor is
still supplied to their renderer. Optional cache warmup uses browser task APIs.
The upstream integration tests mock active-state checks, so actual Series source
determines the option-enabled/factor-1 boundary.

## Behavioral contract

Preserve threshold precedence (series/global/1), DPR-aware floored power-of-two
selection, 512 cap, recursive pair order and folded odd remainders. Canonical
rows, time indexing, source colors and exact-index events stay unchanged.
Line-family aggregates select the extreme farther from close, low on a tie;
OHLC families project first open/high/low/final close. Original counts accumulate.
The aggregate retains its first source index; grouping is independent of viewport.
Custom contexts are typed and callbacks cannot be invoked under a model borrow.
All prohibited public updates must fail before mutation. Retained frame geometry
must remain immutable after cache changes, source replacement and removal.

## Rust design, alternatives and ownership

Two designs were considered: caches in each pane view (easy initial projection
integration but duplicate state and separate low-level paths), or in Series
(one owner/invalidation path shared by public and low-level consumers). Choose
Series-owned derived summaries, revising the plan's initial preference after
reading actual `ChartModel::prepare_*` consumers. Memoization uses RefCell like
existing series read caches; it is synchronous, not shared across threads.

```text
Chart -> model -> existing Rc<RefCell<Series>> -> canonical rows
                                           -> owned conflation summaries
      -> prepared frame -> owned projected geometry
Chart -> custom adapter -> typed cached payloads + existing owned view
Retained custom handle -> model, weak view; no new owner cycles
```

Planned private API: `factor(spacing, dpr, threshold) -> usize`,
`ConflationCache::ensure(rows, level)` and bounded `step`, and
`Series::render_rows(time)` yielding a borrowed raw/cached drawing selection.
Summaries retain source positions, logical indices, counts and OHLC; original
rows supply styles. No new public data handles, callback identity APIs, WeakMap
hashes, unsafe TS casts or browser scheduling objects are reproduced. Custom
adapters own typed caches and release canonical model borrows before reducers.
Cache removal follows owner Drop; no strong closure back-references are added.

Validation/results and exact delivered scope will be recorded as slices pass.

## First slice validation

The actual-TS oracle passes 140 aggregation/count/index/remainder cases. Core
tests cover bounded construction, warm reuse and latest repair versus fresh
results for lengths 1..49 and levels 2..512. Dense public Line retains 8,192
canonical rows while reducing projected points; historical-update rejection is
transactional and depends on the effective factor. 400 library tests and the
two public conflation tests pass for this slice. Other series/custom/warmup and
full raster/performance/physical verification remain pending.
