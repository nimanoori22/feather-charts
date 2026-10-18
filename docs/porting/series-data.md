# Public series data operations

Status: complete, 2026-10-18. Automated, source and renderer verification passed;
available software/wgpu mouse acceptance passed in all three chart modes.
Plan: [series-data-plan.md](../../tasks/series-data-plan.md).

## Source and consumer contract

Investigated SeriesApi pop/data/dataByIndex/barsInLogicalRange, ChartApi.popData,
get-series-data-creator.ts, the complete DataLayer, PlotList search, strict visible
ranges and original data-layer/graphics query tests. Source API sends a pop response
only for nonempty removal; queries expose fulfilled canonical rows and original
horizontal items with style/custom data fields. PlotList lookup uses shared logical
indices with exact/nearest-left/nearest-right matching, including fractional indices.
Range endpoints expand outward with floor/ceil; before/after can be fractional or
negative. A gap may return counts without paired time endpoints.

Pop removes fulfilled rows only, newest first, clamps count to available bars and
leaves whitespace. Shared points survive other series/whitespace membership. Empty
series remain registered; later updates use the resulting last fulfilled time.
Up/down-marker wrapper only intercepts its own setData/update calls; ordinary pop
must not create a price-change marker or alter the plugin's private baseline.
Manual markers/price lines retain identity; their drawing follows remaining data.

The actual local source runs in isolated Chromium, without chart/model stubs, via
scripts/check_series_data_reference.cjs. Hashes and observations are frozen in
series-data-source.json and series-data-states.json. Six states exercise zero/shared/
unique/all/empty removal followed by update, queries in whitespace gaps and outside
history, six built-in style/original-time round trips, custom data and yield removal.
Graph metadata had changed for DataLayer/PlotList; relevant actual code was read
rather than trusting symbol offsets. Existing Rust layer pop, model/coordinator,
custom pop_indices/apply_indices, pointer-data conversion and typed ownership were
inspected. Full caller searches supplement missing interface edges.

## Native contract and API

Chart<B> exposes pop_data(id,count), series_data(id), data_by_index(id,index,direction)
and bars_in_logical_range(id,optional_range). Built-in results use existing
BuiltInSeriesDataItem<B::Item>. CustomSeriesHandle gains owned typed data/index/range
queries, preserving data_at and detached readable models. Chart.pop_custom_data
validates an attached typed handle before mutating the layer. Yield forwards
mutations explicitly and protects private support; retained scaffold policy remains.
BarsInfo<T> owns fractional bars_before/bars_after and an Option<(T,T)> endpoint pair.

Rust usize counts omit JS negative/fractional count inputs. Unknown/detached/foreign
mutations use existing errors; built-in-only calls reject custom IDs. New directional
queries reject nonfinite indices; range queries reject nonfinite/reversed ranges
with InvalidDataQuery. Existing exact custom data_at behavior stays unchanged.
Absent ranges/empty data return None. Zero/empty pops preserve active state.
Actual pops use existing interaction cancellation and revisions, preserving owners
and old frames. Queries never prepare geometry or run custom view callbacks.

## Design and ownership

Chosen: existing synchronous Chart methods and typed custom handles, one shared
row-to-public-data conversion extracted from pointer events, and existing DataLayer
response delivery. Chart owns layer/model/views; handles retain their model and weak
view reference; results own independent data. Custom pop removes old-index payloads
before delivering a full response so surviving adapters receive new indices once.
No RefCell borrow crosses model callbacks; no new owner, cycle, dependency or thread.

Alternative: borrowed row iterators/guards avoid full-query copies but expose internal
rows and borrowing lifetimes, complicating mutation and typed handle semantics.
Owned results fit the source read API and existing event-data policy. Full data is
O(n), lookup/range endpoints O(log n); pop reuses layer mutation instead of rebuilding
history through public data/set_data. Unrelated custom payloads are not cloned.

## Baseline and pending verification

