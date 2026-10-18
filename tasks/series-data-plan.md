# Public series data operations

Date: 2026-10-18. Status: planned; implementation has not started.
Baseline: `71c8419`. Checklist: [series-data-todo.md](series-data-todo.md).
Feature-specific files follow the repository's existing planning convention and
preserve earlier plans and their unfinished acceptance records.

## Outcome and scope

Applications can remove recent value-bearing bars and inspect canonical series
data without rebuilding a series. Deliver public `pop`, full-data queries,
logical-index lookup and bars-in-logical-range information for built-in series;
extend the existing typed custom handles where the source supports equivalent
behavior. Calendar, strike/options and maturity/yield charts share the feature.

Integrate shared horizontal-axis membership, autoscale, conflation, hover/crosshair,
markers, price lines, animation and edit lifecycle. Series IDs, options, drawing
order, pane association and attachments survive popping to empty.

Direct visible-price-range setters, broader chart-option updates, new public
notifications, arbitrary horizontal behavior factories and export are separate
features. No new dependency, async mechanism or renderer architecture is expected.
Drawing-order mouse acceptance, historical timing exceptions and unavailable-device
checks remain open independently; this plan does not accept them.

## Preliminary source and Rust investigation

Codebase-memory searches, bidirectional traces and direct source reads established:

- Source `src/api/series-api.ts`: `pop` returns converted removed rows; `data`
  converts the current fulfilled plot rows; `dataByIndex` uses PlotList search;
  `barsInLogicalRange` uses strict-range rounding and nearest fulfilled endpoints.
- Source `src/api/chart-api.ts`: a nonempty pop response is applied through the
  existing chart update path; an empty pop does not send that update.
- Source `src/api/get-series-data-creator.ts`: results preserve original horizontal
  items, kind-specific colors, OHLC/value fields and custom metadata. Custom-series
  results restore original time over the stored custom payload.
- Source `src/model/data-layer.ts`: count clamps to available fulfilled rows;
  removal returns newest first and only removes an axis point after all series
  memberships disappear. Whitespace rows do not count as popped bars.
- Original tests: `tests/unittests/data-layer.spec.ts` covers single/multiple-series
  pops, oversize/empty/zero counts and index changes. Relevant graphics cases are
  `series/series-update-pop.js`, `api/series-data-by-index.js` and
  `logical-range/{bars-in-range,bars-in-gap}.js`.
- Rust `src/model/data_layer.rs` already has `pop_series_data` and
  `pop_custom_series_data`. The latter returns removed logical indices because
  typed payloads belong to custom series, not an erased data-layer store.
- Rust `src/model/plot_list.rs` already supports exact/nearest-left/nearest-right
  search. `CustomSeriesHandle::data_at` already returns an owned exact-index datum
  with restored original time. Preserve this API and its detached read semantics.
- `src/api/interaction.rs` already converts built-in plot rows for pointer events.
  Reuse/extract that conversion for queries/pop rather than maintain two mappings.

Coverage metadata was best effort. Upstream data-layer metadata had changed, so the
actual pop implementation was read directly rather than trusting graph line bounds.
Task 1 must complete source/consumer analysis and pin hashes before production edits.

## Behavioral contract to freeze

1. Queries read canonical fulfilled rows, not conflated drawing samples, and do not
   measure axes, prepare frames or change model state. Full-data results are oldest
   first; popped results are newest first. Neither operation returns whitespace as
   a plotted datum. Original calendar/numeric items and per-datum styles survive.
2. Zero count and empty data are no-ops. Oversized counts remove all available
   fulfilled rows. Rust uses `usize` rather than reproduce JS negative/fractional
   count inputs. Invalid/wrong-kind/wrong-chart handles fail before mutation.
3. A series emptied by pop stays registered with the same ID, pane, options and
   attached owners. Later updates/appends work against the resulting last-time
   state, including when trailing whitespace remains. Do not call remove_series.
4. Shared axis points survive while another series or whitespace owns them. Removing
   a unique point can reindex other built-in/custom series and annotations; apply
   the complete existing response, not only the selected series' rows.
5. Exact/nearest queries use the shared logical index, not the offset in a series'
   own vector. Freeze fractional and nonfinite input handling, missing data and
   whitespace gaps against source PlotList behavior before exposing signatures.
6. Range queries preserve fractional before/after counts, including negative
   values outside available history. A range entirely in a gap can report counts
   without time endpoints. Endpoints must be present together or absent together.
   Define absent, reversed and nonfinite range handling explicitly.
7. Custom pop captures typed removed payloads using their old indices before shared
   reindexing. Preserve readable typed handles, release RefCell borrows before any
   model/view callback, and apply surviving-row changes exactly once.
