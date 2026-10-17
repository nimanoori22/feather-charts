# Custom series on numeric horizontal axes

Status: in development. Baseline 2a32573. Plans:
[numeric-custom-series-plan.md](../../tasks/numeric-custom-series-plan.md) and
[checklist](../../tasks/numeric-custom-series-todo.md).

## Source and real consumers

Investigated complete custom view/renderer contracts, custom definition construction,
ChartApi.addCustomSeries -> _addSeriesImpl, inherited YieldChartApi construction,
custom pane preparation/hits, DataLayer custom writes and DataConflater context
creation. Numeric upstream type tests require original numeric items, typed custom
options and rejection of calendar or built-in data mismatches. Exact source revision
and working-tree hashes are in numeric-custom-series-source.json. Graph coverage
was checked; changed upstream icustom-series.ts was read directly. No upstream
working-tree edits are made.

Custom bars use logical indices; original data/events preserve prices or maturities.
Value builders drive autoscale/current price; callbacks receive current visible data,
options and optional price conversion. Custom payload and renderer hit data remain
separate. Existing synchronous ownership and mixed-series ordering are retained.

Yield addCustomSeries calls _addSeriesImpl directly, bypassing overridden addSeries:
custom defaults keep last-value/price-line visibility true, unlike yield Line/Area.
Chart-wide yield localization still takes precedence over series price format.
Custom writes must settle the private whitespace scaffold and preserve source extent
semantics. Native finite keys and the existing scaffold bound remain deliberate
validation policies applied before canonical mutation.

## Design and ownership

Keep CustomSeriesHandle<V> and infer item/internal types from V. An alternative
CustomSeriesHandle<V,B=TimeBehavior> was rejected because it repeats types already
fixed by the view and complicates caller inference. The public methods match V to
Chart<B>; pointer accessors match V::Item to PointerEvent<T>. Private cache rows use
the same view-associated types, cloning internal items instead of requiring Copy.
The existing erased UI adapter, model and widget are reused without a second engine.

Chart/model and pane adapter strongly own the model; adapter strongly owns OwnedView.
External handles own model data but hold a weak view. Frames own frozen renderers
and hit geometry. No new owner back-reference/cycle is introduced. Model borrows
are dropped before view callbacks; destroy remains a resource hook invoked once by
OwnedView::drop. Rust ownership replaces shared JS options/lifetime conventions.

## Contract and compatibility checkpoints

The new actual-source runner executes 18 numeric custom DataLayer states (nine per
family), real custom definition/API methods and inherited yield defaults. Its base
chart observation boundary is a narrow stub, not browser DOM integration. Source
state fixtures preserve union membership, whitespace, historical/append writes,
shortening, peer removal and clear. Existing non-associative reducer comparisons
remain part of the unchanged conflation oracle.

Pre-change evidence: target/numeric-custom-series/before, 96 images: 24 each calendar
built-ins, calendar custom (normal/hover/narrow at four DPR on two backends), price
built-ins and yield built-ins. Native source/binary/fixture hashes are retained.
Feature matrix is frozen at 240 images: two numeric modes, two backends, three sizes,
four DPR and five scenes (sparse, mixed, updates, dense, empty).
Work limits retain the existing 100,000-row visible-payload bound (170 copies) and
small-update bound (four), plus bounded precompute/reducer counters.

Behavioral RED: price Chart initially had no custom add/set operations. Generalizing
the API exposed an empty-data regression in calendar preflight (Time(EmptyData));
empty replacement must skip converter selection, as the original DataLayer does.
That was fixed without weakening the existing calendar tests.

## Validation and remaining work

Initial numeric price rendering and all nine calendar custom integration tests pass.
Yield forwarding, numeric event/error/lifecycle/work checks, final captures and
physical acceptance remain pending. Historical timing acceptance and unavailable
touchpad/touchscreen/physical high-DPI checks remain separately open.