Baseline source hashes verified against target/series-order/final/protocol.json;
96 prior compatibility images copied to target/series-data/before. The new 288-image
matrix was frozen before production edits in target/series-data/baseline.json.
The first complete automated/capture runs passed. A final pass after the bounded
bulk-pop improvement is recorded below; native mouse acceptance is pending.
Previous drawing-order mouse acceptance, timing and unavailable-device gates remain
open independently. Direct visible-price-range controls/new events/export are not
part of this milestone.


## Source behavior retained and JS patterns discarded

Mutation flows remain synchronous. Existing plot/data-layer infrastructure owns
canonical membership and custom payloads; no JS object aliasing, nullable retained
SeriesApi objects or hand-managed destroy methods were introduced. Read results
are owned and time endpoints are paired in one Option. Built-in IDs become invalid
after removal; typed custom handles retain their already-established readable data
semantics. Queries do not create new view ownership or expose borrow guards.
Public metadata remains the existing `()` contract; this milestone does not add
arbitrary JSON metadata. Typed custom payload fields remain application-defined.

The oracle compares six complete native/source pop/query states. Additional browser
observations for all built-in styles, original calendar items, custom payloads and
yield behavior are frozen separately and covered by typed native behavioral tests.
Source imports/interface calls were traced through graph and direct search; the
existing up/down wrapper was read to verify ordinary pop has no plugin interception.

## Exact files changed

Production:
- `src/api/series_data.rs`: public BarsInfo, owned built-in queries/pop, shared range/search/conversion helpers.
- `src/api/mod.rs`: module registration and BarsInfo export.
- `src/api/chart.rs`: InvalidDataQuery error variant.
- `src/api/interaction.rs`: pointer events reuse the same row conversion.
- `src/api/custom_series.rs`: typed owned queries and attached-handle pop.
- `src/api/yield_curve.rs` and `src/api/yield_curve/custom_series.rs`: explicit protected pop forwarding.
- `src/model/series.rs`: tail-first existing custom-row removal, preserving input result order and cache invalidation.

Tests and examples:
- `tests/series_data.rs`: seven public data/query/lifecycle tests.
- `tests/pane_management_interaction.rs`: actual pop cancellation, no-op/read preservation, marker expiry and kinetic coverage.
- `tests/primitive_editing.rs`: captured edit cancellation/stale-token coverage includes pop.
- `examples/pane_management.rs`: opt-in --series-data controls and current-last-bar live feed.
- `examples/pane_management/data.rs`: shared three-family setup, readouts, pop/append and mode tests.
- `examples/pane_management/host.rs`: existing host forwards new data operations.
- `examples/pane_management/captures.rs`: partial/empty/append snapshots and retained-frame checks.
- `scripts/capture_series_order.py`: shared runner preserves default ordering behavior and accepts the data matrix.
- `scripts/capture_series_data.py`: small wrapper reusing that runner.
- `scripts/check_series_data_reference.cjs`, source/state JSON, README and this report/plan/checklist retain the contract and evidence.

## Targeted bulk-pop investigation

Final review found CustomSeries.pop_indices scanning the entire prefix for each
newest-first removal. Caller/coverage checks and direct reads confirmed sorted,
unique canonical rows and preserved cache semantics. The production change uses
`rposition` in place of `position`: normal public tail pops now start searching at
the matching end, without a new fast-path abstraction or payload copies.

Same diagnostic, 16,384 typed custom bars, debug build:

| Traversal | Three elapsed samples |
|---|---|
| From the start | 788.480, 772.661, 760.490 ms |
| From the tail | 15.050, 13.674, 13.524 ms |

Evidence: `target/series-data/bulk-before-repeats.log` and
`bulk-after-repeats.log`. The test also asserts newest-first values, zero payload
copies, empty surviving data and retained owner registration. It prints timing
without imposing a machine-sensitive threshold. This is a bounded mutation result,
not acceptance of any earlier frame/update timing exception.

