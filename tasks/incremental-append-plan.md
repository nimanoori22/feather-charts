# Implementation roadmap: incremental live-data append

Date: 2026-10-14. Implementation started: 2026-10-15. Status: engine and functional validation complete; performance acceptance open (19/24 screens pass); physical acceptance pending.
Results: [implementation report](../docs/porting/incremental-append.md).
Foundation: `6c743ca` (preparation, typed custom series and touch navigation).
Execution checklist: [incremental-append-todo.md](incremental-append-todo.md).

This follows the repository's separate-milestone convention. Earlier plans and
their open physical/performance gates remain intact.

## Outcome

A normal live update that introduces a time after the shared axis's last time
extends the shared axis and changed series without rebuilding existing history.
Existing-time replacement stays incremental. A newly introduced time inside the
shared range retains the existing full reindex route. The displayed result,
original times, typed payloads, whitespace, base index, viewport policy, markers,
crosshair and retained frames must preserve the upstream contract.

Keep `Chart::update_data` and `Chart::update_custom_data` usable through their
existing public signatures. Keep the engine synchronous and reuse its existing
model, series owners, invalidation and rendering boundaries.

## Evidence and boundaries

The previous controlled append measurements recorded 33–35 ms for one source
and 535–568 ms for 16 sources at 100,000 rows, excluding the separate snapshot
and glyph phases. They compare `36a2955` with `2e62032`, not a newly measured
baseline of this foundation. Establish fresh evidence before optimizing; preserve
the earlier results and their Candlestick update/glyph p95 exceptions in
[preparation-performance.md](../docs/porting/preparation-performance.md).

Graph lookup and direct source reads identify these current costs:

- A new time in DataLayer copies old keys, reindexes all points, creates a
  full weight vector, and creates full responses for built-in/custom owners.
- TimeScale replaces its full point vector; the existing response cannot describe
  an append without transferring that vector.
- Changed fulfilled membership makes the coordinator gather indexes from all
  owners and copy the aggregate set into TimeScale.
- Base-index lookup walks backward through points; long whitespace tails require
  explicit measurement and a bounded append strategy.

`SeriesChanges::changed_index` and the existing typed custom update path already
support a changed-row handoff. Extend these contracts rather than introduce a
second chart engine or an application-only shortcut.

Graph coverage is best-effort: the Rust paths read reported matching metadata;
upstream DataLayer metadata was changed, so its current update method was read
directly. Dynamic custom callbacks and trait calls were not exhaustively resolved
by the graph. Task 0 must complete the full source/consumer investigation.

### Mutation classification

| Operation | Required route |
| --- | --- |
| First point in an empty shared axis | Explicit startup case with correct initial weight/base |
| New time greater than the shared maximum | Tail append candidate |
| A series joins a time another series already introduced | Existing-point delta; no new shared point |
| Latest replacement or historical update at an existing shared time | Existing-point delta, including whitespace membership changes |
| New time inside the shared range | Full reindex fallback |
| Bulk replacement, prepend, clear, removal or pop | Preserve established full/update routes and cleanup |

An append relative to one series can still be a middle insertion globally. For
example, if A ends at time 5 and B at time 2, B adding time 4 is a shared middle
insertion. Classify using the shared maximum, not just `last_time_by_series`.

## Design decisions to settle in Task 0

Compare an explicit time-point change enum (unchanged/append/full replacement)
with an additional append descriptor on the existing response. Prefer an explicit
contract if it removes inconsistent combinations without unnecessary migration
work. Reuse changed-row updates for built-ins and typed custom owners. Trace every
response constructor and consumer before choosing the exact Rust signatures.

For weights, compare a small optional append capability on HorzScaleBehavior,
with a correct default fallback, against a borrowed weighting context over
existing storage. Native Time weighting can use its predecessor for a tail update;
arbitrary behaviors cannot be assumed to accept a rebased two-point slice. Preserve
the first-point and calendar/intraday rules and inspect both implementations.

For fulfilled membership, prefer a point membership delta from authoritative
DataLayer state when sufficient. Compare it with contributor counts only if
actual consumers require them. Avoid maintaining a second full-history membership
cache without evidence. Middle reindex, replacement and removal can rebuild.

DataLayer owns normalized shared membership; series own plotted/typed rows;
TimeScale owns its viewport points; prepared frames own immutable geometry.
An update transfers owned deltas synchronously between these existing owners.
Document ownership, failure behavior, ordering, signatures and alternatives before
production changes. No callback/model borrow may survive that handoff.

## Dependency order

```text
0 source/design -> 1 baseline and failing guards -> 2 append handoff
    -> 3 first built-in append -> 4 typed custom append
    -> 5 membership/base-index deltas -> 6 weights and tick caches
    -> 7 middle-insertion fallback -> 8 chart integration
    -> 9 performance acceptance -> 10 final validation/report
```

### Task 0 — Derive the complete contract

