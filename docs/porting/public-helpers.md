# Reusable public helpers (milestone D)

Baseline: `09520ec`. Status: D0–D2 implemented; D3 consumer passes; final validation and mouse acceptance pending.

## Source, consumers and contract

Investigated the complete `src/api/time-scale-api.ts` and `series-api.ts`, the
chart synthetic-position/clear methods and their model consumers. TimeScaleApi
converts items with the behavior, uses forward-nearest endpoints, returns original
items, resolves lazy range caches and queues viewport mutations. Reset restores
configured defaults; live navigation animates to configured right offset in 400ms.
SeriesApi converts prices using the series' visible first value and actual scale;
its options are snapshots, price-line enumeration preserves creation order, and
lastValueData distinguishes global and last-visible values. Synthetic crosshair
placement selects the series' pane, then that pane's default price scale, uses a
nearest horizontal index, retains ordinary magnet behavior and suppresses events.

Graph searches and bidirectional traces were used in both projects; several API
calls have sparse graph edges, so actual source usage was searched too. Important
consumers are the time-scale coverage sequence (query/navigation/reset), real-time
update tutorial (return to live), crosshair synchronization tutorial (no recursive
synthetic pointer events), tooltip/drawing/volume-profile plugins (actual series
price conversion), custom-series price-line tests and multi-pane crosshair tests.
These require fresh membership after transfer, optional empty results, owned
snapshots, and no user callbacks during chart mutation. Coverage checks reported no
recorded issues for investigated native/API files (best-effort, not completeness).
The source time-scale/series metadata is older than local edits, and coverage
test subtrees are excluded; the relevant current source and consumers were read
directly instead of relying on those graph entries.

## Design and ownership

Two viable designs: retained scale/series façade objects, or narrow checked owner
methods plus typed handle readbacks. Choose the latter: it matches this port,
avoids retained selectors becoming stale, adds no allocation on conversion, and
requires no public mutable model access. Every owner operation validates current
membership; IDs remain globally unique. Chart owns engine, adapters and view;
custom handles strongly retain typed model data and weakly reference the view.
Owned value snapshots survive removal. Series options contain Box callbacks and
are deliberately scoped rather than cloned; typed custom model reads survive
detachment. Scoped option/formatter callbacks borrow
only for the call, cannot return a borrowed reference, and require neither a Clone
view nor Clone options. No new Rc links, cycles, threads or async work are added.

Proposed API (names finalized with each slice):

```rust,ignore
Chart::visible_logical_range(&mut self) -> Option<LogicalRange>
Chart::visible_horizontal_range(&mut self) -> Option<HorizontalRange<B::Item>>
Chart::set_visible_horizontal_range(&mut self, HorizontalRange<B::Item>) -> Result<(), ChartError>
Chart::reset_time_scale(&mut self)
Chart::scroll_position(&self) -> f64
Chart::scroll_to_real_time(&mut self)
Chart::price_lines(&self, SeriesId) -> Result<Vec<PriceLineId>, ChartError>
Chart::with_series_options<R>(&self, SeriesId, impl FnOnce(&SeriesOptionsMap) -> R) -> Result<R, ChartError>
Chart::price_to_coordinate(&mut self, SeriesId, f64) -> Result<Option<f64>, ChartError>
Chart::coordinate_to_price(&mut self, SeriesId, f64) -> Result<Option<f64>, ChartError>
Chart::set_crosshair_at(&mut self, SeriesId, &B::Item, f64) -> Result<bool, ChartError>
```

Range queries materialize current engine caches without preparing a frame,
consuming pending commands or draining events. As in the source, queued navigation
settles at the next prepare. Invalid inputs are rejected before cancellation or
mutation. Empty scales return None (or false for unavailable placement), instead
of reproducing JavaScript null assertions. Domain inputs are converted through
ChartBehavior; original public items are retained in results. The yield frontend
forwards mutations explicitly, preserving its scaffold ownership. OptionsChart is
already the generic owner; custom application domains need no extra adapter.

Discarded TypeScript details: retained API wrapper identity, cloned wrappers for
price lines, manual destroy, callback-identity subscription APIs and unsafe null
assertions. Existing owned event queues and typed IDs preserve their useful
semantics. Readbacks do not keep views attached.