8. Pop must invalidate relevant conflation caches, first/last prices and animations.
   Determine marker/update-controller behavior from actual source; do not invent
   transient price-change markers for removal or silently discard manual markers.
9. Maturity support remains private and preserves the established retained scaffold
   domain. Pop visible series without removing or rebuilding support unnecessarily.
10. Real mutations use existing capture/edit cancellation if data remapping makes
    an active edit stale; invalid and empty/no-op calls preserve it. Freeze this
    native policy before implementation. Existing frozen frames stay drawable.

## Proposed Rust API and ownership

Names/signatures are provisional until Task 1 resolves edge cases:

```rust
pub struct BarsInfo<T> {
    pub bars_before: f64,
    pub bars_after: f64,
    pub data_range: Option<(T, T)>,
}

impl<B: ChartBehavior> Chart<B> {
    pub fn pop_data(&mut self, id: SeriesId, count: usize)
        -> Result<Vec<BuiltInSeriesDataItem<B::Item>>, ChartError>;
    pub fn series_data(&self, id: SeriesId)
        -> Result<Vec<BuiltInSeriesDataItem<B::Item>>, ChartError>;
    pub fn data_by_index(&self, id: SeriesId, index: TimePointIndex,
        direction: MismatchDirection)
        -> Result<Option<BuiltInSeriesDataItem<B::Item>>, ChartError>;
    pub fn bars_in_logical_range(&self, id: SeriesId, range: Option<LogicalRange>)
        -> Result<Option<BarsInfo<B::Item>>, ChartError>;
    // Typed custom removal: keep V::Data; do not erase it into built-in enums.
    pub fn pop_custom_data<V>(&mut self, handle: &CustomSeriesHandle<V>, count: usize)
        -> Result<Vec<V::Data>, ChartError>; // Existing V/behavior bounds apply.
}
```

Extend `CustomSeriesHandle<V>` with corresponding owned `data`, directional lookup
and range information while keeping existing `data_at(index)` unchanged. Query
placement must respect readable detached handles; all mutations validate attachment
against the receiving chart. YieldCurveChart explicitly forwards mutations, marks
its scaffold dirty only when required and rejects guessed private support IDs.

Preferred design: synchronous methods on the existing chart/typed handles, one
canonical row-to-public-data converter and existing DataLayer update responses.
Returned values are owned snapshots, independent of later chart changes. Full-data
queries allocate proportional to requested data; single-row lookup uses existing
search; pop must not be implemented as public data()+truncate()+set_data().

Alternative: expose borrowed row iterators or RefCell guards. This avoids copying
full query results, but leaks internal row types/borrow lifetimes and complicates
custom-handle reentrancy and subsequent mutation. Start with owned public data;
add streaming queries only if a demonstrated consumer needs them.

Ownership is unchanged: Chart owns DataLayer/model/views; typed custom models own
their payloads; custom handles retain existing model ownership and weak view links;
query/pop results own their returned data. No new Rc cycles, locks, callbacks or
manual destruction APIs are needed. Preserve source behavior, not JS object aliases.

## Ordered implementation tasks

### 1. Freeze source contract and oracle — medium, no dependencies

Inspect complete source API/conversion components and semantic dependencies; trace
DataLayer/model/coordinator, custom payload ownership, marker controllers and yield
support. Inspect real source tests and actual public consumers. Compare the two
Rust designs and freeze API/errors, whitespace, range and edit policies.

Build a focused real-source oracle using existing tooling. Record data/style/time
round trips, newest-first pop, zero/oversize/empty, trailing/interior whitespace,
shared times, subsequent updates, all search directions, fractional/gap ranges,
custom payloads and numeric/yield behavior. Pin source hashes, observed states and
pre-change compatibility captures; preserve source defects separately from native
policies. Read exact local Iced/Flowsurface only if new widget behavior is needed.

**Likely files:** new source comparison script/fixtures and
`docs/porting/series-data.md`. **Exit:** source results reproduce; no unresolved
behavior is silently assumed. Full source review precedes production edits.

### 2. Built-in reads — medium, depends on 1

Write failing tests for typed round trips and lookup/range edge cases. Extract the
existing pointer-event conversion into the smallest shared helper and implement
full-data/index/range queries. Keep pointer-event results unchanged. Use an Option
of paired endpoints and typed errors; do not add a parallel row store.

**Likely files:** new `src/api/series_data.rs`, module exports,
`src/api/interaction.rs`, new `tests/series_data.rs`.
**Exit:** six built-in kinds preserve original items/styles; reads agree with the
source oracle, do not prepare frames and work at dense and sparse zoom levels.

### 3. Built-in pop — medium, depends on 2