## Automated verification

Final results: **730 tests passed in 71 suites**, zero failed or ignored;
**2 compile-fail doctests passed**. Strict all-target Clippy, all-target cargo check,
formatting and diff hygiene pass. Evidence:
- `target/series-data/all-targets-final-accepted.log`
- `target/series-data/doctests-accepted.log`
- `target/series-data/clippy-final-accepted.log`
- `target/series-data/check-accepted.log`

The first full run after query/test setup corrections passed 729 tests; the final
bulk-pop ownership test adds one. Initial failing evidence is retained: red.log and
custom-red.log show absent APIs. Initial dense-test setup used the multi-pane-only
frame accessor on a one-pane chart; it was corrected to retain the existing single
frame, with actual immutable raster checks in the capture suite. No engine defect
was claimed for that test setup issue, and no threshold or assertion was disabled.

Seven behavioral tests cover original times/styles for all six built-ins; canonical
queries under conflation; zero/empty/oversized/newest-first removal; direction and
gap/fractional-range semantics; invalid queries; subsequent latest updates; typed
custom removal/foreign handles/detached reads; zero-copy survivor reindexing; yield
private support and retained domain; and bulk custom removal. Empty built-in/custom charts
prepare at short/fractional sizes and recover at regular size after pop.
Existing Canvas gesture, kinetic, primitive edit, marker expiry and pointer suites
verify integration. Nine generic demo tests cover all three chart families in normal,
ordering and data modes, including short-height recovery, pop-to-empty and append.

## Capture and source verification

First complete run passed **288 feature images and 96 exact baseline comparisons**.
Final-build replay also passed **288 feature images and 96 exact comparisons**:
`target/series-data/accepted/{protocol,results}.json` and `captures-accepted.log`.
Original results remain in `final/` and `captures.log`.
The frozen sizes/DPR/backend/scenes are unchanged. Retained frames draw identically
after pop, pane changes, transfer, removal and chart drop.

Both the initial and final-build sets of **32 source comparisons passed**.
The final replay is `target/series-data/oracles-accepted/results.json`; original logs remain in
`oracles-final/`. Existing expectations are unchanged.

Software calendar partial-pop at 800x500, strike empty selected data at 300x300 and
maturity append-after-pop at 300x90 were visually inspected. Shared peers and private
support retain the expected axes; the emptied selected series and its box disappear.
A final-build wgpu strike append-after-pop frame at DPR 1.5 was also visually
inspected and passed. Native mouse results passed as recorded below.

## Native mouse acceptance

| Backend | Calendar / strike / maturity | Result |
|---|---|---|
| tiny-skia | Pop/queries/append/reset, live navigation, axes, box edits at three sizes and short-height recovery | User reported all checks passed in all three modes; all windows closed normally (exit 0) |
| wgpu | Same checks | User reported all checks passed in all three modes; all windows closed normally (exit 0) |

Both user confirmations were received on 2026-10-18. Runs used the normal
`pane_management --series-data --live-feed` demo, with `--options` for strike
and `--yield-curve` for maturity, and explicit `ICED_BACKEND=tiny-skia` or
`ICED_BACKEND=wgpu`. Checks included both blue built-in and typed custom selections,
Pop 1/3/all, newest-first removed values, Append/Inspect/Reset, remaining empty until
Append/Reset, and pop after pane transfer/order changes. Live-feed wheel zoom,
diagonal dragging, price-axis wheel/drag and box move/resize/Escape passed at regular,
narrow and the earlier problem size; short-height shrink/enlarge recovery passed.
All six demo processes exited 0. Logs are
`target/series-data/mouse-{calendar,price,yield}-{tiny-skia,wgpu}.log`.

Do not infer physical acceptance from deterministic captures. Previous drawing-order
mouse acceptance retains its own pending record; this milestone does not silently
close it. Touchpad/touchscreen/high-DPI physical and historical timing gates also
retain their previous status.
