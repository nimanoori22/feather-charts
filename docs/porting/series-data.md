# Public series data operations

Status: source contract frozen; implementation in progress, 2026-10-18.
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
Implementation, automated/capture/native verification and final report are pending.
Previous drawing-order mouse acceptance, timing and unavailable-device gates remain
open independently. Direct visible-price-range controls/new events/export are not
part of this milestone.