Read complete upstream DataLayer and relevant direct dependencies; trace ChartApi,
Series, TimeScale, custom builders, weighting, whitespace, viewport-shift and
invalidation consumers. Inspect corresponding Rust implementations and real
response construction sites. Follow the AGENTS.md porting workflow, including
two viable designs and ownership analysis.

Acceptance: required behavior and incidental JS response/reference conventions
are separated; classify every operation above; settle the minimal Rust handoff
and generic weighting fallback without promising unproven complexity.
Verification: graph searches/traces/coverage with direct-source fallback and a
consumer-to-API mapping in `docs/porting/incremental-append.md`.
Dependencies: none. Scope: small investigation/documentation slice.

### Task 1 — Establish baseline and failing operation guards

Extend the existing preparation profiler with public built-in and typed-custom
append workloads. Add behavioral/copy-count tests before changing production.
Measure normalization/model, snapshot and glyph phases separately, plus actual
public routing. Keep input generation outside timed intervals.

Acceptance: record fresh release median/p95, noise and budgets at 1,200/20,000/
100,000 rows and 1/16 sources; demonstrate full-history copies/scans with a failing
guard; separately record startup, whitespace tails and allocation growth.
Verification: focused append tests and controlled benchmark logs with exact
commit/artifact identities. Do not run Cargo/captures during acceptance timings.
Likely files: `examples/preparation_profile.rs`, `tests/incremental_append.rs`, report.
Dependencies: 0. Scope: medium; split profiler and test fixtures if necessary.

### Task 2 — Add an explicit append handoff

Introduce the selected response contract and TimeScale/coordinator consumption
while keeping existing producers on their established paths. Adapt all relevant
constructors, fixtures and model validation in compiling increments.

Acceptance: an append extends points without replacing the prefix; existing-row
indexes remain unchanged; malformed/replayed/out-of-order descriptors cannot
partially mutate model state. Define rejection or idempotence from real consumers.
Verification: direct TimeScale/coordinator boundary tests and existing full-update
tests. Preserve data generation, offset correction and required invalidation.
Likely files: DataLayer response types, TimeScale, coordinator, ChartModel and tests.
Dependencies: 0–1. Scope: medium per producer/consumer slice.

### Task 3 — Deliver the first built-in path

Route a Line series's shared tail append through the new handoff. Validate and
construct the new row before changing shared points or per-series last time.
Use existing single-row application and skip unchanged peer-row responses.

Acceptance: public Line append produces correct points/rows/labels at large
history sizes; old indexes and metadata survive; invalid conversion/row input
leaves shared and series state unchanged.
Verification: normal/startup/whitespace/shared-time/invalid-update cases with
operation counts and comparison to the established path/source behavior.
Likely files: `src/model/data_layer.rs`, coordinator, append tests.
Dependencies: 2. Scope: medium.

Checkpoint: the first public append works end to end. Record any remaining
weighting, membership or tick-cache costs; this is not performance acceptance.

### Task 4 — Extend to typed custom and other built-in series

Use the same shared append protocol for all six built-ins and typed custom
owners. Keep custom payloads in their typed stores and send only necessary
changed rows/index information to the coordinator.

Acceptance: mixed owners and two different custom datum types share correct
indexes; builders/metadata/original time survive; stale/foreign handles and invalid
builder results fail transactionally without replacing unrelated rows.
Verification: mixed source ordering, custom/built-in initiators, whitespace,
latest replacement, clear/reload and retained-frame tests.
Likely files: DataLayer, `src/model/series.rs`, ChartModel, public custom tests.
Dependencies: 3. Scope: medium; share the existing engine path.

### Task 5 — Make membership and base-index maintenance incremental

Avoid full fulfilled-index gathering/set copies for an unchanged mapping.
Handle point membership transitions and trailing whitespace without history
walks on ordinary append. Retain full rebuilding for mapping/owner changes.

Acceptance: fulfilled/whitespace transitions keep aggregate membership correct
when multiple sources share a point; base index follows the last fulfilled point;
default and ignore-whitespace behavior agree with the established route.
Verification: append, latest value-to-whitespace replacement, overlapping sources,
long whitespace tails, removal/pop and reload, including operation counters.
Likely files: DataLayer, coordinator, TimeScale, relevant tests.
Dependencies: 4. Scope: medium.

### Task 6 — Bound append weighting and update tick caches correctly

Use the approved Time weighting capability and preserve generic fallback behavior.
Update tick buckets from the append boundary. Measure selection rebuilding before
adding caching: unchanged historical points do not imply cached selected ticks can
remain untouched. Added high-weight ticks and uniform distribution can affect
selection beyond the tail.

Acceptance: time weights and selected labels match upstream event sequences;
eligible native Time appends avoid building a full weight vector; cache invalidation
preserves label priority, alignment and whitespace filtering.
Verification: first/second point, calendar boundaries, sub-second/negative times,
business days, spacing changes, uniform distribution and arbitrary-behavior fallback.
Likely files: HorzScaleBehavior/Time implementation, weight generator, TickMarks,
focused tests; split weighting and selection changes into separate slices.
Dependencies: 5 and design from 0. Scope: medium per slice.