Start with failing public pop/update/shared-membership regressions. Reuse the
existing layer pop and apply its full response through ChartModel. Validate first,
leave no-op calls unchanged, preserve owners and coordinate actual mutation with
interaction revisions and retained-frame invalidation. Fix lower-layer defects
only if source-backed tests establish them; split fixes into focused increments.

**Likely files:** `src/api/series_data.rs`, `tests/series_data.rs`, existing
DataLayer/model files only when necessary. **Exit:** returned order, shared axis,
empty-but-live series and pop-then-update behavior match the contract.

### Checkpoint A

Focused queries/pop tests, existing pointer events and source comparisons pass;
review API/ownership changes and save a local checkpoint before custom mutations.

### 4. Typed custom data operations — medium, depends on 3

Add typed queries and pop, preserving existing exact-index reads and handle
semantics. Capture removed payloads before old-index mappings change, apply the
custom membership response to all survivors and prove no borrow crosses callbacks.
Test wrong-chart/removed handles, whitespace, shared built-in/custom times, custom
conflation and updates after emptying. No erase-to-Any query API is needed.

**Likely files:** `src/api/custom_series.rs`, `src/api/series_data.rs`, focused
custom-query/pop tests. **Exit:** typed data/time values survive; no view destruction
or unrelated payload copying during pop; failure preserves the previous state.

### 5. Numeric frontends and lifecycle — medium slices, depends on 4

Verify generic OptionsChart behavior and explicit yield forwards/support protection.
Exercise reordered/transferred series, multi-pane shared domains, autoscale modes,
crosshair/hit identity, manual/automatic markers, price lines, pulses and edits.
Prove other custom series reindex correctly when the popped series owns unique
times. Test pop-to-empty then update/reset, density transitions, and old frames
after pop and chart drop. Split frontend forwarding from integration test additions.

**Likely files:** `src/api/yield_curve.rs`, `src/api/yield_curve/custom_series.rs`,
focused tests and existing interaction suites. **Exit:** all three families retain
owners and show consistent data/axis/annotation behavior after removal.

### 6. Demo controls and capture fixtures — medium slices, depends on 5

Extend the existing generic pane demo with an opt-in data-operations mode rather
than a new host/editor. Include Pop 1/Pop several/Pop all, Append/Reset and query
output for selected built-in/custom data. Show removed order and range results;
provide sparse/shared-time data to make reindexing observable. Handle a removed
selection safely and preserve existing drawing-order/pane controls.

**Likely files:** `examples/pane_management.rs`, its host/capture modules and README.
**Exit:** example tests verify controls and data readouts in calendar/strike/maturity
modes, including emptying and later updates at short sizes.

### 7. Renderer and physical acceptance — medium slices, depends on 6

Before capture changes, freeze three families × two backends × three sizes
(800x500, 300x300, 300x90) × four DPRs (1, 1.25, 1.5, 2) × four states
(initial, partial-pop, empty, append-after-pop): **288 feature captures**.
Use the previous 96 unchanged compatibility images and assert actual backends.
Inspect data removal, labels, clipped geometry, recovery and immutable old frames.

Run software then wgpu demos for user mouse checks: pop and query both data types,
append/reset, live zoom/pan/price-axis gestures and box editing at regular, narrow
and earlier problem sizes; shrink/enlarge for recovery. Keep initial fixtures
deterministic, with live feed enabled only for native interaction checks.

**Likely files:** existing capture infrastructure, feature fixtures, report.
**Exit:** frozen captures/baselines pass and both available mouse runs are recorded;
unperformed checks remain open, including prior drawing-order acceptance.

### 8. Final verification and report — small, depends on 7

Run formatting, focused and all-target tests, doctests, all-target cargo check,
strict Clippy and diff hygiene. Run the new source oracle plus all existing source
comparison scripts without weakening expectations. Review the final diff and
recheck important source consumers. Record exact files, source/consumer contract,
discarded JS conventions, alternatives, ownership, evidence and remaining limits
in the final report; update plan/checklist/README. Save local commits; push only
on request. No timing gate is redefined by this milestone.

## Main risks and mitigations

| Risk | Mitigation |
|---|---|
| Treat whitespace as popped bars or return chronological order | Source oracle and explicit count/order assertions. |
| Custom payload lookup uses new indices | Capture typed removed rows before mutation; sparse multi-series tests. |
| Other series retain stale logical mappings | Apply complete layer response; query and hit survivors after unique-point deletion. |
| Stale conflation/controller caches | Dense pop/update tests, marker/pulse expiry and canonical query comparisons. |
| Queries duplicate or normalize away original styles/time | Share pointer conversion; round-trip all kinds and calendar input forms. |
| Yield support leaks or domain shrinks incorrectly | Explicit wrappers and retained-scaffold assertions. |

No user preference blocks planning. Task 1 resolves source/runtime edge cases;
implementation starts only after the source contract is documented.
