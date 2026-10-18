# Reusable public helpers (milestone D)

Baseline: `09520ec`. Status: source/design investigation; implementation in progress.

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

Pending implementation, source fixtures, focused behavior tests, external-consumer
demo, four domains/custom series, strict renderer captures and available mouse
acceptance. Preserve all earlier timing, formatting-GPU, series-order and untested
device exceptions. Locale/gamut and milestones E onward are outside D.