Checkpoint: normal built-in/custom appends use deltas throughout the engine.
Document legitimate global cache work and allocator costs rather than silently
claiming every possible append is worst-case constant time.

### Task 7 — Preserve middle insertion and full-update fallback

Exercise fast/slow transitions repeatedly. Keep shared middle insertion, bulk
replacement, prepend, remove, pop and historical restrictions on correct existing
paths. Ensure incremental auxiliary state is rebuilt whenever mapping changes.

Acceptance: no stale indexes/membership/weights after fallback; per-series append
inside the global range reindexes correctly; subsequent tail append resumes safely.
Verification: interleaved multi-source event sequences and source differential
checks, including whitespace/invalid history and typed owner cleanup.
Likely files: DataLayer/coordinator tests and source reference example/script.
Dependencies: 4–6. Scope: small test/fallback slice.

### Task 8 — Verify chart-level integration during live updates

Validate follow-latest versus fixed-history policy, whitespace-shift options,
autoscale/manual/inverted/relative price modes, price lines, markers, crosshair
and primitives across panes. Preserve native pan/touch/kinetic/edit lifecycle
rules while data arrives. Shared coordinate changes can invalidate peer geometry
even when peer rows are unchanged.

Acceptance: hits, exact-index payloads, labels and geometry agree after each update;
old frames remain drawable; live updates preserve valid gestures and cancel only
as required by source behavior or incompatible ownership/viewport changes.
Verification: deterministic public API/native sequences and live-append captures
on TinySkia/WGPU at ratios 1/1.25/1.5/2, including narrow windows.
Likely files: public integration tests, existing example capture helpers, report.
Dependencies: 7. Scope: medium per API/interaction/capture slice.

### Task 9 — Accept performance against the frozen baseline

Repeat baseline/current measurements with stable visible ranges, dimensions,
ratio, last-price formatting and equivalent update sequences. Include Line,
Candlestick, Histogram and typed custom data, mixed sources, trailing whitespace
and bursts; retain evidence for all six built-ins through operation tests.

Acceptance: no old-row/index-prefix cloning or history walks in the eligible
append update path; fixed-source steady-state update median/p95 grows by at most
10% from 1,200 to 100,000 rows; investigate/report any reproducible regression
over 10% in established replacement, static, navigation and fallback workloads.
Use three isolated alternating baseline/current runs, 20 warmups and 100 samples.
Record per-phase and public-route timings, source-count costs, allocation/capacity
growth, first-append and global cache rebuilds separately. Correctness/copy-count
guards run in ordinary tests; noisy timing gates remain controlled measurements.
If the screen fails, diagnose and optimize the measured cause before closing the
gate. Do not drop inconvenient samples or quietly redefine the eligible workload.
Dependencies: 8. Scope: small per measured/optimized slice.

### Task 10 — Final validation and physical acceptance

Run formatting, all-target tests/check/strict Clippy, example builds, all existing
source scripts and the new append source checks. Recheck original consumers and
review connected code/ownership. Publish exact files, chosen design/alternatives,
discarded JS conventions, tests, measurements and remaining dependencies; update
README and this checklist without rewriting earlier acceptance history.

Acceptance: automated gates pass and the performance report identifies satisfied
and remaining limits. Physical live-feed checks record device/backend/scale and
actual observations for updates during mouse/axis/touch/kinetic/box-edit gestures,
half-size windows and high DPI. Hardware-unavailable gates remain unchecked.
Dependencies: 9. Scope: small documentation/verification slices.

## Main risks

| Risk | Required control |
| --- | --- |
| Series-local append mistaken for shared tail append | Classify against shared maximum; test late source time 4 while another ends at 5 |
| Failed builder leaves orphan shared point | Stage conversion/row/weight validation before mutation |
| Generic weight behavior loses context | Explicit native capability with correct generic fallback, or proven borrowed context |
| Membership becomes a new history bottleneck | Point deltas plus copy/visit counters; test shared fulfilled ownership |
| Base index advances on whitespace | Preserve last-fulfilled semantics and startup behavior |
| Cached peer geometry or labels stay stale | Track actual shared-coordinate/scale dependencies, not only changed series ID |
| Tick cache shortcut changes global selection | Source event-sequence checks; retain required global rebuilding |
| Warmups hide capacity or cold-cache latency | Measure cold/capacity events separately; distinguish amortized from worst-case claims |

## Follow-on milestone

After append automated/performance acceptance, derive a separate conflation
rendering roadmap for dense data. Trace actual reducers, custom-series contracts,
OHLC aggregation, metadata/hit mappings and source option restrictions first.
Keep that feature separate so append correctness and performance remain reviewable.
No calendar duration or new completion percentage is assigned here.