## Validation and remaining work

The API checkpoint is `e00f8c3`. Seven focused public tests and four external
consumer tests cover empty/invalid ranges, event queues, configured reset,
animation cancellation, original items, retained custom handles, ordered
price-line IDs, fresh pane/scale membership, all four price modes, missing items,
empty scales and silent synthetic placement. The local source/native probe
checks ranges, nearest indices, values, conversions, enumeration and events.
The four-domain consumer uses only public API and synchronizes two charts.
Final common gates, strict renderer captures and available mouse acceptance
remain pending. Preserve all earlier timing, formatting-GPU, series-order and untested
device exceptions. Locale/gamut and milestones E onward are outside D.


## F06 disposition

| Original finding | Implemented boundary / classification |
|---|---|
| Horizontal-item range setter | `set_visible_horizontal_range`: behavior conversion, checked ordering, source nearest endpoints; empty domain is inert |
| Current logical/horizontal queries | `visible_logical_range` / `visible_horizontal_range`: owned optional values; lazy materialization, no event drain |
| Reset/live owner forwarding | `reset_time_scale` / `scroll_to_real_time`: configured defaults and source 400ms live animation |
| Price-line enumeration | `price_lines`: creation-order owned IDs; existing checked option read/replacement/removal |
| Scale and pane membership | `series_price_scale` already delivered in B; `series_pane` and `series_type` now validate both built-in/custom selectors |
| Options/formatter readback | Native scoped adaptation: built-in owner callbacks; typed custom stored/view callbacks; no Clone bounds or callback ownership changes |
| Current/global value | `last_value_data`: owned optional projection through current scale, built-in and custom |
| Price conversions | Source visible first-value policy and actual owning scale, including overlays and all four modes |
| Domain synthetic crosshair | `set_crosshair_at` / clear: source pane-default scale, nearest item, ordinary magnet behavior, no pointer events; native empty-state result |
| Existing engine-composed horizontal helpers | Checked owner helpers now expose empty/error policy and original item results directly |

These classifications describe the tested operations, not a claim of exhaustive
parity for unrelated interfaces. Source option cloning is adapted to scoped reads
because Rust options own non-cloneable callbacks. `with_series_options` is for
built-ins; typed custom handles expose their actual option types. Retained model
reads remain valid after removal, while owner queries and view reads reject stale
selectors. Conversion queries use settled engine geometry and never implicitly
render or drain events. A synthetic placement with unavailable geometry returns
false without clearing an existing position; invalid input is an error before
mutation. Locale, color, metadata and pane-height gaps remain separate milestones.

## Files and validation procedure

Core: `src/api/{time_scale,series_queries,coordinates,custom_series,yield_curve,mod}.rs`,
`src/model/{chart_model,crosshair}.rs` and `src/model/series/projection.rs`.
Behavior tests: `tests/public_helpers.rs`. External consumer and renderer fixtures:
`examples/pane_management.rs`, `examples/pane_management/{host,helpers,helper_captures}.rs`.
Source observations: `scripts/check_public_helpers_reference.cjs` and pinned
`public-helpers-{source,states}.json`; captures: `scripts/capture_public_helpers.py`.
Inventory classifications are maintained individually in `scripts/audit_public_surface.cjs`.

For each renderer and all four domains, test the helper controls on blue/custom,
return-to-live after panning, both directions of linked crosshair movement and idle
counters; pane/scale transfer and removal/reset; live-feed zoom/pan, axis gestures,
box edits/cancellation/release outside at regular, narrow and earlier problem
sizes; short-height recovery; preview/export. Physical results must be explicitly
reported; synthetic DPR captures do not close physical high-DPI acceptance.


Review caught a source edge case before acceptance: the ordinary pixel path hides
out-of-plot origins, while synthetic source placement retains them and clamps the
applied horizontal index to visible bars. A private synthetic-origin flag now
preserves that distinction through viewport refreshes. The public pixel API keeps
its prior bounds policy. A failing public regression and source observations for
off-screen placement before/after a range change verify the correction; the new
renderer protocol includes an outside-price/item scene. No new public mutable
model boundary or callback ownership was introduced.
